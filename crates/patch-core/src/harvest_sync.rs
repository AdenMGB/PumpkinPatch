//! Mirror Plan SQLite sessions into HarvestMetrics DuckDB (plangem.md §2).

use crate::error::Result;
use harvest_metrics::HarvestStore;
use sqlx::{Row, SqlitePool};
use std::path::Path;

pub async fn sync_harvest_from_plan_sqlite(
    sqlite: &SqlitePool,
    network_id: &str,
    duckdb_path: &Path,
) -> Result<i64> {
    let rows = sqlx::query(
        "SELECT s.uuid AS server_uuid, u.uuid AS player_uuid, u.name AS player_name,
                sess.session_start, sess.session_end, sess.afk_time, sess.mob_kills, sess.deaths
         FROM plan_sessions sess
         JOIN plan_users u ON u.id = sess.user_id
         JOIN plan_servers s ON s.id = sess.server_id
         WHERE s.network_id = ?
         ORDER BY sess.session_start DESC
         LIMIT 5000",
    )
    .bind(network_id)
    .fetch_all(sqlite)
    .await?;

    let mapped: Vec<(String, String, String, i64, Option<i64>, i64, u32, u32)> = rows
        .iter()
        .map(|row| {
            (
                row.get("server_uuid"),
                row.get("player_uuid"),
                row.get("player_name"),
                row.get("session_start"),
                row.get::<Option<i64>, _>("session_end"),
                row.get("afk_time"),
                row.get::<u32, _>("mob_kills"),
                row.get::<u32, _>("deaths"),
            )
        })
        .collect();

    let json_path = if duckdb_path.extension().and_then(|e| e.to_str()) == Some("duckdb") {
        duckdb_path.with_extension("harvest.json")
    } else {
        duckdb_path.to_path_buf()
    };
    let mut store = HarvestStore::open(&json_path)
        .map_err(|e| crate::error::Error::InvalidState(e.to_string()))?;
    store
        .upsert_sessions(network_id, &mapped)
        .map_err(|e| crate::error::Error::InvalidState(e.to_string()))?;

    if duckdb_path.extension().and_then(|e| e.to_str()) == Some("duckdb") {
        return harvest_metrics::sync_sessions_duckdb(duckdb_path, network_id, &mapped)
            .map_err(|e| crate::error::Error::InvalidState(e.to_string()));
    }

    store
        .session_count(network_id)
        .map_err(|e| crate::error::Error::InvalidState(e.to_string()))
}
