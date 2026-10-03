use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HubType {
    Velocity,
    Vine,
}

impl HubType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Velocity => "velocity",
            Self::Vine => "vine",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "velocity" => Some(Self::Velocity),
            "vine" => Some(Self::Vine),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerRole {
    Lobby,
    Backend,
}

impl ServerRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Lobby => "lobby",
            Self::Backend => "backend",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "lobby" => Some(Self::Lobby),
            "backend" => Some(Self::Backend),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum NetworkRuntimeStatus {
    #[default]
    Stopped,
    Starting,
    Running,
    Stopping,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRecord {
    pub id: Uuid,
    pub name: String,
    pub hub_type: String,
    pub hub_port: u16,
    pub forwarding_secret: String,
    pub lobby_server_id: Option<Uuid>,
    pub data_path: String,
    pub analytics_export_token: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerRecord {
    pub id: Uuid,
    pub network_id: Uuid,
    pub role: String,
    pub name: String,
    pub velocity_name: String,
    pub game_port: u16,
    pub data_path: String,
    pub pumpkin_channel: String,
    pub minecraft_version: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSummary {
    pub network: NetworkRecord,
    pub servers: Vec<ServerRecord>,
    pub status: NetworkRuntimeStatus,
    pub join_address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNetworkRequest {
    pub name: String,
    pub hub_port: Option<u16>,
    pub hub_type: Option<HubType>,
    pub backend_names: Vec<String>,
    pub pumpkin_channel: Option<String>,
    pub minecraft_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateServerSettingsRequest {
    pub bind_address: Option<String>,
    pub online_mode: Option<bool>,
    pub max_players: Option<u32>,
    pub view_distance: Option<u8>,
    pub simulation_distance: Option<u8>,
    pub motd: Option<String>,
    pub commands_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBackendRequest {
    pub name: String,
    pub minecraft_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateHubSettingsRequest {
    pub bind: Option<String>,
    pub motd: Option<String>,
    pub online_mode: Option<bool>,
    pub show_max_players: Option<u32>,
    pub player_info_forwarding: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub java_path: String,
    pub bind_host: String,
    pub pumpkin_channel_default: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            java_path: "java".into(),
            bind_host: "127.0.0.1".into(),
            pumpkin_channel_default: "nightly".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub online: bool,
    pub latency_ms: Option<u64>,
    pub version: Option<String>,
    pub motd: Option<String>,
    pub players_online: Option<u32>,
    pub players_max: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinInfo {
    pub host: String,
    pub port: u16,
    pub address: String,
}
