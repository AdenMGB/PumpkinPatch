use crate::error::{Error, Result};
use crate::models::{NetworkRecord, ServerRecord};
use patch_analytics_protocol::{
    config_file_path, export_file_path, verify_signed_export, AnalyticsPayload, PatchPlanConfig,
    SignedExport, EXPORT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAnalyticsSnapshot {
    pub server_id: String,
    pub server_name: String,
    pub generated_at_unix: i64,
    pub online_now: u32,
    pub peak_online: u32,
    pub total_joins: u64,
    pub unique_players: u64,
    pub total_playtime_secs: u64,
    pub top_players: Vec<PlayerRow>,
    pub activity_samples: Vec<ActivityPoint>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRow {
    pub uuid: String,
    pub name: String,
    pub join_count: u64,
    pub playtime_secs: u64,
    pub last_seen_unix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityPoint {
    pub t: i64,
    pub online: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAnalyticsOverview {
    pub network_id: String,
    pub network_name: String,
    pub combined_online: u32,
    pub combined_unique_players: u64,
    pub combined_total_joins: u64,
    pub combined_playtime_secs: u64,
    pub servers: Vec<ServerAnalyticsSnapshot>,
}

pub fn write_patch_plan_config(
    instance_path: &Path,
    server: &ServerRecord,
    network_id: Uuid,
    export_secret: &str,
) -> Result<()> {
    let dir = patch_analytics_protocol::export_data_dir(instance_path);
    std::fs::create_dir_all(&dir)?;
    let config = PatchPlanConfig {
        schema_version: EXPORT_SCHEMA_VERSION,
        server_id: server.id.to_string(),
        server_name: server.name.clone(),
        network_id: network_id.to_string(),
        export_secret: export_secret.to_string(),
    };
    let path = config_file_path(instance_path);
    let json = serde_json::to_string_pretty(&config)?;
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, json)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

pub fn read_server_analytics(
    instance_path: &Path,
    export_secret: &str,
    max_age_secs: i64,
) -> Result<ServerAnalyticsSnapshot> {
    let path = export_file_path(instance_path);
    if !path.exists() {
        return Ok(empty_snapshot(instance_path, true));
    }
    let raw = std::fs::read_to_string(&path)?;
    let signed: SignedExport = serde_json::from_str(&raw)?;
    verify_signed_export(&signed, export_secret)
        .map_err(|e| Error::InvalidState(e))?;
    payload_to_snapshot(&signed.payload, max_age_secs)
}

pub fn aggregate_network_analytics(
    network: &NetworkRecord,
    servers: &[ServerRecord],
    export_secret: &str,
) -> Result<NetworkAnalyticsOverview> {
    let max_age = 900;
    let mut snapshots = Vec::new();
    for server in servers {
        let snap = read_server_analytics(
            Path::new(&server.data_path),
            export_secret,
            max_age,
        )?;
        snapshots.push(snap);
    }

    let combined_online: u32 = snapshots.iter().map(|s| s.online_now).sum();
    let combined_total_joins: u64 = snapshots.iter().map(|s| s.total_joins).sum();
    let combined_playtime: u64 = snapshots.iter().map(|s| s.total_playtime_secs).sum();

    let mut unique_uuids = std::collections::HashSet::new();
    for snap in &snapshots {
        for p in &snap.top_players {
            unique_uuids.insert(p.uuid.clone());
        }
    }

    Ok(NetworkAnalyticsOverview {
        network_id: network.id.to_string(),
        network_name: network.name.clone(),
        combined_online,
        combined_unique_players: unique_uuids.len() as u64,
        combined_total_joins,
        combined_playtime_secs: combined_playtime,
        servers: snapshots,
    })
}

fn payload_to_snapshot(payload: &AnalyticsPayload, max_age_secs: i64) -> Result<ServerAnalyticsSnapshot> {
    let now = chrono::Utc::now().timestamp();
    let stale = now - payload.generated_at_unix > max_age_secs;
    let mut top: Vec<PlayerRow> = payload
        .players
        .iter()
        .map(|p| PlayerRow {
            uuid: p.uuid.clone(),
            name: p.name.clone(),
            join_count: p.join_count,
            playtime_secs: p.playtime_secs,
            last_seen_unix: p.last_seen_unix,
        })
        .collect();
    top.sort_by(|a, b| b.playtime_secs.cmp(&a.playtime_secs));
    top.truncate(25);

    Ok(ServerAnalyticsSnapshot {
        server_id: payload.server_id.clone(),
        server_name: payload.server_name.clone(),
        generated_at_unix: payload.generated_at_unix,
        online_now: payload.online_now,
        peak_online: payload.peak_online,
        total_joins: payload.total_joins,
        unique_players: payload.unique_players,
        total_playtime_secs: payload.total_playtime_secs,
        top_players: top,
        activity_samples: payload
            .activity_samples
            .iter()
            .map(|s| ActivityPoint {
                t: s.t,
                online: s.online,
            })
            .collect(),
        stale,
    })
}

fn empty_snapshot(instance_path: &Path, stale: bool) -> ServerAnalyticsSnapshot {
    ServerAnalyticsSnapshot {
        server_id: String::new(),
        server_name: instance_path
            .file_name()
            .map(|n| n.to_string_lossy().into())
            .unwrap_or_else(|| "server".into()),
        generated_at_unix: 0,
        online_now: 0,
        peak_online: 0,
        total_joins: 0,
        unique_players: 0,
        total_playtime_secs: 0,
        top_players: vec![],
        activity_samples: vec![],
        stale,
    }
}

pub fn generate_export_secret() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
