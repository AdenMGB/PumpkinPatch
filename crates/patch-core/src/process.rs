use crate::error::{Error, Result};
use crate::hub_settings::read_hub_settings;
use crate::instance_config::{rcon_password_for_server, rcon_port_for_game_port};
use crate::network::{NetworkOrchestrator, VELOCITY_BACKEND_HOST};
use crate::port_wait::wait_for_tcp_port;
use crate::rcon::send_rcon_command;
use crate::models::{NetworkRecord, NetworkRuntimeStatus, ServerRecord};
use gourd_proxy::{parse_pool_state, GourdConfig, GourdProxy, GourdRouter};
use vineyard_protocol::RouteWireSpec;
use serde::Serialize;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::timeout;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub network_id: Uuid,
    pub source: String,
    pub line: String,
}

struct ManagedPumpkin {
    child: Child,
    stdin: Option<ChildStdin>,
}

struct ManagedHub {
    velocity_child: Option<Child>,
    gourd_task: Option<JoinHandle<()>>,
    stdin: Option<ChildStdin>,
}

pub struct RunningNetwork {
    pub network_id: Uuid,
    hub: ManagedHub,
    gourd_router: Option<GourdRouter>,
    servers: HashMap<Uuid, ManagedPumpkin>,
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

    pub fn is_running(&self, network_id: Uuid) -> bool {
        self.running.contains_key(&network_id)
    }

    /// Live-update Gourd virtual-host routing when topology hot-wire runs.
    pub async fn apply_gourd_route(
        &self,
        network_id: Uuid,
        spec: &RouteWireSpec,
        servers: &[ServerRecord],
    ) -> Result<()> {
        let running = self
            .running
            .get(&network_id)
            .ok_or_else(|| Error::InvalidState("network not running".into()))?;
        let router = running
            .gourd_router
            .as_ref()
            .ok_or_else(|| Error::InvalidState("network hub is not Gourd".into()))?;
        let backend = servers
            .iter()
            .find(|s| s.id == spec.backend_server_id)
            .ok_or_else(|| Error::InvalidState("backend server not found".into()))?;
        let addr: SocketAddr = format!("{}:{}", VELOCITY_BACKEND_HOST, backend.game_port)
            .parse()
            .map_err(|e| Error::Other(format!("invalid backend address: {e}")))?;
        router
            .register_route_with_state(
                &spec.server_alias,
                addr,
                parse_pool_state(spec.pool_state.as_deref()),
            )
            .await;
        for host in &spec.forced_hosts {
            router.register_forced_host(host, &spec.server_alias).await;
        }
        Ok(())
    }

    pub async fn start_network(
        &mut self,
        network: &NetworkRecord,
        servers: &[ServerRecord],
        pumpkin_binaries: HashMap<String, PathBuf>,
        velocity_jar: PathBuf,
        java_path: &str,
        bind_host: &str,
        gourd_enabled: bool,
        orchestrator: &NetworkOrchestrator,
        log_tx: Option<mpsc::UnboundedSender<LogLine>>,
    ) -> Result<()> {
        if self.running.contains_key(&network.id) {
            return Err(Error::InvalidState("network already running".into()));
        }
        self.statuses.insert(network.id, NetworkRuntimeStatus::Starting);

        let hub_path = PathBuf::from(&network.data_path).join("hub");
        let mut spawned_children = HashMap::new();

        let backends: Vec<_> = servers
            .iter()
            .filter(|s| s.role == "backend")
            .collect();
        let lobbies: Vec<_> = servers.iter().filter(|s| s.role == "lobby").collect();

        let mut hub_child: Option<ManagedHub> = None;
        let mut gourd_router: Option<GourdRouter> = None;

        let start_result: Result<()> = async {
            orchestrator.sync_network_instances(network, servers, bind_host)?;

            let lobby = lobbies
                .first()
                .ok_or_else(|| Error::InvalidState("network has no lobby server".into()))?;

            let lobby_binary = pumpkin_binary_for_server(&pumpkin_binaries, lobby)?;
            let lobby_child = spawn_pumpkin(
                &lobby_binary,
                &PathBuf::from(&lobby.data_path),
                &lobby.name,
                network.id,
                log_tx.clone(),
            )?;
            spawned_children.insert(lobby.id, lobby_child);

            wait_for_tcp_port("127.0.0.1", lobby.game_port, Duration::from_secs(180))
                .await
                .map_err(|e| {
                    Error::Other(format!(
                        "lobby did not open on port {}: {e}. Check console logs for pumpkin:Lobby errors.",
                        lobby.game_port
                    ))
                })?;

            for server in backends {
                let binary = pumpkin_binary_for_server(&pumpkin_binaries, server)?;
                let child = spawn_pumpkin(
                    &binary,
                    &PathBuf::from(&server.data_path),
                    &server.name,
                    network.id,
                    log_tx.clone(),
                )?;
                spawned_children.insert(server.id, child);
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
                let binary = pumpkin_binary_for_server(&pumpkin_binaries, extra)?;
                let child = spawn_pumpkin(
                    &binary,
                    &PathBuf::from(&extra.data_path),
                    &extra.name,
                    network.id,
                    log_tx.clone(),
                )?;
                spawned_children.insert(extra.id, child);
            }

            if gourd_enabled {
                let (hub, router) = spawn_gourd(
                    network,
                    servers,
                    bind_host,
                    &hub_path,
                    log_tx.clone(),
                )
                .await?;
                gourd_router = Some(router);
                hub_child = Some(hub);
            } else {
                hub_child = Some(spawn_velocity(
                    java_path,
                    &velocity_jar,
                    &hub_path,
                    network.id,
                    log_tx.clone(),
                )?);
            }

            Ok(())
        }
        .await;

        if let Err(err) = start_result {
            for (_, managed) in spawned_children.iter_mut() {
                let _ = managed.child.start_kill();
                let _ = managed.child.wait().await;
            }
            if let Some(mut hub) = hub_child {
                stop_managed_hub(&mut hub).await;
            }
            self.statuses.insert(network.id, NetworkRuntimeStatus::Error);
            return Err(err);
        }

        let hub = hub_child.ok_or_else(|| Error::Other("velocity hub missing after start".into()))?;

        self.running.insert(
            network.id,
            RunningNetwork {
                network_id: network.id,
                hub,
                gourd_router,
                servers: spawned_children,
            },
        );
        self.statuses.insert(network.id, NetworkRuntimeStatus::Running);
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
        self.statuses.insert(network_id, NetworkRuntimeStatus::Stopping);

        for server in servers {
            let managed = running.servers.get_mut(&server.id);
            if managed.is_none() {
                continue;
            }
            let managed = managed.unwrap();
            let port = rcon_port_for_game_port(server.game_port);
            let password = rcon_password_for_server(&network.forwarding_secret, server.game_port);
            if send_rcon_command("127.0.0.1", port, &password, "stop")
                .await
                .is_err()
            {
                let _ = pumpkin_console_write(managed.stdin.as_mut(), "stop").await;
            }
        }

        let RunningNetwork {
            mut hub,
            servers: mut pumpkin_children,
            ..
        } = running;
        if let Some(mut stdin) = hub.stdin.as_mut() {
            let _ = stdin.write_all(b"shutdown\n").await;
            let _ = stdin.flush().await;
        }

        let _ = timeout(Duration::from_secs(25), async {
            for (_, managed) in pumpkin_children.iter_mut() {
                let _ = managed.child.wait().await;
            }
            if let Some(child) = hub.velocity_child.as_mut() {
                let _ = child.wait().await;
            }
        })
        .await;

        self.force_kill_children(&mut pumpkin_children, &mut hub)
            .await;
        self.statuses.insert(network_id, NetworkRuntimeStatus::Stopped);
        Ok(())
    }

    async fn force_kill_children(
        &mut self,
        servers: &mut HashMap<Uuid, ManagedPumpkin>,
        hub: &mut ManagedHub,
    ) {
        for (_, managed) in servers.iter_mut() {
            let _ = managed.child.start_kill();
            let _ = managed.child.wait().await;
        }
        stop_managed_hub(hub).await;
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
            if running.hub.gourd_task.is_some() {
                return Err(Error::Other(
                    "Gourd proxy has no interactive console; edit routes via topology hot-wire."
                        .into(),
                ));
            }
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
            return Ok(
                "Sent to Velocity (graceful shutdown: `shutdown`; reload config: `velocity reload`)."
                    .into(),
            );
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
        match send_rcon_command("127.0.0.1", port, &password, command).await {
            Ok(output) => Ok(if output.is_empty() {
                "Command executed (no output).".into()
            } else {
                output
            }),
            Err(_) => {
                let managed = running
                    .servers
                    .get_mut(&server_id)
                    .ok_or_else(|| Error::InvalidState("server process is not running".into()))?;
                pumpkin_console_write(managed.stdin.as_mut(), command).await?;
                Ok("Sent to Pumpkin console (RCON unavailable).".into())
            }
        }
    }
}

async fn pumpkin_console_write(stdin: Option<&mut ChildStdin>, command: &str) -> Result<()> {
    let stdin = stdin
        .ok_or_else(|| Error::Other("Pumpkin console stdin is not available".into()))?;
    let line = if command.ends_with('\n') {
        command.to_string()
    } else {
        format!("{command}\n")
    };
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| Error::Other(format!("failed to write to Pumpkin console: {e}")))?;
    stdin
        .flush()
        .await
        .map_err(|e| Error::Other(format!("failed to flush Pumpkin console: {e}")))?;
    Ok(())
}

fn pumpkin_binary_for_server(
    binaries: &HashMap<String, PathBuf>,
    server: &ServerRecord,
) -> Result<PathBuf> {
    binaries
        .get(&server.pumpkin_channel)
        .cloned()
        .ok_or_else(|| {
            Error::Other(format!(
                "no Pumpkin binary for channel \"{}\"",
                server.pumpkin_channel
            ))
        })
}

fn spawn_pumpkin(
    binary: &PathBuf,
    cwd: &PathBuf,
    name: &str,
    network_id: Uuid,
    log_tx: Option<mpsc::UnboundedSender<LogLine>>,
) -> Result<ManagedPumpkin> {
    let mut cmd = Command::new(binary);
    cmd.current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn()?;
    let stdin = child.stdin.take();
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
    Ok(ManagedPumpkin { child, stdin })
}

async fn build_gourd_router(
    network: &NetworkRecord,
    servers: &[ServerRecord],
) -> Result<GourdRouter> {
    let lobby = servers
        .iter()
        .find(|s| s.role == "lobby")
        .ok_or_else(|| Error::InvalidState("network has no lobby server".into()))?;
    let default_backend: SocketAddr = format!("{}:{}", VELOCITY_BACKEND_HOST, lobby.game_port)
        .parse()
        .map_err(|e| Error::Other(format!("invalid lobby backend address: {e}")))?;
    let router = GourdRouter::new(network.forwarding_secret.clone(), default_backend);
    for server in servers {
        let addr: SocketAddr = format!("{}:{}", VELOCITY_BACKEND_HOST, server.game_port)
            .parse()
            .map_err(|e| Error::Other(format!("invalid backend address: {e}")))?;
        router.register_route(&server.velocity_name, addr).await;
    }
    Ok(router)
}

async fn spawn_gourd(
    network: &NetworkRecord,
    servers: &[ServerRecord],
    bind_host: &str,
    hub_path: &PathBuf,
    log_tx: Option<mpsc::UnboundedSender<LogLine>>,
) -> Result<(ManagedHub, GourdRouter)> {
    let hub_settings = read_hub_settings(hub_path)?;
    let bind: SocketAddr = format!("{}:{}", bind_host, network.hub_port)
        .parse()
        .map_err(|e| Error::Other(format!("invalid gourd bind: {e}")))?;
    let router = build_gourd_router(network, servers).await?;
    let network_id = network.id;
    let proxy = GourdProxy::new(
        GourdConfig {
            bind,
            motd: strip_velocity_motd(&hub_settings.motd),
            max_players: hub_settings.show_max_players as u16,
            version_name: "1.21.4".into(),
            protocol_version: 767,
        },
        router.clone(),
    );
    let task = tokio::spawn(async move {
        if let Some(tx) = log_tx {
            let _ = tx.send(LogLine {
                network_id,
                source: "gourd".into(),
                line: format!("Gourd listening on {}", bind),
            });
        }
        let _ = proxy.run().await;
    });
    Ok((
        ManagedHub {
            velocity_child: None,
            gourd_task: Some(task),
            stdin: None,
        },
        router,
    ))
}

fn strip_velocity_motd(motd: &str) -> String {
    motd.replace("<#e67e22>", "")
        .replace("<white>", "")
        .replace('<', "")
        .replace('>', "")
}

async fn stop_managed_hub(hub: &mut ManagedHub) {
    if let Some(task) = hub.gourd_task.take() {
        task.abort();
        let _ = task.await;
    }
    if let Some(child) = hub.velocity_child.as_mut() {
        let _ = child.start_kill();
        let _ = child.wait().await;
    }
}

fn spawn_velocity(
    java_path: &str,
    jar: &PathBuf,
    cwd: &PathBuf,
    network_id: Uuid,
    log_tx: Option<mpsc::UnboundedSender<LogLine>>,
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
    Ok(ManagedHub {
        velocity_child: Some(child),
        gourd_task: None,
        stdin,
    })
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
