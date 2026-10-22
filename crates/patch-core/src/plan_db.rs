//! Plan database maintenance (indepth.md §7 `db *` commands).

use crate::error::{Error, Result};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize)]
pub struct PlanBackupEntry {
    pub path: String,
    pub bytes: u64,
    pub modified_ms: i64,
}

pub fn list_plan_backups(backups_root: &Path) -> Result<Vec<PlanBackupEntry>> {
    if !backups_root.exists() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(backups_root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|e| e.to_str()) != Some("db") {
            continue;
        }
        let meta = std::fs::metadata(&path)?;
        entries.push(PlanBackupEntry {
            path: path.display().to_string(),
            bytes: meta.len(),
            modified_ms: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0),
        });
    }
    entries.sort_by_key(|e| std::cmp::Reverse(e.modified_ms));
    Ok(entries)
}

pub struct PlanDbBackupReport {
    pub path: PathBuf,
    pub bytes: u64,
}

pub async fn optimize_plan_sqlite(pool: &SqlitePool) -> Result<()> {
    sqlx::query("VACUUM").execute(pool).await?;
    sqlx::query("ANALYZE").execute(pool).await?;
    Ok(())
}

pub async fn restore_plan_sqlite_from_backup(db_path: &Path, backup_path: &Path) -> Result<()> {
    if !backup_path.exists() {
        return Err(Error::NotFound(format!(
            "backup file not found: {}",
            backup_path.display()
        )));
    }
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = db_path.with_extension("restore.part");
    std::fs::copy(backup_path, &tmp)?;
    std::fs::rename(tmp, db_path)?;
    Ok(())
}

pub async fn backup_plan_sqlite_file(db_path: &Path, backups_dir: &Path) -> Result<PlanDbBackupReport> {
    if !db_path.exists() {
        return Err(Error::NotFound(format!(
            "database file not found: {}",
            db_path.display()
        )));
    }
    std::fs::create_dir_all(backups_dir)?;
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let dest = backups_dir.join(format!("plan-backup-{}.db", stamp));
    std::fs::copy(db_path, &dest)?;
    let bytes = std::fs::metadata(&dest)?.len();
    Ok(PlanDbBackupReport { path: dest, bytes })
}

/// GDPR erasure — removes a player and dependent rows across Plan tables.
pub async fn remove_plan_player(pool: &SqlitePool, _network_id: &str, target: &str) -> Result<u64> {
    let row = sqlx::query("SELECT id, uuid FROM plan_users WHERE uuid = ? COLLATE NOCASE OR name = ? COLLATE NOCASE LIMIT 1")
        .bind(target)
        .bind(target)
        .fetch_optional(pool)
        .await?;

    let Some(row) = row else {
        return Err(Error::NotFound(format!("plan user '{target}' not found")));
    };
    let user_id: i64 = row.get("id");
    let uuid: String = row.get("uuid");

    let kills = sqlx::query(
        "DELETE FROM plan_kills WHERE killer_uuid = ? COLLATE NOCASE OR victim_uuid = ? COLLATE NOCASE",
    )
    .bind(&uuid)
    .bind(&uuid)
    .execute(pool)
    .await?
    .rows_affected();

    let deleted = sqlx::query("DELETE FROM plan_users WHERE id = ?")
        .bind(user_id)
        .execute(pool)
        .await?
        .rows_affected();
    crate::plan_data_clear::record_plan_player_data_cleared(&uuid);
    Ok(kills + deleted)
}

/// Drops analytics for one network (servers, sessions, performance samples).
/// Marks a server as decommissioned (`is_installed = 0`, indepth.md §7 `db uninstalled`).
pub async fn mark_plan_server_uninstalled(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid_or_name: &str,
) -> Result<bool> {
    let updated = sqlx::query(
        "UPDATE plan_servers SET is_installed = 0
         WHERE network_id = ? AND (uuid = ? COLLATE NOCASE OR name = ? COLLATE NOCASE)",
    )
    .bind(network_id)
    .bind(server_uuid_or_name)
    .bind(server_uuid_or_name)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(updated > 0)
}

pub async fn clear_plan_network_data(pool: &SqlitePool, network_id: &str) -> Result<u64> {
    let server_ids: Vec<i64> = sqlx::query_scalar(
        "SELECT id FROM plan_servers WHERE network_id = ?",
    )
    .bind(network_id)
    .fetch_all(pool)
    .await?;

    let mut rows = 0u64;
    for sid in server_ids {
        for stmt in [
            "DELETE FROM plan_sessions WHERE server_id = ?",
            "DELETE FROM plan_commandusages WHERE server_id = ?",
            "DELETE FROM plan_tps WHERE server_id = ?",
            "DELETE FROM plan_user_info WHERE server_id = ?",
            "DELETE FROM plan_ping WHERE server_id = ?",
            "DELETE FROM plan_nicknames WHERE server_id = ?",
        ] {
            let n = sqlx::query(stmt)
                .bind(sid)
                .execute(pool)
                .await?
                .rows_affected();
            rows += n;
        }
    }
    let n = sqlx::query("DELETE FROM plan_servers WHERE network_id = ?")
        .bind(network_id)
        .execute(pool)
        .await?
        .rows_affected();
    rows += n;
    Ok(rows)
}

pub async fn process_desktop_plan_requests(
    pool: &SqlitePool,
    network_id: &str,
    db_path: &Path,
    backups_dir: &Path,
    requests: &[patch_analytics_protocol::DesktopPlanRequest],
) -> Result<u32> {
    let mut applied = 0u32;
    for req in requests {
        let ok = match req {
            patch_analytics_protocol::DesktopPlanRequest::DbOptimize => {
                optimize_plan_sqlite(pool).await.is_ok()
            }
            patch_analytics_protocol::DesktopPlanRequest::DbBackup { label } => {
                let dir = backups_dir.join(label.replace('/', "_"));
                backup_plan_sqlite_file(db_path, &dir).await.is_ok()
            }
            patch_analytics_protocol::DesktopPlanRequest::DbRemovePlayer { target } => {
                remove_plan_player(pool, network_id, target).await.is_ok()
            }
            patch_analytics_protocol::DesktopPlanRequest::DbClearNetwork => {
                clear_plan_network_data(pool, network_id).await.is_ok()
            }
            patch_analytics_protocol::DesktopPlanRequest::DbMarkServerUninstalled { server } => {
                mark_plan_server_uninstalled(pool, network_id, server).await.unwrap_or(false)
            }
            patch_analytics_protocol::DesktopPlanRequest::DbRestore { backup_path } => {
                restore_plan_sqlite_from_backup(db_path, Path::new(backup_path))
                    .await
                    .is_ok()
            }
            patch_analytics_protocol::DesktopPlanRequest::ExportStaticBundle => true,
        };
        if ok {
            applied += 1;
        }
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn list_backups_empty_when_missing_dir() {
        let dir = std::env::temp_dir().join("patch-plan-no-backups-dir");
        let _ = std::fs::remove_dir_all(&dir);
        let list = list_plan_backups(&dir).unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn mark_uninstalled_hides_from_network_list() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO plan_servers (uuid, network_id, name) VALUES ('s1', 'net', 'Lobby')",
        )
        .execute(&pool)
        .await
        .unwrap();
        assert!(mark_plan_server_uninstalled(&pool, "net", "Lobby").await.unwrap());
        let rows = crate::plan_store::plan_network_servers_v1(&pool, "net")
            .await
            .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn optimize_runs_on_memory_db() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        optimize_plan_sqlite(&pool).await.unwrap();
    }
}
