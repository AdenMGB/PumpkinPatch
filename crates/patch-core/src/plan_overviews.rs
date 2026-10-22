//! Plan `/v1/onlineOverview`, `/v1/playerbaseOverview`, `/v1/performanceOverview`, `/v1/retention`.

use crate::error::Result;
use crate::plan_store::{list_players_table, PlanPlayerTableRow};
use patch_plan_core::ONE_WEEK_MS;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

const DAY_MS: i64 = 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOnlineOverviewV1 {
    pub numbers: PlanOnlineNumbers,
    pub insights: PlanOnlineInsights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOnlineNumbers {
    pub unique_players_30d: u64,
    pub unique_players_7d: u64,
    pub unique_players_24h: u64,
    pub unique_players_30d_avg: f64,
    pub new_players_30d: u64,
    pub new_players_7d: u64,
    pub new_players_24h: u64,
    pub new_players_retention_30d: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanOnlineInsights {
    pub first_session_avg_length_ms: u64,
    pub first_session_median_length_ms: u64,
    pub lone_joins: u64,
    pub lone_new_joins: u64,
    pub players_online_on_first_join_avg: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerbaseOverviewV1 {
    pub trends: PlanPlayerbaseTrends,
    pub insights: PlanPlayerbaseInsights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerbaseTrends {
    pub total_players_now: u64,
    pub total_players_then: u64,
    pub total_players_trend: f64,
    pub regular_players_now: u64,
    pub regular_players_then: u64,
    pub regular_players_trend: f64,
    pub playtime_avg_now_ms: u64,
    pub playtime_avg_then_ms: u64,
    pub afk_now_ms: u64,
    pub afk_then_ms: u64,
    pub regular_playtime_avg_now_ms: u64,
    pub regular_session_avg_now: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerbaseInsights {
    pub avg_activity_index: f64,
    pub avg_global_playtime_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPerformanceOverviewV1 {
    pub numbers: PlanPerformanceNumbers,
    pub insights: PlanPerformanceInsights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPerformanceNumbers {
    pub low_tps_spikes_30d: u64,
    pub low_tps_spikes_7d: u64,
    pub low_tps_spikes_24h: u64,
    pub server_downtime_30d_ms: u64,
    pub server_downtime_7d_ms: u64,
    pub server_downtime_24h_ms: u64,
    pub avg_server_uptime_30d: f64,
    pub players_30d: f64,
    pub players_7d: f64,
    pub players_24h: f64,
    pub tps_30d: f64,
    pub tps_7d: f64,
    pub tps_24h: f64,
    pub cpu_30d: f64,
    pub cpu_7d: f64,
    pub cpu_24h: f64,
    pub ram_30d: f64,
    pub ram_7d: f64,
    pub ram_24h: f64,
    pub mspt_average_30d: f64,
    pub mspt_95th_30d: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPerformanceInsights {
    pub low_tps_players: f64,
    pub low_tps_chunks: f64,
    pub low_tps_entities: f64,
    pub low_tps_tps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRetentionV1 {
    pub new_players: Vec<PlanRetentionPlayerRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRetentionPlayerRow {
    pub uuid: String,
    pub name: String,
    pub registered: i64,
    pub last_seen: i64,
    pub playtime_ms: u64,
    pub active_playtime_ms: u64,
    pub session_count: u64,
    pub activity_index: f64,
    pub join_address: String,
}

pub async fn online_overview_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    live_online: u32,
) -> Result<PlanOnlineOverviewV1> {
    let now = chrono::Utc::now().timestamp_millis();
    let d24 = now - DAY_MS;
    let d7 = now - ONE_WEEK_MS;
    let d30 = now - 30 * DAY_MS;

    let u24 = unique_since(pool, network_id, server_uuid, d24).await? as u64;
    let u7 = unique_since(pool, network_id, server_uuid, d7).await? as u64;
    let u30 = unique_since(pool, network_id, server_uuid, d30).await? as u64;

    let n24 = new_players_since(pool, network_id, server_uuid, d24).await? as u64;
    let n7 = new_players_since(pool, network_id, server_uuid, d7).await? as u64;
    let n30 = new_players_since(pool, network_id, server_uuid, d30).await? as u64;

    let retention = new_player_retention(pool, network_id, server_uuid, d30).await?;

    let first_lengths = first_session_lengths(pool, network_id, server_uuid, d30).await?;
    let (avg_first, median_first) = avg_and_median(&first_lengths);

    let lone = lone_joins(pool, network_id, server_uuid, d30).await? as u64;
    let lone_new = lone_new_joins(pool, network_id, server_uuid, d30).await? as u64;

    Ok(PlanOnlineOverviewV1 {
        numbers: PlanOnlineNumbers {
            unique_players_30d: u30,
            unique_players_7d: u7,
            unique_players_24h: u24,
            unique_players_30d_avg: u30 as f64 / 30.0,
            new_players_30d: n30,
            new_players_7d: n7,
            new_players_24h: n24,
            new_players_retention_30d: retention,
        },
        insights: PlanOnlineInsights {
            first_session_avg_length_ms: avg_first,
            first_session_median_length_ms: median_first,
            lone_joins: lone,
            lone_new_joins: lone_new,
            players_online_on_first_join_avg: live_online as f64,
        },
    })
}

pub async fn playerbase_overview_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<PlanPlayerbaseOverviewV1> {
    let now = chrono::Utc::now().timestamp_millis();
    let d30 = now - 30 * DAY_MS;

    let players = list_players_table(pool, network_id, server_uuid, 5000).await?;
    let total_now = players.len() as u64;

    let then_count = players_with_session_before(pool, network_id, server_uuid, d30).await? as u64;
    let trend = percent_change(then_count, total_now);

    let regular_now = count_regular(&players);
    let regular_then = regular_players_before(pool, network_id, server_uuid, d30).await? as u64;
    let regular_trend = percent_change(regular_then, regular_now);

    let (play_avg_now, afk_now) = session_playtime_avgs(pool, network_id, server_uuid, None).await?;
    let (play_avg_then, afk_then) =
        session_playtime_avgs(pool, network_id, server_uuid, Some(d30)).await?;

    let regular_play_avg = regular_playtime_avg(&players);
    let regular_sess_avg = regular_session_avg(&players);

    let avg_ai = if players.is_empty() {
        0.0
    } else {
        players.iter().map(|p| p.activity_index).sum::<f64>() / players.len() as f64
    };
    let avg_play = if players.is_empty() {
        0
    } else {
        players.iter().map(|p| p.active_playtime_ms).sum::<u64>() / players.len() as u64
    };

    Ok(PlanPlayerbaseOverviewV1 {
        trends: PlanPlayerbaseTrends {
            total_players_now: total_now,
            total_players_then: then_count,
            total_players_trend: trend,
            regular_players_now: regular_now,
            regular_players_then: regular_then,
            regular_players_trend: regular_trend,
            playtime_avg_now_ms: play_avg_now,
            playtime_avg_then_ms: play_avg_then,
            afk_now_ms: afk_now,
            afk_then_ms: afk_then,
            regular_playtime_avg_now_ms: regular_play_avg,
            regular_session_avg_now: regular_sess_avg,
        },
        insights: PlanPlayerbaseInsights {
            avg_activity_index: avg_ai,
            avg_global_playtime_ms: avg_play,
        },
    })
}

pub async fn performance_overview_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<PlanPerformanceOverviewV1> {
    let now = chrono::Utc::now().timestamp_millis();
    let d24 = now - DAY_MS;
    let d7 = now - ONE_WEEK_MS;
    let d30 = now - 30 * DAY_MS;

    let s30 = perf_window(pool, network_id, server_uuid, d30).await?;
    let s7 = perf_window(pool, network_id, server_uuid, d7).await?;
    let s24 = perf_window(pool, network_id, server_uuid, d24).await?;

    Ok(PlanPerformanceOverviewV1 {
        numbers: PlanPerformanceNumbers {
            low_tps_spikes_30d: s30.low_tps_spikes,
            low_tps_spikes_7d: s7.low_tps_spikes,
            low_tps_spikes_24h: s24.low_tps_spikes,
            server_downtime_30d_ms: s30.downtime_ms,
            server_downtime_7d_ms: s7.downtime_ms,
            server_downtime_24h_ms: s24.downtime_ms,
            avg_server_uptime_30d: s30.uptime_perc,
            players_30d: s30.avg_players,
            players_7d: s7.avg_players,
            players_24h: s24.avg_players,
            tps_30d: s30.avg_tps,
            tps_7d: s7.avg_tps,
            tps_24h: s24.avg_tps,
            cpu_30d: s30.avg_cpu,
            cpu_7d: s7.avg_cpu,
            cpu_24h: s24.avg_cpu,
            ram_30d: s30.avg_ram,
            ram_7d: s7.avg_ram,
            ram_24h: s24.avg_ram,
            mspt_average_30d: s30.avg_mspt,
            mspt_95th_30d: s30.mspt_95th,
        },
        insights: PlanPerformanceInsights {
            low_tps_players: s30.avg_players,
            low_tps_chunks: 0.0,
            low_tps_entities: 0.0,
            low_tps_tps: s30.avg_tps,
        },
    })
}

pub async fn retention_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<PlanRetentionV1> {
    let now = chrono::Utc::now().timestamp_millis();
    let d30 = now - 30 * DAY_MS;

    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT u.name, u.uuid, u.registered,
                    MAX(sess.session_end) AS last_seen,
                    SUM(sess.session_end - sess.session_start) AS playtime,
                    SUM(sess.session_end - sess.session_start - sess.afk_time) AS active_playtime,
                    COUNT(sess.id) AS session_count,
                    MAX(sess.join_address) AS join_address
             FROM plan_users u
             JOIN plan_sessions sess ON sess.user_id = u.id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY u.id
             HAVING MIN(sess.session_start) >= ?
             ORDER BY u.registered DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(d30)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT u.name, u.uuid, u.registered,
                    MAX(sess.session_end) AS last_seen,
                    SUM(sess.session_end - sess.session_start) AS playtime,
                    SUM(sess.session_end - sess.session_start - sess.afk_time) AS active_playtime,
                    COUNT(sess.id) AS session_count,
                    MAX(sess.join_address) AS join_address
             FROM plan_users u
             JOIN plan_sessions sess ON sess.user_id = u.id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
             GROUP BY u.id
             HAVING MIN(sess.session_start) >= ?
             ORDER BY u.registered DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(d30)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let table = list_players_table(pool, network_id, server_uuid, limit).await?;
    let ai_by_uuid: std::collections::HashMap<String, f64> = table
        .iter()
        .map(|p| (p.uuid.clone(), p.activity_index))
        .collect();

    let new_players = rows
        .iter()
        .map(|r| {
            let uuid: String = r.get("uuid");
            PlanRetentionPlayerRow {
                name: r.get("name"),
                uuid: uuid.clone(),
                registered: r.get("registered"),
                last_seen: r.get("last_seen"),
                playtime_ms: r.get::<i64, _>("playtime") as u64,
                active_playtime_ms: r.get::<i64, _>("active_playtime").max(0) as u64,
                session_count: r.get::<i64, _>("session_count") as u64,
                activity_index: ai_by_uuid.get(&uuid).copied().unwrap_or(0.0),
                join_address: r.try_get("join_address").unwrap_or_default(),
            }
        })
        .collect();

    Ok(PlanRetentionV1 { new_players })
}

struct PerfWindow {
    low_tps_spikes: u64,
    downtime_ms: u64,
    uptime_perc: f64,
    avg_players: f64,
    avg_tps: f64,
    avg_cpu: f64,
    avg_ram: f64,
    avg_mspt: f64,
    mspt_95th: f64,
}

async fn perf_window(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<PerfWindow> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT t.date, t.ticks_per_second, t.players_online, t.cpu_usage, t.ram_usage
             FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND t.date >= ?
             ORDER BY t.date ASC",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT t.date, AVG(t.ticks_per_second) AS ticks_per_second,
                    SUM(t.players_online) AS players_online,
                    AVG(t.cpu_usage) AS cpu_usage,
                    AVG(t.ram_usage) AS ram_usage
             FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ? AND t.date >= ?
             GROUP BY t.date
             ORDER BY t.date ASC",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_all(pool)
        .await?
    };

    if rows.is_empty() {
        return Ok(PerfWindow {
            low_tps_spikes: 0,
            downtime_ms: 0,
            uptime_perc: 100.0,
            avg_players: 0.0,
            avg_tps: 20.0,
            avg_cpu: 0.0,
            avg_ram: 0.0,
            avg_mspt: 50.0,
            mspt_95th: 50.0,
        });
    }

    let mut low_spikes = 0u64;
    let mut downtime_ms = 0i64;
    let mut tps_sum = 0.0;
    let mut cpu_sum = 0.0;
    let mut ram_sum = 0.0;
    let mut players_sum = 0.0;
    let mut mspts = Vec::new();
    let gap_threshold = 5 * 60 * 1000;

    for (i, row) in rows.iter().enumerate() {
        let tps: f64 = row.get("ticks_per_second");
        if tps < 18.0 {
            low_spikes += 1;
        }
        tps_sum += tps;
        cpu_sum += row.get::<f64, _>("cpu_usage");
        ram_sum += row.get::<f64, _>("ram_usage");
        players_sum += row.get::<i64, _>("players_online") as f64;
        let mspt = if tps > 0.1 { 1000.0 / tps } else { 1000.0 };
        mspts.push(mspt);
        if i > 0 {
            let prev_date: i64 = rows[i - 1].get("date");
            let date: i64 = row.get("date");
            let gap = date - prev_date;
            if gap > gap_threshold {
                downtime_ms += gap - gap_threshold;
            }
        }
    }

    let n = rows.len() as f64;
    mspts.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p95_idx = ((mspts.len() as f64) * 0.95).floor() as usize;
    let mspt_95th = mspts.get(p95_idx.min(mspts.len() - 1)).copied().unwrap_or(50.0);
    let avg_mspt = mspts.iter().sum::<f64>() / n;

    let window_ms = rows.last().unwrap().get::<i64, _>("date")
        - rows.first().unwrap().get::<i64, _>("date");
    let uptime_perc = if window_ms > 0 {
        ((window_ms - downtime_ms).max(0) as f64 / window_ms as f64) * 100.0
    } else {
        100.0
    };

    Ok(PerfWindow {
        low_tps_spikes: low_spikes,
        downtime_ms: downtime_ms as u64,
        uptime_perc,
        avg_players: players_sum / n,
        avg_tps: tps_sum / n,
        avg_cpu: cpu_sum / n,
        avg_ram: ram_sum / n,
        avg_mspt,
        mspt_95th,
    })
}

async fn unique_since(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<i64> {
    Ok(if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT sess.user_id) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.session_end >= ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT sess.user_id) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.session_end >= ?",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    })
}

async fn new_players_since(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<i64> {
    Ok(if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ? AND s.uuid = ?
                GROUP BY sess.user_id
                HAVING MIN(sess.session_start) >= ?
             )",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ?
                GROUP BY sess.user_id
                HAVING MIN(sess.session_start) >= ?
             )",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    })
}

async fn new_player_retention(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<f64> {
    let total = new_players_since(pool, network_id, server_uuid, since_ms).await? as f64;
    if total < 1.0 {
        return Ok(0.0);
    }
    let returned: i64 = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ? AND s.uuid = ?
                GROUP BY sess.user_id
                HAVING MIN(sess.session_start) >= ? AND COUNT(sess.id) > 1
             )",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ?
                GROUP BY sess.user_id
                HAVING MIN(sess.session_start) >= ? AND COUNT(sess.id) > 1
             )",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    };
    Ok((returned as f64 / total) * 100.0)
}

async fn first_session_lengths(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<Vec<i64>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT (sess.session_end - sess.session_start) AS len
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             JOIN (
                SELECT user_id, MIN(session_start) AS fs
                FROM plan_sessions
                WHERE server_id = (SELECT id FROM plan_servers WHERE network_id = ? AND uuid = ?)
                GROUP BY user_id
                HAVING fs >= ?
             ) first ON first.user_id = sess.user_id AND sess.session_start = first.fs
             WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT (sess.session_end - sess.session_start) AS len
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             JOIN (
                SELECT user_id, MIN(session_start) AS fs
                FROM plan_sessions sess2
                JOIN plan_servers s2 ON s2.id = sess2.server_id
                WHERE s2.network_id = ?
                GROUP BY user_id
                HAVING fs >= ?
             ) first ON first.user_id = sess.user_id AND sess.session_start = first.fs
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .bind(since_ms)
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };
    Ok(rows.iter().map(|r| r.get::<i64, _>("len")).collect())
}

async fn lone_joins(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<i64> {
    Ok(if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ? AND s.uuid = ? AND sess.session_end >= ?
                GROUP BY sess.user_id
                HAVING COUNT(sess.id) = 1
             )",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ? AND sess.session_end >= ?
                GROUP BY sess.user_id
                HAVING COUNT(sess.id) = 1
             )",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    })
}

async fn lone_new_joins(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<i64> {
    Ok(if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ? AND s.uuid = ?
                GROUP BY sess.user_id
                HAVING MIN(sess.session_start) >= ? AND COUNT(sess.id) = 1
             )",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM (
                SELECT sess.user_id FROM plan_sessions sess
                JOIN plan_servers s ON s.id = sess.server_id
                WHERE s.network_id = ?
                GROUP BY sess.user_id
                HAVING MIN(sess.session_start) >= ? AND COUNT(sess.id) = 1
             )",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    })
}

async fn players_with_session_before(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    before_ms: i64,
) -> Result<i64> {
    Ok(if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT sess.user_id) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.session_start < ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(before_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT sess.user_id) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.session_start < ?",
        )
        .bind(network_id)
        .bind(before_ms)
        .fetch_one(pool)
        .await?
    })
}

async fn regular_players_before(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    before_ms: i64,
) -> Result<i64> {
    let count = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT sess.user_id) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
               AND sess.session_start < ?
               AND sess.session_end - sess.session_start >= ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(before_ms)
        .bind(patch_plan_core::DEFAULT_PLAYTIME_THRESHOLD_MS)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(DISTINCT sess.user_id) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
               AND sess.session_start < ?
               AND sess.session_end - sess.session_start >= ?",
        )
        .bind(network_id)
        .bind(before_ms)
        .bind(patch_plan_core::DEFAULT_PLAYTIME_THRESHOLD_MS)
        .fetch_one(pool)
        .await?
    };
    Ok(count)
}

async fn session_playtime_avgs(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    end_before: Option<i64>,
) -> Result<(u64, u64)> {
    let row = if let Some(uuid) = server_uuid {
        if let Some(before) = end_before {
            sqlx::query(
                "SELECT COALESCE(AVG(session_end - session_start), 0) AS play,
                        COALESCE(AVG(afk_time), 0) AS afk
                 FROM plan_sessions sess
                 JOIN plan_servers s ON s.id = sess.server_id
                 WHERE s.network_id = ? AND s.uuid = ? AND sess.session_end < ?",
            )
            .bind(network_id)
            .bind(uuid)
            .bind(before)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query(
                "SELECT COALESCE(AVG(session_end - session_start), 0) AS play,
                        COALESCE(AVG(afk_time), 0) AS afk
                 FROM plan_sessions sess
                 JOIN plan_servers s ON s.id = sess.server_id
                 WHERE s.network_id = ? AND s.uuid = ?",
            )
            .bind(network_id)
            .bind(uuid)
            .fetch_one(pool)
            .await?
        }
    } else if let Some(before) = end_before {
        sqlx::query(
            "SELECT COALESCE(AVG(session_end - session_start), 0) AS play,
                    COALESCE(AVG(afk_time), 0) AS afk
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.session_end < ?",
        )
        .bind(network_id)
        .bind(before)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT COALESCE(AVG(session_end - session_start), 0) AS play,
                    COALESCE(AVG(afk_time), 0) AS afk
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?
    };
    Ok((
        sqlite_avg_column(&row, "play").round() as u64,
        sqlite_avg_column(&row, "afk").round() as u64,
    ))
}

/// SQLite may return `COALESCE(AVG(...), 0)` as INTEGER when the aggregate is empty.
fn sqlite_avg_column(row: &sqlx::sqlite::SqliteRow, column: &str) -> f64 {
    row.try_get::<f64, _>(column)
        .or_else(|_| row.try_get::<i64, _>(column).map(|v| v as f64))
        .unwrap_or(0.0)
}

fn count_regular(players: &[PlanPlayerTableRow]) -> u64 {
    players
        .iter()
        .filter(|p| {
            matches!(
                p.activity_group.as_str(),
                "Very Active" | "Active" | "Regular"
            )
        })
        .count() as u64
}

fn regular_playtime_avg(players: &[PlanPlayerTableRow]) -> u64 {
    let regular: Vec<_> = players
        .iter()
        .filter(|p| {
            matches!(
                p.activity_group.as_str(),
                "Very Active" | "Active" | "Regular"
            )
        })
        .collect();
    if regular.is_empty() {
        return 0;
    }
    regular.iter().map(|p| p.active_playtime_ms).sum::<u64>() / regular.len() as u64
}

fn regular_session_avg(players: &[PlanPlayerTableRow]) -> f64 {
    let regular: Vec<_> = players
        .iter()
        .filter(|p| {
            matches!(
                p.activity_group.as_str(),
                "Very Active" | "Active" | "Regular"
            )
        })
        .collect();
    if regular.is_empty() {
        return 0.0;
    }
    regular.iter().map(|p| p.sessions).sum::<u64>() as f64 / regular.len() as f64
}

fn percent_change(then: u64, now: u64) -> f64 {
    if then == 0 {
        return if now > 0 { 100.0 } else { 0.0 };
    }
    ((now as f64 - then as f64) / then as f64) * 100.0
}

fn avg_and_median(values: &[i64]) -> (u64, u64) {
    if values.is_empty() {
        return (0, 0);
    }
    let sum = values.iter().sum::<i64>();
    let avg = (sum / values.len() as i64) as u64;
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let mid = sorted[sorted.len() / 2] as u64;
    (avg, mid)
}
