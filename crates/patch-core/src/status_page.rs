use crate::models::{NetworkRecord, PingResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct NetworkStatusPage {
    pub name: String,
    pub join_address: String,
    pub hub_online: bool,
    pub motd: Option<String>,
    pub players_online: Option<u32>,
    pub players_max: Option<u32>,
}

pub fn build_status_page(
    network: &NetworkRecord,
    bind_host: &str,
    ping: &PingResult,
) -> NetworkStatusPage {
    NetworkStatusPage {
        name: network.name.clone(),
        join_address: format!("{}:{}", bind_host, network.hub_port),
        hub_online: ping.online,
        motd: ping.motd.clone(),
        players_online: ping.players_online,
        players_max: ping.players_max,
    }
}
