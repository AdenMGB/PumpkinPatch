use crate::analytics::generate_export_secret;
use crate::db::Database;
use crate::error::{Error, Result};
use crate::models::{
    CreateNetworkRequest, HubType, JoinInfo, NetworkRecord, NetworkRuntimeStatus, NetworkSummary,
    ServerRecord, ServerRole,
};
use crate::releases::ReleaseService;
use crate::state::AppDirs;
use chrono::Utc;
use std::path::PathBuf;
use uuid::Uuid;

/// Velocity connects to Pumpkin backends on loopback; public `bind_host` is only for the proxy listen socket.
const VELOCITY_BACKEND_HOST: &str = "127.0.0.1";

#[derive(Clone)]
pub struct NetworkOrchestrator {
    dirs: AppDirs,
    releases: ReleaseService,
}

impl NetworkOrchestrator {
    pub fn new(dirs: AppDirs, releases: ReleaseService) -> Self {
        Self { dirs, releases }
    }

    pub async fn create_network(
        &self,
        db: &Database,
        settings: &crate::models::AppSettings,
        req: CreateNetworkRequest,
    ) -> Result<NetworkSummary> {
        if req.backend_names.is_empty() {
            return Err(Error::InvalidState(
                "at least one backend name is required".into(),
            ));
        }
        let hub_type = req.hub_type.unwrap_or(HubType::Velocity);
        if hub_type != HubType::Velocity {
            return Err(Error::InvalidState(
                "only velocity hub is supported in v1".into(),
            ));
        }
        let network_id = Uuid::new_v4();
        let hub_port = req.hub_port.unwrap_or(25565);
        let channel = req
            .pumpkin_channel
            .unwrap_or_else(|| settings.pumpkin_channel_default.clone());
        let minecraft_version = req
            .minecraft_version
            .unwrap_or_else(|| "1.21.4".to_string());
        let forwarding_secret = generate_forwarding_secret();
        let network_path = self.dirs.networks.join(network_id.to_string());
        std::fs::create_dir_all(&network_path)?;
        let hub_path = network_path.join("hub");
        std::fs::create_dir_all(&hub_path)?;

        let lobby_id = Uuid::new_v4();
        let lobby_port = allocate_port(hub_port, 1);
        let lobby_path = network_path.join("lobby");
        std::fs::create_dir_all(&lobby_path)?;
        self.write_instance_proxy_config(&lobby_path, &forwarding_secret, lobby_port)?;

        let mut servers = Vec::new();
        let lobby = ServerRecord {
            id: lobby_id,
            network_id,
            role: ServerRole::Lobby.as_str().into(),
            name: "Lobby".into(),
            velocity_name: "lobby".into(),
            game_port: lobby_port,
            data_path: lobby_path.to_string_lossy().into(),
            pumpkin_channel: channel.clone(),
            minecraft_version: minecraft_version.clone(),
            created_at: Utc::now(),
        };
        servers.push(lobby.clone());

        for (idx, name) in req.backend_names.iter().enumerate() {
            let backend_id = Uuid::new_v4();
            let game_port = allocate_port(hub_port, 2 + idx as u16);
            let velocity_name = slug_velocity_name(name);
            let backend_path = network_path.join("backends").join(&velocity_name);
            std::fs::create_dir_all(&backend_path)?;
            self.write_instance_proxy_config(&backend_path, &forwarding_secret, game_port)?;
            servers.push(ServerRecord {
                id: backend_id,
                network_id,
                role: ServerRole::Backend.as_str().into(),
                name: name.clone(),
                velocity_name,
                game_port,
                data_path: backend_path.to_string_lossy().into(),
                pumpkin_channel: channel.clone(),
                minecraft_version: minecraft_version.clone(),
                created_at: Utc::now(),
            });
        }

        self.write_velocity_toml(
            &hub_path,
            settings.bind_host.as_str(),
            hub_port,
            &forwarding_secret,
            &servers,
        )?;

        let network = NetworkRecord {
            id: network_id,
            name: req.name,
            hub_type: hub_type.as_str().into(),
            hub_port,
            forwarding_secret,
            lobby_server_id: Some(lobby_id),
            data_path: network_path.to_string_lossy().into(),
            analytics_export_token: generate_export_secret(),
            created_at: Utc::now(),
        };

        db.insert_network(&network).await?;
        for server in &servers {
            db.insert_server(server).await?;
        }

        Ok(NetworkSummary {
            network,
            servers,
            status: NetworkRuntimeStatus::Stopped,
            join_address: format!("{}:{}", settings.bind_host, hub_port),
        })
    }

    pub fn join_info(network: &NetworkRecord, bind_host: &str) -> JoinInfo {
        JoinInfo {
            host: bind_host.to_string(),
            port: network.hub_port,
            address: format!("{}:{}", bind_host, network.hub_port),
        }
    }

    pub fn write_velocity_toml(
        &self,
        hub_path: &PathBuf,
        bind_host: &str,
        hub_port: u16,
        secret: &str,
        servers: &[ServerRecord],
    ) -> Result<()> {
        let mut servers_section = String::new();
        for server in servers {
            servers_section.push_str(&format!(
                "{} = \"{}:{}\"\n",
                server.velocity_name,
                VELOCITY_BACKEND_HOST,
                server.game_port
            ));
        }
        std::fs::write(hub_path.join("forwarding.secret"), secret)?;
        let content = format!(
            r#"# Generated by Pumpkin Patch
config-version = "2.9"
bind = "{bind_host}:{hub_port}"
motd = "<#e67e22>Pumpkin Patch <white>Network"
show-max-players = 500
online-mode = true
force-key-authentication = true
player-info-forwarding-mode = "modern"
forwarding-secret-file = "forwarding.secret"

[players]
ip-forward = true

[servers]
{servers_section}
[try]
list = ["lobby"]

[forced-hosts]

[advanced]
compression-threshold = 256
compression-level = -1
login-ratelimit = 3000

[query]
enabled = false
"#,
            bind_host = bind_host,
            hub_port = hub_port,
            servers_section = servers_section,
        );
        std::fs::write(hub_path.join("velocity.toml"), content)?;
        Ok(())
    }

    fn write_instance_proxy_config(
        &self,
        instance_path: &PathBuf,
        secret: &str,
        port: u16,
    ) -> Result<()> {
        crate::instance_config::sync_server_instance_config(instance_path.as_path(), port, secret)?;
        Ok(())
    }

    pub fn sync_network_instances(
        &self,
        network: &NetworkRecord,
        servers: &[ServerRecord],
        bind_host: &str,
    ) -> Result<()> {
        for server in servers {
            crate::instance_config::sync_server_instance_config(
                std::path::Path::new(&server.data_path),
                server.game_port,
                &network.forwarding_secret,
            )?;
        }
        let hub_path = PathBuf::from(&network.data_path).join("hub");
        self.write_velocity_toml(
            &hub_path,
            bind_host,
            network.hub_port,
            &network.forwarding_secret,
            servers,
        )?;
        Ok(())
    }

    pub async fn ensure_binaries_for_network(
        &self,
        servers: &[ServerRecord],
        progress: Option<tokio::sync::mpsc::Sender<crate::releases::DownloadProgress>>,
    ) -> Result<(PathBuf, PathBuf)> {
        let channel = servers
            .first()
            .map(|s| s.pumpkin_channel.clone())
            .unwrap_or_else(|| "nightly".into());
        let pumpkin = self
            .releases
            .ensure_pumpkin_binary(&channel, progress.clone())
            .await?;
        let velocity = self.releases.ensure_velocity_jar(progress).await?;
        Ok((pumpkin, velocity))
    }

    pub async fn add_backend(
        &self,
        db: &Database,
        network: &NetworkRecord,
        settings: &crate::models::AppSettings,
        name: &str,
        minecraft_version: &str,
    ) -> Result<ServerRecord> {
        let servers = db.list_servers_for_network(network.id).await?;
        let hub_port = network.hub_port;
        let offset = 2 + servers.len() as u16;
        let game_port = allocate_port(hub_port, offset);
        let velocity_name = slug_velocity_name(name);
        let backend_path = PathBuf::from(&network.data_path)
            .join("backends")
            .join(&velocity_name);
        std::fs::create_dir_all(&backend_path)?;
        self.write_instance_proxy_config(
            &backend_path,
            &network.forwarding_secret,
            game_port,
        )?;
        let channel = servers
            .first()
            .map(|s| s.pumpkin_channel.clone())
            .unwrap_or_else(|| settings.pumpkin_channel_default.clone());
        let backend = ServerRecord {
            id: Uuid::new_v4(),
            network_id: network.id,
            role: ServerRole::Backend.as_str().into(),
            name: name.to_string(),
            velocity_name,
            game_port,
            data_path: backend_path.to_string_lossy().into(),
            pumpkin_channel: channel,
            minecraft_version: minecraft_version.to_string(),
            created_at: Utc::now(),
        };
        db.insert_server(&backend).await?;
        let all = db.list_servers_for_network(network.id).await?;
        let hub_path = PathBuf::from(&network.data_path).join("hub");
        self.write_velocity_toml(
            &hub_path,
            settings.bind_host.as_str(),
            hub_port,
            &network.forwarding_secret,
            &all,
        )?;
        Ok(backend)
    }

    pub async fn remove_server(
        &self,
        db: &Database,
        network: &NetworkRecord,
        settings: &crate::models::AppSettings,
        server_id: Uuid,
    ) -> Result<()> {
        let server = db.get_server(server_id).await?;
        if server.role == ServerRole::Lobby.as_str() {
            return Err(Error::InvalidState("cannot remove lobby server".into()));
        }
        db.delete_server(server_id).await?;
        let path = PathBuf::from(&server.data_path);
        if path.exists() {
            std::fs::remove_dir_all(path).ok();
        }
        let all = db.list_servers_for_network(network.id).await?;
        let hub_path = PathBuf::from(&network.data_path).join("hub");
        self.write_velocity_toml(
            &hub_path,
            settings.bind_host.as_str(),
            network.hub_port,
            &network.forwarding_secret,
            &all,
        )?;
        Ok(())
    }
}

fn generate_forwarding_secret() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::rng();
    (0..32)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

fn allocate_port(base: u16, offset: u16) -> u16 {
    base.saturating_add(offset).max(1024)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ServerRecord, ServerRole};
    use crate::releases::ReleaseService;
    use crate::state::AppDirs;
    use chrono::Utc;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn velocity_toml_contains_secret_and_try() {
        let dir = tempdir().unwrap();
        let dirs = AppDirs::new(dir.path().to_path_buf());
        let dirs2 = dirs.clone();
        let orch = NetworkOrchestrator::new(dirs, ReleaseService::new(dirs2));
        let hub = dir.path().join("hub");
        std::fs::create_dir_all(&hub).unwrap();
        let servers = vec![ServerRecord {
            id: Uuid::new_v4(),
            network_id: Uuid::new_v4(),
            role: ServerRole::Lobby.as_str().into(),
            name: "Lobby".into(),
            velocity_name: "lobby".into(),
            game_port: 25566,
            data_path: String::new(),
            pumpkin_channel: "nightly".into(),
            minecraft_version: "1.21.4".into(),
            created_at: Utc::now(),
        }];
        orch.write_velocity_toml(&hub, "127.0.0.1", 25565, "secret-abc", &servers)
            .unwrap();
        let text = std::fs::read_to_string(hub.join("velocity.toml")).unwrap();
        let secret_file = std::fs::read_to_string(hub.join("forwarding.secret")).unwrap();
        assert_eq!(secret_file.trim(), "secret-abc");
        assert!(text.contains("forwarding-secret-file"));
        assert!(text.contains("list = [\"lobby\"]"));
        assert!(text.contains("player-info-forwarding-mode"));
        assert!(text.contains("lobby = \"127.0.0.1:25566\""));
    }
}

fn slug_velocity_name(name: &str) -> String {
    let slug: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "backend".into()
    } else {
        slug
    }
}
