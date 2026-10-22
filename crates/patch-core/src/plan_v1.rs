//! Plan-shaped `/v1/players` and `/v1/sessions` response envelopes.

use crate::error::Result;
use crate::plan_store::{list_players_table, list_sessions, PlanPlayerTableRow, PlanSessionRow};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayersV1 {
    pub data: Vec<PlanPlayerV1Entry>,
}

/// `/v1/playersTable` — players rows plus extension column metadata (DataExtension).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayersTableV1 {
    pub data: Vec<PlanPlayerV1Entry>,
    pub extension_columns: Vec<PlanExtensionColumn>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExtensionColumn {
    pub key: String,
    pub label: String,
    pub plugin_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerV1Entry {
    pub name: String,
    pub uuid: String,
    pub activity_index: f64,
    pub activity_group: String,
    pub active_playtime: u64,
    pub sessions: u64,
    pub registered: i64,
    pub last_seen: i64,
    pub join_address: String,
    pub geolocation: String,
    pub ping_avg: u32,
    pub ping_min: u32,
    pub ping_max: u32,
    pub nick: String,
    pub extension_values: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSessionsV1 {
    pub sessions: Vec<PlanSessionRow>,
    pub most_recent_sessions: Vec<PlanSessionRow>,
}

pub async fn players_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<PlanPlayersV1> {
    let table = players_table_v1(pool, network_id, server_uuid, limit).await?;
    Ok(PlanPlayersV1 { data: table.data })
}

pub async fn players_table_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<PlanPlayersTableV1> {
    let rows = list_players_table(pool, network_id, server_uuid, limit).await?;
    Ok(PlanPlayersTableV1 {
        data: rows.into_iter().map(player_v1_entry).collect(),
        extension_columns: Vec::new(),
    })
}

pub async fn sessions_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<PlanSessionsV1> {
    let sessions = list_sessions(pool, network_id, server_uuid, limit).await?;
    let most_recent_sessions = sessions.iter().take(20).cloned().collect();
    Ok(PlanSessionsV1 {
        sessions,
        most_recent_sessions,
    })
}

pub fn player_v1_entry(row: PlanPlayerTableRow) -> PlanPlayerV1Entry {
    PlanPlayerV1Entry {
        name: row.name,
        uuid: row.uuid,
        activity_index: row.activity_index,
        activity_group: row.activity_group,
        active_playtime: row.active_playtime_ms,
        sessions: row.sessions,
        registered: row.registered,
        last_seen: row.last_seen,
        join_address: row.join_address,
        geolocation: row.geolocation,
        ping_avg: row.ping_avg,
        ping_min: row.ping_min,
        ping_max: row.ping_max,
        nick: row.nick,
        extension_values: json!({}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_active_playtime_field() {
        let entry = player_v1_entry(PlanPlayerTableRow {
            name: "Alex".into(),
            uuid: "u1".into(),
            activity_index: 42.0,
            activity_group: "Active".into(),
            active_playtime_ms: 99_000,
            sessions: 3,
            registered: 1_000,
            last_seen: 2_000,
            join_address: "mc.test".into(),
            geolocation: String::new(),
            ping_avg: 0,
            ping_min: 0,
            ping_max: 0,
            nick: String::new(),
            kick_count: 0,
            banned: false,
            operator: false,
        });
        assert_eq!(entry.active_playtime, 99_000);
        assert_eq!(entry.extension_values, json!({}));
    }
}
