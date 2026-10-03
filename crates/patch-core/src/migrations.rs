use crate::error::Result;
use sqlx::{Row, SqlitePool};

const SCHEMA_VERSION: i64 = 2;

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            version INTEGER NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    let row = sqlx::query("SELECT version FROM schema_version WHERE id = 1")
        .fetch_optional(pool)
        .await?;
    let current = row.map(|r| r.get::<i64, _>("version")).unwrap_or(0);

    if current < 1 {
        apply_v1(pool).await?;
    }
    if current < 2 {
        apply_v2(pool).await?;
    }

    sqlx::query("INSERT INTO schema_version (id, version) VALUES (1, ?) ON CONFLICT(id) DO UPDATE SET version = excluded.version")
        .bind(SCHEMA_VERSION)
        .execute(pool)
        .await?;

    Ok(())
}

async fn apply_v1(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            created_at TEXT NOT NULL,
            action TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            detail TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn apply_v2(pool: &SqlitePool) -> Result<()> {
    let _ = sqlx::query(
        "ALTER TABLE networks ADD COLUMN auto_restart INTEGER NOT NULL DEFAULT 0",
    )
    .execute(pool)
    .await;
    let _ = sqlx::query("ALTER TABLE networks ADD COLUMN last_crash_source TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query("ALTER TABLE networks ADD COLUMN last_crash_at TEXT")
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "ALTER TABLE networks ADD COLUMN restart_attempts INTEGER NOT NULL DEFAULT 0",
    )
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS network_tags (
            network_id TEXT NOT NULL,
            tag TEXT NOT NULL,
            PRIMARY KEY (network_id, tag)
        )",
    )
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS health_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            network_id TEXT NOT NULL,
            overall TEXT NOT NULL,
            recorded_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await;
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS network_meta (
            network_id TEXT PRIMARY KEY NOT NULL,
            favorite INTEGER NOT NULL DEFAULT 0,
            last_played_at TEXT,
            template_id TEXT
        )",
    )
    .execute(pool)
    .await;
    Ok(())
}

pub async fn append_audit(
    pool: &SqlitePool,
    action: &str,
    entity_type: &str,
    entity_id: Option<&str>,
    detail: &str,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO audit_log (created_at, action, entity_type, entity_id, detail)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(chrono::Utc::now().to_rfc3339())
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(detail)
    .execute(pool)
    .await?;
    Ok(())
}
