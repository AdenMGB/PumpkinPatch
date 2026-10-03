use crate::error::{Error, Result};
use crate::instance_config::{rcon_password_for_server, rcon_port_for_game_port};
use crate::network::NetworkOrchestrator;
use crate::port_wait::wait_for_tcp_port;
use crate::rcon::send_rcon_command;
use crate::models::{NetworkRecord, NetworkRuntimeStatus, ServerRecord};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::{mpsc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub network_id: Uuid,
    pub source: String,
    pub line: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessExitEvent {
    pub network_id: Uuid,
    pub source: String,
    pub server_id: Option<Uuid>,
    pub exit_code: Option<i32>,
}

struct ManagedPumpkin {
    child: Arc<Mutex<Child>>,
    server_id: Uuid,
    name: String,
    role: String,
}

struct ManagedHub {
    child: Arc<Mutex<Child>>,
    stdin: Option<ChildStdin>,
}

pub struct RunningNetwork {
    pub network_id: Uuid,
    hub: ManagedHub,
    servers: HashMap<Uuid, ManagedPumpkin>,
    lobby_ids: Vec<Uuid>,
    backend_ids: Vec<Uuid>,
}

pub struct ProcessSupervisor {
    running: HashMap<Uuid, RunningNetwork>,
    statuses: HashMap<Uuid, NetworkRuntimeStatus>,
}

impl ProcessSupervisor {
    pub fn new() -> Self {
        Self {
            running: HashMap::new(),
            statuses: HashMap::new(),
        }
    }

    pub fn status(&self, network_id: Uuid) -> NetworkRuntimeStatus {
        self.statuses
            .get(&network_id)
            .copied()
            .unwrap_or(NetworkRuntimeStatus::Stopped)
    }

    pub fn set_status(&mut self, network_id: Uuid, status: NetworkRuntimeStatus) {
        self.statuses.insert(network_id, status);
    }

    pub fn is_running(&self, network_id: Uuid) -> bool {
        self.running.contains_key(&network_id)
    }

    pub async fn start_network(
        &mut self,
        network: &NetworkRecord,
        servers: &[ServerRecord],
        pumpkin_binary: PathBuf,
        velocity_jar: PathBuf,
        java_path: &str,
        bind_host: &str,
        orchestrator: &NetworkOrchestrator,
        log_tx: Option<mpsc::UnboundedSender<LogLine>>,
        exit_tx: Option<mpsc::UnboundedSender<ProcessExitEvent>>,
    ) -> Result<()> {
        if self.running.contains_key(&network.id) {
            return Err(Error::InvalidState("network already running".into()));
        }
        self.statuses.insert(network.id, NetworkRuntimeStatus::Starting);

        let hub_path = PathBuf::from(&network.data_path).join("hub");
        let mut server_children = HashMap::new();

        let backends: Vec<_> = servers
            .iter()
            .filter(|s| s.role == "backend")
            .collect();
        let lobbies: Vec<_> = servers.iter().filter(|s| s.role == "lobby").collect();
        let backend_ids: Vec<Uuid> = backends.iter().map(|s| s.id).collect();
        let lobby_ids: Vec<Uuid> = lobbies.iter().map(|s| s.id).collect();

        orchestrator.sync_network_instances(network, servers, bind_host)?;

        let lobby = lobbies
            .first()
            .ok_or_else(|| Error::InvalidState("network has no lobby server".into()))?;

        let lobby_child = spawn_pumpkin(
            &pumpkin_binary,
            &PathBuf::from(&lobby.data_path),
            &lobby.name,
            lobby.id,
            "lobby",
            network.id,
            log_tx.clone(),
            exit_tx.clone(),
        )?;
        server_children.insert(lobby.id, lobby_child);

        wait_for_tcp_port("127.0.0.1", lobby.game_port, Duration::from_secs(180))
            .await
            .map_err(|e| {
                Error::Other(format!(
                    "lobby did not open on port {}: {e}. Check console logs for pumpkin:Lobby errors.",
                    lobby.game_port
                ))
            })?;

        for server in backends {
            let child = spawn_pumpkin(
                &pumpkin_binary,
                &PathBuf::from(&server.data_path),
                &server.name,
                server.id,
                "backend",
                network.id,
                log_tx.clone(),
                exit_tx.clone(),
            )?;
            server_children.insert(server.id, child);
            wait_for_tcp_port("127.0.0.1", server.game_port, Duration::from_secs(180))
                .await
                .map_err(|e| {
                    Error::Other(format!(
                        "backend \"{}\" did not open on port {}: {e}. Check pumpkin:{} logs.",
                        server.name,
                        server.game_port,
                        server.name
                    ))
                })?;
        }

        for extra in lobbies.iter().skip(1) {
            let child = spawn_pumpkin(
                &pumpkin_binary,
                &PathBuf::from(&extra.data_path),
                &extra.name,
                extra.id,
                "lobby",
                network.id,
                log_tx.clone(),
                exit_tx.clone(),
            )?;
            server_children.insert(extra.id, child);
        }

        let hub = spawn_velocity(
            java_path,
            &velocity_jar,
            &hub_path,
            network.id,
            log_tx.clone(),
            exit_tx.clone(),
        )?;

        self.running.insert(
            network.id,
            RunningNetwork {
                network_id: network.id,
                hub,
                servers: server_children,
                lobby_ids,
                backend_ids,
            },
        );
        self.statuses
            .insert(network.id, NetworkRuntimeStatus::Running);
        Ok(())
    }

    pub async fn stop_network(
        &mut self,
        network_id: Uuid,
        network: &NetworkRecord,
        servers: &[ServerRecord],
    ) -> Result<()> {
        let mut running = self
            .running
            .remove(&network_id)
            .ok_or_else(|| Error::InvalidState("network not running".into()))?;
        self.statuses
            .insert(network_id, NetworkRuntimeStatus::Stopping);

        if let Some(mut stdin) = running.hub.stdin.take() {
            let _ = stdin.write_all(b"shutdown\n").await;
            let _ = stdin.flush().await;
            tokio::time::sleep(Duration::from_secs(3)).await;
        }

        for server_id in &running.backend_ids {
            if let Some(server) = servers.iter().find(|s| s.id == *server_id) {
                let port = rcon_port_for_game_port(server.game_port);
                let password =
                    rcon_password_for_server(&network.forwarding_secret, server.game_port);
                let _ = send_rcon_command("127.0.0.1", port, &password, "stop").await;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;

        for server_id in &running.lobby_ids {
            if let Some(server) = servers.iter().find(|s| s.id == *server_id) {
                let port = rcon_port_for_game_port(server.game_port);
                let password =
                    rcon_password_for_server(&network.forwarding_secret, server.game_port);
                let _ = send_rcon_command("127.0.0.1", port, &password, "stop").await;
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;

        for (_, managed) in running.servers {
            let mut guard = managed.child.lock().await;
            let _ = guard.kill().await;
        }
        {
            let mut guard = running.hub.child.lock().await;
            let _ = guard.kill().await;
        }

        self.statuses
            .insert(network_id, NetworkRuntimeStatus::Stopped);
        Ok(())
    }

    pub fn mark_error(&mut self, network_id: Uuid) {
        self.statuses.insert(network_id, NetworkRuntimeStatus::Error);
        self.running.remove(&network_id);
    }

    /// `target` is `"hub"` for Velocity, or a Pumpkin server UUID string.
    pub async fn send_console_command(
        &mut self,
        network_id: Uuid,
        network: &NetworkRecord,
        servers: &[ServerRecord],
        target: &str,
        command: &str,
    ) -> Result<String> {
        let running = self
            .running
            .get_mut(&network_id)
            .ok_or_else(|| Error::InvalidState("network not running".into()))?;
        let command = command.trim();
        if command.is_empty() {
            return Err(Error::InvalidState("command is empty".into()));
        }

        if target == "hub" || target.eq_ignore_ascii_case("velocity") {
            let stdin = running
                .hub
                .stdin
                .as_mut()
                .ok_or_else(|| Error::Other("Velocity console is not available".into()))?;
            stdin
                .write_all(format!("{command}\n").as_bytes())
                .await
                .map_err(|e| Error::Other(format!("failed to write to Velocity console: {e}")))?;
            stdin
                .flush()
                .await
                .map_err(|e| Error::Other(format!("failed to flush Velocity console: {e}")))?;
            return Ok("Sent to Velocity console (output appears in logs).".into());
        }

        let server_id = Uuid::parse_str(target)
            .map_err(|_| Error::InvalidState("invalid console target".into()))?;
        let server = servers
            .iter()
            .find(|s| s.id == server_id)
            .ok_or_else(|| Error::InvalidState("server not found".into()))?;

        if !running.servers.contains_key(&server_id) {
            return Err(Error::InvalidState("server process is not running".into()));
        }

        let port = rcon_port_for_game_port(server.game_port);
        let password = rcon_password_for_server(&network.forwarding_secret, server.game_port);
        let output = send_rcon_command("127.0.0.1", port, &password, command).await?;
        Ok(if output.is_empty() {
            "Command executed (no output).".into()
        } else {
            output
        })
    }
}

fn spawn_pumpkin(
    binary: &PathBuf,
    cwd: &PathBuf,
    name: &str,
    server_id: Uuid,
    role: &str,
    network_id: Uuid,
    log_tx: Option<mpsc::UnboundedSender<LogLine>>,
    exit_tx: Option<mpsc::UnboundedSender<ProcessExitEvent>>,
) -> Result<ManagedPumpkin> {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    pipe_logs(
        child.stdout.take(),
        network_id,
        format!("pumpkin:{name}"),
        log_tx.clone(),
    );
    pipe_logs(
        child.stderr.take(),
        network_id,
        format!("pumpkin:{name}:err"),
        log_tx,
    );
    let child = Arc::new(Mutex::new(child));
    spawn_exit_watcher(
        child.clone(),
        network_id,
        format!("pumpkin:{name}"),
        Some(server_id),
        exit_tx,
    );
    Ok(ManagedPumpkin {
        child,
        server_id,
        name: name.into(),
        role: role.into(),
    })
}

fn spawn_velocity(
    java_path: &str,
    jar: &PathBuf,
    cwd: &PathBuf,
    network_id: Uuid,
    log_tx: Option<mpsc::UnboundedSender<LogLine>>,
    exit_tx: Option<mpsc::UnboundedSender<ProcessExitEvent>>,
) -> Result<ManagedHub> {
    let mut cmd = Command::new(java_path);
    cmd.arg("-jar")
        .arg(jar)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let stdin = child.stdin.take();
    pipe_logs(
        child.stdout.take(),
        network_id,
        "velocity".into(),
        log_tx.clone(),
    );
    pipe_logs(
        child.stderr.take(),
        network_id,
        "velocity:err".into(),
        log_tx,
    );
    let child = Arc::new(Mutex::new(child));
    spawn_exit_watcher(
        child.clone(),
        network_id,
        "velocity".into(),
        None,
        exit_tx,
    );
    Ok(ManagedHub { child, stdin })
}

fn spawn_exit_watcher(
    child: Arc<Mutex<Child>>,
    network_id: Uuid,
    source: String,
    server_id: Option<Uuid>,
    exit_tx: Option<mpsc::UnboundedSender<ProcessExitEvent>>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(400)).await;
            let mut guard = child.lock().await;
            match guard.try_wait() {
                Ok(Some(status)) => {
                    let exit_code = status.code();
                    drop(guard);
                    if let Some(tx) = exit_tx {
                        let _ = tx.send(ProcessExitEvent {
                            network_id,
                            source: source.clone(),
                            server_id,
                            exit_code,
                        });
                    }
                    break;
                }
                Ok(None) => continue,
                Err(_) => break,
            }
        }
    });
}

fn pipe_logs<R: AsyncRead + Unpin + Send + 'static>(
    stream: Option<R>,
    network_id: Uuid,
    source: String,
    log_tx: Option<mpsc::UnboundedSender<LogLine>>,
) {
    if let (Some(out), Some(tx)) = (stream, log_tx) {
        tokio::spawn(async move {
            let reader = BufReader::new(out);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = tx.send(LogLine {
                    network_id,
                    source: source.clone(),
                    line,
                });
            }
        });
    }
}
