//! Static Plan JSON export (indepth.md §6 Export system).

use crate::error::{Error, Result};
use crate::plan_theme::plan_theme_json;
use crate::{
    list_join_addresses, list_kills, network_overview_v1, online_overview_v1,
    performance_overview_v1, playerbase_overview_v1, players_v1, retention_v1, server_overview_v1,
    sessions_overview_v1, sessions_v1,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::path::Path;

pub async fn export_plan_static_bundle(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: &str,
    export_dir: &Path,
    live_online: u32,
    network_servers: &[(String, String, u32)],
) -> Result<()> {
    std::fs::create_dir_all(export_dir)?;
    let write = |name: &str, value: serde_json::Value| -> Result<()> {
        let path = export_dir.join(name);
        let tmp = path.with_extension("part");
        std::fs::write(&tmp, serde_json::to_string_pretty(&value)?)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    };

    write(
        "serverOverview.json",
        json!(server_overview_v1(pool, network_id, server_uuid, live_online).await?),
    )?;
    write(
        "onlineOverview.json",
        json!(online_overview_v1(pool, network_id, Some(server_uuid), live_online).await?),
    )?;
    write(
        "sessionsOverview.json",
        json!(sessions_overview_v1(pool, network_id, Some(server_uuid)).await?),
    )?;
    write(
        "playerbaseOverview.json",
        json!(playerbase_overview_v1(pool, network_id, Some(server_uuid)).await?),
    )?;
    write(
        "performanceOverview.json",
        json!(performance_overview_v1(pool, network_id, Some(server_uuid)).await?),
    )?;
    write(
        "sessions.json",
        json!(sessions_v1(pool, network_id, Some(server_uuid), 500).await?),
    )?;
    write(
        "players.json",
        json!(players_v1(pool, network_id, Some(server_uuid), 500).await?),
    )?;
    write(
        "retention.json",
        json!(retention_v1(pool, network_id, Some(server_uuid), 500).await?),
    )?;
    write(
        "kills.json",
        json!({
            "player_kills": list_kills(pool, network_id, Some(server_uuid), 500).await?
        }),
    )?;
    write(
        "joinAddresses.json",
        json!({
            "join_addresses": list_join_addresses(pool, network_id, Some(server_uuid)).await?
        }),
    )?;
    write(
        "networkOverview.json",
        json!(network_overview_v1(pool, network_id, network_servers).await?),
    )?;
    write("theme-default.json", plan_theme_json("default"))?;
    Ok(())
}

pub fn default_export_dir(network_data_path: &Path) -> std::path::PathBuf {
    network_data_path.join("plan-export")
}
