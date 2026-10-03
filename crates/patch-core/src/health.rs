use crate::analytics::read_server_analytics;
use crate::models::{NetworkRecord, PingResult, ServerRecord};
use crate::ping::ping_server;
use crate::ports::can_connect_tcp;
use crate::instance_config::{rcon_password_for_server, rcon_port_for_game_port};
use crate::rcon::send_rcon_command;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthLevel {
    Healthy,
    Degraded,
    Down,
    Unknown,
}

impl HealthLevel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Down => "down",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub id: String,
    pub label: String,
    pub level: HealthLevel,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkHealthReport {
    pub network_id: Uuid,
    pub overall: HealthLevel,
    pub hub: ComponentHealth,
    pub servers: Vec<ComponentHealth>,
    pub plugins: Vec<ComponentHealth>,
    pub checked_at: DateTime<Utc>,
}

fn level_from_ping(ping: &PingResult) -> HealthLevel {
    if ping.online {
        HealthLevel::Healthy
    } else {
        HealthLevel::Down
    }
}

pub async fn check_network_health(
    network: &NetworkRecord,
    servers: &[ServerRecord],
    bind_host: &str,
    analytics_token: &str,
) -> NetworkHealthReport {
    let checked_at = Utc::now();
    let hub_ping = ping_server(bind_host, network.hub_port).await.unwrap_or(PingResult {
        online: false,
        latency_ms: None,
        version: None,
        motd: None,
        players_online: None,
        players_max: None,
    });
    let hub = ComponentHealth {
        id: "hub".into(),
        label: "Velocity hub".into(),
        level: level_from_ping(&hub_ping),
        detail: if hub_ping.online {
            format!(
                "Online{}",
                hub_ping
                    .latency_ms
                    .map(|ms| format!(" · {ms} ms"))
                    .unwrap_or_default()
            )
        } else {
            "Hub not responding to ping".into()
        },
    };

    let mut server_health = Vec::new();
    for server in servers {
        let tcp_ok =
            can_connect_tcp("127.0.0.1", server.game_port, Duration::from_millis(800)).await;
        let mut level = if tcp_ok {
            HealthLevel::Healthy
        } else {
            HealthLevel::Down
        };
        let mut detail = if tcp_ok {
            format!("Port {} open", server.game_port)
        } else {
            format!("Port {} closed", server.game_port)
        };

        if tcp_ok && server.role != "hub" {
            let port = rcon_port_for_game_port(server.game_port);
            let password = rcon_password_for_server(&network.forwarding_secret, server.game_port);
            if send_rcon_command("127.0.0.1", port, &password, "list")
                .await
                .is_err()
            {
                level = HealthLevel::Degraded;
                detail = format!("Port open but RCON failed on {}", port);
            }
        }

        server_health.push(ComponentHealth {
            id: server.id.to_string(),
            label: format!("{} ({})", server.name, server.role),
            level,
            detail,
        });
    }

    let mut plugins = Vec::new();
    for server in servers {
        let instance_path = Path::new(&server.data_path);
        let export_path = instance_path
            .join("plugins")
            .join("data")
            .join("patch-plan-analytics")
            .join("export.json");
        if !export_path.exists() {
            continue;
        }
        let stale = std::fs::metadata(&export_path)
            .ok()
            .and_then(|m| m.modified().ok())
            .map(|t| t.elapsed().map(|e| e.as_secs() > 120).unwrap_or(true))
            .unwrap_or(true);
        let level = if stale {
            HealthLevel::Degraded
        } else {
            HealthLevel::Healthy
        };
        let verify_ok = read_server_analytics(instance_path, analytics_token, &server.name, 300)
            .map(|s| !s.stale)
            .unwrap_or(false);
        plugins.push(ComponentHealth {
            id: format!("patch-plan:{}", server.id),
            label: format!("Patch Plan · {}", server.name),
            level: if verify_ok { level } else { HealthLevel::Degraded },
            detail: if verify_ok {
                "Signed export OK".into()
            } else {
                "Export signature invalid or unreadable".into()
            },
        });
    }

    let overall = overall_level(&hub, &server_health, &plugins);

    NetworkHealthReport {
        network_id: network.id,
        overall,
        hub,
        servers: server_health,
        plugins,
        checked_at,
    }
}

fn overall_level(
    hub: &ComponentHealth,
    servers: &[ComponentHealth],
    plugins: &[ComponentHealth],
) -> HealthLevel {
    if hub.level == HealthLevel::Down {
        return HealthLevel::Down;
    }
    if servers.iter().any(|s| s.level == HealthLevel::Down) {
        return HealthLevel::Degraded;
    }
    if servers.iter().any(|s| s.level == HealthLevel::Degraded)
        || plugins.iter().any(|p| p.level == HealthLevel::Degraded)
    {
        return HealthLevel::Degraded;
    }
    if hub.level == HealthLevel::Degraded {
        return HealthLevel::Degraded;
    }
    HealthLevel::Healthy
}
