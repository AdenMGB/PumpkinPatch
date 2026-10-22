//! Import signed Plan analytics exports into the desktop SQLite store (indepth.md §7 import).

use crate::error::{Error, Result};
use crate::plan_store::{ingest_payload, PlanSyncReport};
use sqlx::SqlitePool;
use std::path::Path;

pub async fn import_signed_plan_export(
    pool: &SqlitePool,
    network_id: &str,
    export_path: &Path,
    export_secret: &str,
) -> Result<PlanSyncReport> {
    if !export_path.exists() {
        return Err(Error::NotFound(format!(
            "export file not found: {}",
            export_path.display()
        )));
    }
    let raw = std::fs::read_to_string(export_path)?;
    let signed: patch_analytics_protocol::SignedExport = serde_json::from_str(&raw)?;
    ingest_signed_export(pool, network_id, &signed, export_secret).await
}

pub async fn ingest_signed_export(
    pool: &SqlitePool,
    network_id: &str,
    signed: &patch_analytics_protocol::SignedExport,
    export_secret: &str,
) -> Result<PlanSyncReport> {
    patch_analytics_protocol::verify_signed_export(signed, export_secret)
        .map_err(Error::InvalidState)?;
    ingest_payload(pool, network_id, &signed.payload).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use patch_analytics_protocol::{
        AnalyticsPayload, ServerTotals, SignedExport, EXPORT_SCHEMA_VERSION,
    };
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::HashMap;

    #[tokio::test]
    async fn imports_verified_export_file() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        let payload = AnalyticsPayload {
            schema_version: EXPORT_SCHEMA_VERSION,
            server_id: "srv-import".into(),
            server_name: "ImportTest".into(),
            network_id: "net-import".into(),
            generated_at_unix: 1,
            online_now: 0,
            peak_online: 0,
            total_joins: 0,
            unique_players: 0,
            total_playtime_secs: 0,
            server_totals: ServerTotals::default(),
            recent_deaths: vec![],
            recent_events: vec![],
            players: vec![],
            activity_samples: vec![],
            command_usages: HashMap::new(),
            recent_pvp_kills: vec![],
            performance_samples: vec![],
            registration_completions: vec![],
            web_admin_actions: vec![],
            desktop_plan_requests: vec![],
            is_proxy: false,
        };
        let secret = "test-secret";
        let sig = patch_analytics_protocol::sign_payload(&payload, secret).unwrap();
        let signed = SignedExport {
            payload,
            signature: sig,
        };
        let dir = std::env::temp_dir().join("patch-plan-import-test");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("export.json");
        std::fs::write(&path, serde_json::to_string_pretty(&signed).unwrap()).unwrap();
        let report = import_signed_plan_export(&pool, "net-import", &path, secret)
            .await
            .unwrap();
        assert_eq!(report.servers_synced, 1);
    }
}
