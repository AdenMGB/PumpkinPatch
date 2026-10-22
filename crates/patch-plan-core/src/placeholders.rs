//! Plan PlaceholderAPI resolver (indepth.md §7.2 — expanded subset).

use crate::{active_playtime_in_weeks, ActivityIndex, DEFAULT_PLAYTIME_THRESHOLD_MS};

const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 7 * DAY_MS;
const MONTH_MS: i64 = 30 * DAY_MS;

#[derive(Debug, Clone, Default)]
pub struct PlanPlaceholderContext {
    pub server_name: String,
    pub server_uuid: String,
    pub players_online: u32,
    pub players_total: u64,
    pub peak_online: u32,
    pub total_joins: u64,
    pub player_name: Option<String>,
    pub player_uuid: Option<String>,
    pub playtime_secs: u64,
    pub afk_secs: u64,
    pub deaths: u32,
    pub player_kills: u32,
    pub mob_kills: u32,
    pub kick_count: u32,
    pub geolocation: String,
    pub registered_unix: i64,
    pub last_seen_unix: i64,
    pub session_triples: Vec<(i64, i64, u64)>,
    pub now_ms: i64,
    pub avg_tps_day: Option<f64>,
    pub avg_tps_week: Option<f64>,
    pub avg_tps_month: Option<f64>,
    pub avg_ping_day: Option<f64>,
    pub avg_ping_week: Option<f64>,
    pub avg_ping_month: Option<f64>,
    pub min_ping_day: Option<u32>,
    pub max_ping_day: Option<u32>,
    pub player_ping_avg: Option<f64>,
    pub player_ping_min: Option<u32>,
    pub player_ping_max: Option<u32>,
    pub players_new_day: u64,
    pub players_new_week: u64,
    pub players_new_month: u64,
    pub players_unique_day: u64,
    pub players_unique_week: u64,
    pub players_unique_month: u64,
    pub sessions_playtime_secs: u64,
    pub sessions_active_secs: u64,
    pub sessions_afk_secs: u64,
    pub sessions_count_day: u64,
    pub sessions_count_week: u64,
    pub sessions_count_month: u64,
    pub timezone_offset: String,
    /// Player's recent PvP kills as `(victim_name, weapon)`.
    pub player_recent_kills: Vec<(String, String)>,
    pub deaths_total: u64,
    pub deaths_day: u64,
    pub deaths_week: u64,
    pub deaths_month: u64,
    pub mob_kills_total: u64,
    pub mob_kills_day: u64,
    pub mob_kills_week: u64,
    pub mob_kills_month: u64,
    pub player_kills_total: u64,
    pub player_kills_day: u64,
    pub player_kills_week: u64,
    pub player_kills_month: u64,
    pub sessions_playtime_day: u64,
    pub sessions_playtime_week: u64,
    pub sessions_playtime_month: u64,
    pub peak_online_date_alltime: String,
    pub peak_online_date_recent: String,
    pub favorite_server: String,
    pub tps_spikes_day: u32,
    pub sessions_avg_length_day: u64,
    pub sessions_avg_length_week: u64,
    pub sessions_avg_length_month: u64,
    pub sessions_avg_unique_players_day: u64,
    pub sessions_active_day: u64,
    pub sessions_active_week: u64,
    pub sessions_active_month: u64,
    pub sessions_afk_day: u64,
    pub sessions_afk_week: u64,
    pub sessions_afk_month: u64,
    pub sessions_avg_unique_players_week: u64,
    pub sessions_avg_unique_players_month: u64,
    pub tps_spikes_week: u32,
    pub tps_spikes_month: u32,
    pub player_is_banned: bool,
    pub player_is_operator: bool,
    pub player_session_length_longest_secs: u64,
    /// Players classified in the Regular activity group (indepth §7.2).
    pub players_regular: u64,
    /// Ranked `(name, playtime_secs, afk_secs)` for top-N placeholders (all-time).
    pub top_playtime: Vec<(String, u64, u64)>,
    pub top_playtime_day: Vec<(String, u64, u64)>,
    pub top_playtime_week: Vec<(String, u64, u64)>,
    pub top_playtime_month: Vec<(String, u64, u64)>,
}

/// Sort players by total playtime (desc) and keep the top `limit` rows.
pub fn sort_top_playtime(
    entries: Vec<(String, u64, u64)>,
    limit: usize,
) -> Vec<(String, u64, u64)> {
    let mut entries = entries;
    entries.sort_by_key(|(_, play, _)| std::cmp::Reverse(*play));
    entries.truncate(limit);
    entries
}

pub fn resolve_placeholder(key: &str, ctx: &PlanPlaceholderContext) -> Option<String> {
    let key = key.trim();
    if !key.starts_with("%plan_") || !key.ends_with('%') {
        return None;
    }
    let inner = key.trim_matches('%');
    if let Some(v) = resolve_top_placeholder(inner, ctx) {
        return Some(v);
    }
    if let Some(v) = resolve_timed_duration(inner, ctx) {
        return Some(v);
    }
    if let Some(v) = resolve_server_session_duration(inner, ctx) {
        return Some(v);
    }
    if let Some(v) = resolve_recent_kill(inner, ctx) {
        return Some(v);
    }
    if let Some(v) = resolve_sessions_combat(inner, ctx) {
        return Some(v);
    }
    match inner {
        "plan_server_name" => Some(ctx.server_name.clone()),
        "plan_server_uuid" => Some(ctx.server_uuid.clone()),
        "plan_players_online" => Some(ctx.players_online.to_string()),
        "plan_players_total" => Some(ctx.players_total.to_string()),
        "plan_players_max_peak_alltime" => Some(ctx.peak_online.to_string()),
        "plan_players_max_peak_recent" => Some(ctx.peak_online.to_string()),
        "plan_players_max_peak_date_alltime" => Some(ctx.peak_online_date_alltime.clone()),
        "plan_players_max_peak_date_recent" => Some(ctx.peak_online_date_recent.clone()),
        "plan_sessions_playtime_day" => Some(format_duration(ctx.sessions_playtime_day)),
        "plan_sessions_playtime_week" => Some(format_duration(ctx.sessions_playtime_week)),
        "plan_sessions_playtime_month" => Some(format_duration(ctx.sessions_playtime_month)),
        "plan_sessions_active_time_day" => Some(format_duration(ctx.sessions_active_day)),
        "plan_sessions_active_time_week" => Some(format_duration(ctx.sessions_active_week)),
        "plan_sessions_active_time_month" => Some(format_duration(ctx.sessions_active_month)),
        "plan_sessions_afk_time_day" => Some(format_duration(ctx.sessions_afk_day)),
        "plan_sessions_afk_time_week" => Some(format_duration(ctx.sessions_afk_week)),
        "plan_sessions_afk_time_month" => Some(format_duration(ctx.sessions_afk_month)),
        "plan_tps_spikes_day" => Some(ctx.tps_spikes_day.to_string()),
        "plan_tps_spikes_week" => Some(ctx.tps_spikes_week.to_string()),
        "plan_tps_spikes_month" => Some(ctx.tps_spikes_month.to_string()),
        "plan_player_favorite_server" => Some(ctx.favorite_server.clone()),
        "plan_players_regular" => Some(ctx.players_regular.to_string()),
        "plan_players_new_day" => Some(ctx.players_new_day.to_string()),
        "plan_players_new_week" => Some(ctx.players_new_week.to_string()),
        "plan_players_new_month" => Some(ctx.players_new_month.to_string()),
        "plan_players_unique_day" => Some(ctx.players_unique_day.to_string()),
        "plan_players_unique_week" => Some(ctx.players_unique_week.to_string()),
        "plan_players_unique_month" => Some(ctx.players_unique_month.to_string()),
        "plan_sessions_playtime_total" => Some(format_duration(ctx.sessions_playtime_secs)),
        "plan_sessions_active_time_total" => Some(format_duration(ctx.sessions_active_secs)),
        "plan_sessions_afk_time_total" => Some(format_duration(ctx.sessions_afk_secs)),
        "plan_sessions_count_day" => Some(ctx.sessions_count_day.to_string()),
        "plan_sessions_count_week" => Some(ctx.sessions_count_week.to_string()),
        "plan_sessions_count_month" => Some(ctx.sessions_count_month.to_string()),
        "plan_sessions_average_length_day" => Some(format_duration(ctx.sessions_avg_length_day)),
        "plan_sessions_average_length_week" => Some(format_duration(ctx.sessions_avg_length_week)),
        "plan_sessions_average_length_month" => Some(format_duration(ctx.sessions_avg_length_month)),
        "plan_sessions_average_unique_players_day" => {
            Some(ctx.sessions_avg_unique_players_day.to_string())
        }
        "plan_sessions_average_unique_players_week" => {
            Some(ctx.sessions_avg_unique_players_week.to_string())
        }
        "plan_sessions_average_unique_players_month" => {
            Some(ctx.sessions_avg_unique_players_month.to_string())
        }
        "plan_server_time_zone" => Some(ctx.timezone_offset.clone()),
        "plan_server_time_zone_offset" => Some(ctx.timezone_offset.clone()),
        "plan_player_name" => ctx.player_name.clone(),
        "plan_player_session_length" => {
            let active = ctx.playtime_secs.saturating_sub(ctx.afk_secs);
            Some(format_duration(active))
        }
        "plan_player_is_operator" => Some(ctx.player_is_operator.to_string()),
        "plan_player_is_banned" => Some(ctx.player_is_banned.to_string()),
        "plan_sessions_count_total" => Some(ctx.total_joins.to_string()),
        "plan_player_time_total" => Some(format_duration(ctx.playtime_secs)),
        "plan_player_time_afk_total" => Some(format_duration(ctx.afk_secs)),
        "plan_player_time_active_total" => {
            let active = ctx.playtime_secs.saturating_sub(ctx.afk_secs);
            Some(format_duration(active))
        }
        "plan_player_deaths" => Some(ctx.deaths.to_string()),
        "plan_player_player_kills" => Some(ctx.player_kills.to_string()),
        "plan_player_mob_kills" => Some(ctx.mob_kills.to_string()),
        "plan_player_kicked_count" => Some(ctx.kick_count.to_string()),
        "plan_player_geolocation" => Some(ctx.geolocation.clone()),
        "plan_player_registered" => Some(ctx.registered_unix.to_string()),
        "plan_player_lastseen" => Some(ctx.last_seen_unix.to_string()),
        "plan_player_kdr" => {
            let k = ctx.player_kills as f64;
            let d = ctx.deaths.max(1) as f64;
            Some(format!("{:.2}", k / d))
        }
        "plan_player_mob_kdr" => {
            let k = ctx.mob_kills as f64;
            let d = ctx.deaths.max(1) as f64;
            Some(format!("{:.2}", k / d))
        }
        "plan_player_activity_index" => Some(format_activity_index(ctx)),
        "plan_player_activity_group" => Some(activity_group_label(ctx)),
        "plan_tps_day" => ctx.avg_tps_day.map(|t| format!("{:.2}", t)),
        "plan_tps_week" => ctx.avg_tps_week.map(|t| format!("{:.2}", t)),
        "plan_tps_month" => ctx.avg_tps_month.map(|t| format!("{:.2}", t)),
        "plan_ping_average_day" => ctx.avg_ping_day.map(|p| format!("{:.0}", p)),
        "plan_ping_average_week" => ctx.avg_ping_week.map(|p| format!("{:.0}", p)),
        "plan_ping_average_month" => ctx.avg_ping_month.map(|p| format!("{:.0}", p)),
        "plan_ping_min_day" => ctx.min_ping_day.map(|p| p.to_string()),
        "plan_ping_max_day" => ctx.max_ping_day.map(|p| p.to_string()),
        "plan_player_ping_average" => ctx.player_ping_avg.map(|p| format!("{:.0}", p)),
        "plan_player_ping_min" => ctx.player_ping_min.map(|p| p.to_string()),
        "plan_player_ping_max" => ctx.player_ping_max.map(|p| p.to_string()),
        "plan_player_sessions_count" => Some(ctx.session_triples.len().to_string()),
        "plan_player_session_length_average" => {
            let n = ctx.session_triples.len().max(1) as u64;
            let total = ctx.playtime_secs;
            Some(format_duration(total / n))
        }
        "plan_player_session_length_Longest" | "plan_player_session_length_longest" => {
            Some(format_duration(ctx.player_session_length_longest_secs))
        }
        "plan_player_player_caused_deaths" => Some(ctx.player_kills.to_string()),
        _ => None,
    }
}

fn resolve_sessions_combat(inner: &str, ctx: &PlanPlaceholderContext) -> Option<String> {
    match inner {
        "plan_sessions_deaths_total" => Some(ctx.deaths_total.to_string()),
        "plan_sessions_deaths_day" => Some(ctx.deaths_day.to_string()),
        "plan_sessions_deaths_week" => Some(ctx.deaths_week.to_string()),
        "plan_sessions_deaths_month" => Some(ctx.deaths_month.to_string()),
        "plan_sessions_mob_kills_total" => Some(ctx.mob_kills_total.to_string()),
        "plan_sessions_mob_kills_day" => Some(ctx.mob_kills_day.to_string()),
        "plan_sessions_mob_kills_week" => Some(ctx.mob_kills_week.to_string()),
        "plan_sessions_mob_kills_month" => Some(ctx.mob_kills_month.to_string()),
        "plan_sessions_player_kills_total" => Some(ctx.player_kills_total.to_string()),
        "plan_sessions_player_kills_day" => Some(ctx.player_kills_day.to_string()),
        "plan_sessions_player_kills_week" => Some(ctx.player_kills_week.to_string()),
        "plan_sessions_player_kills_month" => Some(ctx.player_kills_month.to_string()),
        _ => None,
    }
}

fn resolve_recent_kill(inner: &str, ctx: &PlanPlaceholderContext) -> Option<String> {
    if let Some(rest) = inner.strip_prefix("plan_player_recent_kill_weapon_") {
        let idx = rest.parse::<usize>().ok().filter(|n| *n >= 1 && *n <= 10)?;
        return Some(
            ctx.player_recent_kills
                .get(idx - 1)
                .map(|(_, weapon)| weapon.clone())
                .unwrap_or_default(),
        );
    }
    if let Some(rest) = inner.strip_prefix("plan_player_recent_kill_") {
        let idx = rest.parse::<usize>().ok().filter(|n| *n >= 1 && *n <= 10)?;
        return Some(
            ctx.player_recent_kills
                .get(idx - 1)
                .map(|(victim, _)| victim.clone())
                .unwrap_or_default(),
        );
    }
    None
}

fn resolve_server_session_duration(inner: &str, ctx: &PlanPlaceholderContext) -> Option<String> {
    let raw = inner.ends_with("_raw");
    let base = inner.strip_suffix("_raw").unwrap_or(inner);
    let (secs, ms) = match base {
        "plan_sessions_playtime_total" => (ctx.sessions_playtime_secs, ctx.sessions_playtime_secs * 1000),
        "plan_sessions_playtime_day" => (ctx.sessions_playtime_day, ctx.sessions_playtime_day * 1000),
        "plan_sessions_playtime_week" => (ctx.sessions_playtime_week, ctx.sessions_playtime_week * 1000),
        "plan_sessions_playtime_month" => (ctx.sessions_playtime_month, ctx.sessions_playtime_month * 1000),
        "plan_sessions_active_time_total" => (ctx.sessions_active_secs, ctx.sessions_active_secs * 1000),
        "plan_sessions_active_time_day" => (ctx.sessions_active_day, ctx.sessions_active_day * 1000),
        "plan_sessions_active_time_week" => (ctx.sessions_active_week, ctx.sessions_active_week * 1000),
        "plan_sessions_active_time_month" => (ctx.sessions_active_month, ctx.sessions_active_month * 1000),
        "plan_sessions_afk_time_total" => (ctx.sessions_afk_secs, ctx.sessions_afk_secs * 1000),
        "plan_sessions_afk_time_day" => (ctx.sessions_afk_day, ctx.sessions_afk_day * 1000),
        "plan_sessions_afk_time_week" => (ctx.sessions_afk_week, ctx.sessions_afk_week * 1000),
        "plan_sessions_afk_time_month" => (ctx.sessions_afk_month, ctx.sessions_afk_month * 1000),
        _ => return None,
    };
    if raw {
        Some(ms.to_string())
    } else {
        Some(format_duration(secs))
    }
}

fn resolve_timed_duration(inner: &str, ctx: &PlanPlaceholderContext) -> Option<String> {
    let (window, raw) = match inner {
        s if s.starts_with("plan_player_time_day") => (DAY_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_week") => (WEEK_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_month") => (MONTH_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_active_day") => (DAY_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_active_week") => (WEEK_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_active_month") => (MONTH_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_afk_day") => (DAY_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_afk_week") => (WEEK_MS, s.ends_with("_raw")),
        s if s.starts_with("plan_player_time_afk_month") => (MONTH_MS, s.ends_with("_raw")),
        _ => return None,
    };
    let (active_ms, afk_ms) = playtime_in_window(&ctx.session_triples, ctx.now_ms, window);
    let active = active_ms / 1000;
    let afk = afk_ms / 1000;
    let value_secs = if inner.contains("_afk_") {
        afk
    } else if inner.contains("_active_") {
        active
    } else {
        active + afk
    };
    if raw {
        Some((value_secs * 1000).to_string())
    } else {
        Some(format_duration(value_secs))
    }
}

fn resolve_top_placeholder(inner: &str, ctx: &PlanPlaceholderContext) -> Option<String> {
    // plan_top_player_playtime_total_active_3 | plan_top_amount_playtime_total_active_3
    let parts: Vec<&str> = inner.split('_').collect();
    if parts.len() < 7 || parts[0] != "plan" || parts[1] != "top" {
        return None;
    }
    let (is_amount, play_idx) = match parts[2] {
        "amount" => (true, 3),
        "player" => (false, 3),
        _ => return None,
    };
    if parts.get(play_idx) != Some(&"playtime") {
        return None;
    }
    let window = parts.get(play_idx + 1)?;
    let kind = parts.get(play_idx + 2)?;
    let rank: usize = parts
        .get(play_idx + 3)?
        .parse()
        .ok()
        .filter(|n| *n >= 1 && *n <= 10)?;
    let window_ms = match *window {
        "total" => i64::MAX / 2,
        "day" => DAY_MS,
        "week" => WEEK_MS,
        "month" => MONTH_MS,
        _ => return None,
    };
    let idx = rank - 1;
    let ranked = ranked_for_window(ctx, window_ms);
    if is_amount {
        let secs = match ranked.get(idx) {
            Some(entry) => match *kind {
                "active" => entry.1.saturating_sub(entry.2),
                "afk" => entry.2,
                _ => entry.1,
            },
            None => 0,
        };
        return Some(format_duration(secs));
    }
    Some(
        ranked
            .get(idx)
            .map(|(name, _, _)| name.clone())
            .unwrap_or_default(),
    )
}

fn ranked_for_window(ctx: &PlanPlaceholderContext, window_ms: i64) -> Vec<(String, u64, u64)> {
    match window_ms {
        DAY_MS => ctx.top_playtime_day.clone(),
        WEEK_MS => ctx.top_playtime_week.clone(),
        MONTH_MS => ctx.top_playtime_month.clone(),
        _ => ctx.top_playtime.clone(),
    }
}

fn playtime_in_window(triples: &[(i64, i64, u64)], now_ms: i64, window_ms: i64) -> (u64, u64) {
    let cutoff = now_ms - window_ms;
    let mut active_ms = 0u64;
    let mut afk_ms = 0u64;
    for (start, end, afk) in triples {
        if *end < cutoff {
            continue;
        }
        let clipped_start = (*start).max(cutoff);
        let span = (*end - clipped_start).max(0) as u64;
        let afk_clip = (*afk).min(span);
        afk_ms += afk_clip;
        active_ms += span.saturating_sub(afk_clip);
    }
    (active_ms, afk_ms)
}

fn format_activity_index(ctx: &PlanPlaceholderContext) -> String {
    let (w1, w2, w3) = active_playtime_in_weeks(&ctx.session_triples, ctx.now_ms);
    let ai = ActivityIndex::calculate(w1, w2, w3, DEFAULT_PLAYTIME_THRESHOLD_MS);
    format!("{:.2}", ai.value)
}

fn activity_group_label(ctx: &PlanPlaceholderContext) -> String {
    let (w1, w2, w3) = active_playtime_in_weeks(&ctx.session_triples, ctx.now_ms);
    let ai = ActivityIndex::calculate(w1, w2, w3, DEFAULT_PLAYTIME_THRESHOLD_MS);
    ai.group.label().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_player_online_placeholder() {
        let ctx = PlanPlaceholderContext {
            players_online: 12,
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_players_online%", &ctx).as_deref(),
            Some("12")
        );
    }

    #[test]
    fn resolves_sessions_deaths_total() {
        let ctx = PlanPlaceholderContext {
            deaths_total: 42,
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_sessions_deaths_total%", &ctx).as_deref(),
            Some("42")
        );
    }

    #[test]
    fn resolves_players_new_day() {
        let ctx = PlanPlaceholderContext {
            players_new_day: 3,
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_players_new_day%", &ctx).as_deref(),
            Some("3")
        );
    }

    #[test]
    fn resolves_sessions_playtime_day_raw() {
        let ctx = PlanPlaceholderContext {
            sessions_playtime_day: 90,
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_sessions_playtime_day_raw%", &ctx).as_deref(),
            Some("90000")
        );
    }

    #[test]
    fn resolves_tps_week_from_context() {
        let ctx = PlanPlaceholderContext {
            avg_tps_week: Some(19.5),
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_tps_week%", &ctx).as_deref(),
            Some("19.50")
        );
    }

    #[test]
    fn resolves_player_name() {
        let ctx = PlanPlaceholderContext {
            player_name: Some("Steve".into()),
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_player_name%", &ctx).as_deref(),
            Some("Steve")
        );
    }

    #[test]
    fn resolves_player_banned_flag() {
        let ctx = PlanPlaceholderContext {
            player_is_banned: true,
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_player_is_banned%", &ctx).as_deref(),
            Some("true")
        );
    }

    #[test]
    fn resolves_player_sessions_count_from_triples() {
        let ctx = PlanPlaceholderContext {
            session_triples: vec![(0, 1000, 0), (2000, 3000, 0)],
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_player_sessions_count%", &ctx).as_deref(),
            Some("2")
        );
    }

    #[test]
    fn resolves_players_regular() {
        let ctx = PlanPlaceholderContext {
            players_regular: 7,
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_players_regular%", &ctx).as_deref(),
            Some("7")
        );
    }

    #[test]
    fn indepth_server_placeholder_catalog_resolves() {
        let ctx = PlanPlaceholderContext {
            server_name: "Hub".into(),
            server_uuid: "uuid".into(),
            players_online: 5,
            players_total: 100,
            peak_online: 42,
            players_new_day: 1,
            players_new_week: 2,
            players_new_month: 3,
            players_unique_day: 4,
            players_unique_week: 5,
            players_unique_month: 6,
            sessions_playtime_secs: 3600,
            sessions_active_secs: 3000,
            sessions_afk_secs: 600,
            sessions_count_day: 10,
            sessions_count_week: 20,
            sessions_count_month: 30,
            sessions_avg_length_day: 120,
            sessions_avg_length_week: 180,
            sessions_avg_length_month: 240,
            sessions_avg_unique_players_day: 3,
            sessions_avg_unique_players_week: 4,
            sessions_avg_unique_players_month: 5,
            deaths_total: 9,
            deaths_day: 1,
            deaths_week: 2,
            deaths_month: 3,
            mob_kills_total: 4,
            mob_kills_day: 1,
            mob_kills_week: 1,
            mob_kills_month: 2,
            player_kills_total: 5,
            player_kills_day: 1,
            player_kills_week: 2,
            player_kills_month: 2,
            avg_tps_day: Some(19.8),
            avg_tps_week: Some(19.5),
            avg_tps_month: Some(19.2),
            tps_spikes_day: 1,
            tps_spikes_week: 2,
            tps_spikes_month: 3,
            avg_ping_day: Some(45.0),
            avg_ping_week: Some(50.0),
            avg_ping_month: Some(55.0),
            min_ping_day: Some(20),
            max_ping_day: Some(120),
            timezone_offset: "UTC".into(),
            peak_online_date_alltime: "2020".into(),
            peak_online_date_recent: "2021".into(),
            players_regular: 8,
            total_joins: 500,
            sessions_playtime_day: 100,
            sessions_playtime_week: 200,
            sessions_playtime_month: 300,
            sessions_active_day: 80,
            sessions_active_week: 160,
            sessions_active_month: 240,
            sessions_afk_day: 20,
            sessions_afk_week: 40,
            sessions_afk_month: 60,
            top_playtime: vec![("Alex".into(), 100, 10)],
            ..PlanPlaceholderContext::default()
        };
        let keys = [
            "%plan_server_name%",
            "%plan_server_uuid%",
            "%plan_players_total%",
            "%plan_players_new_day%",
            "%plan_players_online%",
            "%plan_sessions_playtime_total%",
            "%plan_sessions_deaths_total%",
            "%plan_tps_day%",
            "%plan_tps_week%",
            "%plan_tps_month%",
            "%plan_ping_average_day%",
            "%plan_top_player_playtime_total_active_1%",
        ];
        for key in keys {
            assert!(
                resolve_placeholder(key, &ctx).is_some(),
                "expected {key} to resolve"
            );
        }
    }

    #[test]
    fn indepth_full_server_catalog_resolves() {
        use crate::placeholder_catalog::{server_placeholder_keys, wrap_percent};
        let ctx = rich_server_ctx();
        let missing: Vec<_> = server_placeholder_keys()
            .iter()
            .map(|k| wrap_percent(k))
            .filter(|k| resolve_placeholder(k, &ctx).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "unresolved server placeholders: {:?}",
            missing
        );
    }

    #[test]
    fn indepth_full_player_catalog_resolves() {
        use crate::placeholder_catalog::{player_placeholder_keys, wrap_percent};
        let ctx = rich_player_ctx();
        let missing: Vec<_> = player_placeholder_keys()
            .iter()
            .map(|k| wrap_percent(k))
            .filter(|k| resolve_placeholder(k, &ctx).is_none())
            .collect();
        assert!(
            missing.is_empty(),
            "unresolved player placeholders: {:?}",
            missing
        );
    }

    fn rich_server_ctx() -> PlanPlaceholderContext {
        PlanPlaceholderContext {
            server_name: "Hub".into(),
            server_uuid: "uuid".into(),
            players_online: 5,
            players_total: 100,
            peak_online: 42,
            players_new_day: 1,
            players_new_week: 2,
            players_new_month: 3,
            players_unique_day: 4,
            players_unique_week: 5,
            players_unique_month: 6,
            sessions_playtime_secs: 3600,
            sessions_active_secs: 3000,
            sessions_afk_secs: 600,
            sessions_count_day: 10,
            sessions_count_week: 20,
            sessions_count_month: 30,
            sessions_avg_length_day: 120,
            sessions_avg_length_week: 180,
            sessions_avg_length_month: 240,
            sessions_avg_unique_players_day: 3,
            sessions_avg_unique_players_week: 4,
            sessions_avg_unique_players_month: 5,
            deaths_total: 9,
            deaths_day: 1,
            deaths_week: 2,
            deaths_month: 3,
            mob_kills_total: 4,
            mob_kills_day: 1,
            mob_kills_week: 1,
            mob_kills_month: 2,
            player_kills_total: 5,
            player_kills_day: 1,
            player_kills_week: 2,
            player_kills_month: 2,
            avg_tps_day: Some(19.8),
            avg_tps_week: Some(19.5),
            avg_tps_month: Some(19.2),
            tps_spikes_day: 1,
            tps_spikes_week: 2,
            tps_spikes_month: 3,
            avg_ping_day: Some(45.0),
            avg_ping_week: Some(50.0),
            avg_ping_month: Some(55.0),
            min_ping_day: Some(20),
            max_ping_day: Some(120),
            timezone_offset: "UTC".into(),
            peak_online_date_alltime: "2020".into(),
            peak_online_date_recent: "2021".into(),
            players_regular: 8,
            total_joins: 500,
            sessions_playtime_day: 100,
            sessions_playtime_week: 200,
            sessions_playtime_month: 300,
            sessions_active_day: 80,
            sessions_active_week: 160,
            sessions_active_month: 240,
            sessions_afk_day: 20,
            sessions_afk_week: 40,
            sessions_afk_month: 60,
            top_playtime: vec![("Alex".into(), 100, 10)],
            ..PlanPlaceholderContext::default()
        }
    }

    fn rich_player_ctx() -> PlanPlaceholderContext {
        let now_ms = 1_700_000_000_000i64;
        PlanPlaceholderContext {
            player_name: Some("Steve".into()),
            player_uuid: Some("player-1".into()),
            playtime_secs: 7200,
            afk_secs: 600,
            deaths: 3,
            player_kills: 5,
            mob_kills: 12,
            kick_count: 1,
            geolocation: "AU".into(),
            registered_unix: 1_600_000_000,
            last_seen_unix: 1_699_000_000,
            session_triples: vec![(now_ms - 3_600_000, now_ms, 60_000)],
            now_ms,
            avg_tps_day: Some(20.0),
            avg_tps_week: Some(19.0),
            avg_tps_month: Some(18.0),
            avg_ping_day: Some(40.0),
            avg_ping_week: Some(45.0),
            avg_ping_month: Some(50.0),
            min_ping_day: Some(15),
            max_ping_day: Some(90),
            player_ping_avg: Some(42.0),
            player_ping_min: Some(20),
            player_ping_max: Some(80),
            favorite_server: "Survival".into(),
            player_is_banned: false,
            player_is_operator: true,
            player_session_length_longest_secs: 3600,
            player_recent_kills: vec![("Victim".into(), "diamond_sword".into())],
            top_playtime: vec![("Steve".into(), 7200, 600)],
            ..rich_server_ctx()
        }
    }

    #[test]
    fn resolves_top_playtime_name() {
        let ctx = PlanPlaceholderContext {
            top_playtime: vec![("Alex".into(), 3600, 60), ("Bob".into(), 120, 0)],
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_top_player_playtime_total_active_1%", &ctx).as_deref(),
            Some("Alex")
        );
        assert_eq!(
            resolve_placeholder("%plan_top_amount_playtime_total_active_2%", &ctx).as_deref(),
            Some("2m 0s")
        );
    }

    #[test]
    fn top_leaderboard_uses_window_specific_lists() {
        let ctx = PlanPlaceholderContext {
            top_playtime: vec![("AllTime".into(), 9999, 0)],
            top_playtime_day: vec![("Daily".into(), 100, 5)],
            ..PlanPlaceholderContext::default()
        };
        assert_eq!(
            resolve_placeholder("%plan_top_player_playtime_total_active_1%", &ctx).as_deref(),
            Some("AllTime")
        );
        assert_eq!(
            resolve_placeholder("%plan_top_player_playtime_day_active_1%", &ctx).as_deref(),
            Some("Daily")
        );
    }
}

fn format_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{}h {}m", h, m)
    } else if m > 0 {
        format!("{}m {}s", m, s)
    } else {
        format!("{}s", s)
    }
}
