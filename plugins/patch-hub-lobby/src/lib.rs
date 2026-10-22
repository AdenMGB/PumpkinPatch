//! Hub lobby plugin — `/patchjoin` and join hints for Velocity backends.

mod config;

use config::{load_server_list, ServerListState};
use patch_network_protocol::HUB_LOBBY_PLUGIN_NAME;
use pumpkin_plugin_api::command::{Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, StringType};
use pumpkin_plugin_api::server::CommandSender as ServerCommandSender;
use pumpkin_plugin_api::commands::{CommandHandler, CommandSuggestion, CommandSuggestionHandler, CommandSuggestions, SuggestionRequest};
use pumpkin_plugin_api::events::{EventHandler, EventPriority, FromIntoEvent, PlayerJoinEvent};
use pumpkin_plugin_api::permissions::{FS_READ_DATA, FS_WRITE_DATA};
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Result, Server};
use std::sync::{Mutex, OnceLock};

static SERVERS: OnceLock<Mutex<ServerListState>> = OnceLock::new();

fn servers() -> &'static Mutex<ServerListState> {
    SERVERS.get_or_init(|| Mutex::new(ServerListState::default()))
}

struct HubLobbyPlugin;

impl Plugin for HubLobbyPlugin {
    fn new() -> Self {
        HubLobbyPlugin
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: HUB_LOBBY_PLUGIN_NAME.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Pumpkin Patch".into()],
            description: "Velocity hub server selector (/patchjoin)".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<()> {
        let data_folder = context.get_data_folder();
        let list = load_server_list(&data_folder)?;
        servers().lock().map_err(|e| e.to_string())?.replace(list);

        context
            .register_event_handler(JoinHintHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;

        let target = CommandNode::argument(
            "server",
            &ArgumentType::String(StringType::SingleWord),
        )
            .suggest(SuggestServers)
            .execute(JoinServerHandler);

        let names = ["patchjoin".to_string(), "join".to_string()];
        let cmd = Command::new(&names, "Join a game server on this network").then(target);

        context.register_command(cmd, "patchhub.join");

        Ok(())
    }
}

struct JoinHintHandler;

impl EventHandler<PlayerJoinEvent> for JoinHintHandler {
    fn handle(
        &self,
        _server: Server,
        event: <PlayerJoinEvent as FromIntoEvent>::Data,
    ) -> <PlayerJoinEvent as FromIntoEvent>::Data {
        let list = servers()
            .lock()
            .map(|s| s.entries.clone())
            .unwrap_or_default();
        if list.is_empty() {
            event.player.send_system_message(
                TextComponent::text("No game servers configured. Ask an admin to add backends in Pumpkin Patch."),
                false,
            );
            return event;
        }

        event.player.send_system_message(
            TextComponent::text("§6§lPumpkin Patch §r— pick a server with §e/patchjoin <name>"),
            false,
        );
        for entry in &list {
            let line = format!(
                "§7• §f{} §8({}) §7— {}",
                entry.display_name,
                entry.velocity_name,
                entry.description
            );
            event.player.send_system_message(TextComponent::text(&line), false);
        }
        event
    }
}

struct SuggestServers;

impl CommandSuggestionHandler for SuggestServers {
    fn suggest(
        &self,
        _sender: CommandSender,
        _server: Server,
        request: SuggestionRequest,
    ) -> CommandSuggestions {
        let list = servers()
            .lock()
            .map(|s| s.entries.clone())
            .unwrap_or_default();
        let prefix = request.remaining.to_ascii_lowercase();
        let values = list
            .iter()
            .filter(|e| {
                e.velocity_name.to_ascii_lowercase().starts_with(&prefix)
                    || e.display_name.to_ascii_lowercase().starts_with(&prefix)
            })
            .map(|e| CommandSuggestion {
                value: e.velocity_name.clone(),
                tooltip: Some(TextComponent::text(&e.display_name)),
            })
            .collect();
        CommandSuggestions {
            start: request.start,
            length: request.remaining.len() as u32,
            values,
        }
    }
}

struct JoinServerHandler;

impl CommandHandler for JoinServerHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: pumpkin_plugin_api::command::ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let target = args.get_value("server");
        let name = match target {
            Arg::Simple(name) => name,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Expected a server name"),
                ));
            }
        };

        let list = servers()
            .lock()
            .map(|s| s.entries.clone())
            .unwrap_or_default();
        let known = list.iter().any(|e| e.velocity_name.eq_ignore_ascii_case(&name));
        if !known {
            return Err(CommandError::CommandFailed(
                TextComponent::text(&format!("Unknown server \"{name}\". Use /patchjoin with a name from the list.")),
            ));
        }

        let player = sender.as_player().ok_or_else(|| {
            CommandError::CommandFailed(TextComponent::text("Only players can join servers"))
        })?;

        let velocity_cmd = format!("server {name}");
        server.execute_command(&velocity_cmd, ServerCommandSender::Player(player));
        Ok(1)
    }
}

pumpkin_plugin_api::register_plugin!(HubLobbyPlugin);
