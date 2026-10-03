use patch_analytics_protocol::{
    apply_stat_increment, ActivitySample, AnalyticsEvent, AnalyticsPayload, DeathRecord,
    PatchPlanConfig, PlayerStats, ServerTotals, SignedExport, EXPORT_FILENAME, EXPORT_SCHEMA_VERSION,
    MAX_ACTIVITY_SAMPLES, MAX_EXPORT_PLAYERS, MAX_RECENT_DEATHS, MAX_RECENT_EVENTS,
    STATE_FILENAME,
};
use pumpkin_plugin_api::player::Player;
use pumpkin_plugin_api::uuid;
use pumpkin_plugin_api::Result;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub static RUNTIME: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

pub fn with_runtime<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut RuntimeState) -> R,
{
    let lock = RUNTIME.get()?;
    let mut rt = lock.lock().ok()?;
    Some(f(&mut rt))
}

pub struct RuntimeState {
    pub config: PatchPlanConfig,
    pub players: HashMap<String, PlayerStats>,
    pub total_joins: u64,
    pub total_logins: u64,
    pub total_kicks: u64,
    pub peak_online: u32,
    pub activity_samples: Vec<ActivitySample>,
    pub recent_deaths: Vec<DeathRecord>,
    pub recent_events: Vec<AnalyticsEvent>,
    pub data_folder: String,
}

pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn player_id(player: &Player) -> String {
    uuid::to_string(player.get_id())
}

pub fn touch_player<'a>(
    rt: &'a mut RuntimeState,
    player: &Player,
    now: i64,
) -> &'a mut PlayerStats {
    let id = player_id(player);
    let name = player.get_name();
    rt.players.entry(id.clone()).or_insert_with(|| PlayerStats {
        uuid: id,
        name: name.clone(),
        first_seen_unix: now,
        last_seen_unix: now,
        ..PlayerStats::default()
    });
    let entry = rt.players.get_mut(&player_id(player)).unwrap();
    entry.name = name;
    entry.last_seen_unix = now;
    entry
}

pub fn push_event(rt: &mut RuntimeState, kind: &str, player: &Player, detail: String) {
    let now = unix_now();
    rt.recent_events.push(AnalyticsEvent {
        t: now,
        kind: kind.into(),
        player_uuid: player_id(player),
        player_name: player.get_name(),
        detail,
    });
    if rt.recent_events.len() > MAX_RECENT_EVENTS {
        let extra = rt.recent_events.len() - MAX_RECENT_EVENTS;
        rt.recent_events.drain(0..extra);
    }
}

pub fn push_death(rt: &mut RuntimeState, player: &Player, message: String) {
    let now = unix_now();
    rt.recent_deaths.push(DeathRecord {
        t: now,
        player_uuid: player_id(player),
        player_name: player.get_name(),
        message,
    });
    if rt.recent_deaths.len() > MAX_RECENT_DEATHS {
        let extra = rt.recent_deaths.len() - MAX_RECENT_DEATHS;
        rt.recent_deaths.drain(0..extra);
    }
}

pub fn push_sample(rt: &mut RuntimeState, t: i64, online: u32) {
    rt.activity_samples.push(ActivitySample { t, online });
    if rt.activity_samples.len() > MAX_ACTIVITY_SAMPLES {
        let drain = rt.activity_samples.len() - MAX_ACTIVITY_SAMPLES;
        rt.activity_samples.drain(0..drain);
    }
}

pub fn record_stat(rt: &mut RuntimeState, player: &Player, stat_id: &str, amount: i32) {
    let now = unix_now();
    let entry = touch_player(rt, player, now);
    apply_stat_increment(entry, stat_id, amount);
}

pub fn compute_server_totals(rt: &RuntimeState) -> ServerTotals {
    let mut totals = ServerTotals {
        total_logins: rt.total_logins,
        total_kicks: rt.total_kicks,
        ..ServerTotals::default()
    };
    for p in rt.players.values() {
        totals.total_deaths = totals.total_deaths.saturating_add(p.deaths);
        totals.total_advancements = totals.total_advancements.saturating_add(p.advancements);
        totals.total_mob_kills = totals.total_mob_kills.saturating_add(p.mob_kills);
        totals.total_player_kills = totals.total_player_kills.saturating_add(p.player_kills);
        totals.total_recipes_discovered =
            totals.total_recipes_discovered.saturating_add(p.recipes_discovered);
        totals.total_blocks_harvested =
            totals.total_blocks_harvested.saturating_add(p.blocks_harvested);
        totals.total_chat_messages = totals.total_chat_messages.saturating_add(p.chat_messages);
        totals.total_commands = totals.total_commands.saturating_add(p.commands_used);
        for (k, v) in &p.statistic_totals {
            *totals.statistic_totals.entry(k.clone()).or_insert(0) += v;
        }
    }
    totals
}

pub fn write_export(rt: &RuntimeState, online_now: u32) -> Result<(), String> {
    let now = unix_now();
    let unique = rt.players.len() as u64;
    let total_playtime: u64 = rt.players.values().map(|p| p.playtime_secs).sum();
    let mut players: Vec<PlayerStats> = rt.players.values().cloned().collect();
    players.sort_by(|a, b| b.playtime_secs.cmp(&a.playtime_secs));
    players.truncate(MAX_EXPORT_PLAYERS);

    let payload = AnalyticsPayload {
        schema_version: EXPORT_SCHEMA_VERSION,
        server_id: rt.config.server_id.clone(),
        server_name: rt.config.server_name.clone(),
        network_id: rt.config.network_id.clone(),
        generated_at_unix: now,
        online_now,
        peak_online: rt.peak_online,
        total_joins: rt.total_joins,
        unique_players: unique,
        total_playtime_secs: total_playtime,
        server_totals: compute_server_totals(rt),
        recent_deaths: rt.recent_deaths.clone(),
        recent_events: rt.recent_events.clone(),
        players,
        activity_samples: rt.activity_samples.clone(),
    };
    let signature =
        patch_analytics_protocol::sign_payload(&payload, &rt.config.export_secret)?;
    let signed = SignedExport {
        payload,
        signature,
    };
    let path = format!("{}/{EXPORT_FILENAME}", rt.data_folder);
    let json = serde_json::to_string_pretty(&signed).map_err(|e| e.to_string())?;
    let tmp = format!("{path}.part");
    std::fs::write(&tmp, json).map_err(|e| e.to_string())?;
    std::fs::rename(tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PersistedState {
    pub players: HashMap<String, PlayerStats>,
    pub total_joins: u64,
    pub total_logins: u64,
    pub total_kicks: u64,
    pub peak_online: u32,
    pub activity_samples: Vec<ActivitySample>,
    pub recent_deaths: Vec<DeathRecord>,
    pub recent_events: Vec<AnalyticsEvent>,
}

pub fn persist_state(rt: &RuntimeState) -> Result<(), String> {
    let path = format!("{}/{STATE_FILENAME}", rt.data_folder);
    let body = PersistedState {
        players: rt.players.clone(),
        total_joins: rt.total_joins,
        total_logins: rt.total_logins,
        total_kicks: rt.total_kicks,
        peak_online: rt.peak_online,
        activity_samples: rt.activity_samples.clone(),
        recent_deaths: rt.recent_deaths.clone(),
        recent_events: rt.recent_events.clone(),
    };
    let json = serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_persisted_state(path: &str) -> Result<PersistedState, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

pub fn gamemode_label(mode: pumpkin_plugin_api::wit::pumpkin::plugin::common::GameMode) -> String {
    use pumpkin_plugin_api::wit::pumpkin::plugin::common::GameMode;
    match mode {
        GameMode::Survival => "survival".into(),
        GameMode::Creative => "creative".into(),
        GameMode::Adventure => "adventure".into(),
        GameMode::Spectator => "spectator".into(),
    }
}
