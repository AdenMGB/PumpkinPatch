//! Patch Plan — lightweight Plan-style analytics for Pumpkin servers (Pumpkin Patch).

use patch_analytics_protocol::{
    sign_payload, ActivitySample, AnalyticsPayload, PatchPlanConfig, PlayerStats,
    SignedExport, CONFIG_FILENAME, EXPORT_FILENAME, EXPORT_SCHEMA_VERSION, PLUGIN_DATA_DIR_NAME,
    STATE_FILENAME,
};
use pumpkin_plugin_api::events::{EventHandler, EventPriority, PlayerJoinEvent, PlayerLeaveEvent};
use pumpkin_plugin_api::permissions::{FS_READ_DATA, FS_WRITE_DATA};
use pumpkin_plugin_api::scheduler::SchedulerExt;
use pumpkin_plugin_api::uuid;
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Result, Server};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

struct RuntimeState {
    config: PatchPlanConfig,
    players: HashMap<String, PlayerStats>,
    total_joins: u64,
    peak_online: u32,
    activity_samples: Vec<ActivitySample>,
    data_folder: String,
}

static RUNTIME: OnceLock<Mutex<RuntimeState>> = OnceLock::new();

struct PatchPlanPlugin;

impl Plugin for PatchPlanPlugin {
    fn new() -> Self {
        PatchPlanPlugin
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: PLUGIN_DATA_DIR_NAME.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Pumpkin Patch".into()],
            description: "Plan-inspired player analytics export for Pumpkin Patch".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<()> {
        let data_folder = context.get_data_folder();
        std::fs::create_dir_all(&data_folder).map_err(|e| e.to_string())?;

        let config_path = format!("{data_folder}/{CONFIG_FILENAME}");
        let config: PatchPlanConfig = if std::path::Path::new(&config_path).exists() {
            let raw = std::fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
            serde_json::from_str(&raw).map_err(|e| e.to_string())?
        } else {
            return Err(format!(
                "missing {CONFIG_FILENAME} — deploy via Pumpkin Patch to configure analytics export"
            ));
        };

        let state_path = format!("{data_folder}/{STATE_FILENAME}");
        let (players, total_joins, peak_online, activity_samples) =
            if std::path::Path::new(&state_path).exists() {
                load_persisted_state(&state_path)?
            } else {
                (HashMap::new(), 0, 0, Vec::new())
            };

        let rt = RuntimeState {
            config,
            players,
            total_joins,
            peak_online,
            activity_samples,
            data_folder: data_folder.clone(),
        };
        let _ = RUNTIME.set(Mutex::new(rt));

        context
            .register_event_handler(JoinHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(LeaveHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;

        context.schedule_repeating_task(100, 6000, |server| {
            let _ = flush_export(&server);
        });

        flush_export(&context.get_server()).map_err(|e| e.to_string())?;
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<()> {
        if let Some(lock) = RUNTIME.get() {
            if let Ok(rt) = lock.lock() {
                let _ = persist_state(&rt);
                let _ = write_export(&rt, 0);
            }
        }
        Ok(())
    }
}

struct JoinHandler;

impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(
        &self,
        server: Server,
        event: <PlayerJoinEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerJoinEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        let now = unix_now();
        if let Some(lock) = RUNTIME.get() {
            if let Ok(mut rt) = lock.lock() {
                let id = uuid::to_string(event.player.get_id());
                let name = event.player.get_name();
                rt.total_joins += 1;
                let entry = rt.players.entry(id.clone()).or_insert_with(|| PlayerStats {
                    uuid: id.clone(),
                    name: name.clone(),
                    first_seen_unix: now,
                    last_seen_unix: now,
                    join_count: 0,
                    playtime_secs: 0,
                });
                entry.name = name;
                entry.last_seen_unix = now;
                entry.join_count += 1;
                let online = server.get_player_count();
                rt.peak_online = rt.peak_online.max(online);
                push_sample(&mut rt, now, online);
            }
        }
        event
    }
}

struct LeaveHandler;

impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(
        &self,
        server: Server,
        event: <PlayerLeaveEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerLeaveEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        let now = unix_now();
        if let Some(lock) = RUNTIME.get() {
            if let Ok(mut rt) = lock.lock() {
                let id = uuid::to_string(event.player.get_id());
                if let Some(entry) = rt.players.get_mut(&id) {
                    let delta = (now - entry.last_seen_unix).max(0) as u64;
                    entry.playtime_secs = entry.playtime_secs.saturating_add(delta);
                    entry.last_seen_unix = now;
                }
                let online = server.get_player_count().saturating_sub(1);
                push_sample(&mut rt, now, online);
            }
        }
        event
    }
}

fn push_sample(rt: &mut RuntimeState, t: i64, online: u32) {
    rt.activity_samples.push(ActivitySample { t, online });
    if rt.activity_samples.len() > 288 {
        let drain = rt.activity_samples.len() - 288;
        rt.activity_samples.drain(0..drain);
    }
}

fn flush_export(server: &Server) -> std::result::Result<(), String> {
    let Some(lock) = RUNTIME.get() else {
        return Ok(());
    };
    let rt = lock.lock().map_err(|e| e.to_string())?;
    let online = server.get_player_count();
    write_export(&rt, online)?;
    persist_state(&rt)
}

fn write_export(rt: &RuntimeState, online_now: u32) -> Result<(), String> {
    let now = unix_now();
    let unique = rt.players.len() as u64;
    let total_playtime: u64 = rt.players.values().map(|p| p.playtime_secs).sum();
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
        players: rt.players.values().cloned().collect(),
        activity_samples: rt.activity_samples.clone(),
    };
    let signature = sign_payload(&payload, &rt.config.export_secret)?;
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

fn persist_state(rt: &RuntimeState) -> Result<(), String> {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Persisted {
        players: HashMap<String, PlayerStats>,
        total_joins: u64,
        peak_online: u32,
        activity_samples: Vec<ActivitySample>,
    }
    let path = format!("{}/{STATE_FILENAME}", rt.data_folder);
    let body = Persisted {
        players: rt.players.clone(),
        total_joins: rt.total_joins,
        peak_online: rt.peak_online,
        activity_samples: rt.activity_samples.clone(),
    };
    let json = serde_json::to_string_pretty(&body).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_persisted_state(
    path: &str,
) -> Result<
    (
        HashMap<String, PlayerStats>,
        u64,
        u32,
        Vec<ActivitySample>,
    ),
    String,
> {
    #[derive(serde::Serialize, serde::Deserialize)]
    struct Persisted {
        players: HashMap<String, PlayerStats>,
        total_joins: u64,
        peak_online: u32,
        activity_samples: Vec<ActivitySample>,
    }
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let p: Persisted = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    Ok((
        p.players,
        p.total_joins,
        p.peak_online,
        p.activity_samples,
    ))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pumpkin_plugin_api::register_plugin!(PatchPlanPlugin);
