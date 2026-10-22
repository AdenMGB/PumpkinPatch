use crate::analytics::read_server_analytics;
use crate::db::Database;
use crate::error::Result;
use crate::models::{NetworkRecord, ServerRecord};
use crate::plan_store::{ingest_payload, PlanSyncReport};
use sqlx::Row;
use std::path::Path;

pub async fn sync_network_plan_exports(
    db: &Database,
    network: &NetworkRecord,
    servers: &[ServerRecord],
    export_secret: &str,
) -> Result<Vec<PlanSyncReport>> {
    let pool = db.pool();
    crate::plan_store::ensure_plan_schema(pool).await?;

    let max_age = 900;
    let mut reports = Vec::new();
    for server in servers {
        match sync_server_plan_export(
            pool,
            network,
            server,
            export_secret,
            max_age,
        )
        .await
        {
            Ok(Some(report)) => reports.push(report),
            Ok(None) | Err(_) => {}
        }
    }
    Ok(reports)
}

async fn sync_server_plan_export(
    pool: &sqlx::SqlitePool,
    network: &NetworkRecord,
    server: &ServerRecord,
    export_secret: &str,
    max_age: i64,
) -> Result<Option<PlanSyncReport>> {
    let snap = read_server_analytics(
        Path::new(&server.data_path),
        export_secret,
        &server.name,
        max_age,
    )?;
    if snap.stale && snap.generated_at_unix == 0 {
        return Ok(None);
    }
    let path = patch_analytics_protocol::export_file_path(Path::new(&server.data_path));
    if !path.exists() {
        return Ok(None);
    }
    let raw = std::fs::read_to_string(&path)?;
    let signed: patch_analytics_protocol::SignedExport = serde_json::from_str(&raw)?;
    patch_analytics_protocol::verify_signed_export(&signed, export_secret)
        .map_err(crate::error::Error::InvalidState)?;
    let _ = crate::plan_auth::process_registration_completions(
        pool,
        &signed.payload.registration_completions,
    )
    .await;
    let _ = crate::plan_auth::process_web_admin_actions(pool, &signed.payload.web_admin_actions)
        .await;
    let report = ingest_payload(pool, &network.id.to_string(), &signed.payload).await?;
    if let Ok(row) = sqlx::query("SELECT id FROM plan_servers WHERE uuid = ?")
        .bind(server.id.to_string())
        .fetch_optional(pool)
        .await
    {
        if let Some(row) = row {
            let plan_server_id: i64 = row.get("id");
            let _ = crate::plan_perf::sample_server_performance(
                pool,
                plan_server_id,
                signed.payload.online_now,
            )
            .await;
        }
    }
    Ok(Some(report))
}
