//! Canonical Plan placeholder keys from indepth.md §7.2 (for coverage tests).

pub fn server_placeholder_keys() -> Vec<String> {
    let mut keys: Vec<String> = vec![
        "plan_server_name",
        "plan_server_uuid",
        "plan_players_total",
        "plan_players_new_day",
        "plan_players_new_week",
        "plan_players_new_month",
        "plan_players_unique_day",
        "plan_players_unique_week",
        "plan_players_unique_month",
        "plan_players_online",
        "plan_players_max_peak_alltime",
        "plan_players_max_peak_date_alltime",
        "plan_players_max_peak_recent",
        "plan_players_max_peak_date_recent",
        "plan_players_regular",
        "plan_sessions_playtime_total",
        "plan_sessions_playtime_day",
        "plan_sessions_playtime_week",
        "plan_sessions_playtime_month",
        "plan_sessions_active_time_total",
        "plan_sessions_afk_time_total",
        "plan_sessions_count_total",
        "plan_sessions_count_day",
        "plan_sessions_count_week",
        "plan_sessions_count_month",
        "plan_sessions_average_length_day",
        "plan_sessions_average_length_week",
        "plan_sessions_average_length_month",
        "plan_sessions_average_unique_players_day",
        "plan_sessions_average_unique_players_week",
        "plan_sessions_average_unique_players_month",
        "plan_sessions_player_kills_total",
        "plan_sessions_player_kills_day",
        "plan_sessions_player_kills_week",
        "plan_sessions_player_kills_month",
        "plan_sessions_mob_kills_total",
        "plan_sessions_mob_kills_day",
        "plan_sessions_mob_kills_week",
        "plan_sessions_mob_kills_month",
        "plan_sessions_deaths_total",
        "plan_sessions_deaths_day",
        "plan_sessions_deaths_week",
        "plan_sessions_deaths_month",
        "plan_tps_day",
        "plan_tps_week",
        "plan_tps_month",
        "plan_tps_spikes_day",
        "plan_tps_spikes_week",
        "plan_tps_spikes_month",
        "plan_ping_average_day",
        "plan_ping_average_week",
        "plan_ping_average_month",
        "plan_ping_min_day",
        "plan_ping_max_day",
        "plan_server_time_zone",
        "plan_server_time_zone_offset",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for window in ["total", "day", "week", "month"] {
        for kind in ["active", "afk"] {
            for rank in 1..=10 {
                keys.push(format!("plan_top_player_playtime_{window}_{kind}_{rank}"));
                keys.push(format!("plan_top_amount_playtime_{window}_{kind}_{rank}"));
            }
        }
    }
    for base in [
        "plan_sessions_playtime_total",
        "plan_sessions_playtime_day",
        "plan_sessions_active_time_total",
        "plan_sessions_afk_time_day",
    ] {
        keys.push(format!("{base}_raw"));
    }
    keys
}

pub fn player_placeholder_keys() -> Vec<String> {
    let mut keys: Vec<String> = vec![
        "plan_player_time_total",
        "plan_player_time_day",
        "plan_player_time_week",
        "plan_player_time_month",
        "plan_player_time_active_total",
        "plan_player_time_active_day",
        "plan_player_time_active_week",
        "plan_player_time_active_month",
        "plan_player_time_afk_total",
        "plan_player_time_afk_day",
        "plan_player_time_afk_week",
        "plan_player_time_afk_month",
        "plan_player_session_length",
        "plan_player_session_length_average",
        "plan_player_session_length_Longest",
        "plan_player_sessions_count",
        "plan_player_registered",
        "plan_player_lastseen",
        "plan_player_is_banned",
        "plan_player_is_operator",
        "plan_player_kicked_count",
        "plan_player_activity_index",
        "plan_player_activity_group",
        "plan_player_geolocation",
        "plan_player_ping_average",
        "plan_player_ping_max",
        "plan_player_ping_min",
        "plan_player_favorite_server",
        "plan_player_player_kills",
        "plan_player_mob_kills",
        "plan_player_deaths",
        "plan_player_kdr",
        "plan_player_mob_kdr",
        "plan_player_player_caused_deaths",
        "plan_player_name",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for n in 1..=10 {
        keys.push(format!("plan_player_recent_kill_{n}"));
        keys.push(format!("plan_player_recent_kill_weapon_{n}"));
    }
    for base in [
        "plan_player_time_day",
        "plan_player_time_active_week",
        "plan_player_time_afk_month",
    ] {
        keys.push(format!("{base}_raw"));
    }
    keys
}

pub fn wrap_percent(key: &str) -> String {
    format!("%{key}%")
}
