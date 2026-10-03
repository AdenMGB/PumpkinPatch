use crate::error::{Error, Result};
use crate::models::{NetworkRecord, ServerRecord};
use patch_analytics_protocol::{
    config_file_path, export_file_path, verify_signed_export, AnalyticsPayload, PatchPlanConfig,
    ServerTotals, SignedExport, EXPORT_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerTotalsView {
    pub total_logins: u64,
    pub total_kicks: u64,
    pub total_deaths: u64,
    pub total_advancements: u64,
    pub total_mob_kills: u64,
    pub total_player_kills: u64,
    pub total_recipes_discovered: u64,
    pub total_blocks_harvested: u64,
    pub total_chat_messages: u64,
    pub total_commands: u64,
    pub top_statistics: Vec<StatisticRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticRow {
    pub id: String,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathRow {
    pub t: i64,
    pub player_uuid: String,
    pub player_name: String,
    pub message: String,
    pub server_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRow {
    pub t: i64,
    pub kind: String,
    pub player_uuid: String,
    pub player_name: String,
    pub detail: String,
    pub server_name: String,
}

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
    pub server_totals: ServerTotalsView,
    pub top_players: Vec<PlayerDetailRow>,
    pub recent_deaths: Vec<DeathRow>,
    pub recent_events: Vec<EventRow>,
    pub activity_samples: Vec<ActivityPoint>,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerDetailRow {
    pub uuid: String,
    pub name: String,
    pub join_count: u64,
    pub login_count: u64,
    pub playtime_secs: u64,
    pub deaths: u64,
    pub mob_kills: u64,
    pub player_kills: u64,
    pub advancements: u64,
    pub recipes_discovered: u64,
    pub blocks_harvested: u64,
    pub chat_messages: u64,
    pub commands_used: u64,
    pub current_gamemode: String,
    pub last_world: String,
    pub xp_level: i32,
    pub top_statistics: Vec<StatisticRow>,
    pub recent_advancements: Vec<String>,
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
    pub combined_deaths: u64,
    pub combined_advancements: u64,
    pub combined_mob_kills: u64,
    pub combined_player_kills: u64,
    pub combined_top_statistics: Vec<StatisticRow>,
    pub recent_deaths: Vec<DeathRow>,
    pub recent_events: Vec<EventRow>,
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
    server_name: &str,
    max_age_secs: i64,
) -> Result<ServerAnalyticsSnapshot> {
    let path = export_file_path(instance_path);
    if !path.exists() {
        return Ok(empty_snapshot(instance_path, server_name, true));
    }
    let raw = std::fs::read_to_string(&path)?;
    let signed: SignedExport = serde_json::from_str(&raw)?;
    verify_signed_export(&signed, export_secret)
        .map_err(|e| Error::InvalidState(e))?;
    payload_to_snapshot(&signed.payload, server_name, max_age_secs)
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
            &server.name,
            max_age,
        )?;
        snapshots.push(snap);
    }

    let combined_online: u32 = snapshots.iter().map(|s| s.online_now).sum();
    let combined_total_joins: u64 = snapshots.iter().map(|s| s.total_joins).sum();
    let combined_playtime: u64 = snapshots.iter().map(|s| s.total_playtime_secs).sum();
    let combined_deaths: u64 = snapshots
        .iter()
        .map(|s| s.server_totals.total_deaths)
        .sum();
    let combined_advancements: u64 = snapshots
        .iter()
        .map(|s| s.server_totals.total_advancements)
        .sum();
    let combined_mob_kills: u64 = snapshots
        .iter()
        .map(|s| s.server_totals.total_mob_kills)
        .sum();
    let combined_player_kills: u64 = snapshots
        .iter()
        .map(|s| s.server_totals.total_player_kills)
        .sum();

    let mut unique_uuids = HashSet::new();
    let mut stat_map: HashMap<String, u64> = HashMap::new();
    let mut all_deaths = Vec::new();
    let mut all_events = Vec::new();

    for snap in &snapshots {
        for p in &snap.top_players {
            unique_uuids.insert(p.uuid.clone());
        }
        for row in &snap.server_totals.top_statistics {
            *stat_map.entry(row.id.clone()).or_insert(0) += row.total;
        }
        all_deaths.extend(snap.recent_deaths.clone());
        all_events.extend(snap.recent_events.clone());
    }

    all_deaths.sort_by(|a, b| b.t.cmp(&a.t));
    all_deaths.truncate(150);
    all_events.sort_by(|a, b| b.t.cmp(&a.t));
    all_events.truncate(200);

    Ok(NetworkAnalyticsOverview {
        network_id: network.id.to_string(),
        network_name: network.name.clone(),
        combined_online,
        combined_unique_players: unique_uuids.len() as u64,
        combined_total_joins,
        combined_playtime_secs: combined_playtime,
        combined_deaths,
        combined_advancements,
        combined_mob_kills,
        combined_player_kills,
        combined_top_statistics: top_stats_from_map(&stat_map, 30),
        recent_deaths: all_deaths,
        recent_events: all_events,
        servers: snapshots,
    })
}

fn payload_to_snapshot(
    payload: &AnalyticsPayload,
    server_name: &str,
    max_age_secs: i64,
) -> Result<ServerAnalyticsSnapshot> {
    let now = chrono::Utc::now().timestamp();
    let stale = now - payload.generated_at_unix > max_age_secs;

    let mut top_players: Vec<PlayerDetailRow> = payload
        .players
        .iter()
        .map(|p| player_detail_from_stats(p))
        .collect();
    top_players.sort_by(|a, b| b.playtime_secs.cmp(&a.playtime_secs));
    top_players.truncate(50);

    Ok(ServerAnalyticsSnapshot {
        server_id: payload.server_id.clone(),
        server_name: payload.server_name.clone(),
        generated_at_unix: payload.generated_at_unix,
        online_now: payload.online_now,
        peak_online: payload.peak_online,
        total_joins: payload.total_joins,
        unique_players: payload.unique_players,
        total_playtime_secs: payload.total_playtime_secs,
        server_totals: totals_view(&payload.server_totals),
        top_players,
        recent_deaths: payload
            .recent_deaths
            .iter()
            .map(|d| DeathRow {
                t: d.t,
                player_uuid: d.player_uuid.clone(),
                player_name: d.player_name.clone(),
                message: d.message.clone(),
                server_name: server_name.to_string(),
            })
            .collect(),
        recent_events: payload
            .recent_events
            .iter()
            .map(|e| EventRow {
                t: e.t,
                kind: e.kind.clone(),
                player_uuid: e.player_uuid.clone(),
                player_name: e.player_name.clone(),
                detail: e.detail.clone(),
                server_name: server_name.to_string(),
            })
            .collect(),
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

fn player_detail_from_stats(
    p: &patch_analytics_protocol::PlayerStats,
) -> PlayerDetailRow {
    PlayerDetailRow {
        uuid: p.uuid.clone(),
        name: p.name.clone(),
        join_count: p.join_count,
        login_count: p.login_count,
        playtime_secs: p.playtime_secs,
        deaths: p.deaths,
        mob_kills: p.mob_kills,
        player_kills: p.player_kills,
        advancements: p.advancements,
        recipes_discovered: p.recipes_discovered,
        blocks_harvested: p.blocks_harvested,
        chat_messages: p.chat_messages,
        commands_used: p.commands_used,
        current_gamemode: p.current_gamemode.clone(),
        last_world: p.last_world.clone(),
        xp_level: p.xp_level,
        top_statistics: top_stats_from_map(&p.statistic_totals, 12),
        recent_advancements: p.recent_advancements.clone(),
        last_seen_unix: p.last_seen_unix,
    }
}

fn totals_view(t: &ServerTotals) -> ServerTotalsView {
    ServerTotalsView {
        total_logins: t.total_logins,
        total_kicks: t.total_kicks,
        total_deaths: t.total_deaths,
        total_advancements: t.total_advancements,
        total_mob_kills: t.total_mob_kills,
        total_player_kills: t.total_player_kills,
        total_recipes_discovered: t.total_recipes_discovered,
        total_blocks_harvested: t.total_blocks_harvested,
        total_chat_messages: t.total_chat_messages,
        total_commands: t.total_commands,
        top_statistics: top_stats_from_map(&t.statistic_totals, 25),
    }
}

fn top_stats_from_map(map: &HashMap<String, u64>, limit: usize) -> Vec<StatisticRow> {
    let mut rows: Vec<StatisticRow> = map
        .iter()
        .map(|(id, total)| StatisticRow {
            id: id.clone(),
            total: *total,
        })
        .collect();
    rows.sort_by(|a, b| b.total.cmp(&a.total));
    rows.truncate(limit);
    rows
}

fn empty_snapshot(instance_path: &Path, server_name: &str, stale: bool) -> ServerAnalyticsSnapshot {
    ServerAnalyticsSnapshot {
        server_id: String::new(),
        server_name: server_name.to_string(),
        generated_at_unix: 0,
        online_now: 0,
        peak_online: 0,
        total_joins: 0,
        unique_players: 0,
        total_playtime_secs: 0,
        server_totals: ServerTotalsView::default(),
        top_players: vec![],
        recent_deaths: vec![],
        recent_events: vec![],
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
