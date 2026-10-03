use crate::state::{
    gamemode_label, player_id, push_death, push_event, push_sample, record_stat, touch_player,
    unix_now, with_runtime,
};
use patch_analytics_protocol::MAX_RECENT_ADVANCEMENTS_PER_PLAYER;
use pumpkin_plugin_api::events::{
    EventHandler, PlayerAdvancementDoneEvent, PlayerBedEnterEvent, PlayerChangedWorldEvent,
    PlayerChatEvent, PlayerCommandSendEvent, PlayerDeathEvent, PlayerDropItemEvent,
    PlayerExpChangeEvent, PlayerFishEvent, PlayerGamemodeChangeEvent, PlayerHarvestBlockEvent,
    PlayerItemBreakEvent, PlayerItemConsumeEvent, PlayerItemDamageEvent, PlayerJoinEvent,
    PlayerKickEvent, PlayerLeaveEvent, PlayerLevelChangeEvent, PlayerLoginEvent, PlayerPortalEvent,
    PlayerRecipeDiscoverEvent, PlayerRespawnEvent, PlayerStatisticIncrementEvent,
    PlayerTeleportEvent,
};
use pumpkin_plugin_api::Server;

pub struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(
        &self,
        server: Server,
        event: <PlayerJoinEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerJoinEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        let now = unix_now();
        with_runtime(|rt| {
            rt.total_joins += 1;
            let entry = touch_player(rt, &event.player, now);
            entry.join_count += 1;
            let online = server.get_player_count();
            rt.peak_online = rt.peak_online.max(online);
            push_sample(rt, now, online);
            push_event(rt, "join", &event.player, "joined server".into());
        });
        event
    }
}

pub struct LeaveHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(
        &self,
        server: Server,
        event: <PlayerLeaveEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerLeaveEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        let now = unix_now();
        with_runtime(|rt| {
            let id = player_id(&event.player);
            if let Some(entry) = rt.players.get_mut(&id) {
                let delta = (now - entry.last_seen_unix).max(0) as u64;
                entry.playtime_secs = entry.playtime_secs.saturating_add(delta);
                entry.last_seen_unix = now;
            }
            push_event(rt, "leave", &event.player, "left server".into());
            let online = server.get_player_count().saturating_sub(1);
            push_sample(rt, now, online);
        });
        event
    }
}

pub struct LoginHandler;
impl EventHandler<PlayerLoginEvent> for LoginHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerLoginEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerLoginEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            rt.total_logins += 1;
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.login_count += 1;
            push_event(rt, "login", &event.player, "authenticated".into());
        });
        event
    }
}

pub struct KickHandler;
impl EventHandler<PlayerKickEvent> for KickHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerKickEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerKickEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            rt.total_kicks += 1;
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.kick_count += 1;
            push_event(rt, "kick", &event.player, event.reason.clone());
        });
        event
    }
}

pub struct DeathHandler;
impl EventHandler<PlayerDeathEvent> for DeathHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerDeathEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerDeathEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        let msg = event.death_message.get_text();
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.deaths += 1;
            push_death(rt, &event.player, msg.clone());
            push_event(rt, "death", &event.player, msg);
        });
        event
    }
}

pub struct RespawnHandler;
impl EventHandler<PlayerRespawnEvent> for RespawnHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerRespawnEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerRespawnEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            push_event(rt, "respawn", &event.player, "respawned".into());
        });
        event
    }
}

pub struct AdvancementHandler;
impl EventHandler<PlayerAdvancementDoneEvent> for AdvancementHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerAdvancementDoneEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerAdvancementDoneEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.advancements += 1;
            entry.recent_advancements.push(event.advancement_id.clone());
            if entry.recent_advancements.len() > MAX_RECENT_ADVANCEMENTS_PER_PLAYER {
                let extra = entry.recent_advancements.len() - MAX_RECENT_ADVANCEMENTS_PER_PLAYER;
                entry.recent_advancements.drain(0..extra);
            }
            push_event(
                rt,
                "advancement",
                &event.player,
                event.advancement_id.clone(),
            );
        });
        event
    }
}

pub struct StatisticHandler;
impl EventHandler<PlayerStatisticIncrementEvent> for StatisticHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerStatisticIncrementEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerStatisticIncrementEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            record_stat(rt, &event.player, &event.statistic_id, event.amount);
        });
        event
    }
}

pub struct GamemodeHandler;
impl EventHandler<PlayerGamemodeChangeEvent> for GamemodeHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerGamemodeChangeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerGamemodeChangeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        let new_mode = gamemode_label(event.new_gamemode);
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.current_gamemode = new_mode.clone();
            push_event(
                rt,
                "gamemode",
                &event.player,
                format!(
                    "{} -> {}",
                    gamemode_label(event.previous_gamemode),
                    new_mode
                ),
            );
        });
        event
    }
}

pub struct WorldChangeHandler;
impl EventHandler<PlayerChangedWorldEvent> for WorldChangeHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerChangedWorldEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerChangedWorldEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        let world = event.to_world.get_name();
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.last_world = world.clone();
            entry.world_changes += 1;
            push_event(rt, "world", &event.player, world);
        });
        event
    }
}

pub struct TeleportHandler;
impl EventHandler<PlayerTeleportEvent> for TeleportHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerTeleportEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerTeleportEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.teleports += 1;
            push_event(rt, "teleport", &event.player, "teleported".into());
        });
        event
    }
}

pub struct HarvestHandler;
impl EventHandler<PlayerHarvestBlockEvent> for HarvestHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerHarvestBlockEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerHarvestBlockEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.blocks_harvested += 1;
        });
        event
    }
}

pub struct ConsumeHandler;
impl EventHandler<PlayerItemConsumeEvent> for ConsumeHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerItemConsumeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerItemConsumeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.items_consumed += 1;
            push_event(rt, "consume", &event.player, event.item_name.clone());
        });
        event
    }
}

pub struct ItemBreakHandler;
impl EventHandler<PlayerItemBreakEvent> for ItemBreakHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerItemBreakEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerItemBreakEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.items_broken += 1;
            push_event(rt, "item_break", &event.player, event.item_name.clone());
        });
        event
    }
}

pub struct RecipeHandler;
impl EventHandler<PlayerRecipeDiscoverEvent> for RecipeHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerRecipeDiscoverEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerRecipeDiscoverEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.recipes_discovered += 1;
            push_event(rt, "recipe", &event.player, event.recipe_id.clone());
        });
        event
    }
}

pub struct ExpHandler;
impl EventHandler<PlayerExpChangeEvent> for ExpHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerExpChangeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerExpChangeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.xp_gained += event.amount as i64;
        });
        event
    }
}

pub struct LevelHandler;
impl EventHandler<PlayerLevelChangeEvent> for LevelHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerLevelChangeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerLevelChangeEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.xp_level = event.new_level;
            push_event(
                rt,
                "level",
                &event.player,
                format!("{} -> {}", event.old_level, event.new_level),
            );
        });
        event
    }
}

pub struct ChatHandler;
impl EventHandler<PlayerChatEvent> for ChatHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerChatEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerChatEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.chat_messages += 1;
        });
        event
    }
}

pub struct CommandHandler;
impl EventHandler<PlayerCommandSendEvent> for CommandHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerCommandSendEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerCommandSendEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.commands_used += 1;
            push_event(rt, "command", &event.player, event.command.clone());
        });
        event
    }
}

pub struct FishHandler;
impl EventHandler<PlayerFishEvent> for FishHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerFishEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerFishEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        use pumpkin_plugin_api::wit::pumpkin::plugin::event::PlayerFishState;
        if event.cancelled {
            return event;
        }
        if !matches!(event.state, PlayerFishState::CaughtFish) {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.fish_caught += 1;
            push_event(
                rt,
                "fish",
                &event.player,
                format!("caught {}", event.caught_type),
            );
        });
        event
    }
}

pub struct DropItemHandler;
impl EventHandler<PlayerDropItemEvent> for DropItemHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerDropItemEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerDropItemEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.items_dropped += 1;
        });
        event
    }
}

pub struct BedEnterHandler;
impl EventHandler<PlayerBedEnterEvent> for BedEnterHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerBedEnterEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerBedEnterEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.beds_entered += 1;
            push_event(rt, "bed", &event.player, "entered bed".into());
        });
        event
    }
}

pub struct ItemDamageHandler;
impl EventHandler<PlayerItemDamageEvent> for ItemDamageHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerItemDamageEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerItemDamageEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.items_damaged += 1;
        });
        event
    }
}

pub struct PortalHandler;
impl EventHandler<PlayerPortalEvent> for PortalHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerPortalEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerPortalEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        with_runtime(|rt| {
            let now = unix_now();
            let entry = touch_player(rt, &event.player, now);
            entry.portal_uses += 1;
            push_event(rt, "portal", &event.player, "used portal".into());
        });
        event
    }
}
