use crate::analytics::read_server_analytics;
use crate::db::Database;
use crate::error::Result;
use crate::models::{NetworkRecord, ServerRecord};
use crate::plan_export::{default_export_dir, export_plan_static_bundle};
use crate::plan_store::{ingest_payload, PlanSyncReport};
use sqlx::Row;
use std::path::Path;

pub async fn sync_network_plan_exports(
    db: &Database,
    network: &NetworkRecord,
    servers: &[ServerRecord],
    export_secret: &str,
) -> Result<Vec<PlanSyncReport>> {
    let backups_dir = Path::new(&network.data_path).join("plan-backups");
    let pool = db.pool();
    crate::plan_store::ensure_plan_schema(pool).await?;

    let max_age = 900;
    let mut reports = Vec::new();
    let mut export_static_bundle = false;
    for server in servers {
        match sync_server_plan_export(
            db,
            pool,
            network,
            server,
            export_secret,
            max_age,
            &backups_dir,
            &mut export_static_bundle,
        )
        .await
        {
            Ok(Some(report)) => reports.push(report),
            Ok(None) | Err(_) => {}
        }
    }
    if export_static_bundle && !servers.is_empty() {
        let overview = crate::aggregate_network_analytics(network, servers, export_secret)?;
        let triples: Vec<(String, String, u32)> = overview
            .servers
            .iter()
            .map(|s| (s.server_id.clone(), s.server_name.clone(), s.online_now))
            .collect();
        let primary = &servers[0];
        let live_online = overview
            .servers
            .iter()
            .find(|s| s.server_id == primary.id.to_string())
            .map(|s| s.online_now)
            .unwrap_or(0);
        let dir = default_export_dir(Path::new(&network.data_path));
        let _ = export_plan_static_bundle(
            pool,
            &network.id.to_string(),
            &primary.id.to_string(),
            &dir,
            live_online,
            &triples,
        )
        .await;
    }
    Ok(reports)
}

async fn sync_server_plan_export(
    db: &Database,
    pool: &sqlx::SqlitePool,
    network: &NetworkRecord,
    server: &ServerRecord,
    export_secret: &str,
    max_age: i64,
    backups_dir: &Path,
    export_static_bundle: &mut bool,
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
    if signed
        .payload
        .desktop_plan_requests
        .iter()
        .any(|r| {
            matches!(
                r,
                patch_analytics_protocol::DesktopPlanRequest::ExportStaticBundle
            )
        })
    {
        *export_static_bundle = true;
    }
    let _ = crate::plan_db::process_desktop_plan_requests(
        pool,
        &network.id.to_string(),
        db.file_path(),
        &backups_dir,
        &signed.payload.desktop_plan_requests,
    )
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
