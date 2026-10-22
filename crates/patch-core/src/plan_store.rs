//! Plan-compatible SQLite persistence and ingest (indepth.md §4).

use crate::error::{Error, Result};
use patch_analytics_protocol::{
    AnalyticsPayload, PerformanceSample, PlayerStats, PvpKillRecord, SessionRecord,
};
use patch_plan_core::{active_playtime_in_weeks, ActivityIndex, DEFAULT_PLAYTIME_THRESHOLD_MS};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};

const PLAN_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS plan_servers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    network_id TEXT NOT NULL,
    name TEXT NOT NULL,
    is_proxy INTEGER NOT NULL DEFAULT 0,
    max_players INTEGER NOT NULL DEFAULT -1,
    plan_version TEXT NOT NULL DEFAULT 'patch-plan-0.3'
);

CREATE TABLE IF NOT EXISTS plan_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    uuid TEXT NOT NULL UNIQUE,
    registered BIGINT NOT NULL,
    name TEXT NOT NULL,
    times_kicked INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS plan_user_info (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    server_id INTEGER NOT NULL,
    join_address TEXT NOT NULL DEFAULT '',
    registered BIGINT NOT NULL,
    opped INTEGER NOT NULL DEFAULT 0,
    banned INTEGER NOT NULL DEFAULT 0,
    nick TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    UNIQUE (user_id, server_id)
);

CREATE TABLE IF NOT EXISTS plan_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    server_id INTEGER NOT NULL,
    session_start BIGINT NOT NULL,
    session_end BIGINT NOT NULL,
    mob_kills INTEGER NOT NULL DEFAULT 0,
    deaths INTEGER NOT NULL DEFAULT 0,
    afk_time BIGINT NOT NULL DEFAULT 0,
    join_address TEXT NOT NULL DEFAULT '',
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    UNIQUE (user_id, server_id, session_start, session_end)
);

CREATE TABLE IF NOT EXISTS plan_commandusages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL,
    command TEXT NOT NULL,
    times_used INTEGER NOT NULL DEFAULT 1,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    UNIQUE (server_id, command)
);

CREATE TABLE IF NOT EXISTS plan_tps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL,
    date BIGINT NOT NULL,
    ticks_per_second REAL NOT NULL DEFAULT 20.0,
    players_online INTEGER NOT NULL DEFAULT 0,
    cpu_usage REAL NOT NULL DEFAULT 0,
    ram_usage BIGINT NOT NULL DEFAULT 0,
    entities INTEGER NOT NULL DEFAULT 0,
    chunks_loaded INTEGER NOT NULL DEFAULT 0,
    free_disk_space BIGINT NOT NULL DEFAULT 0,
    mspt_average REAL NOT NULL DEFAULT -1,
    mspt_95th_percentile REAL NOT NULL DEFAULT -1,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS plan_sessions_user_id_index ON plan_sessions(user_id);
CREATE INDEX IF NOT EXISTS plan_sessions_server_id_index ON plan_sessions(server_id);
CREATE INDEX IF NOT EXISTS plan_tps_date_index ON plan_tps(server_id, date);

CREATE TABLE IF NOT EXISTS plan_ping (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    date BIGINT NOT NULL,
    avg_ping INTEGER NOT NULL DEFAULT 0,
    min_ping INTEGER NOT NULL DEFAULT 0,
    max_ping INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS plan_ping_server_date ON plan_ping(server_id, date);

CREATE TABLE IF NOT EXISTS plan_geolocations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    geolocation TEXT NOT NULL,
    last_used BIGINT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    UNIQUE (user_id, geolocation)
);

CREATE INDEX IF NOT EXISTS plan_geolocations_user_index ON plan_geolocations(user_id);

CREATE TABLE IF NOT EXISTS plan_nicknames (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    server_id INTEGER NOT NULL,
    nickname TEXT NOT NULL,
    last_used BIGINT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    UNIQUE (user_id, server_id, nickname)
);

CREATE INDEX IF NOT EXISTS plan_nicknames_user_index ON plan_nicknames(user_id);

CREATE TABLE IF NOT EXISTS plan_query_cache (
    token TEXT PRIMARY KEY,
    network_id TEXT NOT NULL,
    server_uuid TEXT NOT NULL DEFAULT '',
    request_json TEXT NOT NULL,
    result_json TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS plan_query_cache_expires ON plan_query_cache(expires_at_ms);

CREATE TABLE IF NOT EXISTS plan_kills (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    killer_uuid TEXT NOT NULL,
    victim_uuid TEXT NOT NULL,
    server_uuid TEXT NOT NULL,
    session_id INTEGER,
    date BIGINT NOT NULL,
    weapon TEXT NOT NULL,
    killer_name TEXT NOT NULL DEFAULT '',
    victim_name TEXT NOT NULL DEFAULT '',
    UNIQUE (killer_uuid, victim_uuid, server_uuid, date, weapon)
);

CREATE TABLE IF NOT EXISTS plan_world_times (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    world_name TEXT NOT NULL,
    survival_time BIGINT NOT NULL DEFAULT 0,
    creative_time BIGINT NOT NULL DEFAULT 0,
    adventure_time BIGINT NOT NULL DEFAULT 0,
    spectator_time BIGINT NOT NULL DEFAULT 0,
    FOREIGN KEY (session_id) REFERENCES plan_sessions(id) ON DELETE CASCADE,
    UNIQUE (session_id, world_name)
);

CREATE INDEX IF NOT EXISTS plan_kills_date_index ON plan_kills(date);

CREATE TABLE IF NOT EXISTS plan_web_group (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS plan_web_permission (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    permission TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS plan_web_group_to_permission (
    group_id INTEGER NOT NULL,
    permission_id INTEGER NOT NULL,
    FOREIGN KEY (group_id) REFERENCES plan_web_group(id) ON DELETE CASCADE,
    FOREIGN KEY (permission_id) REFERENCES plan_web_permission(id) ON DELETE CASCADE,
    UNIQUE (group_id, permission_id)
);

CREATE TABLE IF NOT EXISTS plan_security (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    salted_pass_hash TEXT NOT NULL,
    linked_to_uuid TEXT NOT NULL DEFAULT '',
    group_id INTEGER NOT NULL,
    FOREIGN KEY (group_id) REFERENCES plan_web_group(id)
);

CREATE TABLE IF NOT EXISTS plan_cookies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    web_username TEXT NOT NULL,
    expires BIGINT NOT NULL,
    cookie TEXT NOT NULL UNIQUE,
    FOREIGN KEY (web_username) REFERENCES plan_security(username) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS plan_access_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    time BIGINT NOT NULL,
    from_ip TEXT,
    request_method TEXT NOT NULL,
    request_uri TEXT NOT NULL,
    response_code INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS plan_pending_registration (
    code TEXT PRIMARY KEY,
    username TEXT NOT NULL,
    salted_pass_hash TEXT NOT NULL,
    created_ms BIGINT NOT NULL,
    expires_ms BIGINT NOT NULL
);
";

pub async fn ensure_plan_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(PLAN_SCHEMA).execute(pool).await?;
    crate::plan_auth::ensure_plan_web_security(pool).await?;
    let _ = sqlx::query("ALTER TABLE plan_user_info ADD COLUMN nick TEXT NOT NULL DEFAULT ''")
        .execute(pool)
        .await;
    let _ = sqlx::query(
        "CREATE UNIQUE INDEX IF NOT EXISTS plan_tps_server_minute ON plan_tps(server_id, date)",
    )
    .execute(pool)
    .await;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSyncReport {
    pub servers_synced: u32,
    pub users_upserted: u32,
    pub sessions_inserted: u32,
    pub commands_merged: u32,
    pub activity_points_inserted: u32,
    pub kills_inserted: u32,
    pub world_time_rows_inserted: u32,
    pub performance_samples_inserted: u32,
    pub geolocation_rows_upserted: u32,
    pub ping_rows_inserted: u32,
}

pub async fn ingest_payload(
    pool: &SqlitePool,
    network_id: &str,
    payload: &AnalyticsPayload,
) -> Result<PlanSyncReport> {
    let mut report = PlanSyncReport {
        servers_synced: 0,
        users_upserted: 0,
        sessions_inserted: 0,
        commands_merged: 0,
        activity_points_inserted: 0,
        kills_inserted: 0,
        world_time_rows_inserted: 0,
        performance_samples_inserted: 0,
        geolocation_rows_upserted: 0,
        ping_rows_inserted: 0,
    };

    let server_id = upsert_plan_server(pool, network_id, payload).await?;
    let server_uuid = payload.server_id.clone();
    report.servers_synced = 1;

    for player in &payload.players {
        let user_id = upsert_plan_user(pool, player).await?;
        upsert_user_info(pool, user_id, server_id, player).await?;
        report.users_upserted += 1;

        if !player.geolocation.is_empty() {
            if upsert_geolocation(pool, user_id, &player.geolocation, payload.generated_at_unix * 1000)
                .await?
            {
                report.geolocation_rows_upserted += 1;
            }
        }

        for window in &player.ping_windows {
            if insert_ping_window(pool, server_id, user_id, window).await? {
                report.ping_rows_inserted += 1;
            }
        }

        for session in &player.completed_sessions {
            if let Some(session_row_id) =
                upsert_session(pool, user_id, server_id, session).await?
            {
                if session_row_id > 0 {
                    report.sessions_inserted += 1;
                }
                for wt in &session.world_times {
                    if insert_world_times(pool, session_row_id, wt).await? {
                        report.world_time_rows_inserted += 1;
                    }
                }
            }
        }
    }

    for kill in &payload.recent_pvp_kills {
        if insert_kill(pool, &server_uuid, kill).await? {
            report.kills_inserted += 1;
        }
    }

    for (command, times) in &payload.command_usages {
        merge_command_usage(pool, server_id, command, *times).await?;
        report.commands_merged += 1;
    }

    for sample in &payload.activity_samples {
        if insert_activity_point(pool, server_id, sample.t * 1000, sample.online).await? {
            report.activity_points_inserted += 1;
        }
    }

    for bucket in aggregate_performance_minutes(&payload.performance_samples) {
        if upsert_minute_performance(pool, server_id, &bucket).await? {
            report.performance_samples_inserted += 1;
        }
    }

    Ok(report)
}

#[derive(Debug, Clone)]
struct MinutePerformanceBucket {
    date_ms: i64,
    avg_tps: f64,
    max_players: i64,
    avg_mspt: f64,
    mspt_p95: f64,
}

fn aggregate_performance_minutes(samples: &[PerformanceSample]) -> Vec<MinutePerformanceBucket> {
    use std::collections::HashMap;
    let mut by_minute: HashMap<i64, Vec<&PerformanceSample>> = HashMap::new();
    for sample in samples {
        let minute = (sample.t / 60) * 60;
        by_minute.entry(minute).or_default().push(sample);
    }
    by_minute
        .into_iter()
        .map(|(minute, group)| {
            let n = group.len() as f64;
            let avg_tps = group.iter().map(|s| s.ticks_per_second).sum::<f64>() / n;
            let max_players = group
                .iter()
                .map(|s| s.players_online as i64)
                .max()
                .unwrap_or(0);
            let mut mspts: Vec<f64> = group.iter().map(|s| s.mspt_avg).collect();
            mspts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let avg_mspt = mspts.iter().sum::<f64>() / n;
            let idx = ((mspts.len() as f64) * 0.95).ceil() as usize;
            let mspt_p95 = mspts
                .get(idx.saturating_sub(1))
                .copied()
                .unwrap_or(*mspts.last().unwrap_or(&avg_mspt));
            MinutePerformanceBucket {
                date_ms: minute * 1000,
                avg_tps,
                max_players,
                avg_mspt,
                mspt_p95,
            }
        })
        .collect()
}

async fn upsert_minute_performance(
    pool: &SqlitePool,
    server_id: i64,
    bucket: &MinutePerformanceBucket,
) -> Result<bool> {
    let result = sqlx::query(
        "INSERT INTO plan_tps
         (server_id, date, ticks_per_second, players_online, mspt_average, mspt_95th_percentile)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(server_id, date) DO UPDATE SET
           ticks_per_second = excluded.ticks_per_second,
           players_online = MAX(plan_tps.players_online, excluded.players_online),
           mspt_average = excluded.mspt_average,
           mspt_95th_percentile = excluded.mspt_95th_percentile",
    )
    .bind(server_id)
    .bind(bucket.date_ms)
    .bind(bucket.avg_tps)
    .bind(bucket.max_players)
    .bind(bucket.avg_mspt)
    .bind(bucket.mspt_p95)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

async fn upsert_plan_server(
    pool: &SqlitePool,
    network_id: &str,
    payload: &AnalyticsPayload,
) -> Result<i64> {
    sqlx::query(
        "INSERT INTO plan_servers (uuid, network_id, name) VALUES (?, ?, ?)
         ON CONFLICT(uuid) DO UPDATE SET name = excluded.name, network_id = excluded.network_id",
    )
    .bind(&payload.server_id)
    .bind(network_id)
    .bind(&payload.server_name)
    .execute(pool)
    .await?;

    let row = sqlx::query("SELECT id FROM plan_servers WHERE uuid = ?")
        .bind(&payload.server_id)
        .fetch_one(pool)
        .await?;
    Ok(row.get("id"))
}

async fn upsert_plan_user(pool: &SqlitePool, player: &PlayerStats) -> Result<i64> {
    let registered_ms = player.first_seen_unix * 1000;
    sqlx::query(
        "INSERT INTO plan_users (uuid, registered, name, times_kicked)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(uuid) DO UPDATE SET
           name = excluded.name,
           times_kicked = MAX(plan_users.times_kicked, excluded.times_kicked)",
    )
    .bind(&player.uuid)
    .bind(registered_ms)
    .bind(&player.name)
    .bind(player.kick_count as i64)
    .execute(pool)
    .await?;

    let row = sqlx::query("SELECT id FROM plan_users WHERE uuid = ?")
        .bind(&player.uuid)
        .fetch_one(pool)
        .await?;
    Ok(row.get("id"))
}

async fn upsert_geolocation(
    pool: &SqlitePool,
    user_id: i64,
    geolocation: &str,
    last_used_ms: i64,
) -> Result<bool> {
    let result = sqlx::query(
        "INSERT INTO plan_geolocations (user_id, geolocation, last_used)
         VALUES (?, ?, ?)
         ON CONFLICT(user_id, geolocation) DO UPDATE SET
           last_used = MAX(plan_geolocations.last_used, excluded.last_used)",
    )
    .bind(user_id)
    .bind(geolocation)
    .bind(last_used_ms)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

async fn insert_ping_window(
    pool: &SqlitePool,
    server_id: i64,
    user_id: i64,
    window: &patch_analytics_protocol::PingWindowRecord,
) -> Result<bool> {
    let date_ms = window.t_unix * 1000;
    let result = sqlx::query(
        "INSERT OR IGNORE INTO plan_ping (server_id, user_id, date, avg_ping, min_ping, max_ping)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(server_id)
    .bind(user_id)
    .bind(date_ms)
    .bind(window.avg_ping as i64)
    .bind(window.min_ping as i64)
    .bind(window.max_ping as i64)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

async fn upsert_plan_nickname(
    pool: &SqlitePool,
    user_id: i64,
    server_id: i64,
    nickname: &str,
    last_used_ms: i64,
) -> Result<()> {
    if nickname.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO plan_nicknames (user_id, server_id, nickname, last_used)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(user_id, server_id, nickname) DO UPDATE SET
           last_used = MAX(plan_nicknames.last_used, excluded.last_used)",
    )
    .bind(user_id)
    .bind(server_id)
    .bind(nickname)
    .bind(last_used_ms)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_user_info(
    pool: &SqlitePool,
    user_id: i64,
    server_id: i64,
    player: &PlayerStats,
) -> Result<()> {
    let registered_ms = player.first_seen_unix * 1000;
    let last_seen_ms = player.last_seen_unix * 1000;
    sqlx::query(
        "INSERT INTO plan_user_info (user_id, server_id, join_address, registered, nick)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(user_id, server_id) DO UPDATE SET
           join_address = excluded.join_address,
           registered = MIN(plan_user_info.registered, excluded.registered),
           nick = CASE WHEN excluded.nick != '' THEN excluded.nick ELSE plan_user_info.nick END",
    )
    .bind(user_id)
    .bind(server_id)
    .bind(&player.join_address)
    .bind(registered_ms)
    .bind(&player.nick)
    .execute(pool)
    .await?;
    let display_nick = if player.nick.is_empty() {
        player.name.as_str()
    } else {
        player.nick.as_str()
    };
    upsert_plan_nickname(pool, user_id, server_id, display_nick, last_seen_ms).await?;
    Ok(())
}

async fn upsert_session(
    pool: &SqlitePool,
    user_id: i64,
    server_id: i64,
    session: &SessionRecord,
) -> Result<Option<i64>> {
    let start_ms = session.start_unix * 1000;
    let end_ms = session.end_unix * 1000;
    let afk_ms = session.afk_secs * 1000;
    sqlx::query(
        "INSERT OR IGNORE INTO plan_sessions
         (user_id, server_id, session_start, session_end, mob_kills, deaths, afk_time, join_address)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(user_id)
    .bind(server_id)
    .bind(start_ms)
    .bind(end_ms)
    .bind(session.mob_kills)
    .bind(session.deaths)
    .bind(afk_ms as i64)
    .bind(&session.join_address)
    .execute(pool)
    .await?;

    let row = sqlx::query(
        "SELECT id FROM plan_sessions WHERE user_id = ? AND server_id = ? AND session_start = ? AND session_end = ?",
    )
    .bind(user_id)
    .bind(server_id)
    .bind(start_ms)
    .bind(end_ms)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.get("id")))
}

async fn insert_world_times(
    pool: &SqlitePool,
    session_id: i64,
    wt: &patch_analytics_protocol::WorldTimeSlice,
) -> Result<bool> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO plan_world_times
         (session_id, world_name, survival_time, creative_time, adventure_time, spectator_time)
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(session_id)
    .bind(&wt.world)
    .bind(wt.survival_ms as i64)
    .bind(wt.creative_ms as i64)
    .bind(wt.adventure_ms as i64)
    .bind(wt.spectator_ms as i64)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

async fn insert_kill(pool: &SqlitePool, server_uuid: &str, kill: &PvpKillRecord) -> Result<bool> {
    let date_ms = kill.t * 1000;
    let result = sqlx::query(
        "INSERT OR IGNORE INTO plan_kills
         (killer_uuid, victim_uuid, server_uuid, date, weapon, killer_name, victim_name)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&kill.killer_uuid)
    .bind(&kill.victim_uuid)
    .bind(server_uuid)
    .bind(date_ms)
    .bind(&kill.weapon)
    .bind(&kill.killer_name)
    .bind(&kill.victim_name)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

async fn merge_command_usage(
    pool: &SqlitePool,
    server_id: i64,
    command: &str,
    times: u64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO plan_commandusages (server_id, command, times_used)
         VALUES (?, ?, ?)
         ON CONFLICT(server_id, command) DO UPDATE SET
           times_used = MAX(plan_commandusages.times_used, excluded.times_used)",
    )
    .bind(server_id)
    .bind(command)
    .bind(times as i64)
    .execute(pool)
    .await?;
    Ok(())
}

async fn insert_activity_point(
    pool: &SqlitePool,
    server_id: i64,
    date_ms: i64,
    players_online: u32,
) -> Result<bool> {
    let result = sqlx::query(
        "INSERT OR IGNORE INTO plan_tps (server_id, date, ticks_per_second, players_online)
         VALUES (?, ?, 20.0, ?)",
    )
    .bind(server_id)
    .bind(date_ms)
    .bind(players_online as i64)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPingTableRow {
    pub country: String,
    pub avg_ping: u32,
    pub min_ping: u32,
    pub max_ping: u32,
}

pub async fn ping_table_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Vec<PlanPingTableRow>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT COALESCE(g.geolocation, 'Unknown') AS country,
                    CAST(AVG(p.avg_ping) AS INTEGER) AS avg_ping,
                    MIN(p.min_ping) AS min_ping,
                    MAX(p.max_ping) AS max_ping
             FROM plan_ping p
             JOIN plan_servers s ON s.id = p.server_id
             LEFT JOIN plan_geolocations g ON g.user_id = p.user_id
               AND g.last_used = (
                 SELECT MAX(last_used) FROM plan_geolocations WHERE user_id = p.user_id
               )
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY country
             ORDER BY avg_ping ASC",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT COALESCE(g.geolocation, 'Unknown') AS country,
                    CAST(AVG(p.avg_ping) AS INTEGER) AS avg_ping,
                    MIN(p.min_ping) AS min_ping,
                    MAX(p.max_ping) AS max_ping
             FROM plan_ping p
             JOIN plan_servers s ON s.id = p.server_id
             LEFT JOIN plan_geolocations g ON g.user_id = p.user_id
               AND g.last_used = (
                 SELECT MAX(last_used) FROM plan_geolocations WHERE user_id = p.user_id
               )
             WHERE s.network_id = ?
             GROUP BY country
             ORDER BY avg_ping ASC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .iter()
        .map(|r| PlanPingTableRow {
            country: r.get("country"),
            avg_ping: r.get::<i64, _>("avg_ping") as u32,
            min_ping: r.get::<i64, _>("min_ping") as u32,
            max_ping: r.get::<i64, _>("max_ping") as u32,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanSessionWorldSlice {
    pub name: String,
    pub y: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanSessionGmSeries {
    pub survival: u64,
    pub creative: u64,
    pub adventure: u64,
    pub spectator: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSessionRow {
    pub player_name: String,
    pub player_uuid: String,
    pub server_name: String,
    pub server_uuid: String,
    pub start: i64,
    pub end: i64,
    pub length: i64,
    pub afk_time: i64,
    pub mob_kills: i32,
    pub deaths: i32,
    pub join_address: String,
    #[serde(default)]
    pub first_session: bool,
    #[serde(default)]
    pub world_series: Vec<PlanSessionWorldSlice>,
    #[serde(default)]
    pub gm_series: PlanSessionGmSeries,
    #[serde(default)]
    pub player_kills: Vec<PlanKillRow>,
}

pub async fn list_sessions(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<Vec<PlanSessionRow>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT sess.id AS session_id, sess.user_id, u.name, u.uuid, s.name AS server_name, s.uuid AS server_uuid,
                    sess.session_start, sess.session_end, sess.afk_time,
                    sess.mob_kills, sess.deaths, sess.join_address
             FROM plan_sessions sess
             JOIN plan_users u ON u.id = sess.user_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             ORDER BY sess.session_end DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT sess.id AS session_id, sess.user_id, u.name, u.uuid, s.name AS server_name, s.uuid AS server_uuid,
                    sess.session_start, sess.session_end, sess.afk_time,
                    sess.mob_kills, sess.deaths, sess.join_address
             FROM plan_sessions sess
             JOIN plan_users u ON u.id = sess.user_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
             ORDER BY sess.session_end DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let mut out: Vec<PlanSessionRow> = rows
        .iter()
        .map(|row| {
            let start: i64 = row.get("session_start");
            let end: i64 = row.get("session_end");
            PlanSessionRow {
                player_name: row.get("name"),
                player_uuid: row.get("uuid"),
                server_name: row.get("server_name"),
                server_uuid: row.get("server_uuid"),
                start,
                end,
                length: end - start,
                afk_time: row.get("afk_time"),
                mob_kills: row.get("mob_kills"),
                deaths: row.get("deaths"),
                join_address: row.get("join_address"),
                first_session: false,
                world_series: Vec::new(),
                gm_series: PlanSessionGmSeries::default(),
                player_kills: Vec::new(),
            }
        })
        .collect();

    if out.is_empty() {
        return Ok(out);
    }

    let session_ids: Vec<i64> = rows.iter().map(|r| r.get("session_id")).collect();
    let user_ids: Vec<i64> = rows
        .iter()
        .map(|r| r.get::<i64, _>("user_id"))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    enrich_session_rows(pool, &session_ids, &user_ids, &rows, &mut out).await?;
    Ok(out)
}

pub async fn list_player_sessions(
    pool: &SqlitePool,
    network_id: &str,
    player_uuid: &str,
    limit: i64,
) -> Result<Vec<PlanSessionRow>> {
    let rows = sqlx::query(
        "SELECT sess.id AS session_id, sess.user_id, u.name, u.uuid, s.name AS server_name, s.uuid AS server_uuid,
                sess.session_start, sess.session_end, sess.afk_time,
                sess.mob_kills, sess.deaths, sess.join_address
         FROM plan_sessions sess
         JOIN plan_users u ON u.id = sess.user_id
         JOIN plan_servers s ON s.id = sess.server_id
         WHERE s.network_id = ? AND u.uuid = ?
         ORDER BY sess.session_end DESC
         LIMIT ?",
    )
    .bind(network_id)
    .bind(player_uuid)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut out: Vec<PlanSessionRow> = rows
        .iter()
        .map(|row| {
            let start: i64 = row.get("session_start");
            let end: i64 = row.get("session_end");
            PlanSessionRow {
                player_name: row.get("name"),
                player_uuid: row.get("uuid"),
                server_name: row.get("server_name"),
                server_uuid: row.get("server_uuid"),
                start,
                end,
                length: end - start,
                afk_time: row.get("afk_time"),
                mob_kills: row.get("mob_kills"),
                deaths: row.get("deaths"),
                join_address: row.get("join_address"),
                first_session: false,
                world_series: Vec::new(),
                gm_series: PlanSessionGmSeries::default(),
                player_kills: Vec::new(),
            }
        })
        .collect();

    if out.is_empty() {
        return Ok(out);
    }

    let session_ids: Vec<i64> = rows.iter().map(|r| r.get("session_id")).collect();
    let user_ids: Vec<i64> = rows
        .iter()
        .map(|r| r.get::<i64, _>("user_id"))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    enrich_session_rows(pool, &session_ids, &user_ids, &rows, &mut out).await?;
    Ok(out)
}

async fn enrich_session_rows(
    pool: &SqlitePool,
    session_ids: &[i64],
    user_ids: &[i64],
    raw_rows: &[sqlx::sqlite::SqliteRow],
    out: &mut [PlanSessionRow],
) -> Result<()> {
    let placeholders = session_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let world_sql = format!(
        "SELECT session_id, world_name, survival_time, creative_time, adventure_time, spectator_time
         FROM plan_world_times WHERE session_id IN ({placeholders})"
    );
    let mut world_q = sqlx::query(&world_sql);
    for id in session_ids {
        world_q = world_q.bind(id);
    }
    let world_rows = world_q.fetch_all(pool).await?;

    let mut world_by_session: std::collections::HashMap<i64, Vec<&sqlx::sqlite::SqliteRow>> =
        std::collections::HashMap::new();
    for wr in &world_rows {
        let sid: i64 = wr.get("session_id");
        world_by_session.entry(sid).or_default().push(wr);
    }

    let user_placeholders = user_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let first_sql = format!(
        "SELECT user_id, MIN(session_start) AS first_start FROM plan_sessions
         WHERE user_id IN ({user_placeholders}) GROUP BY user_id"
    );
    let mut first_q = sqlx::query(&first_sql);
    for uid in user_ids {
        first_q = first_q.bind(uid);
    }
    let first_rows = first_q.fetch_all(pool).await?;
    let mut first_start_by_user: std::collections::HashMap<i64, i64> =
        std::collections::HashMap::new();
    for fr in first_rows {
        first_start_by_user.insert(fr.get("user_id"), fr.get("first_start"));
    }

    for (idx, raw) in raw_rows.iter().enumerate() {
        let session_id: i64 = raw.get("session_id");
        let user_id: i64 = raw.get("user_id");
        let start: i64 = raw.get("session_start");
        if first_start_by_user.get(&user_id) == Some(&start) {
            out[idx].first_session = true;
        }
        let mut gm = PlanSessionGmSeries::default();
        let mut world_series = Vec::new();
        if let Some(wts) = world_by_session.get(&session_id) {
            for wt in wts {
                let survival: i64 = wt.get("survival_time");
                let creative: i64 = wt.get("creative_time");
                let adventure: i64 = wt.get("adventure_time");
                let spectator: i64 = wt.get("spectator_time");
                gm.survival += survival as u64;
                gm.creative += creative as u64;
                gm.adventure += adventure as u64;
                gm.spectator += spectator as u64;
                let total = (survival + creative + adventure + spectator) as u64;
                if total > 0 {
                    world_series.push(PlanSessionWorldSlice {
                        name: wt.get("world_name"),
                        y: total,
                    });
                }
            }
        }
        out[idx].world_series = world_series;
        out[idx].gm_series = gm;

        let player_uuid: String = raw.get("uuid");
        let start: i64 = raw.get("session_start");
        let end: i64 = raw.get("session_end");
        let kill_rows = sqlx::query(
            "SELECT killer_name, killer_uuid, victim_name, victim_uuid, server_uuid, weapon, date
             FROM plan_kills
             WHERE killer_uuid = ? AND date >= ? AND date <= ?
             ORDER BY date DESC LIMIT 12",
        )
        .bind(&player_uuid)
        .bind(start)
        .bind(end)
        .fetch_all(pool)
        .await?;
        out[idx].player_kills = kill_rows
            .iter()
            .map(|kr| PlanKillRow {
                killer_name: kr.get("killer_name"),
                killer_uuid: kr.get("killer_uuid"),
                victim_name: kr.get("victim_name"),
                victim_uuid: kr.get("victim_uuid"),
                server_uuid: kr.get("server_uuid"),
                weapon: kr.get("weapon"),
                date: kr.get("date"),
            })
            .collect();
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCommandUsageRow {
    pub command: String,
    pub times_used: u64,
    pub server_name: String,
}

pub async fn list_command_usages(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<Vec<PlanCommandUsageRow>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT c.command, c.times_used, s.name AS server_name
             FROM plan_commandusages c
             JOIN plan_servers s ON s.id = c.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             ORDER BY c.times_used DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT c.command, SUM(c.times_used) AS times_used, 'network' AS server_name
             FROM plan_commandusages c
             JOIN plan_servers s ON s.id = c.server_id
             WHERE s.network_id = ?
             GROUP BY c.command
             ORDER BY times_used DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .iter()
        .map(|row| PlanCommandUsageRow {
            command: row.get("command"),
            times_used: row.get::<i64, _>("times_used") as u64,
            server_name: row.get("server_name"),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerTableRow {
    pub name: String,
    pub uuid: String,
    pub activity_index: f64,
    pub activity_group: String,
    pub active_playtime_ms: u64,
    pub sessions: u64,
    pub registered: i64,
    pub last_seen: i64,
    pub join_address: String,
    pub geolocation: String,
    pub ping_avg: u32,
    pub ping_min: u32,
    pub ping_max: u32,
    pub nick: String,
    pub kick_count: u64,
}

pub async fn list_players_table(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<Vec<PlanPlayerTableRow>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT u.name, u.uuid, u.registered, u.times_kicked, ui.join_address, ui.nick,
                    (SELECT MAX(session_end) FROM plan_sessions WHERE user_id = u.id) AS last_seen,
                    (SELECT COUNT(*) FROM plan_sessions WHERE user_id = u.id AND server_id = s.id) AS sessions
             FROM plan_users u
             JOIN plan_user_info ui ON ui.user_id = u.id
             JOIN plan_servers s ON s.id = ui.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             ORDER BY last_seen DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT u.name, u.uuid, u.registered, MAX(u.times_kicked) AS times_kicked,
                    MAX(ui.join_address) AS join_address, MAX(ui.nick) AS nick,
                    MAX(sess.session_end) AS last_seen,
                    COUNT(sess.id) AS sessions
             FROM plan_users u
             JOIN plan_sessions sess ON sess.user_id = u.id
             JOIN plan_servers s ON s.id = sess.server_id
             LEFT JOIN plan_user_info ui ON ui.user_id = u.id AND ui.server_id = s.id
             WHERE s.network_id = ?
             GROUP BY u.id
             ORDER BY last_seen DESC
             LIMIT ?",
        )
        .bind(network_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    let now_ms = chrono::Utc::now().timestamp_millis();
    let mut out = Vec::new();
    for row in rows {
        let user_uuid: String = row.get("uuid");
        let session_rows = sqlx::query(
            "SELECT session_start, session_end, afk_time FROM plan_sessions sess
             JOIN plan_users u ON u.id = sess.user_id
             WHERE u.uuid = ?",
        )
        .bind(&user_uuid)
        .fetch_all(pool)
        .await?;

        let triples: Vec<(i64, i64, u64)> = session_rows
            .iter()
            .map(|r| {
                (
                    r.get::<i64, _>("session_start"),
                    r.get::<i64, _>("session_end"),
                    r.get::<i64, _>("afk_time") as u64,
                )
            })
            .collect();
        let (w1, w2, w3) = active_playtime_in_weeks(&triples, now_ms);
        let ai = ActivityIndex::calculate(w1, w2, w3, DEFAULT_PLAYTIME_THRESHOLD_MS);
        let active_playtime_ms = w1 + w2 + w3;

        let geo_row = sqlx::query(
            "SELECT g.geolocation FROM plan_geolocations g
             JOIN plan_users u ON u.id = g.user_id
             WHERE u.uuid = ?
             ORDER BY g.last_used DESC LIMIT 1",
        )
        .bind(&user_uuid)
        .fetch_optional(pool)
        .await?;
        let geolocation = geo_row
            .map(|r| r.get::<String, _>("geolocation"))
            .unwrap_or_default();

        let ping_row = sqlx::query(
            "SELECT CAST(AVG(p.avg_ping) AS INTEGER) AS avg_ping,
                    MIN(p.min_ping) AS min_ping,
                    MAX(p.max_ping) AS max_ping
             FROM plan_ping p
             JOIN plan_users u ON u.id = p.user_id
             WHERE u.uuid = ?",
        )
        .bind(&user_uuid)
        .fetch_optional(pool)
        .await?;
        let (ping_avg, ping_min, ping_max) = if let Some(pr) = ping_row {
            (
                pr.get::<i64, _>("avg_ping") as u32,
                pr.get::<i64, _>("min_ping") as u32,
                pr.get::<i64, _>("max_ping") as u32,
            )
        } else {
            (0, 0, 0)
        };

        out.push(PlanPlayerTableRow {
            name: row.get("name"),
            uuid: user_uuid,
            activity_index: ai.value,
            activity_group: ai.group.label().to_string(),
            active_playtime_ms,
            sessions: row.get::<i64, _>("sessions") as u64,
            registered: row.get("registered"),
            last_seen: row.try_get("last_seen").unwrap_or(0),
            join_address: row.try_get("join_address").unwrap_or_default(),
            geolocation,
            ping_avg,
            ping_min,
            ping_max,
            nick: row.try_get("nick").unwrap_or_default(),
            kick_count: row.try_get::<i64, _>("times_kicked").unwrap_or(0) as u64,
        });
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGraphPlayersOnline {
    pub series: Vec<[i64; 2]>,
}

pub async fn graph_players_online(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<PlanGraphPlayersOnline> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT t.date, t.players_online FROM plan_tps t
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
            "SELECT t.date, SUM(t.players_online) AS players_online FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.network_id = ?
             GROUP BY t.date
             ORDER BY t.date ASC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let series = rows
        .iter()
        .map(|row| {
            let date: i64 = row.get("date");
            let online: i64 = row.get("players_online");
            [date, online]
        })
        .collect();
    Ok(PlanGraphPlayersOnline { series })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanServerOverviewV1 {
    pub server_uuid: String,
    pub server_name: String,
    pub numbers: PlanServerNumbers,
    pub last_7_days: PlanLast7Days,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanServerNumbers {
    pub total_players: u64,
    pub online_players: u32,
    pub total_sessions: u64,
    pub total_playtime_ms: u64,
    pub total_deaths: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanLast7Days {
    pub unique_players: u64,
    pub session_count: u64,
    pub avg_session_length_ms: u64,
}

pub async fn server_overview_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: &str,
    live_online: u32,
) -> Result<PlanServerOverviewV1> {
    let server_row = sqlx::query(
        "SELECT id, name FROM plan_servers WHERE network_id = ? AND uuid = ?",
    )
    .bind(network_id)
    .bind(server_uuid)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| Error::NotFound(format!("plan server {server_uuid}")))?;

    let server_id: i64 = server_row.get("id");
    let server_name: String = server_row.get("name");

    let week_ago_ms = chrono::Utc::now().timestamp_millis() - patch_plan_core::ONE_WEEK_MS;

    let total_players: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM plan_user_info WHERE server_id = ?",
    )
    .bind(server_id)
    .fetch_one(pool)
    .await?;

    let total_sessions: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM plan_sessions WHERE server_id = ?",
    )
    .bind(server_id)
    .fetch_one(pool)
    .await?;

    let total_playtime_ms: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(session_end - session_start), 0) FROM plan_sessions WHERE server_id = ?",
    )
    .bind(server_id)
    .fetch_one(pool)
    .await?;

    let total_deaths: i64 = sqlx::query_scalar(
        "SELECT COALESCE(SUM(deaths), 0) FROM plan_sessions WHERE server_id = ?",
    )
    .bind(server_id)
    .fetch_one(pool)
    .await?;

    let unique_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT user_id) FROM plan_sessions
         WHERE server_id = ? AND session_end >= ?",
    )
    .bind(server_id)
    .bind(week_ago_ms)
    .fetch_one(pool)
    .await?;

    let sessions_7d: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM plan_sessions WHERE server_id = ? AND session_end >= ?",
    )
    .bind(server_id)
    .bind(week_ago_ms)
    .fetch_one(pool)
    .await?;

    let avg_len: f64 = sqlx::query_scalar(
        "SELECT COALESCE(AVG(session_end - session_start), 0) FROM plan_sessions
         WHERE server_id = ? AND session_end >= ?",
    )
    .bind(server_id)
    .bind(week_ago_ms)
    .fetch_one(pool)
    .await?;

    Ok(PlanServerOverviewV1 {
        server_uuid: server_uuid.to_string(),
        server_name,
        numbers: PlanServerNumbers {
            total_players: total_players as u64,
            online_players: live_online,
            total_sessions: total_sessions as u64,
            total_playtime_ms: total_playtime_ms as u64,
            total_deaths: total_deaths as u64,
        },
        last_7_days: PlanLast7Days {
            unique_players: unique_7d as u64,
            session_count: sessions_7d as u64,
            avg_session_length_ms: avg_len.round() as u64,
        },
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanKillRow {
    pub killer_name: String,
    pub killer_uuid: String,
    pub victim_name: String,
    pub victim_uuid: String,
    pub server_uuid: String,
    pub weapon: String,
    pub date: i64,
}

pub async fn list_kills(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    limit: i64,
) -> Result<Vec<PlanKillRow>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT k.* FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.server_uuid = ?
             ORDER BY k.date DESC LIMIT ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(limit)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT k.* FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ?
             ORDER BY k.date DESC LIMIT ?",
        )
        .bind(network_id)
        .bind(limit)
        .fetch_all(pool)
        .await?
    };

    Ok(rows
        .iter()
        .map(|row| PlanKillRow {
            killer_name: row.get("killer_name"),
            killer_uuid: row.get("killer_uuid"),
            victim_name: row.get("victim_name"),
            victim_uuid: row.get("victim_uuid"),
            server_uuid: row.get("server_uuid"),
            weapon: row.get("weapon"),
            date: row.get("date"),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerVersusV1 {
    pub numbers: PlanVersusNumbers,
    pub insights: PlanVersusInsights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVersusNumbers {
    pub player_kills_total: u64,
    pub player_kills_30d: u64,
    pub player_kills_7d: u64,
    pub deaths_total: u64,
    pub mob_kills_total: u64,
    pub kdr_total: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanVersusInsights {
    pub deadliest_pvp_weapon: String,
    pub second_deadliest_pvp_weapon: String,
    pub third_deadliest_pvp_weapon: String,
}

pub async fn player_versus_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<PlanPlayerVersusV1> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let d7 = now_ms - patch_plan_core::ONE_WEEK_MS;
    let d30 = now_ms - 30_i64 * 24 * 60 * 60 * 1000;

    let (kills_total, kills_7d, kills_30d) = if let Some(uuid) = server_uuid {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.server_uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_one(pool)
        .await?;
        let k7: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.server_uuid = ? AND k.date >= ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(d7)
        .fetch_one(pool)
        .await?;
        let k30: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.server_uuid = ? AND k.date >= ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(d30)
        .fetch_one(pool)
        .await?;
        (total, k7, k30)
    } else {
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?;
        let k7: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.date >= ?",
        )
        .bind(network_id)
        .bind(d7)
        .fetch_one(pool)
        .await?;
        let k30: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.date >= ?",
        )
        .bind(network_id)
        .bind(d30)
        .fetch_one(pool)
        .await?;
        (total, k7, k30)
    };

    let deaths_total: i64 = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(sess.deaths), 0) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(sess.deaths), 0) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?
    };

    let mob_kills_total: i64 = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(sess.mob_kills), 0) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COALESCE(SUM(sess.mob_kills), 0) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?
    };

    let kdr = kills_total as f64 / (deaths_total.max(1)) as f64;

    let weapons = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT weapon, COUNT(*) AS c FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.server_uuid = ?
             GROUP BY weapon ORDER BY c DESC LIMIT 3",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT weapon, COUNT(*) AS c FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ?
             GROUP BY weapon ORDER BY c DESC LIMIT 3",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let weapon_names: Vec<String> = weapons
        .iter()
        .map(|r| r.get::<String, _>("weapon"))
        .collect();

    Ok(PlanPlayerVersusV1 {
        numbers: PlanVersusNumbers {
            player_kills_total: kills_total as u64,
            player_kills_30d: kills_30d as u64,
            player_kills_7d: kills_7d as u64,
            deaths_total: deaths_total as u64,
            mob_kills_total: mob_kills_total as u64,
            kdr_total: kdr,
        },
        insights: PlanVersusInsights {
            deadliest_pvp_weapon: weapon_names.first().cloned().unwrap_or_else(|| "—".into()),
            second_deadliest_pvp_weapon: weapon_names.get(1).cloned().unwrap_or_else(|| "—".into()),
            third_deadliest_pvp_weapon: weapon_names.get(2).cloned().unwrap_or_else(|| "—".into()),
        },
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNetworkOverviewV1 {
    pub network_id: String,
    pub servers: Vec<PlanServerOverviewV1>,
    pub combined_players_online: u32,
}

pub async fn network_overview_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuids: &[(String, String, u32)],
) -> Result<PlanNetworkOverviewV1> {
    let mut servers = Vec::new();
    let mut combined_online = 0u32;
    for (uuid, _name, online) in server_uuids {
        combined_online += online;
        servers.push(server_overview_v1(pool, network_id, uuid, *online).await?);
    }
    Ok(PlanNetworkOverviewV1 {
        network_id: network_id.to_string(),
        servers,
        combined_players_online: combined_online,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerProfileStats {
    pub total_playtime_ms: u64,
    pub afk_time_ms: u64,
    pub mob_kills: u64,
    pub deaths: u64,
    pub session_count: u64,
    pub player_kill_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerNicknameRow {
    pub nickname: String,
    pub server_name: String,
    pub server_uuid: String,
    pub last_used: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerConnectionRow {
    pub geolocation: String,
    pub date: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerPunchcardPoint {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerWorldSlice {
    pub name: String,
    pub y: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanPlayerProfileV1 {
    pub info: PlanPlayerTableRow,
    pub stats: PlanPlayerProfileStats,
    pub nicknames: Vec<PlanPlayerNicknameRow>,
    pub connections: Vec<PlanPlayerConnectionRow>,
    pub punchcard_series: Vec<PlanPlayerPunchcardPoint>,
    pub world_pie_series: Vec<PlanPlayerWorldSlice>,
    pub server_pie_series: Vec<PlanPlayerWorldSlice>,
    pub sessions: Vec<PlanSessionRow>,
    pub kills_as_killer: Vec<PlanKillRow>,
    pub kills_as_victim: Vec<PlanKillRow>,
}

pub async fn player_profile_v1(
    pool: &SqlitePool,
    network_id: &str,
    player_uuid: &str,
) -> Result<PlanPlayerProfileV1> {
    let players = list_players_table(pool, network_id, None, 2000).await?;
    let info = players
        .into_iter()
        .find(|p| p.uuid == player_uuid)
        .ok_or_else(|| Error::NotFound(format!("player {player_uuid}")))?;

    let session_rows = list_player_sessions(pool, network_id, player_uuid, 40).await?;

    let kills_as_killer = list_kills_for_player(pool, network_id, player_uuid, true, 30).await?;
    let kills_as_victim = list_kills_for_player(pool, network_id, player_uuid, false, 30).await?;

    let stats = profile_stats_from_sessions(&session_rows, kills_as_killer.len() as u64);
    let nicknames = list_player_nicknames(pool, network_id, player_uuid).await?;
    let connections = list_player_connections(pool, player_uuid).await?;
    let punchcard_series = player_punchcard_from_sessions(&session_rows);
    let world_pie_series = player_world_pie_from_sessions(&session_rows);
    let server_pie_series = player_server_pie_from_sessions(&session_rows);

    Ok(PlanPlayerProfileV1 {
        info,
        stats,
        nicknames,
        connections,
        punchcard_series,
        world_pie_series,
        server_pie_series,
        sessions: session_rows,
        kills_as_killer,
        kills_as_victim,
    })
}

fn profile_stats_from_sessions(sessions: &[PlanSessionRow], player_kill_count: u64) -> PlanPlayerProfileStats {
    let mut total_playtime_ms = 0u64;
    let mut afk_time_ms = 0u64;
    let mut mob_kills = 0u64;
    let mut deaths = 0u64;
    for s in sessions {
        total_playtime_ms += s.length.max(0) as u64;
        afk_time_ms += s.afk_time as u64;
        mob_kills += s.mob_kills as u64;
        deaths += s.deaths as u64;
    }
    PlanPlayerProfileStats {
        total_playtime_ms,
        afk_time_ms,
        mob_kills,
        deaths,
        session_count: sessions.len() as u64,
        player_kill_count,
    }
}

async fn list_player_nicknames(
    pool: &SqlitePool,
    network_id: &str,
    player_uuid: &str,
) -> Result<Vec<PlanPlayerNicknameRow>> {
    let rows = sqlx::query(
        "SELECT n.nickname, n.last_used, s.name AS server_name, s.uuid AS server_uuid
         FROM plan_nicknames n
         JOIN plan_users u ON u.id = n.user_id
         JOIN plan_servers s ON s.id = n.server_id
         WHERE u.uuid = ? AND s.network_id = ?
         ORDER BY n.last_used DESC
         LIMIT 32",
    )
    .bind(player_uuid)
    .bind(network_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| PlanPlayerNicknameRow {
            nickname: r.get("nickname"),
            server_name: r.get("server_name"),
            server_uuid: r.get("server_uuid"),
            last_used: r.get("last_used"),
        })
        .collect())
}

async fn list_player_connections(
    pool: &SqlitePool,
    player_uuid: &str,
) -> Result<Vec<PlanPlayerConnectionRow>> {
    let rows = sqlx::query(
        "SELECT g.geolocation, g.last_used
         FROM plan_geolocations g
         JOIN plan_users u ON u.id = g.user_id
         WHERE u.uuid = ?
         ORDER BY g.last_used DESC
         LIMIT 24",
    )
    .bind(player_uuid)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| PlanPlayerConnectionRow {
            geolocation: r.get("geolocation"),
            date: r.get("last_used"),
        })
        .collect())
}

fn player_punchcard_from_sessions(sessions: &[PlanSessionRow]) -> Vec<PlanPlayerPunchcardPoint> {
    use chrono::{Datelike, Timelike, Utc};
    let mut grid: [[u32; 24]; 7] = [[0; 24]; 7];
    for s in sessions {
        let ms = s.start;
        let dt = chrono::DateTime::<Utc>::from_timestamp_millis(ms).unwrap_or_else(Utc::now);
        let dow = dt.weekday().num_days_from_monday() as usize;
        let hour = dt.hour() as usize;
        grid[dow][hour] += 1;
    }
    let mut out = Vec::new();
    for (dow, row) in grid.iter().enumerate() {
        for (hour, count) in row.iter().enumerate() {
            if *count > 0 {
                out.push(PlanPlayerPunchcardPoint {
                    x: hour as u32,
                    y: dow as u32,
                    z: *count,
                });
            }
        }
    }
    out
}

fn player_world_pie_from_sessions(sessions: &[PlanSessionRow]) -> Vec<PlanPlayerWorldSlice> {
    use std::collections::HashMap;
    let mut totals: HashMap<String, u64> = HashMap::new();
    for s in sessions {
        for w in &s.world_series {
            *totals.entry(w.name.clone()).or_insert(0) += w.y;
        }
    }
    let mut slices = totals
        .into_iter()
        .map(|(name, y)| PlanPlayerWorldSlice { name, y })
        .collect::<Vec<_>>();
    slices.sort_by(|a, b| b.y.cmp(&a.y));
    slices.truncate(16);
    slices
}

fn player_server_pie_from_sessions(sessions: &[PlanSessionRow]) -> Vec<PlanPlayerWorldSlice> {
    use std::collections::HashMap;
    let mut totals: HashMap<String, u64> = HashMap::new();
    for s in sessions {
        let key = format!("{} ({})", s.server_name, s.server_uuid);
        *totals.entry(key).or_insert(0) += s.length.max(0) as u64;
    }
    let mut slices = totals
        .into_iter()
        .map(|(name, y)| PlanPlayerWorldSlice { name, y })
        .collect::<Vec<_>>();
    slices.sort_by(|a, b| b.y.cmp(&a.y));
    slices.truncate(12);
    slices
}

async fn list_kills_for_player(
    pool: &SqlitePool,
    network_id: &str,
    player_uuid: &str,
    as_killer: bool,
    limit: i64,
) -> Result<Vec<PlanKillRow>> {
    let col = if as_killer { "killer_uuid" } else { "victim_uuid" };
    let sql = format!(
        "SELECT k.* FROM plan_kills k
         JOIN plan_servers s ON s.uuid = k.server_uuid
         WHERE s.network_id = ? AND k.{col} = ?
         ORDER BY k.date DESC LIMIT ?"
    );
    let rows = sqlx::query(&sql)
        .bind(network_id)
        .bind(player_uuid)
        .bind(limit)
        .fetch_all(pool)
        .await?;
    Ok(rows
        .iter()
        .map(|row| PlanKillRow {
            killer_name: row.get("killer_name"),
            killer_uuid: row.get("killer_uuid"),
            victim_name: row.get("victim_name"),
            victim_uuid: row.get("victim_uuid"),
            server_uuid: row.get("server_uuid"),
            weapon: row.get("weapon"),
            date: row.get("date"),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSessionsOverviewV1 {
    pub insights: PlanSessionsInsights,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSessionsInsights {
    pub total_playtime_ms: u64,
    pub afk_time_ms: u64,
    pub afk_time_perc: f64,
    pub server_occupied_perc: f64,
    pub most_active_gamemode: String,
}

pub async fn sessions_overview_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<PlanSessionsOverviewV1> {
    let (play_ms, afk_ms) = if let Some(uuid) = server_uuid {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(session_end - session_start), 0) AS play,
                    COALESCE(SUM(afk_time), 0) AS afk
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_one(pool)
        .await?;
        (row.get::<i64, _>("play"), row.get::<i64, _>("afk"))
    } else {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(session_end - session_start), 0) AS play,
                    COALESCE(SUM(afk_time), 0) AS afk
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?;
        (row.get::<i64, _>("play"), row.get::<i64, _>("afk"))
    };

    let afk_perc = if play_ms > 0 {
        (afk_ms as f64 / play_ms as f64) * 100.0
    } else {
        0.0
    };
    let active_ms = play_ms - afk_ms;
    let occupied = if play_ms > 0 {
        (active_ms as f64 / play_ms as f64) * 100.0
    } else {
        0.0
    };

    let mode_rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT 'survival' AS mode, SUM(survival_time) AS t FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ? AND s.uuid = ?
             UNION ALL
             SELECT 'creative', SUM(creative_time) FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ? AND s.uuid = ?
             UNION ALL
             SELECT 'adventure', SUM(adventure_time) FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ? AND s.uuid = ?
             UNION ALL
             SELECT 'spectator', SUM(spectator_time) FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(network_id)
        .bind(uuid)
        .bind(network_id)
        .bind(uuid)
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT 'survival' AS mode, SUM(survival_time) AS t FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ?
             UNION ALL
             SELECT 'creative', SUM(creative_time) FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ?
             UNION ALL
             SELECT 'adventure', SUM(adventure_time) FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ?
             UNION ALL
             SELECT 'spectator', SUM(spectator_time) FROM plan_world_times wt
             JOIN plan_sessions sess ON sess.id = wt.session_id
             JOIN plan_servers s ON s.id = sess.server_id WHERE s.network_id = ?",
        )
        .bind(network_id)
        .bind(network_id)
        .bind(network_id)
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let mut best_mode = "survival".to_string();
    let mut best_t = 0i64;
    for r in mode_rows {
        let t: i64 = r.get("t");
        let mode: String = r.get("mode");
        if t > best_t {
            best_t = t;
            best_mode = mode;
        }
    }

    Ok(PlanSessionsOverviewV1 {
        insights: PlanSessionsInsights {
            total_playtime_ms: play_ms as u64,
            afk_time_ms: afk_ms as u64,
            afk_time_perc: afk_perc,
            server_occupied_perc: occupied,
            most_active_gamemode: best_mode,
        },
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanJoinAddressRow {
    pub join_address: String,
    pub count: u64,
}

pub async fn list_join_addresses(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Vec<PlanJoinAddressRow>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT sess.join_address, COUNT(*) AS c FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.join_address != ''
             GROUP BY sess.join_address ORDER BY c DESC",
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
             GROUP BY sess.join_address ORDER BY c DESC",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };
    Ok(rows
        .iter()
        .map(|r| PlanJoinAddressRow {
            join_address: r.get("join_address"),
            count: r.get::<i64, _>("c") as u64,
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNetworkServerRow {
    pub id: i64,
    pub uuid: String,
    pub name: String,
}

pub async fn plan_network_servers_v1(
    pool: &SqlitePool,
    network_id: &str,
) -> Result<Vec<PlanNetworkServerRow>> {
    let rows = sqlx::query(
        "SELECT id, uuid, name FROM plan_servers WHERE network_id = ? ORDER BY name ASC",
    )
    .bind(network_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|row| PlanNetworkServerRow {
            id: row.get("id"),
            uuid: row.get("uuid"),
            name: row.get("name"),
        })
        .collect())
}

#[cfg(test)]
mod ingest_tests {
    use super::*;
    use patch_analytics_protocol::{
        AnalyticsPayload, PerformanceSample, PlayerStats, ServerTotals, SessionRecord,
    };
    use sqlx::sqlite::SqlitePoolOptions;

    fn sample_payload() -> AnalyticsPayload {
        let now = 1_700_000_000_i64;
        AnalyticsPayload {
            schema_version: 5,
            server_id: "srv-uuid-1".into(),
            server_name: "Lobby".into(),
            network_id: "net-1".into(),
            generated_at_unix: now,
            online_now: 2,
            peak_online: 5,
            total_joins: 1,
            unique_players: 1,
            total_playtime_secs: 3600,
            server_totals: ServerTotals::default(),
            recent_deaths: vec![],
            recent_events: vec![],
            players: vec![PlayerStats {
                uuid: "player-uuid-1".into(),
                name: "Alex".into(),
                first_seen_unix: now - 7200,
                last_seen_unix: now,
                join_address: "play.test".into(),
                completed_sessions: vec![SessionRecord {
                    start_unix: now - 3600,
                    end_unix: now - 1800,
                    afk_secs: 60,
                    mob_kills: 2,
                    deaths: 1,
                    join_address: "play.test".into(),
                    world_times: vec![],
                }],
                ..PlayerStats::default()
            }],
            activity_samples: vec![],
            command_usages: Default::default(),
            recent_pvp_kills: vec![],
            performance_samples: vec![PerformanceSample {
                t: now,
                ticks_per_second: 19.2,
                mspt_avg: 52.0,
                players_online: 2,
            }],
            registration_completions: vec![],
            web_admin_actions: vec![],
            desktop_plan_requests: vec![],
        }
    }

    #[tokio::test]
    async fn ingest_writes_sessions_and_performance_tps() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("memory db");
        ensure_plan_schema(&pool).await.expect("schema");
        let payload = sample_payload();
        let report = ingest_payload(&pool, "net-1", &payload)
            .await
            .expect("ingest");
        assert_eq!(report.servers_synced, 1);
        assert_eq!(report.users_upserted, 1);
        assert_eq!(report.sessions_inserted, 1);
        assert_eq!(report.performance_samples_inserted, 1);

        let tps: f64 = sqlx::query_scalar(
            "SELECT ticks_per_second FROM plan_tps t
             JOIN plan_servers s ON s.id = t.server_id
             WHERE s.uuid = ?",
        )
        .bind("srv-uuid-1")
        .fetch_one(&pool)
        .await
        .expect("tps row");
        assert!((tps - 19.2).abs() < 0.01);

        let sessions: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?",
        )
        .bind("net-1")
        .fetch_one(&pool)
        .await
        .expect("session count");
        assert_eq!(sessions, 1);
    }

    #[tokio::test]
    async fn ingest_geo_and_ping_windows() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("memory db");
        ensure_plan_schema(&pool).await.expect("schema");
        let now = 1_700_000_000_i64;
        let payload = AnalyticsPayload {
            schema_version: 5,
            server_id: "srv-geo".into(),
            server_name: "Lobby".into(),
            network_id: "net-geo".into(),
            generated_at_unix: now,
            online_now: 1,
            peak_online: 1,
            total_joins: 1,
            unique_players: 1,
            total_playtime_secs: 100,
            server_totals: ServerTotals::default(),
            recent_deaths: vec![],
            recent_events: vec![],
            players: vec![PlayerStats {
                uuid: "p-geo".into(),
                name: "GeoPlayer".into(),
                first_seen_unix: now,
                last_seen_unix: now,
                geolocation: "Local Machine".into(),
                ping_windows: vec![patch_analytics_protocol::PingWindowRecord {
                    t_unix: now,
                    avg_ping: 42,
                    min_ping: 30,
                    max_ping: 55,
                }],
                ..PlayerStats::default()
            }],
            activity_samples: vec![],
            command_usages: Default::default(),
            recent_pvp_kills: vec![],
            performance_samples: vec![],
            registration_completions: vec![],
            web_admin_actions: vec![],
            desktop_plan_requests: vec![],
        };
        let report = ingest_payload(&pool, "net-geo", &payload)
            .await
            .expect("ingest");
        assert_eq!(report.geolocation_rows_upserted, 1);
        assert_eq!(report.ping_rows_inserted, 1);

        let ping_table = ping_table_v1(&pool, "net-geo", None)
            .await
            .expect("ping table");
        assert_eq!(ping_table.len(), 1);
        assert_eq!(ping_table[0].country, "Local Machine");
        assert_eq!(ping_table[0].avg_ping, 42);
    }

    #[test]
    fn aggregates_performance_samples_into_minute_buckets() {
        let t0 = 1_700_000_000_i64;
        let buckets = aggregate_performance_minutes(&[
            PerformanceSample {
                t: t0,
                ticks_per_second: 20.0,
                mspt_avg: 50.0,
                players_online: 3,
            },
            PerformanceSample {
                t: t0 + 30,
                ticks_per_second: 18.0,
                mspt_avg: 60.0,
                players_online: 5,
            },
        ]);
        assert_eq!(buckets.len(), 1);
        let minute_start = (t0 / 60) * 60 * 1000;
        assert_eq!(buckets[0].date_ms, minute_start);
        assert!((buckets[0].avg_tps - 19.0).abs() < 0.01);
        assert_eq!(buckets[0].max_players, 5);
    }
}
