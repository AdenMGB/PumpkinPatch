//! Build `PlanPlaceholderContext` from desktop Plan SQLite (HTTP / plugin parity).

use crate::error::Result;
use crate::plan_overviews::{online_overview_v1, performance_overview_v1};
use crate::plan_store::{list_kills, list_player_sessions, list_players_table};
use patch_plan_core::{sort_top_playtime, PlanPlaceholderContext};
use sqlx::{Row, SqlitePool};

const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 7 * DAY_MS;
const MONTH_MS: i64 = 30 * DAY_MS;

pub async fn plan_placeholder_context_from_sql(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    player_uuid: Option<&str>,
    players_online: u32,
) -> Result<PlanPlaceholderContext> {
    let now_ms = chrono::Utc::now().timestamp_millis();
    let now_sec = now_ms / 1000;

    let (server_name, server_uuid_str) = resolve_server_identity(pool, network_id, server_uuid).await?;

    let online = online_overview_v1(pool, network_id, server_uuid, players_online).await?;
    let perf = performance_overview_v1(pool, network_id, server_uuid).await?;
    let players = list_players_table(pool, network_id, server_uuid, 5000).await?;
    let players_total = players.len() as u64;
    let players_regular = players
        .iter()
        .filter(|p| p.activity_group.eq_ignore_ascii_case("regular"))
        .count() as u64;

    let peak_online: i64 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(t.players_online), 0) FROM plan_tps t
         JOIN plan_servers s ON s.id = t.server_id
         WHERE s.network_id = ?",
    )
    .bind(network_id)
    .fetch_one(pool)
    .await?;

    let total_joins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM plan_sessions sess
         JOIN plan_servers s ON s.id = sess.server_id
         WHERE s.network_id = ?",
    )
    .bind(network_id)
    .fetch_one(pool)
    .await?;

    let top = top_playtime_since(pool, network_id, server_uuid, 0).await?;
    let top_day = top_playtime_since(pool, network_id, server_uuid, now_ms - DAY_MS).await?;
    let top_week = top_playtime_since(pool, network_id, server_uuid, now_ms - WEEK_MS).await?;
    let top_month = top_playtime_since(pool, network_id, server_uuid, now_ms - MONTH_MS).await?;

    let (sessions_playtime_secs, sessions_afk_secs) = window_playtime(pool, network_id, server_uuid, 0).await?;
    let sessions_active_secs = sessions_playtime_secs.saturating_sub(sessions_afk_secs);
    let (sessions_playtime_day, sessions_afk_day) =
        window_playtime(pool, network_id, server_uuid, now_ms - DAY_MS).await?;
    let (sessions_playtime_week, sessions_afk_week) =
        window_playtime(pool, network_id, server_uuid, now_ms - WEEK_MS).await?;
    let (sessions_playtime_month, sessions_afk_month) =
        window_playtime(pool, network_id, server_uuid, now_ms - MONTH_MS).await?;

    let sessions_count_day = session_count_since(pool, network_id, server_uuid, now_ms - DAY_MS).await?;
    let sessions_count_week = session_count_since(pool, network_id, server_uuid, now_ms - WEEK_MS).await?;
    let sessions_count_month = session_count_since(pool, network_id, server_uuid, now_ms - MONTH_MS).await?;

    let sessions_avg_length_day = avg_session_len_since(pool, network_id, server_uuid, now_ms - DAY_MS).await?;
    let sessions_avg_length_week = avg_session_len_since(pool, network_id, server_uuid, now_ms - WEEK_MS).await?;
    let sessions_avg_length_month = avg_session_len_since(pool, network_id, server_uuid, now_ms - MONTH_MS).await?;

    let sessions_avg_unique_players_day =
        unique_players_since(pool, network_id, server_uuid, now_ms - DAY_MS).await?;
    let sessions_avg_unique_players_week =
        unique_players_since(pool, network_id, server_uuid, now_ms - WEEK_MS).await?;
    let sessions_avg_unique_players_month =
        unique_players_since(pool, network_id, server_uuid, now_ms - MONTH_MS).await?;

    let (deaths_total, mob_kills_total, player_kills_total) =
        combat_totals(pool, network_id, server_uuid).await?;
    let (deaths_day, mob_day, pvp_day) =
        combat_since(pool, network_id, server_uuid, now_ms - DAY_MS).await?;
    let (deaths_week, mob_week, pvp_week) =
        combat_since(pool, network_id, server_uuid, now_ms - WEEK_MS).await?;
    let (deaths_month, mob_month, pvp_month) =
        combat_since(pool, network_id, server_uuid, now_ms - MONTH_MS).await?;

    let player_row = player_uuid.and_then(|u| players.iter().find(|p| p.uuid == u));
    let session_triples = if let Some(uuid) = player_uuid {
        let rows = list_player_sessions(pool, network_id, uuid, 200).await?;
        rows.iter()
            .map(|s| (s.start, s.end, s.afk_time as u64))
            .collect()
    } else {
        Vec::new()
    };

    let player_recent_kills: Vec<(String, String)> = if let Some(uuid) = player_uuid {
        list_kills(pool, network_id, server_uuid, 30)
            .await?
            .into_iter()
            .filter(|k| k.killer_uuid == uuid)
            .take(10)
            .map(|k| (k.victim_name, k.weapon))
            .collect()
    } else {
        Vec::new()
    };

    let player_session_length_longest_secs = if let Some(uuid) = player_uuid {
        list_player_sessions(pool, network_id, uuid, 500)
            .await?
            .iter()
            .map(|s| (s.length.max(0) as u64) / 1000)
            .max()
            .unwrap_or(0)
    } else {
        0
    };

    let (player_deaths, player_mob, player_pvp) = if let Some(uuid) = player_uuid {
        let rows = list_player_sessions(pool, network_id, uuid, 500).await?;
        let deaths = rows.iter().map(|s| s.deaths as u32).sum();
        let mob = rows.iter().map(|s| s.mob_kills as u32).sum();
        let pvp = rows.iter().map(|s| s.player_kills.len() as u32).sum();
        (deaths, mob, pvp)
    } else {
        (0, 0, 0)
    };

    let favorite_server = server_name.clone();

    Ok(PlanPlaceholderContext {
        server_name,
        server_uuid: server_uuid_str,
        players_online,
        players_total,
        peak_online: peak_online.min(i32::MAX as i64) as u32,
        total_joins: total_joins as u64,
        player_name: player_row.map(|p| p.name.clone()),
        player_uuid: player_uuid.map(|s| s.to_string()),
        playtime_secs: player_row.map(|p| p.active_playtime_ms / 1000).unwrap_or(0),
        afk_secs: 0,
        deaths: player_deaths,
        player_kills: player_pvp,
        mob_kills: player_mob,
        kick_count: 0,
        geolocation: player_row.map(|p| p.geolocation.clone()).unwrap_or_default(),
        registered_unix: player_row.map(|p| p.registered / 1000).unwrap_or(0),
        last_seen_unix: player_row.map(|p| p.last_seen / 1000).unwrap_or(0),
        session_triples,
        now_ms,
        avg_tps_day: Some(perf.numbers.tps_24h),
        avg_tps_week: Some(perf.numbers.tps_7d),
        avg_tps_month: Some(perf.numbers.tps_30d),
        avg_ping_day: None,
        avg_ping_week: None,
        avg_ping_month: None,
        min_ping_day: player_row.map(|p| p.ping_min),
        max_ping_day: player_row.map(|p| p.ping_max),
        player_ping_avg: player_row.map(|p| p.ping_avg as f64),
        player_ping_min: player_row.map(|p| p.ping_min),
        player_ping_max: player_row.map(|p| p.ping_max),
        players_new_day: online.numbers.new_players_24h,
        players_new_week: online.numbers.new_players_7d,
        players_new_month: online.numbers.new_players_30d,
        players_unique_day: online.numbers.unique_players_24h,
        players_unique_week: online.numbers.unique_players_7d,
        players_unique_month: online.numbers.unique_players_30d,
        sessions_playtime_secs,
        sessions_active_secs,
        sessions_afk_secs,
        sessions_count_day,
        sessions_count_week,
        sessions_count_month,
        timezone_offset: "+00:00".into(),
        player_recent_kills,
        deaths_total,
        deaths_day,
        deaths_week,
        deaths_month,
        mob_kills_total,
        mob_kills_day: mob_day,
        mob_kills_week: mob_week,
        mob_kills_month: mob_month,
        player_kills_total,
        player_kills_day: pvp_day,
        player_kills_week: pvp_week,
        player_kills_month: pvp_month,
        sessions_playtime_day,
        sessions_playtime_week,
        sessions_playtime_month,
        peak_online_date_alltime: now_sec.to_string(),
        peak_online_date_recent: now_sec.to_string(),
        favorite_server,
        tps_spikes_day: perf.numbers.low_tps_spikes_24h.min(u32::MAX as u64) as u32,
        sessions_avg_length_day,
        sessions_avg_length_week,
        sessions_avg_length_month,
        sessions_avg_unique_players_day,
        sessions_active_day: sessions_playtime_day.saturating_sub(sessions_afk_day),
        sessions_active_week: sessions_playtime_week.saturating_sub(sessions_afk_week),
        sessions_active_month: sessions_playtime_month.saturating_sub(sessions_afk_month),
        sessions_afk_day,
        sessions_afk_week,
        sessions_afk_month,
        sessions_avg_unique_players_week,
        sessions_avg_unique_players_month,
        tps_spikes_week: perf.numbers.low_tps_spikes_7d.min(u32::MAX as u64) as u32,
        tps_spikes_month: perf.numbers.low_tps_spikes_30d.min(u32::MAX as u64) as u32,
        player_is_banned: false,
        player_is_operator: false,
        player_session_length_longest_secs,
        players_regular,
        top_playtime: top,
        top_playtime_day: top_day,
        top_playtime_week: top_week,
        top_playtime_month: top_month,
    })
}

async fn resolve_server_identity(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<(String, String)> {
    if let Some(uuid) = server_uuid {
        let row = sqlx::query(
            "SELECT name FROM plan_servers WHERE network_id = ? AND uuid = ? LIMIT 1",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_optional(pool)
        .await?;
        if let Some(row) = row {
            return Ok((row.get("name"), uuid.to_string()));
        }
    }
    let row = sqlx::query(
        "SELECT name, uuid FROM plan_servers WHERE network_id = ? ORDER BY id LIMIT 1",
    )
    .bind(network_id)
    .fetch_optional(pool)
    .await?;
    Ok(match row {
        Some(r) => (r.get("name"), r.get("uuid")),
        None => ("Network".into(), String::new()),
    })
}

async fn top_playtime_since(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<Vec<(String, u64, u64)>> {
    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT u.name AS name,
                    COALESCE(SUM(sess.session_end - sess.session_start), 0) / 1000 AS play_secs,
                    COALESCE(SUM(sess.afk_time), 0) / 1000 AS afk_secs
             FROM plan_users u
             JOIN plan_sessions sess ON sess.user_id = u.id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
               AND (? <= 0 OR sess.session_end >= ?)
             GROUP BY u.id",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .bind(since_ms)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT u.name AS name,
                    COALESCE(SUM(sess.session_end - sess.session_start), 0) / 1000 AS play_secs,
                    COALESCE(SUM(sess.afk_time), 0) / 1000 AS afk_secs
             FROM plan_users u
             JOIN plan_sessions sess ON sess.user_id = u.id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
               AND (? <= 0 OR sess.session_end >= ?)
             GROUP BY u.id",
        )
        .bind(network_id)
        .bind(since_ms)
        .bind(since_ms)
        .fetch_all(pool)
        .await?
    };
    let entries = rows
        .iter()
        .map(|r| {
            (
                r.get::<String, _>("name"),
                r.get::<i64, _>("play_secs") as u64,
                r.get::<i64, _>("afk_secs") as u64,
            )
        })
        .collect();
    Ok(sort_top_playtime(entries, 10))
}

async fn window_playtime(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<(u64, u64)> {
    let row = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT COALESCE(SUM(sess.session_end - sess.session_start), 0) / 1000 AS play_secs,
                    COALESCE(SUM(sess.afk_time), 0) / 1000 AS afk_secs
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
               AND (? <= 0 OR sess.session_end >= ?)",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT COALESCE(SUM(sess.session_end - sess.session_start), 0) / 1000 AS play_secs,
                    COALESCE(SUM(sess.afk_time), 0) / 1000 AS afk_secs
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
               AND (? <= 0 OR sess.session_end >= ?)",
        )
        .bind(network_id)
        .bind(since_ms)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    };
    Ok((
        row.get::<i64, _>("play_secs") as u64,
        row.get::<i64, _>("afk_secs") as u64,
    ))
}

async fn session_count_since(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<u64> {
    let n: i64 = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_sessions sess
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
            "SELECT COUNT(*) FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.session_end >= ?",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    };
    Ok(n as u64)
}

async fn avg_session_len_since(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<u64> {
    let avg_ms: i64 = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COALESCE(CAST(AVG(sess.session_end - sess.session_start) AS INTEGER), 0)
             FROM plan_sessions sess
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
            "SELECT COALESCE(CAST(AVG(sess.session_end - sess.session_start) AS INTEGER), 0)
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.session_end >= ?",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    };
    Ok((avg_ms as u64) / 1000)
}

async fn unique_players_since(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<u64> {
    let n: i64 = if let Some(uuid) = server_uuid {
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
    };
    Ok(n as u64)
}

async fn combat_totals(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<(u64, u64, u64)> {
    let (deaths, mob): (i64, i64) = if let Some(uuid) = server_uuid {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(sess.deaths), 0) AS deaths,
                    COALESCE(SUM(sess.mob_kills), 0) AS mob
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_one(pool)
        .await?;
        (row.get("deaths"), row.get("mob"))
    } else {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(sess.deaths), 0) AS deaths,
                    COALESCE(SUM(sess.mob_kills), 0) AS mob
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?;
        (row.get("deaths"), row.get("mob"))
    };
    let pvp: i64 = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND s.uuid = ?",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ?",
        )
        .bind(network_id)
        .fetch_one(pool)
        .await?
    };
    Ok((deaths as u64, mob as u64, pvp as u64))
}

async fn combat_since(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    since_ms: i64,
) -> Result<(u64, u64, u64)> {
    let (deaths, mob): (i64, i64) = if let Some(uuid) = server_uuid {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(sess.deaths), 0) AS deaths,
                    COALESCE(SUM(sess.mob_kills), 0) AS mob
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.session_end >= ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_one(pool)
        .await?;
        (row.get("deaths"), row.get("mob"))
    } else {
        let row = sqlx::query(
            "SELECT COALESCE(SUM(sess.deaths), 0) AS deaths,
                    COALESCE(SUM(sess.mob_kills), 0) AS mob
             FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.session_end >= ?",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?;
        (row.get("deaths"), row.get("mob"))
    };
    let pvp: i64 = if let Some(uuid) = server_uuid {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND s.uuid = ? AND k.date >= ?",
        )
        .bind(network_id)
        .bind(uuid)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    } else {
        sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_kills k
             JOIN plan_servers s ON s.uuid = k.server_uuid
             WHERE s.network_id = ? AND k.date >= ?",
        )
        .bind(network_id)
        .bind(since_ms)
        .fetch_one(pool)
        .await?
    };
    Ok((deaths as u64, mob as u64, pvp as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_store::{ensure_plan_schema, ingest_payload};
    use patch_analytics_protocol::{
        AnalyticsPayload, PerformanceSample, PlayerStats, SessionRecord, ServerTotals,
    };
    use patch_plan_core::resolve_placeholder;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn sql_context_resolves_core_server_placeholders() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        ensure_plan_schema(&pool).await.unwrap();
        let now = 1_700_000_000_i64;
        ingest_payload(
            &pool,
            "net",
            &AnalyticsPayload {
                schema_version: 5,
                server_id: "srv".into(),
                server_name: "Lobby".into(),
                network_id: "net".into(),
                generated_at_unix: now,
                online_now: 3,
                peak_online: 3,
                total_joins: 1,
                unique_players: 1,
                total_playtime_secs: 100,
                server_totals: ServerTotals::default(),
                recent_deaths: vec![],
                recent_events: vec![],
                players: vec![PlayerStats {
                    uuid: "player-1".into(),
                    name: "Alex".into(),
                    first_seen_unix: now - 3600,
                    last_seen_unix: now,
                    join_address: "localhost".into(),
                    completed_sessions: vec![SessionRecord {
                        start_unix: now - 1800,
                        end_unix: now - 900,
                        afk_secs: 0,
                        mob_kills: 0,
                        deaths: 0,
                        join_address: "localhost".into(),
                        world_times: vec![],
                    }],
                    ..PlayerStats::default()
                }],
                activity_samples: vec![],
                command_usages: Default::default(),
                recent_pvp_kills: vec![],
                performance_samples: vec![PerformanceSample {
                    t: now,
                    ticks_per_second: 20.0,
                    mspt_avg: 50.0,
                    players_online: 3,
                }],
                registration_completions: vec![],
                web_admin_actions: vec![],
                desktop_plan_requests: vec![],
                is_proxy: false,
            },
        )
        .await
        .unwrap();
        let ctx = plan_placeholder_context_from_sql(&pool, "net", Some("srv"), None, 3)
            .await
            .unwrap();
        assert_eq!(ctx.server_name, "Lobby");
        assert_eq!(ctx.players_online, 3);
        assert_eq!(
            resolve_placeholder("%plan_players_online%", &ctx).as_deref(),
            Some("3")
        );
        assert!(
            resolve_placeholder("%plan_server_name%", &ctx).is_some(),
            "server name placeholder"
        );
    }
}
