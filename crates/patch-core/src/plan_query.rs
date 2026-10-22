//! Plan /v1/query and /v1/filters subset for the desktop app.

use crate::error::{Error, Result};
use crate::plan_store::{list_players_table, PlanPlayerTableRow};
use serde::{Deserialize, Serialize};
use sqlx::{Column, Row, SqlitePool};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFilterTypeInfo {
    pub id: String,
    pub label: String,
    pub implemented: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanFiltersCatalog {
    pub activity_groups: Vec<String>,
    pub join_addresses: Vec<String>,
    pub geolocations: Vec<String>,
    pub filter_types: Vec<PlanFilterTypeInfo>,
}

const QUERY_CACHE_TTL_MS: i64 = 7 * 24 * 60 * 60 * 1000;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanQueryRequest {
    pub activity_groups: Vec<String>,
    pub join_addresses: Vec<String>,
    pub min_activity_index: Option<f64>,
    pub max_activity_index: Option<f64>,
    pub min_sessions: Option<u64>,
    pub registered_after_ms: Option<i64>,
    pub registered_before_ms: Option<i64>,
    #[serde(default)]
    pub geolocations: Vec<String>,
    /// When true, skip the 7-day result cache (Plan share-link cache).
    #[serde(default)]
    pub bypass_cache: bool,
    /// Cohort dashboard window start (`viewAfterDate` in Plan).
    pub view_after_ms: Option<i64>,
    /// Cohort dashboard window end (`viewBeforeDate` in Plan).
    pub view_before_ms: Option<i64>,
    pub last_seen_after_ms: Option<i64>,
    pub last_seen_before_ms: Option<i64>,
    /// `playedBetween` filter — session overlap window (epoch ms).
    pub played_after_ms: Option<i64>,
    pub played_before_ms: Option<i64>,
    #[serde(default)]
    pub played_server_uuids: Vec<String>,
    /// `Banned` / `Not banned` (Plan `/v1/filters`).
    #[serde(default)]
    pub banned: Vec<String>,
    /// `Operators` / `Non-operators`.
    #[serde(default)]
    pub operators: Vec<String>,
    /// Plan React `pluginsBooleanGroups` extension filters.
    #[serde(default, rename = "pluginsBooleanGroups")]
    pub plugins_boolean_groups: Vec<crate::plan_extension::PluginBooleanGroupFilter>,
    /// Plan React `pluginNumbers` extension range filters.
    #[serde(default, rename = "pluginNumbers")]
    pub plugin_numbers: Vec<crate::plan_extension::PluginNumberFilter>,
    /// Historical activity index filter (`activityIndexOn` in Plan React).
    #[serde(default, rename = "activityIndexOn")]
    pub activity_index_on: Option<ActivityIndexOnFilter>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityIndexOnFilter {
    pub date: Option<i64>,
    #[serde(default)]
    pub groups: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlanQueryCohortView {
    pub view_after_ms: i64,
    pub view_before_ms: i64,
    pub sessions_in_window: u64,
    pub playtime_ms: u64,
    pub afk_ms: u64,
    pub players_active_in_window: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanQueryResult {
    pub matched_count: u64,
    pub players: Vec<PlanPlayerTableRow>,
    pub avg_activity_index: f64,
    pub avg_active_playtime_ms: u64,
    pub cache_token: String,
    pub from_cache: bool,
    #[serde(default)]
    pub cohort_view: Option<PlanQueryCohortView>,
}

fn query_cache_token(
    network_id: &str,
    server_uuid: Option<&str>,
    request: &PlanQueryRequest,
) -> String {
    use std::hash::{Hash, Hasher};
    let request_json = serde_json::to_string(request).unwrap_or_default();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    network_id.hash(&mut hasher);
    server_uuid.unwrap_or("").hash(&mut hasher);
    request_json.hash(&mut hasher);
    format!("q{:016x}", hasher.finish())
}

pub async fn plan_filters_catalog(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
) -> Result<PlanFiltersCatalog> {
    let groups = vec![
        "Very Active".into(),
        "Active".into(),
        "Regular".into(),
        "Irregular".into(),
        "Inactive".into(),
    ];

    let rows = if let Some(uuid) = server_uuid {
        sqlx::query(
            "SELECT DISTINCT sess.join_address FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND s.uuid = ? AND sess.join_address != ''
             ORDER BY sess.join_address LIMIT 100",
        )
        .bind(network_id)
        .bind(uuid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            "SELECT DISTINCT sess.join_address FROM plan_sessions sess
             JOIN plan_servers s ON s.id = sess.server_id
             WHERE s.network_id = ? AND sess.join_address != ''
             ORDER BY sess.join_address LIMIT 100",
        )
        .bind(network_id)
        .fetch_all(pool)
        .await?
    };

    let join_addresses = rows
        .iter()
        .map(|r| r.get::<String, _>("join_address"))
        .collect();

    let geo_rows = sqlx::query(
        "SELECT DISTINCT g.geolocation FROM plan_geolocations g
         JOIN plan_users u ON u.id = g.user_id
         JOIN plan_user_info ui ON ui.user_id = u.id
         JOIN plan_servers s ON s.id = ui.server_id
         WHERE s.network_id = ? AND g.geolocation != ''
         ORDER BY g.geolocation LIMIT 100",
    )
    .bind(network_id)
    .fetch_all(pool)
    .await?;
    let geolocations = geo_rows
        .iter()
        .map(|r| r.get::<String, _>("geolocation"))
        .collect();

    let filter_types = vec![
        PlanFilterTypeInfo {
            id: "activityIndexNow".into(),
            label: "Activity group / min index".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "joinAddresses".into(),
            label: "Join address".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "minSessions".into(),
            label: "Minimum session count".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "registeredBetween".into(),
            label: "Registered between dates".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "geolocations".into(),
            label: "Geolocation".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "lastSeenBetween".into(),
            label: "Last seen between dates".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "viewAfterDate".into(),
            label: "Cohort view after date".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "viewBeforeDate".into(),
            label: "Cohort view before date".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "playedBetween".into(),
            label: "Played between dates".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "banned".into(),
            label: "Ban status".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "operators".into(),
            label: "Operator status".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "activityIndexOn".into(),
            label: "Activity index on historical date".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "pluginsBooleanGroups".into(),
            label: "DataExtension boolean / group filters".into(),
            implemented: true,
        },
        PlanFilterTypeInfo {
            id: "pluginNumbers".into(),
            label: "DataExtension numeric range filters".into(),
            implemented: true,
        },
    ];

    Ok(PlanFiltersCatalog {
        activity_groups: groups,
        join_addresses,
        geolocations,
        filter_types,
    })
}

pub async fn execute_plan_query(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    request: &PlanQueryRequest,
) -> Result<PlanQueryResult> {
    crate::plan_store::ensure_plan_schema(pool).await?;
    let token = query_cache_token(network_id, server_uuid, request);
    if !request.bypass_cache {
        if let Some(cached) = load_query_cache(pool, &token).await? {
            return Ok(cached);
        }
    }
    let result = run_plan_query_uncached(pool, network_id, server_uuid, request).await?;
    let result = PlanQueryResult {
        matched_count: result.matched_count,
        players: result.players,
        avg_activity_index: result.avg_activity_index,
        avg_active_playtime_ms: result.avg_active_playtime_ms,
        cache_token: token.clone(),
        from_cache: false,
        cohort_view: result.cohort_view,
    };
    store_query_cache(pool, &token, network_id, server_uuid, request, &result).await?;
    Ok(result)
}

pub async fn load_cached_plan_query(pool: &SqlitePool, token: &str) -> Result<PlanQueryResult> {
    crate::plan_store::ensure_plan_schema(pool).await?;
    load_query_cache(pool, token)
        .await?
        .ok_or_else(|| crate::error::Error::NotFound(format!("query cache {token}")))
}

async fn load_query_cache(pool: &SqlitePool, token: &str) -> Result<Option<PlanQueryResult>> {
    let now = chrono::Utc::now().timestamp_millis();
    let row = sqlx::query(
        "SELECT result_json FROM plan_query_cache WHERE token = ? AND expires_at_ms >= ?",
    )
    .bind(token)
    .bind(now)
    .fetch_optional(pool)
    .await?;
    if let Some(row) = row {
        let json: String = row.get("result_json");
        let mut result: PlanQueryResult = serde_json::from_str(&json)?;
        result.from_cache = true;
        return Ok(Some(result));
    }
    Ok(None)
}

async fn store_query_cache(
    pool: &SqlitePool,
    token: &str,
    network_id: &str,
    server_uuid: Option<&str>,
    request: &PlanQueryRequest,
    result: &PlanQueryResult,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let expires = now + QUERY_CACHE_TTL_MS;
    let request_json = serde_json::to_string(request)?;
    let result_json = serde_json::to_string(result)?;
    sqlx::query(
        "INSERT INTO plan_query_cache (token, network_id, server_uuid, request_json, result_json, created_at_ms, expires_at_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(token) DO UPDATE SET
           result_json = excluded.result_json,
           created_at_ms = excluded.created_at_ms,
           expires_at_ms = excluded.expires_at_ms",
    )
    .bind(token)
    .bind(network_id)
    .bind(server_uuid.unwrap_or(""))
    .bind(request_json)
    .bind(result_json)
    .bind(now)
    .bind(expires)
    .execute(pool)
    .await?;
    let _ = sqlx::query("DELETE FROM plan_query_cache WHERE expires_at_ms < ?")
        .bind(now)
        .execute(pool)
        .await;
    Ok(())
}

async fn run_plan_query_uncached(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    request: &PlanQueryRequest,
) -> Result<PlanQueryResult> {
    let mut players = list_players_table(pool, network_id, server_uuid, 2000).await?;
    apply_query_filters(&mut players, request);
    apply_activity_index_on_filter(pool, &mut players, request).await?;
    crate::plan_extension::filter_players_by_extension_query(
        &mut players,
        &request.plugins_boolean_groups,
        &request.plugin_numbers,
    );
    if request.played_after_ms.is_some() || request.played_before_ms.is_some() {
        let active = uuids_played_between(pool, network_id, server_uuid, request).await?;
        players.retain(|p| active.contains(&p.uuid));
    }

    let matched_count = players.len() as u64;
    let avg_activity_index = if players.is_empty() {
        0.0
    } else {
        players.iter().map(|p| p.activity_index).sum::<f64>() / players.len() as f64
    };
    let avg_active_playtime_ms = if players.is_empty() {
        0
    } else {
        players.iter().map(|p| p.active_playtime_ms).sum::<u64>() / players.len() as u64
    };

    players.sort_by(|a, b| b.activity_index.partial_cmp(&a.activity_index).unwrap());

    let cohort_view =
        if let (Some(after), Some(before)) = (request.view_after_ms, request.view_before_ms) {
            Some(
                compute_cohort_view(
                    pool,
                    network_id,
                    server_uuid,
                    &players,
                    after,
                    before,
                )
                .await?,
            )
        } else {
            None
        };

    Ok(PlanQueryResult {
        matched_count,
        players,
        avg_activity_index,
        avg_active_playtime_ms,
        cache_token: String::new(),
        from_cache: false,
        cohort_view,
    })
}

async fn uuids_played_between(
    pool: &SqlitePool,
    network_id: &str,
    scope_server_uuid: Option<&str>,
    request: &PlanQueryRequest,
) -> Result<std::collections::HashSet<String>> {
    let after = request.played_after_ms.unwrap_or(0);
    let before = request.played_before_ms.unwrap_or(i64::MAX);
    let mut sql = String::from(
        "SELECT DISTINCT u.uuid FROM plan_sessions sess
         JOIN plan_users u ON u.id = sess.user_id
         JOIN plan_servers s ON s.id = sess.server_id
         WHERE s.network_id = ? AND sess.session_end >= ? AND sess.session_start <= ?",
    );
    if scope_server_uuid.is_some() {
        sql.push_str(" AND s.uuid = ?");
    }
    if !request.played_server_uuids.is_empty() {
        let placeholders = request
            .played_server_uuids
            .iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");
        sql.push_str(&format!(" AND s.uuid IN ({placeholders})"));
    }
    let mut query = sqlx::query(&sql).bind(network_id).bind(after).bind(before);
    if let Some(uuid) = scope_server_uuid {
        query = query.bind(uuid);
    }
    for uuid in &request.played_server_uuids {
        query = query.bind(uuid);
    }
    let rows = query.fetch_all(pool).await?;
    Ok(rows
        .iter()
        .map(|r| r.get::<String, _>("uuid"))
        .collect())
}

async fn compute_cohort_view(
    pool: &SqlitePool,
    network_id: &str,
    server_uuid: Option<&str>,
    players: &[PlanPlayerTableRow],
    view_after: i64,
    view_before: i64,
) -> Result<PlanQueryCohortView> {
    if players.is_empty() || view_before <= view_after {
        return Ok(PlanQueryCohortView {
            view_after_ms: view_after,
            view_before_ms: view_before,
            sessions_in_window: 0,
            playtime_ms: 0,
            afk_ms: 0,
            players_active_in_window: 0,
        });
    }

    let uuids: Vec<String> = players.iter().map(|p| p.uuid.clone()).collect();
    let mut sessions_in_window = 0u64;
    let mut playtime_ms = 0u64;
    let mut afk_ms = 0u64;
    let mut active = std::collections::HashSet::new();

    for chunk in uuids.chunks(80) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = if server_uuid.is_some() {
            format!(
                "SELECT u.uuid, sess.session_start, sess.session_end, sess.afk_time
                 FROM plan_sessions sess
                 JOIN plan_users u ON u.id = sess.user_id
                 JOIN plan_servers s ON s.id = sess.server_id
                 WHERE s.network_id = ? AND s.uuid = ? AND u.uuid IN ({placeholders})
                   AND sess.session_end >= ? AND sess.session_start <= ?"
            )
        } else {
            format!(
                "SELECT u.uuid, sess.session_start, sess.session_end, sess.afk_time
                 FROM plan_sessions sess
                 JOIN plan_users u ON u.id = sess.user_id
                 JOIN plan_servers s ON s.id = sess.server_id
                 WHERE s.network_id = ? AND u.uuid IN ({placeholders})
                   AND sess.session_end >= ? AND sess.session_start <= ?"
            )
        };

        let mut query = sqlx::query(&sql).bind(network_id);
        if let Some(uuid) = server_uuid {
            query = query.bind(uuid);
        }
        for u in chunk {
            query = query.bind(u);
        }
        query = query.bind(view_after).bind(view_before);
        let rows = query.fetch_all(pool).await?;

        for row in rows {
            let uuid: String = row.get("uuid");
            let start: i64 = row.get("session_start");
            let end: i64 = row.get("session_end");
            let afk: u64 = row.get::<i64, _>("afk_time") as u64;
            let overlap_start = start.max(view_after);
            let overlap_end = end.min(view_before);
            if overlap_end <= overlap_start {
                continue;
            }
            let overlap = (overlap_end - overlap_start) as u64;
            let session_len = (end - start).max(1) as u64;
            sessions_in_window += 1;
            playtime_ms += overlap;
            afk_ms += afk * overlap / session_len;
            active.insert(uuid);
        }
    }

    Ok(PlanQueryCohortView {
        view_after_ms: view_after,
        view_before_ms: view_before,
        sessions_in_window,
        playtime_ms,
        afk_ms,
        players_active_in_window: active.len() as u64,
    })
}

/// APIv5 QueryService read path — single `SELECT` only (no writes).
pub async fn execute_readonly_sql(pool: &SqlitePool, sql: &str) -> Result<Vec<serde_json::Value>> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(Error::InvalidState("empty SQL".into()));
    }
    let upper = trimmed.to_ascii_uppercase();
    if !upper.starts_with("SELECT") {
        return Err(Error::InvalidState("only SELECT queries are permitted".into()));
    }
    if trimmed.contains(';') {
        return Err(Error::InvalidState("multiple SQL statements are not permitted".into()));
    }
    for forbidden in [
        "INSERT ", "UPDATE ", "DELETE ", "DROP ", "ATTACH ", "ALTER ", "CREATE ", "REPLACE ",
        "PRAGMA ", "VACUUM",
    ] {
        if upper.contains(forbidden) {
            return Err(Error::InvalidState(format!(
                "forbidden keyword in read-only query: {}",
                forbidden.trim()
            )));
        }
    }
    let rows = sqlx::query(trimmed).fetch_all(pool).await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let cols = row.columns();
        let mut obj = serde_json::Map::new();
        for (idx, col) in cols.iter().enumerate() {
            let name = col.name();
            obj.insert(name.to_string(), sqlite_cell_json(&row, idx));
        }
        out.push(serde_json::Value::Object(obj));
    }
    Ok(out)
}

fn sqlite_cell_json(row: &sqlx::sqlite::SqliteRow, idx: usize) -> serde_json::Value {
    use sqlx::ValueRef;
    if let Ok(raw) = row.try_get_raw(idx) {
        if raw.is_null() {
            return serde_json::Value::Null;
        }
    }
    if let Ok(v) = row.try_get::<i64, _>(idx) {
        return serde_json::json!(v);
    }
    if let Ok(v) = row.try_get::<f64, _>(idx) {
        return serde_json::Number::from_f64(v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null);
    }
    if let Ok(v) = row.try_get::<Vec<u8>, _>(idx) {
        return serde_json::Value::String(String::from_utf8_lossy(&v).into_owned());
    }
    row.try_get::<String, _>(idx)
        .ok()
        .map(serde_json::Value::String)
        .unwrap_or(serde_json::Value::Null)
}

fn activity_index_at_ms(triples: &[(i64, i64, u64)], at_ms: i64) -> patch_plan_core::ActivityIndex {
    use patch_plan_core::{active_playtime_in_weeks, ActivityIndex, DEFAULT_PLAYTIME_THRESHOLD_MS};
    let (w1, w2, w3) = active_playtime_in_weeks(triples, at_ms);
    ActivityIndex::calculate(w1, w2, w3, DEFAULT_PLAYTIME_THRESHOLD_MS)
}

async fn apply_activity_index_on_filter(
    pool: &SqlitePool,
    players: &mut Vec<PlanPlayerTableRow>,
    request: &PlanQueryRequest,
) -> Result<()> {
    let Some(filter) = &request.activity_index_on else {
        return Ok(());
    };
    let Some(at_ms) = filter.date else {
        return Ok(());
    };
    for player in players.iter_mut() {
        let triples = crate::plan_store::session_triples_for_user(pool, &player.uuid).await?;
        let ai = activity_index_at_ms(&triples, at_ms);
        player.activity_index = ai.value;
        player.activity_group = ai.group.label().to_string();
    }
    if !filter.groups.is_empty() {
        players.retain(|p| filter.groups.iter().any(|g| g == &p.activity_group));
    } else if !request.activity_groups.is_empty() {
        players.retain(|p| request.activity_groups.iter().any(|g| g == &p.activity_group));
    }
    if let Some(min) = request.min_activity_index {
        players.retain(|p| p.activity_index >= min);
    }
    if let Some(max) = request.max_activity_index {
        players.retain(|p| p.activity_index <= max);
    }
    Ok(())
}

/// In-memory filter used by `execute_plan_query` (unit-tested).
pub fn apply_query_filters(players: &mut Vec<PlanPlayerTableRow>, request: &PlanQueryRequest) {
    let historical_ai = request
        .activity_index_on
        .as_ref()
        .and_then(|f| f.date)
        .is_some();
    if !historical_ai && !request.activity_groups.is_empty() {
        players.retain(|p| request.activity_groups.iter().any(|g| g == &p.activity_group));
    }
    if !request.join_addresses.is_empty() {
        players.retain(|p| {
            request
                .join_addresses
                .iter()
                .any(|j| j.eq_ignore_ascii_case(&p.join_address))
        });
    }
    if !historical_ai {
        if let Some(min) = request.min_activity_index {
            players.retain(|p| p.activity_index >= min);
        }
        if let Some(max) = request.max_activity_index {
            players.retain(|p| p.activity_index <= max);
        }
    }
    if let Some(min) = request.min_sessions {
        players.retain(|p| p.sessions >= min);
    }
    if let Some(after) = request.registered_after_ms {
        players.retain(|p| p.registered >= after);
    }
    if let Some(before) = request.registered_before_ms {
        players.retain(|p| p.registered <= before);
    }
    if !request.geolocations.is_empty() {
        players.retain(|p| {
            request
                .geolocations
                .iter()
                .any(|g| g.eq_ignore_ascii_case(&p.geolocation))
        });
    }
    if let Some(after) = request.last_seen_after_ms {
        players.retain(|p| p.last_seen >= after);
    }
    if let Some(before) = request.last_seen_before_ms {
        players.retain(|p| p.last_seen <= before);
    }
    if request.banned.iter().any(|b| b.eq_ignore_ascii_case("Banned")) {
        players.retain(|p| p.banned);
    } else if request
        .banned
        .iter()
        .any(|b| b.eq_ignore_ascii_case("Not banned"))
    {
        players.retain(|p| !p.banned);
    }
    if request.operators.iter().any(|b| b.eq_ignore_ascii_case("Operators")) {
        players.retain(|p| p.operator);
    } else if request
        .operators
        .iter()
        .any(|b| b.eq_ignore_ascii_case("Non-operators"))
    {
        players.retain(|p| !p.operator);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_player(group: &str, index: f64, join: &str) -> PlanPlayerTableRow {
        PlanPlayerTableRow {
            name: "Steve".into(),
            uuid: "uuid".into(),
            activity_index: index,
            activity_group: group.into(),
            active_playtime_ms: 1000,
            sessions: 1,
            registered: 1,
            last_seen: 0,
            join_address: join.into(),
            geolocation: String::new(),
            ping_avg: 0,
            ping_min: 0,
            ping_max: 0,
            nick: String::new(),
            kick_count: 0,
            banned: false,
            operator: false,
        }
    }

    #[test]
    fn filters_by_banned() {
        let mut players = vec![
            sample_player("Active", 50.0, "a"),
            PlanPlayerTableRow {
                name: "Banned".into(),
                uuid: "b".into(),
                activity_index: 10.0,
                activity_group: "Inactive".into(),
                active_playtime_ms: 0,
                sessions: 1,
                registered: 1,
                last_seen: 0,
                join_address: "a".into(),
                geolocation: String::new(),
                ping_avg: 0,
                ping_min: 0,
                ping_max: 0,
                nick: String::new(),
                kick_count: 0,
                banned: true,
                operator: false,
            },
        ];
        let req = PlanQueryRequest {
            banned: vec!["Banned".into()],
            ..PlanQueryRequest::default()
        };
        apply_query_filters(&mut players, &req);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Banned");
    }

    #[test]
    fn filters_by_activity_group_and_min_index() {
        let mut players = vec![
            sample_player("Active", 50.0, "play.example.com"),
            sample_player("Inactive", 10.0, "play.example.com"),
        ];
        let req = PlanQueryRequest {
            activity_groups: vec!["Active".into()],
            min_activity_index: Some(40.0),
            ..PlanQueryRequest::default()
        };
        apply_query_filters(&mut players, &req);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].activity_group, "Active");
    }

    #[test]
    fn filters_by_min_sessions() {
        let mut players = vec![
            sample_player("Active", 50.0, "a"),
            PlanPlayerTableRow {
                name: "Bob".into(),
                uuid: "b".into(),
                activity_index: 40.0,
                activity_group: "Active".into(),
                active_playtime_ms: 1000,
                sessions: 5,
                registered: 1,
                last_seen: 0,
                join_address: "a".into(),
                geolocation: String::new(),
                ping_avg: 0,
                ping_min: 0,
                ping_max: 0,
                nick: String::new(),
                kick_count: 0,
                banned: false,
                operator: false,
            },
        ];
        players[0].sessions = 1;
        let req = PlanQueryRequest {
            min_sessions: Some(3),
            ..PlanQueryRequest::default()
        };
        apply_query_filters(&mut players, &req);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Bob");
    }

    #[test]
    fn filters_by_last_seen_range() {
        let mut players = vec![
            sample_player("Active", 50.0, "a"),
            PlanPlayerTableRow {
                name: "Old".into(),
                uuid: "old".into(),
                activity_index: 10.0,
                activity_group: "Inactive".into(),
                active_playtime_ms: 0,
                sessions: 1,
                registered: 1,
                last_seen: 50,
                join_address: "a".into(),
                geolocation: String::new(),
                ping_avg: 0,
                ping_min: 0,
                ping_max: 0,
                nick: String::new(),
                kick_count: 0,
                banned: false,
                operator: false,
            },
        ];
        players[0].last_seen = 500;
        let req = PlanQueryRequest {
            last_seen_after_ms: Some(100),
            ..PlanQueryRequest::default()
        };
        apply_query_filters(&mut players, &req);
        assert_eq!(players.len(), 1);
        assert_eq!(players[0].name, "Steve");
    }

    #[test]
    fn activity_index_on_defers_current_activity_group_filter() {
        let mut players = vec![sample_player("Active", 50.0, "a")];
        let req = PlanQueryRequest {
            activity_groups: vec!["Inactive".into()],
            activity_index_on: Some(ActivityIndexOnFilter {
                date: Some(1_700_000_000_000),
                groups: vec![],
            }),
            ..PlanQueryRequest::default()
        };
        apply_query_filters(&mut players, &req);
        assert_eq!(players.len(), 1);
    }

    #[test]
    fn activity_index_at_ms_uses_session_windows() {
        use patch_plan_core::ONE_WEEK_MS;
        let at_ms = 3 * ONE_WEEK_MS;
        let triples = vec![(at_ms - 86_400_000, at_ms - 43_200_000, 0)];
        let ai = activity_index_at_ms(&triples, at_ms);
        assert!(ai.value > 0.0);
    }

    #[tokio::test]
    async fn readonly_sql_allows_select_and_blocks_writes() {
        use sqlx::sqlite::SqlitePoolOptions;
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("memory sqlite");
        sqlx::query("CREATE TABLE demo (id INTEGER, label TEXT)")
            .execute(&pool)
            .await
            .expect("create");
        sqlx::query("INSERT INTO demo (id, label) VALUES (2, 'ok')")
            .execute(&pool)
            .await
            .expect("insert");
        let rows = execute_readonly_sql(&pool, "SELECT id, label FROM demo WHERE id = 2")
            .await
            .expect("select");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].get("id").and_then(|v| v.as_i64()), Some(2));
        assert!(execute_readonly_sql(&pool, "DELETE FROM demo").await.is_err());
        assert!(execute_readonly_sql(&pool, "SELECT 1; SELECT 2").await.is_err());
    }
}
