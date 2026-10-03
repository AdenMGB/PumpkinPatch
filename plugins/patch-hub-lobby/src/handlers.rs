use crate::state::{reload_list_from_disk, with_runtime};
use pumpkin_plugin_api::events::{
    EventHandler, PlayerCommandSendEvent, PlayerJoinEvent,
};
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::Server;

fn tell(player: &pumpkin_plugin_api::Player, message: &str) {
    let _ = player.send_system_message(TextComponent::text(message), false);
}

pub struct JoinHandler;

impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerJoinEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerJoinEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        with_runtime(|rt| {
            reload_list_from_disk(rt);
            tell(&event.player, "§6§lPumpkin Patch §7— choose a server:");
            if rt.list.servers.is_empty() {
                tell(
                    &event.player,
                    "§cNo backends configured yet. Ask an admin to add servers in Patch.",
                );
                return;
            }
            for entry in &rt.list.servers {
                let line = format!(
                    "§e{} §7— {} §8(/join {} or /server {})",
                    entry.display_name,
                    entry.description,
                    entry.velocity_name,
                    entry.velocity_name
                );
                tell(&event.player, &line);
            }
        });
        event
    }
}

pub struct CommandHandler;

impl EventHandler<PlayerCommandSendEvent> for CommandHandler {
    fn handle(
        &self,
        _server: Server,
        mut event: <PlayerCommandSendEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data,
    ) -> <PlayerCommandSendEvent as pumpkin_plugin_api::events::FromIntoEvent>::Data {
        if event.cancelled {
            return event;
        }
        let cmd = event.command.trim();
        let lower = cmd.to_ascii_lowercase();
        if lower == "/servers" || lower == "/hub" {
            event.cancelled = true;
            with_runtime(|rt| {
                reload_list_from_disk(rt);
                tell(&event.player, "§6Available servers:");
                for entry in &rt.list.servers {
                    tell(
                        &event.player,
                        &format!(
                            "§e{} §7→ §f/server {}",
                            entry.display_name, entry.velocity_name
                        ),
                    );
                }
            });
            return event;
        }
        if let Some(target) = lower.strip_prefix("/join ") {
            let target = target.trim();
            event.cancelled = true;
            with_runtime(|rt| {
                reload_list_from_disk(rt);
                let known = rt.list.servers.iter().any(|s| s.velocity_name == target);
                if known {
                    tell(
                        &event.player,
                        &format!("§aRun §f/server {target} §a(on Velocity) to connect."),
                    );
                } else {
                    tell(
                        &event.player,
                        "§cUnknown server. Use §f/servers §cto list options.",
                    );
                }
            });
        }
        event
    }
}
