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
    write_plan_export_index(export_dir)?;
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
