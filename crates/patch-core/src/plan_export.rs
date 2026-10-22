//! Static Plan JSON export (indepth.md §6 Export system).

use crate::error::Result;
use crate::plan_theme::plan_theme_json;
use crate::{
    extension_data_merged, game_allowlist_bounces_v1, list_command_usages, list_join_addresses,
    list_kills, network_overview_v1, online_overview_v1, performance_overview_v1, plan_graph,
    player_versus_v1, playerbase_overview_v1, players_table_v1, players_v1, ping_table_v1,
    retention_v1, server_overview_v1, sessions_overview_v1, sessions_v1, statistics_v1,
};
use serde_json::json;
use sqlx::SqlitePool;
use std::path::{Path, PathBuf};

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
        write_json_atomic(&export_dir.join(name), &value)
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
    export_plan_react_network_data(pool, network_id, export_dir).await?;
    export_plan_react_server_data(pool, network_id, server_uuid, export_dir, live_online)
        .await?;
    export_plan_react_graph_data(pool, network_id, export_dir, Some(server_uuid)).await?;
    export_plan_react_graph_data(pool, network_id, export_dir, None).await?;
    write_plan_export_index(export_dir)?;
    write_plan_export_static_assets(export_dir)?;
    Ok(())
}

/// Full-network static export: every server's `/data` snapshots plus shared network files.
pub async fn export_plan_network_static_sync(
    pool: &SqlitePool,
    network_id: &str,
    export_dir: &Path,
    server_online: &[(String, u32)],
    network_servers: &[(String, String, u32)],
    primary_server_uuid: &str,
    primary_live_online: u32,
) -> Result<()> {
    std::fs::create_dir_all(export_dir)?;
    let write_root = |name: &str, value: serde_json::Value| -> Result<()> {
        write_json_atomic(&export_dir.join(name), &value)
    };
    write_root(
        "networkOverview.json",
        json!(network_overview_v1(pool, network_id, network_servers).await?),
    )?;
    write_root("theme-default.json", plan_theme_json("default"))?;
    export_plan_react_network_data(pool, network_id, export_dir).await?;
    for (uuid, online) in server_online {
        export_plan_react_server_data(pool, network_id, uuid, export_dir, *online).await?;
        export_plan_react_graph_data(pool, network_id, export_dir, Some(uuid)).await?;
    }
    export_plan_react_graph_data(pool, network_id, export_dir, None).await?;
    write_root(
        "serverOverview.json",
        json!(
            server_overview_v1(pool, network_id, primary_server_uuid, primary_live_online)
                .await?
        ),
    )?;
    write_root(
        "onlineOverview.json",
        json!(
            online_overview_v1(
                pool,
                network_id,
                Some(primary_server_uuid),
                primary_live_online,
            )
            .await?
        ),
    )?;
    write_plan_export_index(export_dir)?;
    write_plan_export_static_assets(export_dir)?;
    Ok(())
}

fn write_json_atomic(path: &Path, value: &serde_json::Value) -> Result<()> {
    let tmp = path.with_extension("part");
    std::fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

fn write_react_data(export_dir: &Path, name: &str, value: serde_json::Value) -> Result<()> {
    let data_dir = export_dir.join("data");
    std::fs::create_dir_all(&data_dir)?;
    write_json_atomic(&data_dir.join(name), &value)
}

const REACT_GRAPH_TYPES: &[(&str, &str)] = &[
    ("playersOnline", "graph-playersOnline"),
    ("activity", "graph-activity"),
    ("uniqueAndNew", "graph-uniqueAndNew"),
    ("hourlyUniqueAndNew", "graph-hourlyUniqueAndNew"),
    ("serverCalendar", "graph-serverCalendar"),
    ("punchCard", "graph-punchCard"),
    ("geolocation", "graph-geolocation"),
    ("optimizedPerformance", "graph-optimizedPerformance"),
    ("aggregatedPing", "graph-aggregatedPing"),
];

/// Graph JSON files consumed by Plan React static mode (`/data/graph-*.json`).
pub async fn export_plan_react_graph_data(
    pool: &SqlitePool,
    network_id: &str,
    export_dir: &Path,
    server_uuid: Option<&str>,
) -> Result<()> {
    for (graph_type, file_prefix) in REACT_GRAPH_TYPES {
        let value = plan_graph(pool, network_id, server_uuid, graph_type).await?;
        let name = match server_uuid {
            Some(uuid) => format!("{}_{}.json", file_prefix, uuid),
            None => format!("{}.json", file_prefix),
        };
        write_react_data(export_dir, &name, value)?;
    }
    if server_uuid.is_none() {
        let proxies = plan_graph(pool, network_id, None, "playersOnlineProxies").await?;
        write_react_data(export_dir, "graph-playersOnlineProxies.json", proxies)?;
    }
    Ok(())
}

/// Network-scoped `/data/*.json` (written once per export run).
pub async fn export_plan_react_network_data(
    pool: &SqlitePool,
    network_id: &str,
    export_dir: &Path,
) -> Result<()> {
    write_react_data(
        export_dir,
        "playersTable.json",
        json!(players_table_v1(pool, network_id, None, 500).await?),
    )?;
    write_react_data(
        export_dir,
        "retention.json",
        json!(retention_v1(pool, network_id, None, 500).await?),
    )?;
    write_react_data(
        export_dir,
        "joinAddresses.json",
        json!({
            "join_addresses": list_join_addresses(pool, network_id, None).await?
        }),
    )?;
    write_react_data(
        export_dir,
        "network-playerbaseOverview.json",
        json!(playerbase_overview_v1(pool, network_id, None).await?),
    )?;
    write_react_data(
        export_dir,
        "network-pingTable.json",
        json!(ping_table_v1(pool, network_id, None).await?),
    )?;
    write_react_data(
        export_dir,
        "statistics-.json",
        json!(statistics_v1(pool, network_id, None, false).await?),
    )?;
    write_react_data(
        export_dir,
        "statistics--aggregate.json",
        json!(statistics_v1(pool, network_id, None, true).await?),
    )?;
    Ok(())
}

/// Server-scoped `/data/*-{uuid}.json` snapshots.
pub async fn export_plan_react_server_data(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: &str,
    export_dir: &Path,
    live_online: u32,
) -> Result<()> {

    let server_row = sqlx::query_as::<_, (String, String)>(
        "SELECT uuid, name FROM plan_servers WHERE network_id = ? AND uuid = ? LIMIT 1",
    )
    .bind(network_id)
    .bind(server_uuid)
    .fetch_optional(pool)
    .await?;

    let (srv_uuid, server_name) = server_row
        .map(|(u, n)| (u, n))
        .unwrap_or_else(|| (server_uuid.to_string(), server_uuid.to_string()));

    write_react_data(
        export_dir,
        &format!("serverIdentity-{}.json", srv_uuid),
        json!({
            "serverName": server_name,
            "serverUuid": srv_uuid,
            "proxyMode": false,
        }),
    )?;

    let online = online_overview_v1(pool, network_id, Some(&srv_uuid), live_online).await?;
    write_react_data(
        export_dir,
        &format!("onlineInsights-{}.json", srv_uuid),
        json!({ "insights": online.insights }),
    )?;
    write_react_data(
        export_dir,
        &format!("playerbaseOverview-{}.json", srv_uuid),
        json!(playerbase_overview_v1(pool, network_id, Some(&srv_uuid)).await?),
    )?;
    write_react_data(
        export_dir,
        &format!("playerVersus-{}.json", srv_uuid),
        json!(player_versus_v1(pool, network_id, Some(&srv_uuid)).await?),
    )?;
    write_react_data(
        export_dir,
        &format!("extensionData-{}.json", srv_uuid),
        json!(extension_data_merged(pool, network_id).await?),
    )?;
    write_react_data(
        export_dir,
        &format!("kills-{}.json", srv_uuid),
        json!({
            "player_kills": list_kills(pool, network_id, Some(&srv_uuid), 500).await?
        }),
    )?;
    write_react_data(
        export_dir,
        &format!("playersTable-{}.json", srv_uuid),
        json!(players_table_v1(pool, network_id, Some(&srv_uuid), 500).await?),
    )?;
    write_react_data(
        export_dir,
        &format!("gameAllowlistBounces-{}.json", srv_uuid),
        json!(game_allowlist_bounces_v1(pool, network_id, Some(&srv_uuid)).await?),
    )?;
    write_react_data(
        export_dir,
        &format!("pingTable-{}.json", srv_uuid),
        json!(ping_table_v1(pool, network_id, Some(&srv_uuid)).await?),
    )?;
    write_react_data(
        export_dir,
        &format!("retention-{}.json", srv_uuid),
        json!(retention_v1(pool, network_id, Some(&srv_uuid), 500).await?),
    )?;
    write_react_data(
        export_dir,
        &format!("joinAddresses-{}.json", srv_uuid),
        json!({
            "join_addresses": list_join_addresses(pool, network_id, Some(&srv_uuid)).await?
        }),
    )?;
    write_react_data(
        export_dir,
        &format!("statistics-{}.json", srv_uuid),
        json!(statistics_v1(pool, network_id, Some(&srv_uuid), false).await?),
    )?;

    let plugin_rows = list_command_usages(pool, network_id, Some(&srv_uuid), 200).await?;
    let history = plugin_rows
        .into_iter()
        .map(|row| {
            json!({
                "pluginName": row.command,
                "timesUsed": row.times_used,
                "serverName": row.server_name,
            })
        })
        .collect::<Vec<_>>();
    write_react_data(
        export_dir,
        &format!("pluginHistory-{}.json", srv_uuid),
        json!({ "history": history }),
    )?;

    Ok(())
}

/// Pre-rendered `/data/*.json` for one server plus shared network files (Tauri single-server export).
pub async fn export_plan_react_static_data(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: &str,
    export_dir: &Path,
    live_online: u32,
) -> Result<()> {
    export_plan_react_network_data(pool, network_id, export_dir).await?;
    export_plan_react_server_data(pool, network_id, server_uuid, export_dir, live_online)
        .await?;
    Ok(())
}

fn write_plan_export_static_assets(export_dir: &Path) -> Result<()> {
    const DASHBOARD: &str = include_str!("../../patch-plan-http/static/plan-dashboard.html");
    const REGISTER: &str = include_str!("../../patch-plan-http/static/register.html");
    const THEME: &str = include_str!("../../patch-plan-http/static/theme-editor.html");
    std::fs::write(export_dir.join("app.html"), DASHBOARD)?;
    std::fs::write(export_dir.join("register.html"), REGISTER)?;
    std::fs::write(export_dir.join("theme-editor.html"), THEME)?;
    const MANAGE: &str = include_str!("../../patch-plan-http/static/manage.html");
    std::fs::write(export_dir.join("manage.html"), MANAGE)?;
    Ok(())
}

pub fn write_plan_export_index(export_dir: &Path) -> Result<()> {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title>Pumpkin Patch Plan Export</title>
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <style>
    body { font-family: system-ui, sans-serif; margin: 2rem; max-width: 52rem; background: #0f172a; color: #e2e8f0; }
    a { color: #f97316; }
    li { margin: 0.35rem 0; }
    .cards { display: grid; grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr)); gap: 1rem; margin: 1.5rem 0; }
    .card { background: #1e293b; border-radius: 0.5rem; padding: 1rem; }
    .card strong { display: block; font-size: 1.5rem; color: #f8fafc; }
    .muted { color: #94a3b8; font-size: 0.9rem; }
    #status { margin-top: 1rem; }
  </style>
</head>
<body>
  <h1>Plan static export</h1>
  <p class="muted">Pre-rendered JSON bundles for offline hosting (indepth.md §6.2). Pair with Plan React dashboard assets or Pumpkin Patch Analytics UI.</p>
  <div class="cards" id="cards" aria-live="polite">
    <div class="card"><span class="muted">Online</span><strong id="online">—</strong></div>
    <div class="card"><span class="muted">Total players</span><strong id="total">—</strong></div>
    <div class="card"><span class="muted">7d unique</span><strong id="unique7">—</strong></div>
    <div class="card"><span class="muted">Avg TPS (7d)</span><strong id="tps">—</strong></div>
  </div>
  <p id="status" class="muted">Loading overview…</p>
  <ul>
    <li><a href="serverOverview.json">serverOverview.json</a></li>
    <li><a href="onlineOverview.json">onlineOverview.json</a></li>
    <li><a href="sessions.json">sessions.json</a></li>
    <li><a href="players.json">players.json</a></li>
    <li><a href="networkOverview.json">networkOverview.json</a></li>
    <li><a href="theme-default.json">theme-default.json</a></li>
  </ul>
  <p class="muted">When Plan HTTP is enabled on port 8804, live JSON is also at <code>/v1/serverOverview?server=…</code> and GDPR exports at <code>/v1/player/json?player=…</code>.</p>
  <script>
    fetch('serverOverview.json').then(r => r.json()).then(d => {
      const n = d.numbers || {};
      const w = d.last_7_days || {};
      document.getElementById('online').textContent = n.online_players ?? '—';
      document.getElementById('total').textContent = n.total_players ?? '—';
      document.getElementById('unique7').textContent = w.unique_players ?? '—';
      document.getElementById('tps').textContent = w.avg_tps != null ? Number(w.avg_tps).toFixed(2) : '—';
      document.getElementById('status').textContent = 'Overview loaded from serverOverview.json';
    }).catch(() => {
      document.getElementById('status').textContent = 'Could not load serverOverview.json (open this folder via Plan HTTP /export or a static host).';
    });
  </script>
</body>
</html>"#;
    std::fs::write(export_dir.join("index.html"), html)?;
    Ok(())
}

pub fn default_export_dir(network_data_path: &Path) -> std::path::PathBuf {
    network_data_path.join("plan-export")
}

/// PEM TLS material for Plan HTTPS (`network/data/plan/tls/cert.pem` + `key.pem`).
pub fn plan_tls_pem_paths(network_data_path: &Path) -> Option<(PathBuf, PathBuf)> {
    let dir = network_data_path.join("plan/tls");
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    if cert.is_file() && key.is_file() {
        Some((cert, key))
    } else {
        None
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use crate::plan_store::ensure_plan_schema;

    #[tokio::test]
    async fn react_static_data_files_written_under_data_dir() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        ensure_plan_schema(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO plan_servers (network_id, uuid, name, is_installed) VALUES ('net', 'srv-1', 'Lobby', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();

        let dir = tempfile::tempdir().unwrap();
        export_plan_react_static_data(&pool, "net", "srv-1", dir.path(), 3)
            .await
            .unwrap();

        let identity = dir.path().join("data/serverIdentity-srv-1.json");
        assert!(identity.is_file(), "missing {}", identity.display());
        let players = dir.path().join("data/playersTable.json");
        assert!(players.is_file());
        let network_ret = dir.path().join("data/retention.json");
        assert!(network_ret.is_file());
    }

    #[tokio::test]
    async fn react_graph_snapshots_written_for_server_and_network() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        ensure_plan_schema(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO plan_servers (network_id, uuid, name, is_installed) VALUES ('net', 'srv-a', 'A', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        export_plan_react_graph_data(&pool, "net", dir.path(), Some("srv-a"))
            .await
            .unwrap();
        export_plan_react_graph_data(&pool, "net", dir.path(), None)
            .await
            .unwrap();
        assert!(dir.path().join("data/graph-activity_srv-a.json").is_file());
        assert!(dir.path().join("data/graph-activity.json").is_file());
        assert!(dir
            .path()
            .join("data/graph-playersOnlineProxies.json")
            .is_file());
    }
}

#[cfg(test)]
mod tls_path_tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn discovers_pem_pair_under_plan_tls() {
        let tmp = tempfile::tempdir().unwrap();
        let tls = tmp.path().join("plan/tls");
        std::fs::create_dir_all(&tls).unwrap();
        std::fs::File::create(tls.join("cert.pem"))
            .unwrap()
            .write_all(b"cert")
            .unwrap();
        std::fs::File::create(tls.join("key.pem"))
            .unwrap()
            .write_all(b"key")
            .unwrap();
        let paths = plan_tls_pem_paths(tmp.path()).expect("pem pair");
        assert!(paths.0.ends_with("cert.pem"));
        assert!(paths.1.ends_with("key.pem"));
    }
}
