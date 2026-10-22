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

pub use crate::plan_extension_sql::PlanExtensionColumn;

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
    Ok(PlanPlayersV1 {
        data: table.data,
    })
}

pub async fn players_table_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<PlanPlayersTableV1> {
    let rows = list_players_table(pool, network_id, server_uuid, limit).await?;
    let uuids: Vec<String> = rows.iter().map(|r| r.uuid.clone()).collect();
    let extension_columns =
        crate::plan_extension_sql::extension_columns_for_table(pool, network_id).await?;
    let extension_map =
        crate::plan_extension_sql::extension_values_by_uuid(pool, network_id, &uuids).await?;
    let data = rows
        .into_iter()
        .map(|row| {
            let mut entry = player_v1_entry(row);
            entry.extension_values = extension_map
                .get(&entry.uuid)
                .cloned()
                .unwrap_or_else(|| json!({}));
            entry
        })
        .collect();
    Ok(PlanPlayersTableV1 {
        data,
        extension_columns,
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

    #[tokio::test]
    async fn players_table_merges_sql_extension_values() {
        use sqlx::sqlite::SqlitePoolOptions;
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO plan_servers (uuid, network_id, name) VALUES ('srv', 'net', 'S')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let server_id: i64 = sqlx::query_scalar("SELECT id FROM plan_servers LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO plan_users (uuid, registered, name) VALUES ('u1', 1, 'Steve')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let user_id: i64 = sqlx::query_scalar("SELECT id FROM plan_users LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO plan_user_info (user_id, server_id, join_address, registered, opped, banned)
             VALUES (?, ?, '', 1, 0, 0)",
        )
        .bind(user_id)
        .bind(server_id)
        .execute(&pool)
        .await
        .unwrap();
        crate::plan_extension_sql::sync_registered_extensions_to_sql(&pool, "net", server_id)
            .await
            .unwrap();
        let table = players_table_v1(&pool, "net", Some("srv"), 50)
            .await
            .unwrap();
        assert!(!table.extension_columns.is_empty());
        assert_eq!(table.data.len(), 1);
        assert!(table.data[0].extension_values.get("HarvestMetrics").is_some());
    }

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
