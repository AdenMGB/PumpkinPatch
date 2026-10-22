//! Network topology routing & Velocity hot-wire (plangem.md §3.1, §7.5).

use crate::error::{Error, Result};
use crate::models::{NetworkRecord, ServerRecord};
use crate::network::NetworkOrchestrator;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use vineyard_protocol::RouteWireSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: Uuid,
    pub kind: String,
    pub label: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub proxy_id: Uuid,
    pub backend_server_id: Uuid,
    pub server_alias: String,
    pub forced_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    pub network_id: Uuid,
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

pub fn build_topology(network: &NetworkRecord, servers: &[ServerRecord]) -> NetworkTopology {
    let mut nodes = vec![TopologyNode {
        id: network.id,
        kind: "proxy".into(),
        label: format!("{} (Velocity :{})", network.name, network.hub_port),
        port: Some(network.hub_port),
    }];
    for server in servers {
        nodes.push(TopologyNode {
            id: server.id,
            kind: server.role.clone(),
            label: server.name.clone(),
            port: Some(server.game_port),
        });
    }
    let edges = servers
        .iter()
        .map(|s| TopologyEdge {
            proxy_id: network.id,
            backend_server_id: s.id,
            server_alias: s.velocity_name.clone(),
            forced_hosts: Vec::new(),
        })
        .collect();
    NetworkTopology {
        network_id: network.id,
        nodes,
        edges,
    }
}

pub fn hot_wire_velocity_toml(
    orchestrator: &NetworkOrchestrator,
    network: &NetworkRecord,
    servers: &[ServerRecord],
    bind_host: &str,
    spec: &RouteWireSpec,
) -> Result<String> {
    let backend = servers
        .iter()
        .find(|s| s.id == spec.backend_server_id)
        .ok_or_else(|| Error::InvalidState("backend server not found".into()))?;
    let hub_path = PathBuf::from(&network.data_path).join("hub");
    orchestrator.write_velocity_toml(
        &hub_path,
        bind_host,
        network.hub_port,
        &network.forwarding_secret,
        servers,
    )?;
    let toml_path = hub_path.join("velocity.toml");
    let content = std::fs::read_to_string(&toml_path)?;
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| Error::InvalidState(e.to_string()))?;
    if let Some(forced) = doc.get_mut("forced-hosts") {
        if let Some(table) = forced.as_table_mut() {
            for host in &spec.forced_hosts {
                table.insert(
                    host,
                    toml_edit::Item::Value(toml_edit::Value::Array(
                        toml_edit::Array::from_iter([
                            toml_edit::Value::String(toml_edit::Formatted::new(
                                backend.velocity_name.clone(),
                            )),
                        ]),
                    )),
                );
            }
        }
    }
    std::fs::write(toml_path, doc.to_string())?;
    Ok(format!(
        "Live-wired '{}' to proxy on port {}",
        spec.server_alias,
        network.hub_port
    ))
}
