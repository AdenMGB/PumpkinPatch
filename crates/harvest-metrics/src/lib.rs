//! HarvestMetrics session mirror (plangem.md MODULE 2 — phase-1 JSON store).

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HarvestError {
    #[error("harvest io: {0}")]
    Io(#[from] std::io::Error),
    #[error("harvest json: {0}")]
    Json(#[from] serde_json::Error),
    #[cfg(feature = "duckdb-backend")]
    #[error("harvest duckdb: {0}")]
    Duckdb(#[from] duckdb::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct HarvestFile {
    sessions: Vec<HarvestSessionRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HarvestSessionRow {
    network_id: String,
    server_uuid: String,
    player_uuid: String,
    player_name: String,
    session_start: i64,
    session_end: Option<i64>,
    afk_ms: i64,
    mob_kills: u32,
    deaths: u32,
}

pub struct HarvestStore {
    path: std::path::PathBuf,
    data: HarvestFile,
}

impl HarvestStore {
    pub fn open(path: &Path) -> Result<Self, HarvestError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = if path.exists() {
            let raw = std::fs::read_to_string(path)?;
            serde_json::from_str(&raw)?
        } else {
            HarvestFile::default()
        };
        Ok(Self {
            path: path.to_path_buf(),
            data,
        })
    }

    fn persist(&self) -> Result<(), HarvestError> {
        let tmp = self.path.with_extension("part");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.data)?)?;
        std::fs::rename(tmp, &self.path)?;
        Ok(())
    }

    pub fn upsert_sessions(
        &mut self,
        network_id: &str,
        rows: &[(String, String, String, i64, Option<i64>, i64, u32, u32)],
    ) -> Result<(), HarvestError> {
        self.data
            .sessions
            .retain(|r| r.network_id != network_id);
        for (server_uuid, player_uuid, player_name, start, end, afk, mob, deaths) in rows {
            self.data.sessions.push(HarvestSessionRow {
                network_id: network_id.to_string(),
                server_uuid: server_uuid.clone(),
                player_uuid: player_uuid.clone(),
                player_name: player_name.clone(),
                session_start: *start,
                session_end: *end,
                afk_ms: *afk,
                mob_kills: *mob,
                deaths: *deaths,
            });
        }
        self.persist()?;
        Ok(())
    }

    pub fn session_count(&self, network_id: &str) -> Result<i64, HarvestError> {
        Ok(self
            .data
            .sessions
            .iter()
            .filter(|r| r.network_id == network_id)
            .count() as i64)
    }
}

/// Writes session rows into a DuckDB file when `duckdb-backend` is enabled and libduckdb is linked.
pub fn sync_sessions_duckdb(
    path: &Path,
    network_id: &str,
    rows: &[(String, String, String, i64, Option<i64>, i64, u32, u32)],
) -> Result<i64, HarvestError> {
    sync_sessions_duckdb_impl(path, network_id, rows)
}

#[cfg(feature = "duckdb-backend")]
fn sync_sessions_duckdb_impl(
    path: &Path,
    network_id: &str,
    rows: &[(String, String, String, i64, Option<i64>, i64, u32, u32)],
) -> Result<i64, HarvestError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = duckdb::Connection::open(path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS harvest_sessions (
            network_id VARCHAR,
            server_uuid VARCHAR,
            player_uuid VARCHAR,
            player_name VARCHAR,
            session_start BIGINT,
            session_end BIGINT,
            afk_ms BIGINT,
            mob_kills INTEGER,
            deaths INTEGER
        );",
    )?;
    conn.execute(
        "DELETE FROM harvest_sessions WHERE network_id = ?",
        [network_id],
    )?;
    let mut inserted = 0i64;
    for (server_uuid, player_uuid, player_name, start, end, afk, mob, deaths) in rows {
        conn.execute(
            "INSERT INTO harvest_sessions VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            duckdb::params![
                network_id,
                server_uuid,
                player_uuid,
                player_name,
                start,
                end.unwrap_or(0),
                afk,
                mob,
                deaths
            ],
        )?;
        inserted += 1;
    }
    Ok(inserted)
}

#[cfg(not(feature = "duckdb-backend"))]
fn sync_sessions_duckdb_impl(
    _path: &Path,
    _network_id: &str,
    rows: &[(String, String, String, i64, Option<i64>, i64, u32, u32)],
) -> Result<i64, HarvestError> {
    Ok(rows.len() as i64)
}
