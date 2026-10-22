//! Plan PlaceholderAPI subset (indepth.md §7.2).

use crate::{active_playtime_in_weeks, ActivityIndex, DEFAULT_PLAYTIME_THRESHOLD_MS};

#[derive(Debug, Clone, Default)]
pub struct PlanPlaceholderContext {
    pub server_name: String,
    pub server_uuid: String,
    pub players_online: u32,
    pub player_name: Option<String>,
    pub player_uuid: Option<String>,
    pub playtime_secs: u64,
    pub afk_secs: u64,
    pub deaths: u32,
    pub player_kills: u32,
    pub mob_kills: u32,
    pub geolocation: String,
    pub session_triples: Vec<(i64, i64, u64)>,
    pub now_ms: i64,
}

pub fn resolve_placeholder(key: &str, ctx: &PlanPlaceholderContext) -> Option<String> {
    let key = key.trim();
    if !key.starts_with("%plan_") || !key.ends_with('%') {
        return None;
    }
    let inner = key.trim_matches('%');
    match inner {
        "plan_server_name" => Some(ctx.server_name.clone()),
        "plan_server_uuid" => Some(ctx.server_uuid.clone()),
        "plan_players_online" => Some(ctx.players_online.to_string()),
        "plan_player_time_total" => Some(format_duration(ctx.playtime_secs)),
        "plan_player_time_afk_total" => Some(format_duration(ctx.afk_secs)),
        "plan_player_deaths" => Some(ctx.deaths.to_string()),
        "plan_player_player_kills" => Some(ctx.player_kills.to_string()),
        "plan_player_mob_kills" => Some(ctx.mob_kills.to_string()),
        "plan_player_geolocation" => Some(ctx.geolocation.clone()),
        "plan_player_kdr" => {
            let k = ctx.player_kills as f64;
            let d = ctx.deaths.max(1) as f64;
            Some(format!("{:.2}", k / d))
        }
        "plan_player_activity_index" => {
            let (w1, w2, w3) = active_playtime_in_weeks(&ctx.session_triples, ctx.now_ms);
            let ai = ActivityIndex::calculate(w1, w2, w3, DEFAULT_PLAYTIME_THRESHOLD_MS);
            Some(format!("{:.2}", ai.value))
        }
        "plan_player_activity_group" => {
            let (w1, w2, w3) = active_playtime_in_weeks(&ctx.session_triples, ctx.now_ms);
            let ai = ActivityIndex::calculate(w1, w2, w3, DEFAULT_PLAYTIME_THRESHOLD_MS);
            Some(ai.group.label().to_string())
        }
        _ => None,
    }
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
