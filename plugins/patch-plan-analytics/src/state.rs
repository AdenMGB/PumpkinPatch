use patch_analytics_protocol::{
    apply_stat_increment, ActivitySample, AnalyticsEvent, AnalyticsPayload, DeathRecord,
    PatchPlanConfig, PerformanceSample, PingWindowRecord, PlayerStats, PvpKillRecord,
    ServerTotals, SessionRecord, SignedExport, WorldTimeSlice, EXPORT_FILENAME,
    CONFIG_FILENAME, EXPORT_SCHEMA_VERSION, MAX_ACTIVITY_SAMPLES, MAX_COMPLETED_SESSIONS_PER_PLAYER,
    MAX_EXPORT_PLAYERS, MAX_PERFORMANCE_SAMPLES, MAX_PING_WINDOWS_PER_PLAYER,
    MAX_RECENT_DEATHS, MAX_RECENT_EVENTS, MAX_RECENT_PVP_KILLS, MAX_SERVER_COMMAND_USAGE_KEYS,
    STATE_FILENAME,
};
use pumpkin_plugin_api::Server;
use patch_plan_core::{AfkTracker, ActiveSession, GameMode, WorldTimes};
use pumpkin_plugin_api::player::Player;
use pumpkin_plugin_api::uuid;
use pumpkin_plugin_api::Result;
use std::collections::HashMap;
use patch_plan_core::normalize_join_address;
use patch_plan_core::JoinAddressOptions;
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

/// Runs `f` only when Plan analytics collection is enabled (`/plan disable` turns this off).
pub fn with_analytics<F>(f: F) -> Option<()>
where
    F: FnOnce(&mut RuntimeState),
{
    with_runtime(|rt| {
        if !rt.analytics_enabled {
            return;
        }
        f(rt);
    })
}

pub fn reload_config_from_disk(rt: &mut RuntimeState) -> Result<(), String> {
    let config_path = format!("{}/{CONFIG_FILENAME}", rt.data_folder);
    let raw = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
    rt.config = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    rt.analytics_enabled = true;
    rt.kick_counting_enabled = true;
    Ok(())
}

pub struct RuntimeState {
    pub config: PatchPlanConfig,
    pub players: HashMap<String, PlayerStats>,
    pub active_sessions: HashMap<String, ActiveSession>,
    pub afk_tracker: AfkTracker,
    pub command_usages: HashMap<String, u64>,
    pub total_joins: u64,
    pub total_logins: u64,
    pub total_kicks: u64,
    pub peak_online: u32,
    pub activity_samples: Vec<ActivitySample>,
    pub recent_deaths: Vec<DeathRecord>,
    pub recent_events: Vec<AnalyticsEvent>,
    pub recent_pvp_kills: Vec<PvpKillRecord>,
    pub performance_samples: Vec<PerformanceSample>,
    pub last_perf_instant: Option<std::time::Instant>,
    pub data_folder: String,
    pub pending_geolocation: HashMap<String, String>,
    pub online_ping: HashMap<String, OnlinePingTrack>,
    pub server_enabled_at: i64,
    pub registration_completions: Vec<patch_analytics_protocol::RegistrationCompletion>,
    pub web_admin_actions: Vec<patch_analytics_protocol::WebAdminAction>,
    pub desktop_plan_requests: Vec<patch_analytics_protocol::DesktopPlanRequest>,
    pub pending_db_op: Option<PendingDbOp>,
    /// When false, telemetry handlers no-op (Plan `/plan disable`).
    pub analytics_enabled: bool,
    /// When false, kick events are not counted (`/plan disable <kickcount>` prelude).
    pub kick_counting_enabled: bool,
    pub vineyard_sink: Option<pumpkin_hook_sdk::TelemetrySink>,
}

#[derive(Debug, Clone)]
pub enum PendingDbOp {
    ClearNetwork,
    RemovePlayer { target: String },
}

#[derive(Debug, Clone)]
pub struct OnlinePingTrack {
    pub joined_at: i64,
    pub samples: Vec<u32>,
}

const PING_JOIN_DELAY_SECS: i64 = 30;
const PING_SERVER_ENABLE_DELAY_SECS: i64 = 300;
const PING_SAMPLES_PER_WINDOW: usize = 30;

/// Samples TPS from wall-clock between 100-tick plugin callbacks.
pub fn sample_performance(rt: &mut RuntimeState, online: u32, tick_period: f64) {
    if !rt.analytics_enabled {
        return;
    }
    let now = std::time::Instant::now();
    if let Some(prev) = rt.last_perf_instant {
        let elapsed = now.duration_since(prev).as_secs_f64();
        if elapsed >= 0.25 {
            let tps = (tick_period / elapsed).clamp(0.0, 20.0);
            let mspt = if tps > 0.1 {
                1000.0 / tps
            } else {
                1000.0
            };
            rt.performance_samples.push(PerformanceSample {
                t: unix_now(),
                ticks_per_second: tps,
                mspt_avg: mspt,
                players_online: online,
            });
            if rt.performance_samples.len() > MAX_PERFORMANCE_SAMPLES {
                let drain = rt.performance_samples.len() - MAX_PERFORMANCE_SAMPLES;
                rt.performance_samples.drain(0..drain);
            }
            emit_vineyard_telemetry(rt, online, mspt);
        }
    }
    rt.last_perf_instant = Some(now);
}

fn instance_root_from_data_folder(data_folder: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(data_folder);
    path.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
}

fn server_id_bytes(server_id: &str) -> [u8; 16] {
    uuid::Uuid::parse_str(server_id)
        .map(|u| *u.as_bytes())
        .unwrap_or([0u8; 16])
}

fn emit_vineyard_telemetry(rt: &mut RuntimeState, online: u32, mspt: f64) {
    let root = instance_root_from_data_folder(&rt.data_folder);
    let root = match root {
        Some(r) => r,
        None => return,
    };
    if rt.vineyard_sink.is_none() {
        let id = server_id_bytes(&rt.config.server_id);
        rt.vineyard_sink = Some(pumpkin_hook_sdk::TelemetrySink::open(&id, &root));
    }
    if let Some(sink) = &mut rt.vineyard_sink {
        let _ = sink.emit_host_sample(
            online.min(u16::MAX as u32) as u16,
            (mspt * 1000.0).clamp(0.0, 1_000_000.0) as u32,
        );
    }
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

pub fn note_prelogin_geolocation(rt: &mut RuntimeState, player_uuid: &str, ip_address: &str) {
    let geo = patch_plan_core::geolocation_from_ip(ip_address);
    if !geo.is_empty() {
        rt.pending_geolocation
            .insert(player_uuid.to_string(), geo);
    }
}

pub fn apply_player_identity(rt: &mut RuntimeState, player: &Player, now: i64) {
    let id = player_id(player);
    let geo = rt.pending_geolocation.remove(&id);
    let entry = touch_player(rt, player, now);
    if let Some(geo) = geo {
        entry.geolocation = geo;
    }
    let nick = player.get_display_name().get_text();
    if !nick.is_empty() && nick != entry.name {
        entry.nick = nick.chars().take(36).collect();
    }
}

pub fn register_online_ping(rt: &mut RuntimeState, player_uuid: &str, now: i64) {
    rt.online_ping.insert(
        player_uuid.to_string(),
        OnlinePingTrack {
            joined_at: now,
            samples: Vec::new(),
        },
    );
}

pub fn unregister_online_ping(rt: &mut RuntimeState, player_uuid: &str) {
    if let Some(track) = rt.online_ping.remove(player_uuid) {
        flush_ping_track(rt, player_uuid, &track, unix_now());
    }
}

fn flush_ping_track(rt: &mut RuntimeState, player_uuid: &str, track: &OnlinePingTrack, now: i64) {
    if track.samples.is_empty() {
        return;
    }
    let min = *track.samples.iter().min().unwrap_or(&0);
    let max = *track.samples.iter().max().unwrap_or(&0);
    let avg = (track.samples.iter().map(|v| *v as u64).sum::<u64>()
        / track.samples.len().max(1) as u64) as u32;
    let window = PingWindowRecord {
        t_unix: now,
        avg_ping: avg,
        min_ping: min,
        max_ping: max,
    };
    if let Some(entry) = rt.players.get_mut(player_uuid) {
        entry.ping_windows.push(window);
        if entry.ping_windows.len() > MAX_PING_WINDOWS_PER_PLAYER {
            let extra = entry.ping_windows.len() - MAX_PING_WINDOWS_PER_PLAYER;
            entry.ping_windows.drain(0..extra);
        }
    }
}

pub fn sample_online_pings(rt: &mut RuntimeState, server: &Server) {
    if !rt.analytics_enabled {
        return;
    }
    let now = unix_now();
    if now - rt.server_enabled_at < PING_SERVER_ENABLE_DELAY_SECS {
        return;
    }
    let ids: Vec<String> = rt.online_ping.keys().cloned().collect();
    for id in ids {
        let Some(track) = rt.online_ping.get(&id) else {
            continue;
        };
        if now - track.joined_at < PING_JOIN_DELAY_SECS {
            continue;
        }
        let parsed = uuid::parse(&id);
        if parsed.is_none() {
            continue;
        }
        let player = server.get_player_by_uuid(parsed.unwrap());
        if player.is_none() {
            continue;
        }
        let player = player.unwrap();
        let ping = player.get_ping();
        let mut flush = false;
        if let Some(track) = rt.online_ping.get_mut(&id) {
            if ping > 0 {
                track.samples.push(ping);
            }
            if track.samples.len() >= PING_SAMPLES_PER_WINDOW {
                flush = true;
            }
        }
        if flush {
            let track = rt.online_ping.remove(&id).unwrap();
            flush_ping_track(rt, &id, &track, now);
            rt.online_ping.insert(
                id,
                OnlinePingTrack {
                    joined_at: track.joined_at,
                    samples: Vec::new(),
                },
            );
        }
    }
}

pub fn resolve_join_address(rt: &RuntimeState, _player: &Player) -> String {
    rt.config.default_join_address.clone()
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
    let id = player_id(player);
    let before_pk = rt.players.get(&id).map(|p| p.player_kills).unwrap_or(0);
    let before_mk = rt.players.get(&id).map(|p| p.mob_kills).unwrap_or(0);
    {
        let entry = touch_player(rt, player, now);
        apply_stat_increment(entry, stat_id, amount);
    }
    session_activity(rt, player, now);

    let after_pk = rt.players.get(&id).map(|p| p.player_kills).unwrap_or(0);
    let after_mk = rt.players.get(&id).map(|p| p.mob_kills).unwrap_or(0);
    let player_name = player.get_name();
    let weapon = stat_weapon_label(stat_id);

    if after_mk > before_mk {
        let delta = (after_mk - before_mk) as u32;
        if let Some(session) = rt.active_sessions.get_mut(&id) {
            session.mob_kills = session.mob_kills.saturating_add(delta);
        }
    }
    if after_pk > before_pk {
        let delta = after_pk - before_pk;
        for _ in 0..delta {
            push_pvp_kill(rt, &id, &player_name, "", "unknown", &weapon, now);
        }
    }
}

fn stat_weapon_label(stat_id: &str) -> String {
    stat_id
        .split(':')
        .last()
        .unwrap_or("melee")
        .replace('_', " ")
}

/// Parses vanilla-style death messages for PvP attribution (English patterns).
pub fn parse_pvp_killer_from_death_message(message: &str) -> Option<(String, String)> {
    let plain = message.to_ascii_lowercase();
    let markers = [" was slain by ", " was shot by ", " was killed by ", " was pummeled by "];
    for marker in markers {
        if let Some(idx) = plain.find(marker) {
            let killer_rest = message[idx + marker.len()..].trim();
            let killer_name = killer_rest
                .split(" using ")
                .next()
                .unwrap_or(killer_rest)
                .trim()
                .to_string();
            let weapon = if let Some(widx) = plain.find(" using ") {
                message[widx + 7..].trim().to_string()
            } else {
                "melee".into()
            };
            if !killer_name.is_empty() {
                return Some((killer_name, weapon));
            }
        }
    }
    None
}

pub fn push_pvp_kill(
    rt: &mut RuntimeState,
    killer_uuid: &str,
    killer_name: &str,
    victim_uuid: &str,
    victim_name: &str,
    weapon: &str,
    now: i64,
) {
    if killer_uuid.is_empty() && killer_name.is_empty() {
        return;
    }
    if victim_uuid.is_empty() && victim_name.is_empty() {
        return;
    }
    rt.recent_pvp_kills.push(PvpKillRecord {
        t: now,
        killer_uuid: killer_uuid.to_string(),
        killer_name: killer_name.to_string(),
        victim_uuid: victim_uuid.to_string(),
        victim_name: victim_name.to_string(),
        weapon: weapon.to_string(),
    });
    if rt.recent_pvp_kills.len() > MAX_RECENT_PVP_KILLS {
        let extra = rt.recent_pvp_kills.len() - MAX_RECENT_PVP_KILLS;
        rt.recent_pvp_kills.drain(0..extra);
    }
}

pub fn player_afk_exempt(rt: &RuntimeState, player_uuid: &str) -> bool {
    rt.config
        .afk_exempt_uuids
        .iter()
        .any(|u| u.eq_ignore_ascii_case(player_uuid))
}

pub fn player_command_usage_exempt(rt: &RuntimeState, player_uuid: &str) -> bool {
    rt.config
        .command_usage_exempt_uuids
        .iter()
        .any(|u| u.eq_ignore_ascii_case(player_uuid))
}

pub fn session_activity(rt: &mut RuntimeState, player: &Player, now_ms: i64) {
    let id = player_id(player);
    let ignore_afk = player_afk_exempt(rt, &id);
    if let Some(session) = rt.active_sessions.get_mut(&id) {
        rt.afk_tracker.on_activity(&id, now_ms * 1000, ignore_afk, session);
    }
}

pub fn begin_session(rt: &mut RuntimeState, player: &Player, join_address: String, now: i64) {
    let id = player_id(player);
    let entry = touch_player(rt, player, now);
    let normalized = normalize_join_address(&join_address, &JoinAddressOptions::default());
    if !normalized.is_empty() {
        entry.join_address = normalized.clone();
    }
    let gm = GameMode::from_label(&entry.current_gamemode);
    let world = if entry.last_world.is_empty() {
        "world".into()
    } else {
        entry.last_world.clone()
    };
    let session = ActiveSession {
        user_uuid: id.clone(),
        session_start_ms: now * 1000,
        afk_time_ms: 0,
        mob_kills: 0,
        deaths: 0,
        join_address: normalized,
        world_times: WorldTimes::begin(world, gm, now * 1000),
    };
    rt.active_sessions.insert(id.clone(), session);
    if let Some(session) = rt.active_sessions.get_mut(&id) {
        rt.afk_tracker.on_activity(&id, now * 1000, false, session);
    }
}

pub fn end_session(rt: &mut RuntimeState, player: &Player, now: i64) {
    let id = player_id(player);
    if let Some(mut session) = rt.active_sessions.remove(&id) {
        rt.afk_tracker.on_quit(&id, now * 1000, &mut session);
        session.world_times.finalize(now * 1000);
        let afk_secs = (session.afk_time_ms / 1000) as u64;
        let world_times = session
            .world_times
            .export_slices()
            .into_iter()
            .map(|(world, s, c, a, sp)| WorldTimeSlice {
                world,
                survival_ms: s,
                creative_ms: c,
                adventure_ms: a,
                spectator_ms: sp,
            })
            .collect();
        let record = SessionRecord {
            start_unix: session.session_start_ms / 1000,
            end_unix: now,
            afk_secs,
            mob_kills: session.mob_kills,
            deaths: session.deaths,
            join_address: session.join_address.clone(),
            world_times,
        };
        if let Some(entry) = rt.players.get_mut(&id) {
            entry.afk_time_secs = entry.afk_time_secs.saturating_add(afk_secs);
            entry.completed_sessions.push(record);
            if entry.completed_sessions.len() > MAX_COMPLETED_SESSIONS_PER_PLAYER {
                let extra = entry.completed_sessions.len() - MAX_COMPLETED_SESSIONS_PER_PLAYER;
                entry.completed_sessions.drain(0..extra);
            }
        }
    }
}

pub fn session_world_change(rt: &mut RuntimeState, player: &Player, world: String, now: i64) {
    let id = player_id(player);
    session_activity(rt, player, now);
    if let Some(session) = rt.active_sessions.get_mut(&id) {
        session.world_times.change_world(world, now * 1000);
    }
}

pub fn session_gamemode_change(rt: &mut RuntimeState, player: &Player, mode: GameMode, now: i64) {
    let id = player_id(player);
    session_activity(rt, player, now);
    if let Some(session) = rt.active_sessions.get_mut(&id) {
        session.world_times.change_gamemode(mode, now * 1000);
    }
}

pub fn record_command_usage(rt: &mut RuntimeState, command_line: &str) {
    let root = command_root_label(command_line);
    if root.is_empty() {
        return;
    }
    *rt.command_usages.entry(root).or_insert(0) += 1;
    if rt.command_usages.len() > MAX_SERVER_COMMAND_USAGE_KEYS {
        let key = rt
            .command_usages
            .iter()
            .min_by_key(|(_, v)| *v)
            .map(|(k, _)| k.clone());
        if let Some(k) = key {
            rt.command_usages.remove(&k);
        }
    }
}

fn command_root_label(command_line: &str) -> String {
    let trimmed = command_line.trim();
    let without_slash = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let root = without_slash
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    root.chars().take(20).collect()
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
        command_usages: rt.command_usages.clone(),
        recent_pvp_kills: rt.recent_pvp_kills.clone(),
        performance_samples: rt.performance_samples.clone(),
        registration_completions: rt.registration_completions.clone(),
        web_admin_actions: rt.web_admin_actions.clone(),
        desktop_plan_requests: rt.desktop_plan_requests.clone(),
        is_proxy: rt.config.is_proxy,
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

pub fn queue_registration_completion(
    rt: &mut RuntimeState,
    code: &str,
    player_uuid: &str,
    player_name: &str,
    grant_admin: bool,
) {
    rt.registration_completions.push(patch_analytics_protocol::RegistrationCompletion {
        code: code.to_string(),
        player_uuid: player_uuid.to_string(),
        player_name: player_name.to_string(),
        grant_admin,
    });
    if rt.registration_completions.len() > 32 {
        let drain = rt.registration_completions.len() - 32;
        rt.registration_completions.drain(0..drain);
    }
}

pub fn drain_registration_completions(rt: &mut RuntimeState) {
    rt.registration_completions.clear();
}

pub fn queue_web_admin_action(rt: &mut RuntimeState, action: patch_analytics_protocol::WebAdminAction) {
    rt.web_admin_actions.push(action);
    if rt.web_admin_actions.len() > 32 {
        let drain = rt.web_admin_actions.len() - 32;
        rt.web_admin_actions.drain(0..drain);
    }
}

pub fn drain_web_admin_actions(rt: &mut RuntimeState) {
    rt.web_admin_actions.clear();
}

pub fn queue_desktop_plan_request(
    rt: &mut RuntimeState,
    action: patch_analytics_protocol::DesktopPlanRequest,
) {
    rt.desktop_plan_requests.push(action);
    if rt.desktop_plan_requests.len() > 16 {
        let drain = rt.desktop_plan_requests.len() - 16;
        rt.desktop_plan_requests.drain(0..drain);
    }
}

pub fn drain_desktop_plan_requests(rt: &mut RuntimeState) {
    rt.desktop_plan_requests.clear();
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct PersistedState {
    pub players: HashMap<String, PlayerStats>,
    #[serde(default)]
    pub command_usages: HashMap<String, u64>,
    pub total_joins: u64,
    pub total_logins: u64,
    pub total_kicks: u64,
    pub peak_online: u32,
    pub activity_samples: Vec<ActivitySample>,
    pub recent_deaths: Vec<DeathRecord>,
    pub recent_events: Vec<AnalyticsEvent>,
    #[serde(default)]
    pub recent_pvp_kills: Vec<PvpKillRecord>,
    #[serde(default)]
    pub performance_samples: Vec<PerformanceSample>,
}

pub fn persist_state(rt: &RuntimeState) -> Result<(), String> {
    let path = format!("{}/{STATE_FILENAME}", rt.data_folder);
    let body = PersistedState {
        players: rt.players.clone(),
        command_usages: rt.command_usages.clone(),
        total_joins: rt.total_joins,
        total_logins: rt.total_logins,
        total_kicks: rt.total_kicks,
        peak_online: rt.peak_online,
        activity_samples: rt.activity_samples.clone(),
        recent_deaths: rt.recent_deaths.clone(),
        recent_events: rt.recent_events.clone(),
        recent_pvp_kills: rt.recent_pvp_kills.clone(),
        performance_samples: rt.performance_samples.clone(),
    };
    let json = serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_persisted_state(path: &str) -> Result<PersistedState, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

fn count_players_with_activity_group(
    rt: &RuntimeState,
    group: patch_plan_core::ActivityGroup,
) -> u64 {
    let now_ms = unix_now() * 1000;
    rt.players
        .values()
        .filter(|p| {
            let triples: Vec<(i64, i64, u64)> = p
                .completed_sessions
                .iter()
                .map(|s| (s.start_unix * 1000, s.end_unix * 1000, s.afk_secs * 1000))
                .collect();
            let (w1, w2, w3) =
                patch_plan_core::active_playtime_in_weeks(&triples, now_ms);
            let ai = patch_plan_core::ActivityIndex::calculate(
                w1,
                w2,
                w3,
                patch_plan_core::DEFAULT_PLAYTIME_THRESHOLD_MS,
            );
            ai.group == group
        })
        .count() as u64
}

pub fn placeholder_context_for(
    rt: &RuntimeState,
    target_uuid: &str,
    online: u32,
) -> patch_plan_core::PlanPlaceholderContext {
    let now_ms = unix_now() * 1000;
    let mut top: Vec<(String, u64, u64)> = rt
        .players
        .values()
        .map(|p| (p.name.clone(), p.playtime_secs, p.afk_time_secs))
        .collect();
    top.sort_by_key(|(_, play, _)| std::cmp::Reverse(*play));
    top.truncate(10);

    let avg_tps_day = {
        let cutoff = unix_now() - 86_400;
        let samples: Vec<f64> = rt
            .performance_samples
            .iter()
            .filter(|s| s.t >= cutoff)
            .map(|s| s.ticks_per_second)
            .collect();
        if samples.is_empty() {
            None
        } else {
            Some(samples.iter().sum::<f64>() / samples.len() as f64)
        }
    };

    let now = unix_now();
    let count_first_seen_since = |cutoff: i64| {
        rt.players
            .values()
            .filter(|p| p.first_seen_unix >= cutoff)
            .count() as u64
    };
    let count_active_since = |cutoff: i64| {
        rt.players
            .values()
            .filter(|p| p.last_seen_unix >= cutoff)
            .count() as u64
    };
    let count_sessions_since = |cutoff: i64| {
        rt.players
            .values()
            .flat_map(|p| p.completed_sessions.iter())
            .filter(|s| s.start_unix >= cutoff)
            .count() as u64
    };
    let avg_session_len_since = |cutoff: i64| -> u64 {
        let sessions: Vec<u64> = rt
            .players
            .values()
            .flat_map(|p| p.completed_sessions.iter())
            .filter(|s| s.end_unix >= cutoff)
            .map(|s| (s.end_unix - s.start_unix).max(0) as u64)
            .collect();
        if sessions.is_empty() {
            0
        } else {
            sessions.iter().sum::<u64>() / sessions.len() as u64
        }
    };
    let unique_players_since = |cutoff: i64| -> u64 {
        rt.players
            .values()
            .filter(|p| p.last_seen_unix >= cutoff)
            .count() as u64
    };
    let playtime_in_window = |cutoff: i64| -> u64 {
        rt.players
            .values()
            .flat_map(|p| p.completed_sessions.iter())
            .filter(|s| s.end_unix >= cutoff)
            .map(|s| (s.end_unix - s.start_unix).max(0) as u64)
            .sum()
    };
    let afk_in_window = |cutoff: i64| -> u64 {
        rt.players
            .values()
            .flat_map(|p| p.completed_sessions.iter())
            .filter(|s| s.end_unix >= cutoff)
            .map(|s| s.afk_secs)
            .sum()
    };

    let peak_sample = rt
        .activity_samples
        .iter()
        .max_by_key(|s| s.online);
    let peak_date_recent = peak_sample
        .map(|s| s.t.to_string())
        .unwrap_or_else(|| now.to_string());
    let peak_date_alltime = peak_date_recent.clone();

    let tps_spikes_since = |cutoff: i64| -> u32 {
        rt.performance_samples
            .iter()
            .filter(|s| s.t >= cutoff && s.ticks_per_second < 15.0)
            .count() as u32
    };
    let tps_spikes_day = tps_spikes_since(now - 86_400);
    let tps_spikes_week = tps_spikes_since(now - 7 * 86_400);
    let tps_spikes_month = tps_spikes_since(now - 30 * 86_400);

    let (sessions_playtime_secs, sessions_afk_secs) = rt.players.values().fold((0u64, 0u64), |acc, p| {
        (
            acc.0 + p.playtime_secs,
            acc.1 + p.afk_time_secs,
        )
    });
    let sessions_active_secs = sessions_playtime_secs.saturating_sub(sessions_afk_secs);

    let ping_stats_for_cutoff = |cutoff: i64| -> (Option<f64>, Option<u32>, Option<u32>) {
        let mut sum = 0f64;
        let mut n = 0u64;
        let mut min = None;
        let mut max = None;
        for p in rt.players.values() {
            for w in &p.ping_windows {
                if w.t_unix < cutoff {
                    continue;
                }
                sum += w.avg_ping as f64;
                n += 1;
                min = Some(min.map_or(w.min_ping, |m: u32| m.min(w.min_ping)));
                max = Some(max.map_or(w.max_ping, |m: u32| m.max(w.max_ping)));
            }
        }
        let avg = if n > 0 { Some(sum / n as f64) } else { None };
        (avg, min, max)
    };
    let (avg_ping_day, min_ping_day, max_ping_day) = ping_stats_for_cutoff(now - 86_400);
    let (avg_ping_week, _, _) = ping_stats_for_cutoff(now - 7 * 86_400);
    let (avg_ping_month, _, _) = ping_stats_for_cutoff(now - 30 * 86_400);

    let entry = rt.players.get(target_uuid);
    let session_triples = entry
        .map(|e| {
            e.completed_sessions
                .iter()
                .map(|s| {
                    (
                        s.start_unix * 1000,
                        s.end_unix * 1000,
                        s.afk_secs * 1000,
                    )
                })
                .collect()
        })
        .unwrap_or_default();

    let combat_in_window = |cutoff: i64| -> (u64, u64, u64) {
        let mut deaths = 0u64;
        let mut mob = 0u64;
        let mut pvp = 0u64;
        for p in rt.players.values() {
            for s in &p.completed_sessions {
                if s.end_unix >= cutoff {
                    deaths += s.deaths as u64;
                    mob += s.mob_kills as u64;
                }
            }
            if p.last_seen_unix >= cutoff {
                pvp += p.player_kills;
            }
        }
        (deaths, mob, pvp)
    };
    let (deaths_day, mob_day, pvp_day) = combat_in_window(now - 86_400);
    let (deaths_week, mob_week, pvp_week) = combat_in_window(now - 7 * 86_400);
    let (deaths_month, mob_month, pvp_month) = combat_in_window(now - 30 * 86_400);
    let deaths_total = rt.players.values().map(|p| p.deaths).sum();
    let mob_kills_total = rt.players.values().map(|p| p.mob_kills).sum();
    let player_kills_total = rt.players.values().map(|p| p.player_kills).sum();

    let player_recent_kills: Vec<(String, String)> = rt
        .recent_pvp_kills
        .iter()
        .filter(|k| k.killer_uuid == target_uuid)
        .take(10)
        .map(|k| (k.victim_name.clone(), k.weapon.clone()))
        .collect();

    let player_session_length_longest_secs = entry
        .map(|e| {
            e.completed_sessions
                .iter()
                .map(|s| (s.end_unix - s.start_unix).max(0) as u64)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);

    let (player_ping_avg, player_ping_min, player_ping_max) = entry
        .map(|e| {
            let mut sum = 0f64;
            let mut n = 0u64;
            let mut min = None;
            let mut max = None;
            for w in &e.ping_windows {
                if w.t_unix < now - 86_400 {
                    continue;
                }
                sum += w.avg_ping as f64;
                n += 1;
                min = Some(min.map_or(w.min_ping, |m| m.min(w.min_ping)));
                max = Some(max.map_or(w.max_ping, |m| m.max(w.max_ping)));
            }
            let avg = if n > 0 { Some(sum / n as f64) } else { None };
            (avg, min, max)
        })
        .unwrap_or((None, None, None));

    patch_plan_core::PlanPlaceholderContext {
        server_name: rt.config.server_name.clone(),
        server_uuid: rt.config.server_id.clone(),
        players_online: online,
        players_total: rt.players.len() as u64,
        peak_online: rt.peak_online,
        total_joins: rt.total_joins,
        player_name: entry.map(|e| e.name.clone()),
        player_uuid: Some(target_uuid.to_string()),
        playtime_secs: entry.map(|e| e.playtime_secs).unwrap_or(0),
        afk_secs: entry.map(|e| e.afk_time_secs).unwrap_or(0),
        deaths: entry.map(|e| e.deaths as u32).unwrap_or(0),
        player_kills: entry.map(|e| e.player_kills as u32).unwrap_or(0),
        mob_kills: entry.map(|e| e.mob_kills as u32).unwrap_or(0),
        kick_count: entry.map(|e| e.kick_count as u32).unwrap_or(0),
        geolocation: entry
            .map(|e| e.geolocation.clone())
            .unwrap_or_default(),
        registered_unix: entry.map(|e| e.first_seen_unix).unwrap_or(0),
        last_seen_unix: entry.map(|e| e.last_seen_unix).unwrap_or(0),
        session_triples,
        now_ms,
        avg_tps_day,
        avg_ping_day,
        avg_ping_week,
        avg_ping_month,
        min_ping_day,
        max_ping_day,
        player_ping_avg,
        player_ping_min,
        player_ping_max,
        players_new_day: count_first_seen_since(now - 86_400),
        players_new_week: count_first_seen_since(now - 7 * 86_400),
        players_new_month: count_first_seen_since(now - 30 * 86_400),
        players_unique_day: count_active_since(now - 86_400),
        players_unique_week: count_active_since(now - 7 * 86_400),
        players_unique_month: count_active_since(now - 30 * 86_400),
        sessions_playtime_secs,
        sessions_active_secs,
        sessions_afk_secs,
        sessions_count_day: count_sessions_since(now - 86_400),
        sessions_count_week: count_sessions_since(now - 7 * 86_400),
        sessions_count_month: count_sessions_since(now - 30 * 86_400),
        timezone_offset: "UTC".into(),
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
        sessions_playtime_day: playtime_in_window(now - 86_400),
        sessions_playtime_week: playtime_in_window(now - 7 * 86_400),
        sessions_playtime_month: playtime_in_window(now - 30 * 86_400),
        sessions_active_day: playtime_in_window(now - 86_400)
            .saturating_sub(afk_in_window(now - 86_400)),
        sessions_active_week: playtime_in_window(now - 7 * 86_400)
            .saturating_sub(afk_in_window(now - 7 * 86_400)),
        sessions_active_month: playtime_in_window(now - 30 * 86_400)
            .saturating_sub(afk_in_window(now - 30 * 86_400)),
        sessions_afk_day: afk_in_window(now - 86_400),
        sessions_afk_week: afk_in_window(now - 7 * 86_400),
        sessions_afk_month: afk_in_window(now - 30 * 86_400),
        peak_online_date_alltime: peak_date_alltime,
        peak_online_date_recent: peak_date_recent,
        favorite_server: rt.config.server_name.clone(),
        tps_spikes_day,
        tps_spikes_week,
        tps_spikes_month,
        sessions_avg_length_day: avg_session_len_since(now - 86_400),
        sessions_avg_length_week: avg_session_len_since(now - 7 * 86_400),
        sessions_avg_length_month: avg_session_len_since(now - 30 * 86_400),
        sessions_avg_unique_players_day: unique_players_since(now - 86_400),
        sessions_avg_unique_players_week: unique_players_since(now - 7 * 86_400),
        sessions_avg_unique_players_month: unique_players_since(now - 30 * 86_400),
        player_is_banned: entry.map(|e| e.banned).unwrap_or(false),
        player_is_operator: entry.map(|e| e.operator).unwrap_or(false),
        player_session_length_longest_secs,
        players_regular: count_players_with_activity_group(
            rt,
            patch_plan_core::ActivityGroup::Regular,
        ),
        top_playtime: top,
    }
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
