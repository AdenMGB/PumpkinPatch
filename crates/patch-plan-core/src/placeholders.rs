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
    pub avg_ping_day: Option<f64>,
    /// Ranked `(name, playtime_secs, afk_secs)` for top-N placeholders.
    pub top_playtime: Vec<(String, u64, u64)>,
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
    match inner {
        "plan_server_name" => Some(ctx.server_name.clone()),
        "plan_server_uuid" => Some(ctx.server_uuid.clone()),
        "plan_players_online" => Some(ctx.players_online.to_string()),
        "plan_players_total" => Some(ctx.players_total.to_string()),
        "plan_players_max_peak_alltime" => Some(ctx.peak_online.to_string()),
        "plan_players_max_peak_recent" => Some(ctx.peak_online.to_string()),
        "plan_players_regular" => Some(ctx.players_total.to_string()),
        "plan_player_session_length" => {
            let active = ctx.playtime_secs.saturating_sub(ctx.afk_secs);
            Some(format_duration(active))
        }
        "plan_player_is_operator" => Some("false".into()),
        "plan_player_is_banned" => Some("false".into()),
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
        "plan_tps_day" | "plan_tps_week" | "plan_tps_month" => ctx
            .avg_tps_day
            .map(|t| format!("{:.2}", t)),
        "plan_ping_average_day" => ctx
            .avg_ping_day
            .map(|p| format!("{:.0}", p)),
        _ => None,
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
        let entry = ranked.get(idx)?;
        let secs = match *kind {
            "active" => entry.1.saturating_sub(entry.2),
            "afk" => entry.2,
            _ => entry.1,
        };
        return Some(format_duration(secs));
    }
    ranked.get(idx).map(|(name, _, _)| name.clone())
}

fn ranked_for_window(ctx: &PlanPlaceholderContext, _window_ms: i64) -> Vec<(String, u64, u64)> {
    ctx.top_playtime.clone()
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
