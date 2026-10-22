//! Minecraft game allowlist bounce tracking (Plan `/v1/gameAllowlistBounces`).

use crate::error::Result;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

pub use patch_analytics_protocol::ALLOWLIST_BOUNCE_EVENT;

pub async fn record_allowlist_bounce(
    pool: &SqlitePool,
    server_id: i64,
    player_uuid: &str,
    player_name: &str,
    at_ms: i64,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO plan_allowlist_bounces (server_id, player_uuid, player_name, bounce_count, last_bounce_ms)
         VALUES (?, ?, ?, 1, ?)
         ON CONFLICT(server_id, player_uuid) DO UPDATE SET
           player_name = excluded.player_name,
           bounce_count = plan_allowlist_bounces.bounce_count + 1,
           last_bounce_ms = excluded.last_bounce_ms",
    )
    .bind(server_id)
    .bind(player_uuid)
    .bind(player_name)
    .bind(at_ms)
    .execute(pool)
    .await?;
    Ok(())
}

/// React dashboard shape (`ServerAllowList` view).
pub async fn game_allowlist_bounces_v1(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<Value> {
    let bounces = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT b.player_uuid, b.player_name, b.bounce_count, b.last_bounce_ms
             FROM plan_allowlist_bounces b
             JOIN plan_servers s ON s.id = b.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             ORDER BY b.bounce_count DESC, b.last_bounce_ms DESC
             LIMIT 500",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT b.player_uuid, b.player_name, b.bounce_count, b.last_bounce_ms
             FROM plan_allowlist_bounces b
             JOIN plan_servers s ON s.id = b.server_id
             WHERE s.network_id = ?
             ORDER BY b.bounce_count DESC, b.last_bounce_ms DESC
             LIMIT 500",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let allowlist_bounces: Vec<Value> = bounces
        .iter()
        .map(|row| {
            let player_uuid: String = row.get("player_uuid");
            let player_name: String = row.get("player_name");
            let count: i64 = row.get("bounce_count");
            let last_time: i64 = row.get("last_bounce_ms");
            json!({
                "playerUUID": player_uuid,
                "playerName": player_name,
                "count": count,
                "lastTime": last_time,
                "lastBounce": last_time,
            })
        })
        .collect();

    let last_seen_rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT u.uuid, MAX(sess.session_end) * 1000 AS last_seen_ms
             FROM plan_sessions sess
             JOIN plan_users u ON u.id = sess.user_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ?
             GROUP BY u.uuid",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT u.uuid, MAX(sess.session_end) * 1000 AS last_seen_ms
             FROM plan_sessions sess
             JOIN plan_users u ON u.id = sess.user_id
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ?
             GROUP BY u.uuid",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let mut last_seen_by_uuid = serde_json::Map::new();
    for row in last_seen_rows {
        let uuid: String = row.get("uuid");
        let ms: i64 = row.get("last_seen_ms");
        last_seen_by_uuid.insert(uuid, json!(ms));
    }

    Ok(json!({
        "allowlistBounces": allowlist_bounces,
        "lastSeenByUUID": last_seen_by_uuid,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use patch_analytics_protocol::{AnalyticsEvent, AnalyticsPayload, ServerTotals};
    use sqlx::sqlite::SqlitePoolOptions;

    fn empty_payload() -> AnalyticsPayload {
        AnalyticsPayload {
            schema_version: 7,
            server_id: "srv-uuid".into(),
            server_name: "S".into(),
            network_id: "net".into(),
            generated_at_unix: 1_700_000_000,
            online_now: 0,
            peak_online: 0,
            total_joins: 0,
            unique_players: 0,
            total_playtime_secs: 0,
            server_totals: ServerTotals::default(),
            recent_deaths: vec![],
            recent_events: vec![AnalyticsEvent {
                t: 1_700_000_100,
                kind: ALLOWLIST_BOUNCE_EVENT.into(),
                player_uuid: "u1".into(),
                player_name: "Steve".into(),
                detail: String::new(),
            }],
            players: vec![],
            activity_samples: vec![],
            command_usages: Default::default(),
            recent_pvp_kills: vec![],
            performance_samples: vec![],
            registration_completions: vec![],
            web_admin_actions: vec![],
            desktop_plan_requests: vec![],
            is_proxy: false,
        }
    }

    #[tokio::test]
    async fn ingest_records_allowlist_bounce_event() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        let payload = empty_payload();
        crate::plan_store::ingest_payload(&pool, "net", &payload)
            .await
            .unwrap();
        let v = game_allowlist_bounces_v1(&pool, "net", Some("srv-uuid"))
            .await
            .unwrap();
        let bounces = v["allowlistBounces"].as_array().unwrap();
        assert_eq!(bounces.len(), 1);
        assert_eq!(bounces[0]["playerName"], "Steve");
        assert_eq!(bounces[0]["count"], 1);
    }
}
