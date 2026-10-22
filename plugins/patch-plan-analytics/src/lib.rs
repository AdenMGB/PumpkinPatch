//! Patch Plan — Plan-style analytics for Pumpkin (Pumpkin Patch).

mod handlers;
mod plan_cmd;
mod state;

use handlers::{
    AdvancementHandler, ChatHandler, CommandHandler, ConsumeHandler, DeathHandler, ExpHandler,
    BedEnterHandler, DropItemHandler, FishHandler, GamemodeHandler, HarvestHandler,
    ItemBreakHandler, ItemDamageHandler,     JoinHandler, KickHandler, LeaveHandler, LevelHandler,
    LoginHandler, PortalHandler, PreLoginHandler, RecipeHandler, RespawnHandler, StatisticHandler,
    TeleportHandler,
    WorldChangeHandler,
};
use patch_analytics_protocol::{PatchPlanConfig, CONFIG_FILENAME, PLUGIN_DATA_DIR_NAME};
use pumpkin_plugin_api::events::EventPriority;
use pumpkin_plugin_api::permissions::{FS_READ_DATA, FS_WRITE_DATA};
use pumpkin_plugin_api::scheduler::SchedulerExt;
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Result};
use patch_plan_core::DEFAULT_AFK_THRESHOLD_MS;
use patch_plan_core::AfkTracker;
use state::{load_persisted_state, persist_state, write_export, RuntimeState, RUNTIME};

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
            description: "Plan-inspired player analytics (stats, deaths, advancements, sessions)".into(),
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

        let state_path = format!("{data_folder}/{}", patch_analytics_protocol::STATE_FILENAME);
        let persisted = if std::path::Path::new(&state_path).exists() {
            load_persisted_state(&state_path)?
        } else {
            state::PersistedState {
                players: Default::default(),
                command_usages: Default::default(),
                total_joins: 0,
                total_logins: 0,
                total_kicks: 0,
                peak_online: 0,
                activity_samples: vec![],
                recent_deaths: vec![],
                recent_events: vec![],
                recent_pvp_kills: vec![],
                performance_samples: vec![],
            }
        };

        let rt = RuntimeState {
            config,
            players: persisted.players,
            active_sessions: Default::default(),
            afk_tracker: AfkTracker::new(DEFAULT_AFK_THRESHOLD_MS),
            command_usages: persisted.command_usages,
            total_joins: persisted.total_joins,
            total_logins: persisted.total_logins,
            total_kicks: persisted.total_kicks,
            peak_online: persisted.peak_online,
            activity_samples: persisted.activity_samples,
            recent_deaths: persisted.recent_deaths,
            recent_events: persisted.recent_events,
            recent_pvp_kills: persisted.recent_pvp_kills,
            performance_samples: persisted.performance_samples,
            last_perf_instant: None,
            data_folder: data_folder.clone(),
            pending_geolocation: Default::default(),
            online_ping: Default::default(),
            server_enabled_at: state::unix_now(),
            registration_completions: Vec::new(),
            web_admin_actions: Vec::new(),
            vineyard_sink: None,
        };
        let _ = RUNTIME.set(std::sync::Mutex::new(rt));

        context
            .register_event_handler(PreLoginHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(JoinHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(LeaveHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(LoginHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(KickHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(DeathHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(RespawnHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(AdvancementHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(StatisticHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(GamemodeHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(WorldChangeHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(TeleportHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(HarvestHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(ConsumeHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(ItemBreakHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(RecipeHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(ExpHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(LevelHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(ChatHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(CommandHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(FishHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(PortalHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(DropItemHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(BedEnterHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(ItemDamageHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;

        plan_cmd::register_plan_command(&context)?;

        context.schedule_repeating_task(100, 6000, |server| {
            let _ = state::with_runtime(|rt| {
                let online = server.get_player_count();
                write_export(rt, online).ok();
                persist_state(rt).ok();
            });
        });

        context.schedule_repeating_task(20, 100, |server| {
            let _ = state::with_runtime(|rt| {
                state::sample_performance(rt, server.get_player_count(), 100.0);
            });
        });

        context.schedule_repeating_task(40, 40, |server| {
            let _ = state::with_runtime(|rt| state::sample_online_pings(rt, &server));
        });

        let server = context.get_server();
        if let Some(Err(e)) = state::with_runtime(|rt| write_export(rt, server.get_player_count())) {
            return Err(e);
        }
        if let Some(Err(e)) = state::with_runtime(|rt| persist_state(rt)) {
            return Err(e);
        }

        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<()> {
        state::with_runtime(|rt| {
            persist_state(rt).ok();
            write_export(rt, 0).ok();
        });
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(PatchPlanPlugin);
