//! Plan /v1/graph-style responses (subset) for the desktop UI.

use crate::error::{Error, Result};
use chrono::{Datelike, Timelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

pub async fn plan_graph(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    graph_type: &str,
) -> Result<Value> {
    let graph_type = match graph_type {
        "joinAddressByDay" => "joinAddressesByDay",
        "optimizedPerformance" => "tps",
        "aggregatedPing" => "ping",
        other => other,
    };
    match graph_type {
        "playersOnline" => {
            let g = crate::plan_store::graph_players_online(pool, network_id, server_uuid).await?;
            Ok(json!({ "players_online": g.series }))
        }
        "playersOnlineProxies" => {
            let g = crate::plan_store::graph_players_online_proxies(pool, network_id).await?;
            Ok(json!({ "players_online": g.series }))
        }
        "worldPie" => Ok(json!(world_pie(pool, network_id, server_uuid).await?)),
        "punchCard" => Ok(json!({ "punchcard": punch_card(pool, network_id, server_uuid).await? })),
        "joinAddressesPie" => {
            Ok(json!({ "join_addresses": join_addresses_pie(pool, network_id, server_uuid).await? }))
        }
        "activity" => Ok(json!(activity_pie(pool, network_id, server_uuid).await?)),
        "cpuRam" => Ok(json!(cpu_ram_series(pool, network_id, server_uuid).await?)),
        "tps" => Ok(json!(tps_series(pool, network_id, server_uuid).await?)),
        "uniqueAndNew" => Ok(json!(unique_and_new_series(pool, network_id, server_uuid).await?)),
        "disk" => Ok(json!(disk_series(pool, network_id, server_uuid).await?)),
        "worldLoad" => Ok(json!(world_load_series(pool, network_id, server_uuid).await?)),
        "geolocation" => Ok(json!(geolocation_series(pool, network_id, server_uuid).await?)),
        "hourlyUniqueAndNew" => Ok(hourly_unique_and_new_series(pool, network_id, server_uuid).await?),
        "joinAddressesByDay" => Ok(join_addresses_by_day(pool, network_id, server_uuid).await?),
        "serverCalendar" => Ok(server_calendar(pool, network_id, server_uuid).await?),
        "ping" => Ok(ping_series(pool, network_id, server_uuid).await?),
        _ => Err(Error::InvalidState(format!("unsupported graph type: {graph_type}"))),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldPieSlice {
    pub name: String,
    pub y: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmPieSlice {
    pub name: String,
    pub y: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldPieResponse {
    pub world_series: Vec<WorldPieSlice>,
    pub gm_series: Vec<GmPieSlice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeolocationSlice {
    pub code: String,
    pub name: String,
    pub value: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeolocationBarSlice {
    pub name: String,
    pub value: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeolocationGraphResponse {
    pub geolocation_series: Vec<GeolocationSlice>,
    pub geolocation_bar_series: Vec<GeolocationBarSlice>,
}

async fn geolocation_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<GeolocationGraphResponse> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT g.geolocation AS name, COUNT(DISTINCT g.user_id) AS value
             FROM plan_geolocations g
             JOIN plan_user_info ui ON ui.user_id = g.user_id
             JOIN plan_servers s ON s.id = ui.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY g.geolocation
             ORDER BY value DESC LIMIT 32",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT g.geolocation AS name, COUNT(DISTINCT g.user_id) AS value
             FROM plan_geolocations g
             JOIN plan_user_info ui ON ui.user_id = g.user_id
             JOIN plan_servers s ON s.id = ui.server_id
             WHERE s.network_id = ?
             GROUP BY g.geolocation
             ORDER BY value DESC LIMIT 32",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let geolocation_series = rows
        .iter()
        .map(|r| {
            let name: String = r.get("name");
            GeolocationSlice {
                code: geolocation_code(&name),
                name,
                value: r.get::<i64, _>("value") as u64,
            }
        })
        .collect::<Vec<_>>();
    let geolocation_bar_series = geolocation_series
        .iter()
        .take(16)
        .map(|s| GeolocationBarSlice {
            name: s.name.clone(),
            value: s.value,
        })
        .collect();
    Ok(GeolocationGraphResponse {
        geolocation_series,
        geolocation_bar_series,
    })
}

fn geolocation_code(name: &str) -> String {
    match name {
        "Local Machine" => "LOC".to_string(),
        "Unknown" => "UNK".to_string(),
        other => other
            .split_whitespace()
            .filter_map(|w| w.chars().next())
            .take(3)
            .collect::<String>()
            .to_uppercase(),
    }
}

async fn world_pie(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<WorldPieResponse> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT wt.world_name,
                    SUM(wt.survival_time + wt.creative_time + wt.adventure_time + wt.spectator_time) AS total
             FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY wt.world_name
             ORDER BY total DESC LIMIT 24",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT wt.world_name,
                    SUM(wt.survival_time + wt.creative_time + wt.adventure_time + wt.spectator_time) AS total
             FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
             GROUP BY wt.world_name
             ORDER BY total DESC LIMIT 24",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let gm_rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT SUM(wt.survival_time) AS survival,
                    SUM(wt.creative_time) AS creative,
                    SUM(wt.adventure_time) AS adventure,
                    SUM(wt.spectator_time) AS spectator
             FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT SUM(wt.survival_time) AS survival,
                    SUM(wt.creative_time) AS creative,
                    SUM(wt.adventure_time) AS adventure,
                    SUM(wt.spectator_time) AS spectator
             FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?
    };

    let gm_series = [
        ("Survival", "survival"),
        ("Creative", "creative"),
        ("Adventure", "adventure"),
        ("Spectator", "spectator"),
    ]
    .iter()
    .map(|(label, col)| GmPieSlice {
        name: (*label).into(),
        y: gm_rows.get::<i64, _>(*col).max(0) as u64,
    })
    .filter(|s| s.y > 0)
    .collect();

    Ok(WorldPieResponse {
        world_series: rows
            .iter()
            .map(|r| WorldPieSlice {
                name: r.get("world_name"),
                y: r.get::<i64, _>("total") as u64,
            })
            .collect(),
        gm_series,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PunchCardPoint {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

async fn punch_card(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Vec<PunchCardPoint>> {
    let since = Utc::now().timestamp_millis() - 30_i64 * 24 * 60 * 60 * 1000;
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT session_start FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND session_start >= ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT session_start FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND session_start >= ?",
        )
        .bind(network_id)
        .bind(since)
        .fetch_all(pool)
        .await?
    };

    let mut grid: [[u32; 24]; 7] = [[0; 24]; 7];
    for row in rows {
        let ms: i64 = row.get("session_start");
        let dt = chrono::DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_else(Utc::now);
        let dow = dt.weekday().num_days_from_monday() as usize;
        let hour = dt.hour() as usize;
        grid[dow][hour] += 1;
    }

    let mut out = Vec::new();
    for (dow, row) in grid.iter().enumerate() {
        for (hour, count) in row.iter().enumerate() {
            if *count > 0 {
                out.push(PunchCardPoint {
                    x: hour as u32,
                    y: dow as u32,
                    z: *count,
                });
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinAddressSlice {
    pub join_address: String,
    pub count: u64,
}

async fn join_addresses_pie(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Vec<JoinAddressSlice>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT sess.join_address, COUNT(*) AS c FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.join_address != ''
             GROUP BY sess.join_address ORDER BY c DESC LIMIT 20",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT sess.join_address, COUNT(*) AS c FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.join_address != ''
             GROUP BY sess.join_address ORDER BY c DESC LIMIT 20",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| JoinAddressSlice {
            join_address: r.get("join_address"),
            count: r.get::<i64, _>("c") as u64,
        })
        .collect())
}

async fn activity_pie(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    let players = crate::plan_store::list_players_table(pool, network_id, server_uuid, 500).await?;
    let mut groups: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    for p in &players {
        *groups.entry(p.activity_group.clone()).or_insert(0) += 1;
    }
    let pie_series = groups
        .iter()
        .map(|(name, count)| json!({ "name": name, "y": count }))
        .collect::<Vec<_>>();

    const DAY_MS: i64 = 86_400_000;
    let since = Utc::now().timestamp_millis() - 30 * DAY_MS;
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day, u.uuid
             FROM plan_sessions sess
             JOIN plan_users u ON u.id = sess.user_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.session_start >= ?",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .bind(uuid)
        .bind(since)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day, u.uuid
             FROM plan_sessions sess
             JOIN plan_users u ON u.id = sess.user_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.session_start >= ?",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .bind(since)
        .fetch_all(pool)
        .await?
    };

    let group_by_uuid: std::collections::HashMap<String, String> = players
        .iter()
        .map(|p| (p.uuid.clone(), p.activity_group.clone()))
        .collect();
    let mut daily: std::collections::HashMap<String, std::collections::HashMap<i64, u32>> =
        std::collections::HashMap::new();
    for row in rows {
        let day: i64 = row.get("day");
        let uuid: String = row.get("uuid");
        let group = group_by_uuid
            .get(&uuid)
            .cloned()
            .unwrap_or_else(|| "Inactive".into());
        daily
            .entry(group)
            .or_default()
            .entry(day)
            .and_modify(|c| *c += 1)
            .or_insert(1);
    }
    let activity_series: serde_json::Map<String, Value> = daily
        .into_iter()
        .map(|(group, days)| {
            let points = days
                .into_iter()
                .map(|(day, count)| json!([day, count]))
                .collect::<Vec<_>>();
            (group, json!(points))
        })
        .collect();

    Ok(json!({
        "activity_pie_series": pie_series,
        "activity_series": activity_series,
    }))
}

async fn cpu_ram_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT date, cpu_usage, ram_usage FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             ORDER BY date ASC",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT date, AVG(cpu_usage) AS cpu_usage, AVG(ram_usage) AS ram_usage FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ?
             GROUP BY date ORDER BY date ASC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let cpu_pairs = rows
        .iter()
        .map(|r| (r.get::<i64, _>("date"), r.get::<f64, _>("cpu_usage")))
        .collect();
    let ram_pairs = rows
        .iter()
        .map(|r| (r.get::<i64, _>("date"), r.get::<f64, _>("ram_usage")))
        .collect();
    Ok(json!({
        "cpu": json_from_downsampled_pairs(cpu_pairs),
        "ram": json_from_downsampled_pairs(ram_pairs),
    }))
}

async fn tps_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT t.date, t.ticks_per_second, t.mspt_average, t.mspt_95th_percentile
             FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND t.ticks_per_second > 0
             ORDER BY t.date ASC",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT t.date, AVG(t.ticks_per_second) AS ticks_per_second,
                    AVG(t.mspt_average) AS mspt_average,
                    AVG(t.mspt_95th_percentile) AS mspt_95th_percentile
             FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND t.ticks_per_second > 0
             GROUP BY t.date
             ORDER BY t.date ASC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let tps_pairs = rows
        .iter()
        .map(|r| (r.get::<i64, _>("date"), r.get::<f64, _>("ticks_per_second")))
        .collect();
    let mspt_avg_pairs = rows
        .iter()
        .map(|r| {
            let v: f64 = r.get("mspt_average");
            (r.get::<i64, _>("date"), if v > 0.0 { v } else { 50.0 })
        })
        .collect();
    let mspt_95_pairs = rows
        .iter()
        .map(|r| {
            let v: f64 = r.get("mspt_95th_percentile");
            (r.get::<i64, _>("date"), if v > 0.0 { v } else { 50.0 })
        })
        .collect();
    Ok(json!({
        "tps": json_from_downsampled_pairs(tps_pairs),
        "mspt_avg": json_from_downsampled_pairs(mspt_avg_pairs),
        "mspt_95th": json_from_downsampled_pairs(mspt_95_pairs),
    }))
}

/// Plan downsamples series older than 7 days into 15-minute buckets while keeping extrema.
const SERIES_RECENT_WINDOW_MS: i64 = 7 * 24 * 60 * 60 * 1000;
const SERIES_OLD_BUCKET_MS: i64 = 15 * 60 * 1000;

fn json_from_downsampled_pairs(pairs: Vec<(i64, f64)>) -> Vec<Value> {
    downsample_epoch_series(pairs)
        .into_iter()
        .map(|(t, v)| json!([t, v]))
        .collect()
}

fn downsample_epoch_series(points: Vec<(i64, f64)>) -> Vec<(i64, f64)> {
    if points.len() < 4 {
        return points;
    }
    let now = Utc::now().timestamp_millis();
    let cutoff = now - SERIES_RECENT_WINDOW_MS;
    let mut recent = Vec::new();
    let mut old = Vec::new();
    for p in points {
        if p.0 >= cutoff {
            recent.push(p);
        } else {
            old.push(p);
        }
    }
    if old.is_empty() {
        return recent;
    }
    recent.extend(bucket_preserve_extrema(old, SERIES_OLD_BUCKET_MS));
    recent.sort_by_key(|p| p.0);
    recent
}

fn bucket_preserve_extrema(points: Vec<(i64, f64)>, bucket_ms: i64) -> Vec<(i64, f64)> {
    use std::collections::BTreeMap;
    let mut buckets: BTreeMap<i64, Vec<f64>> = BTreeMap::new();
    for (t, v) in points {
        let b = (t / bucket_ms) * bucket_ms;
        buckets.entry(b).or_default().push(v);
    }
    let mut out = Vec::new();
    for (b, vals) in buckets {
        let min_v = vals.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_v = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        out.push((b, min_v));
        if (max_v - min_v).abs() > f64::EPSILON {
            out.push((b + bucket_ms / 2, max_v));
        }
    }
    out.sort_by_key(|p| p.0);
    out
}

async fn unique_and_new_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    const DAY_MS: i64 = 86_400_000;
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day,
                    COUNT(DISTINCT sess.user_id) AS unique_players
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY day
             ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day,
                    COUNT(DISTINCT sess.user_id) AS unique_players
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
             GROUP BY day
             ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let unique = rows
        .iter()
        .map(|r| {
            json!([
                r.get::<i64, _>("day"),
                r.get::<i64, _>("unique_players"),
                0_i64,
            ])
        })
        .collect::<Vec<_>>();

    let new_rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT (first_fs / ?) * ? AS day, COUNT(*) AS new_players FROM (
                SELECT user_id, MIN(session_start) AS first_fs
                FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ? AND s.uuid = ?
                GROUP BY user_id
             ) GROUP BY day ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT (first_fs / ?) * ? AS day, COUNT(*) AS new_players FROM (
                SELECT user_id, MIN(session_start) AS first_fs
                FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ?
                GROUP BY user_id
             ) GROUP BY day ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let mut new_by_day: std::collections::HashMap<i64, i64> = std::collections::HashMap::new();
    for r in new_rows {
        new_by_day.insert(r.get("day"), r.get("new_players"));
    }

    let series = unique
        .iter()
        .map(|pt| {
            let day = pt[0].as_i64().unwrap_or(0);
            let u = pt[1].as_i64().unwrap_or(0);
            let n = new_by_day.get(&day).copied().unwrap_or(0);
            json!([day, u, n])
        })
        .collect::<Vec<_>>();

    Ok(json!({ "unique_and_new": series }))
}

async fn hourly_unique_and_new_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    const HOUR_MS: i64 = 3_600_000;
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS hour,
                    COUNT(DISTINCT sess.user_id) AS unique_players
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY hour
             ORDER BY hour ASC",
        )
        .bind(HOUR_MS)
        .bind(HOUR_MS)
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS hour,
                    COUNT(DISTINCT sess.user_id) AS unique_players
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
             GROUP BY hour
             ORDER BY hour ASC",
        )
        .bind(HOUR_MS)
        .bind(HOUR_MS)
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let series = rows
        .iter()
        .map(|r| {
            json!([
                r.get::<i64, _>("hour"),
                r.get::<i64, _>("unique_players"),
                0_i64,
            ])
        })
        .collect::<Vec<_>>();
    Ok(json!({ "hourly_unique_and_new": series }))
}

async fn join_addresses_by_day(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    const DAY_MS: i64 = 86_400_000;
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day, sess.join_address, COUNT(*) AS joins
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.join_address != ''
             GROUP BY day, sess.join_address
             ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day, sess.join_address, COUNT(*) AS joins
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.join_address != ''
             GROUP BY day, sess.join_address
             ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let points = rows
        .iter()
        .map(|r| {
            json!({
                "day": r.get::<i64, _>("day"),
                "join_address": r.get::<String, _>("join_address"),
                "joins": r.get::<i64, _>("joins"),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "join_addresses_by_day": points }))
}

async fn server_calendar(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    const DAY_MS: i64 = 86_400_000;
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day,
                    COUNT(DISTINCT sess.user_id) AS unique_players,
                    COUNT(sess.id) AS sessions,
                    SUM(sess.session_end - sess.session_start) AS playtime_ms
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY day
             ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT (sess.session_start / ?) * ? AS day,
                    COUNT(DISTINCT sess.user_id) AS unique_players,
                    COUNT(sess.id) AS sessions,
                    SUM(sess.session_end - sess.session_start) AS playtime_ms
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
             GROUP BY day
             ORDER BY day ASC",
        )
        .bind(DAY_MS)
        .bind(DAY_MS)
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let events = rows
        .iter()
        .map(|r| {
            let day: i64 = r.get("day");
            let unique: i64 = r.get("unique_players");
            let sessions: i64 = r.get("sessions");
            let playtime: i64 = r.get("playtime_ms");
            json!({
                "title": format!("{unique} players"),
                "start": day,
                "allDay": true,
                "extendedProps": {
                    "unique_players": unique,
                    "sessions": sessions,
                    "playtime_ms": playtime,
                },
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({ "calendar_events": events }))
}

async fn ping_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT p.date,
                    CAST(AVG(p.avg_ping) AS INTEGER) AS avg_ping,
                    MIN(p.min_ping) AS min_ping,
                    MAX(p.max_ping) AS max_ping
             FROM plan_ping p
             JOIN plan_servers s ON s.id = p.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY p.date
             ORDER BY p.date ASC",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT p.date,
                    CAST(AVG(p.avg_ping) AS INTEGER) AS avg_ping,
                    MIN(p.min_ping) AS min_ping,
                    MAX(p.max_ping) AS max_ping
             FROM plan_ping p
             JOIN plan_servers s ON s.id = p.server_id
             WHERE s.network_id = ?
             GROUP BY p.date
             ORDER BY p.date ASC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let avg = rows
        .iter()
        .map(|r| json!([r.get::<i64, _>("date"), r.get::<i64, _>("avg_ping")]))
        .collect::<Vec<_>>();
    let min = rows
        .iter()
        .map(|r| json!([r.get::<i64, _>("date"), r.get::<i64, _>("min_ping")]))
        .collect::<Vec<_>>();
    let max = rows
        .iter()
        .map(|r| json!([r.get::<i64, _>("date"), r.get::<i64, _>("max_ping")]))
        .collect::<Vec<_>>();
    Ok(json!({ "avg_ping": avg, "min_ping": min, "max_ping": max }))
}

async fn disk_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT t.date, t.free_disk_space FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND t.free_disk_space > 0
             ORDER BY t.date ASC",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT t.date, AVG(t.free_disk_space) AS free_disk_space FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND t.free_disk_space > 0
             GROUP BY t.date ORDER BY t.date ASC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };
    let disk = rows
        .iter()
        .map(|r| json!([r.get::<i64, _>("date"), r.get::<i64, _>("free_disk_space")]))
        .collect::<Vec<_>>();
    Ok(json!({ "free_disk_space": disk }))
}

async fn world_load_series(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT t.date, t.entities, t.chunks_loaded FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             ORDER BY t.date ASC",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT t.date, SUM(t.entities) AS entities, SUM(t.chunks_loaded) AS chunks_loaded
             FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ?
             GROUP BY t.date ORDER BY t.date ASC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };
    let entities = rows
        .iter()
        .map(|r| json!([r.get::<i64, _>("date"), r.get::<i64, _>("entities")]))
        .collect::<Vec<_>>();
    let chunks = rows
        .iter()
        .map(|r| json!([r.get::<i64, _>("date"), r.get::<i64, _>("chunks_loaded")]))
        .collect::<Vec<_>>();
    Ok(json!({ "entities": entities, "chunks_loaded": chunks }))
}

#[cfg(test)]
mod downsampling_tests {
    use super::*;

    #[test]
    fn keeps_recent_points_and_buckets_old_extrema() {
        let now = Utc::now().timestamp_millis();
        let old = now - SERIES_RECENT_WINDOW_MS - 60_000;
        let points = vec![
            (old, 10.0),
            (old + 30_000, 20.0),
            (now - 1000, 18.0),
        ];
        let out = downsample_epoch_series(points);
        assert!(out.iter().any(|(_, v)| *v == 18.0));
        assert!(out.len() >= 2);
    }
}
