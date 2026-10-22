//! In-game `/patchplan` and `/plan` commands (Plan §7 subset).

use crate::state::{
    drain_desktop_plan_requests, drain_registration_completions, drain_web_admin_actions,
    persist_state, placeholder_context_for, queue_desktop_plan_request, PendingDbOp,
    player_id, queue_registration_completion, queue_web_admin_action, unix_now, with_runtime,
    write_export,
};
use patch_analytics_protocol::{DesktopPlanRequest, WebAdminAction};
use patch_plan_core::{
    active_playtime_in_weeks, resolve_placeholder, ActivityIndex, DEFAULT_PLAYTIME_THRESHOLD_MS,
};
use pumpkin_plugin_api::command::{
    Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs,
    StringType,
};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::Server;

pub fn register_plan_command(context: &pumpkin_plugin_api::Context) -> pumpkin_plugin_api::Result<()> {
    let names = [
        "patchplan".to_string(),
        "plan".to_string(),
        "planproxy".to_string(),
        "planbungee".to_string(),
        "planvelocity".to_string(),
    ];
    let cmd = Command::new(&names, "Patch Plan player analytics (Pumpkin Patch)")
        .execute(StatsHandler)
        .then(CommandNode::literal("help").execute(HelpHandler))
        .then(CommandNode::literal("top").execute(TopHandler))
        .then(
            literal_aliases("ingame", &["qinspect"], IngameHandler)
                .then(
                    CommandNode::argument("target", &ArgumentType::String(StringType::Word))
                        .execute(IngameOtherHandler),
                ),
        )
        .then(CommandNode::literal("sessions").execute(SessionsHandler))
        .then(
            CommandNode::literal("search").then(
                CommandNode::argument("query", &ArgumentType::String(StringType::Greedy))
                    .execute(SearchHandler),
            ),
        )
        .then(CommandNode::literal("info").execute(InfoHandler))
        .then(CommandNode::literal("json").execute(JsonHandler))
        .then(literal_aliases("servers", &["serverlist", "sl", "ls"], ServersListHandler))
        .then(CommandNode::literal("reload").execute(ReloadHandler))
        .then(
            CommandNode::literal("register").then(
                CommandNode::argument("code", &ArgumentType::String(StringType::Greedy))
                    .execute(RegisterHandler),
            ),
        )
        .then(literal_aliases("network", &["netw"], NetworkLinkHandler))
        .then(literal_aliases("players", &["pl", "playerlist", "list"], PlayersLinkHandler))
        .then(
            literal_aliases("player", &["inspect"], PlayerSelfLinkHandler).then(
                CommandNode::argument("target", &ArgumentType::String(StringType::Word))
                    .execute(PlayerLinkHandler),
            ),
        )
        .then(CommandNode::literal("export").execute(ExportHandler))
        .then(literal_aliases("server", &["analyze", "a", "analysis"], ServerLinkHandler))
        .then(literal_aliases("users", &["webusers"], UsersHandler))
        .then(
            CommandNode::literal("placeholder").then(
                CommandNode::argument("key", &ArgumentType::String(StringType::Greedy))
                    .execute(PlaceholderHandler),
            ),
        )
        .then(CommandNode::literal("disable").execute(DisableHandler))
        .then(
            CommandNode::literal("setgroup").then(
                CommandNode::argument("user", &ArgumentType::String(StringType::Word)).then(
                    CommandNode::argument("group", &ArgumentType::String(StringType::Word))
                        .execute(SetGroupHandler),
                ),
            ),
        )
        .then(
            CommandNode::literal("logout").then(
                CommandNode::argument("user", &ArgumentType::String(StringType::Word))
                    .execute(LogoutHandler),
            ),
        )
        .then(
            CommandNode::literal("unregister")
                .execute(UnregisterSelfHandler)
                .then(
                    CommandNode::argument("user", &ArgumentType::String(StringType::Word))
                        .execute(UnregisterOtherHandler),
                ),
        )
        .then(
            CommandNode::literal("db")
                .then(CommandNode::literal("status").execute(DbStatusHandler))
                .then(CommandNode::literal("optimize").execute(DbOptimizeHandler))
                .then(
                    CommandNode::literal("backup").then(
                        CommandNode::argument("db_type", &ArgumentType::String(StringType::Word))
                            .execute(DbBackupHandler),
                    ),
                )
                .then(
                    CommandNode::literal("remove").then(
                        CommandNode::argument("player", &ArgumentType::String(StringType::Word))
                            .execute(DbRemovePlayerHandler),
                    ),
                )
                .then(
                    CommandNode::literal("clear").then(
                        CommandNode::argument("db_type", &ArgumentType::String(StringType::Word))
                            .execute(DbClearPromptHandler),
                    ),
                )
                .then(
                    CommandNode::literal("uninstalled").then(
                        CommandNode::argument("server", &ArgumentType::String(StringType::Word))
                            .execute(DbUninstalledHandler),
                    ),
                )
                .then(
                    CommandNode::literal("restore").then(
                        CommandNode::argument("file", &ArgumentType::String(StringType::Greedy))
                            .execute(DbRestoreHandler),
                    ),
                ),
        )
        .then(literal_aliases("accept", &["yes", "y"], AcceptDbHandler))
        .then(literal_aliases("cancel", &["no", "n"], CancelDbHandler))
        .then(CommandNode::literal("import").execute(ImportHandler));
    context.register_command(cmd, "patchplan.command");
    Ok(())
}

fn literal_aliases<H: CommandHandler + Copy>(
    literal: &str,
    aliases: &[&str],
    handler: H,
) -> CommandNode {
    let mut node = CommandNode::literal(literal).execute(handler);
    for alias in aliases {
        node = node.then(CommandNode::literal(alias).execute(handler));
    }
    node
}

struct HelpHandler;

impl CommandHandler for HelpHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        for line in [
            "§6Patch Plan§r — analytics commands:",
            "§e/plan§r — your stats & activity index",
            "§e/plan top§r — top players by playtime (this server export)",
            "§e/plan ingame§r — same as /plan (chat stats summary)",
            "§e/plan sessions§r — your recent closed sessions",
            "§e/plan search <name>§r — find players on this server export",
            "§e/plan info§r — plugin & export status",
            "§e/plan json§r — GDPR JSON export path hint",
            "§e/plan help§r — this message",
            "§e/plan register§r — link a pending web registration code",
            "§e/plan setgroup <user> <group>§r — web RBAC (console)",
            "§e/plan logout <user>§r — invalidate web sessions (console)",
            "§e/plan unregister§r — delete your linked web account",
            "§e/plan placeholder <key>§r — resolve a %plan_*% placeholder",
            "§e/plan db status§r — SQLite export / flush status",
            "§e/plan db optimize§r — queue DB maintenance (desktop Plan store)",
        ] {
            player.send_system_message(TextComponent::text(line), false);
        }
        Ok(1)
    }
}

struct TopHandler;

impl CommandHandler for TopHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let lines = with_runtime(|rt| {
            let mut ranked: Vec<_> = rt.players.values().collect();
            ranked.sort_by_key(|p| std::cmp::Reverse(p.playtime_secs));
            let mut out = vec!["§6Top playtime (this server):".to_string()];
            for (i, entry) in ranked.iter().take(8).enumerate() {
                out.push(format!(
                    "§7{}. §f{}§7 — §f{}s§7 playtime",
                    i + 1,
                    entry.name,
                    entry.playtime_secs
                ));
            }
            if ranked.is_empty() {
                out.push("§7No player data yet.".into());
            }
            out
        })
        .unwrap_or_else(|| vec!["§7No player data yet.".into()]);
        for line in lines {
            player.send_system_message(TextComponent::text(&line), false);
        }
        Ok(1)
    }
}

struct SearchHandler;

impl CommandHandler for SearchHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let query = match args.get_value("query") {
            Arg::Simple(q) => q.to_lowercase(),
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan search <name>"),
                ));
            }
        };
        if query.len() < 2 {
            return Err(CommandError::CommandFailed(
                TextComponent::text("Search query must be at least 2 characters"),
            ));
        }
        let lines = with_runtime(|rt| {
            let mut matches: Vec<_> = rt
                .players
                .values()
                .filter(|p| p.name.to_lowercase().contains(&query))
                .collect();
            matches.sort_by(|a, b| a.name.cmp(&b.name));
            let mut out = vec![format!("§6Search§r for \"{}\":", query)];
            for p in matches.iter().take(10) {
                out.push(format!("§f{}§7 — {}s playtime", p.name, p.playtime_secs));
            }
            if matches.is_empty() {
                out.push("§7No matching players in this server export.".into());
            }
            Some(out)
        })
        .flatten()
        .unwrap_or_else(|| vec!["§7No player data yet.".into()]);
        for line in lines {
            player.send_system_message(TextComponent::text(&line), false);
        }
        Ok(1)
    }
}

struct SessionsHandler;

impl CommandHandler for SessionsHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let id = player_id(&player);
        let lines = with_runtime(|rt| {
            let entry = rt.players.get(&id)?;
            let mut out = vec![format!(
                "§6Sessions§r — {} closed this export cycle:",
                entry.completed_sessions.len()
            )];
            for (i, s) in entry.completed_sessions.iter().rev().take(6).enumerate() {
                let len = s.end_unix - s.start_unix;
                out.push(format!(
                    "§7{}. §f{}s§7 play · §f{}s§7 AFK · join §f{}",
                    i + 1,
                    len,
                    s.afk_secs,
                    if s.join_address.is_empty() {
                        "—".to_string()
                    } else {
                        s.join_address.clone()
                    }
                ));
            }
            if entry.completed_sessions.is_empty() {
                out.push("§7No closed sessions yet — stay online or relog to flush.".into());
            }
            Some(out)
        })
        .flatten()
        .unwrap_or_else(|| vec!["§7No session data yet.".into()]);
        for line in lines {
            player.send_system_message(TextComponent::text(&line), false);
        }
        Ok(1)
    }
}

struct InfoHandler;

impl CommandHandler for InfoHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let lines = with_runtime(|rt| {
            vec![
                "§6Patch Plan§r — Pumpkin Patch native analytics".into(),
                format!("§7Schema§r v{}", patch_analytics_protocol::EXPORT_SCHEMA_VERSION),
                format!("§7Players tracked§r {}", rt.players.len()),
                format!(
                    "§7Closed sessions§r {}",
                    rt.players
                        .values()
                        .map(|p| p.completed_sessions.len())
                        .sum::<usize>()
                ),
                "§7Desktop§r sync via signed export.json + HarvestMetrics".into(),
            ]
        })
        .unwrap_or_else(|| vec!["§7No runtime state.".into()]);
        for line in lines {
            player.send_system_message(TextComponent::text(&line), false);
        }
        Ok(1)
    }
}

struct JsonHandler;

impl CommandHandler for JsonHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let id = player_id(&player);
        let msg = with_runtime(|rt| {
            let entry = rt.players.get(&id)?;
            format!(
                "§7GDPR export§r: open Pumpkin Patch → Analytics for §f{}§7 (uuid {}) — or run §e/plan json§7 after static export.",
                entry.name,
                id
            )
        })
        .flatten()
        .unwrap_or_else(|| {
            "§7No player row yet — rejoin after Patch Plan WASM is deployed.".into()
        });
        player.send_system_message(TextComponent::text(&msg), false);
        Ok(1)
    }
}

struct ServersListHandler;

impl CommandHandler for ServersListHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let lines = with_runtime(|rt| {
            let mut out = vec!["§6Patch Plan§r — this server export:".to_string()];
            out.push(format!(
                "§7Players tracked: §f{}§7 · sessions closed: §f{}",
                rt.players.len(),
                rt.players
                    .values()
                    .map(|p| p.completed_sessions.len())
                    .sum::<usize>()
            ));
            out.push("§7Open Pumpkin Patch Analytics for full network server list.".into());
            out
        })
        .unwrap_or_else(|| vec!["§7No runtime state.".into()]);
        for line in lines {
            player.send_system_message(TextComponent::text(&line), false);
        }
        Ok(1)
    }
}

struct NetworkLinkHandler;

impl CommandHandler for NetworkLinkHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        player.send_system_message(
            TextComponent::text("§6Plan§r — open Pumpkin Patch §eAnalytics§r → Network for the full dashboard."),
            false,
        );
        Ok(1)
    }
}

struct PlayersLinkHandler;

impl CommandHandler for PlayersLinkHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        player.send_system_message(
            TextComponent::text("§6Plan§r — player directory: use desktop §eAnalytics → Players§r or Plan HTTP `/v1/players`."),
            false,
        );
        Ok(1)
    }
}

struct PlayerLinkHandler;

impl CommandHandler for PlayerLinkHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let target = match args.get_value("target") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan player <name>"),
                ));
            }
        };
        player.send_system_message(
            TextComponent::text(format!(
                "§6Plan§r — profile lookup for §f{}§r (synced via Plan HTTP `/v1/player`).",
                target
            )),
            false,
        );
        Ok(1)
    }
}

struct ServerLinkHandler;

impl CommandHandler for ServerLinkHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let url = with_runtime(|rt| {
            format!(
                "§6Plan§r server §f{}§r — open Analytics in Pumpkin Patch or Plan HTTP `/v1/serverOverview?server={}`",
                rt.config.server_name,
                rt.config.server_id
            )
        })
        .unwrap_or_else(|| "§7Plan server link unavailable.".into());
        player.send_system_message(TextComponent::text(&url), false);
        Ok(1)
    }
}

struct UsersHandler;

impl CommandHandler for UsersHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        player.send_system_message(
            TextComponent::text(
                "§6Plan web users§r — manage accounts in Pumpkin Patch Analytics → Plan web access, or Plan HTTP after sync.",
            ),
            false,
        );
        Ok(1)
    }
}

struct ExportHandler;

impl CommandHandler for ExportHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let _ = with_runtime(|rt| {
            let online = server.get_player_count();
            write_export(rt, online).ok();
            persist_state(rt).ok();
        });
        player.send_system_message(
            TextComponent::text("§aPlan export§r snapshot written for this server (Pumpkin Patch sync)."),
            false,
        );
        Ok(1)
    }
}

fn sync_plan_export(rt: &mut crate::state::RuntimeState, online: u32) {
    write_export(rt, online).ok();
    drain_registration_completions(rt);
    drain_web_admin_actions(rt);
    drain_desktop_plan_requests(rt);
    persist_state(rt).ok();
}

fn require_console(sender: &CommandSender) -> Result<(), CommandError> {
    if sender.as_player().is_some() {
        return Err(CommandError::CommandFailed(
            TextComponent::text("This subcommand requires the server console"),
        ));
    }
    Ok(())
}

struct SetGroupHandler;

impl CommandHandler for SetGroupHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let user = match args.get_value("user") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan setgroup <user> <group>"),
                ));
            }
        };
        let group = match args.get_value("group") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan setgroup <user> <group>"),
                ));
            }
        };
        let _ = with_runtime(|rt| {
            queue_web_admin_action(
                rt,
                WebAdminAction::SetGroup {
                    username: user.to_string(),
                    group_name: group.to_string(),
                },
            );
            sync_plan_export(rt, server.get_player_count());
        });
        Ok(1)
    }
}

struct LogoutHandler;

impl CommandHandler for LogoutHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let user = match args.get_value("user") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan logout <user>"),
                ));
            }
        };
        let _ = with_runtime(|rt| {
            queue_web_admin_action(
                rt,
                WebAdminAction::LogoutUser {
                    username: user.to_string(),
                },
            );
            sync_plan_export(rt, server.get_player_count());
        });
        Ok(1)
    }
}

struct UnregisterSelfHandler;

impl CommandHandler for UnregisterSelfHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let uuid = player_id(&player);
        let _ = with_runtime(|rt| {
            queue_web_admin_action(
                rt,
                WebAdminAction::DeleteUserByUuid {
                    player_uuid: uuid,
                },
            );
            sync_plan_export(rt, server.get_player_count());
        });
        player.send_system_message(
            TextComponent::text(
                "§ePlan unregister queued§7 — your linked web account will be removed on the next Pumpkin Patch sync.",
            ),
            false,
        );
        Ok(1)
    }
}

struct UnregisterOtherHandler;

impl CommandHandler for UnregisterOtherHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let user = match args.get_value("user") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan unregister <username>"),
                ));
            }
        };
        let _ = with_runtime(|rt| {
            queue_web_admin_action(
                rt,
                WebAdminAction::DeleteUser {
                    username: user.to_string(),
                },
            );
            sync_plan_export(rt, server.get_player_count());
        });
        Ok(1)
    }
}

struct RegisterHandler;

impl CommandHandler for RegisterHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let code = match args.get_value("code") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan register <code>"),
                ));
            }
        };
        if let Some(player) = sender.as_player() {
            let uuid = player_id(&player);
            let name = player.get_display_name().get_text();
            let _ = with_runtime(|rt| {
                queue_registration_completion(rt, code, &uuid, &name, false);
                sync_plan_export(rt, 1);
            });
            player.send_system_message(
                TextComponent::text(
                    "§aPlan registration queued§7 — Pumpkin Patch will link your web account on the next analytics sync.",
                ),
                false,
            );
        } else {
            let _ = with_runtime(|rt| {
                queue_registration_completion(rt, code, "", "console", true);
                sync_plan_export(rt, 0);
            });
        }
        Ok(1)
    }
}

struct DbStatusHandler;

impl CommandHandler for DbStatusHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let online = server.get_player_count();
        let line = with_runtime(|rt| {
            format!(
                "§6Plan DB§r — players §f{}§7 tracked, §f{}§7 online, export token active",
                rt.players.len(),
                online
            )
        })
        .unwrap_or_else(|| "§7Plan runtime not initialized.".into());
        player.send_system_message(TextComponent::text(&line), false);
        Ok(1)
    }
}

struct ReloadHandler;

impl CommandHandler for ReloadHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let _ = with_runtime(|rt| {
            let online = server.get_player_count();
            write_export(rt, online).ok();
            persist_state(rt).ok();
        });
        player.send_system_message(
            TextComponent::text("§aPatch Plan§r reloaded export snapshot for this server."),
            false,
        );
        Ok(1)
    }
}

struct IngameHandler;

impl CommandHandler for IngameHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let id = player_id(&player);
        send_ingame_summary(&player, &id, server.get_player_count());
        Ok(1)
    }
}

struct IngameOtherHandler;

impl CommandHandler for IngameOtherHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let target_name = match args.get_value("target") {
            Arg::Simple(s) => s.to_lowercase(),
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan ingame <player>"),
                ));
            }
        };
        let target_uuid = with_runtime(|rt| {
            rt.players
                .values()
                .find(|p| p.name.to_lowercase() == target_name)
                .map(|p| p.uuid.clone())
        })
        .flatten();
        let Some(target_uuid) = target_uuid else {
            return Err(CommandError::CommandFailed(
                TextComponent::text("Player not found in Plan export data"),
            ));
        };
        send_ingame_summary(&player, &target_uuid, server.get_player_count());
        Ok(1)
    }
}

struct PlaceholderHandler;

impl CommandHandler for PlaceholderHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let key = match args.get_value("key") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan placeholder %plan_players_online%"),
                ));
            }
        };
        let id = player_id(&player);
        let online = server.get_player_count();
        let msg = with_runtime(|rt| {
            let ctx = placeholder_context_for(rt, &id, online);
            resolve_placeholder(key, &ctx)
                .map(|v| format!("§6{}§r → §f{}", key, v))
                .or_else(|| {
                    Some(format!("§7Unknown or unsupported placeholder: §f{}", key))
                })
        })
        .flatten()
        .unwrap_or_else(|| "§7Plan runtime not initialized.".into());
        player.send_system_message(TextComponent::text(&msg), false);
        Ok(1)
    }
}

struct DisableHandler;

impl CommandHandler for DisableHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        Ok(1)
    }
}

struct DbOptimizeHandler;

impl CommandHandler for DbOptimizeHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let _ = with_runtime(|rt| {
            queue_desktop_plan_request(rt, DesktopPlanRequest::DbOptimize);
            sync_plan_export(rt, server.get_player_count());
        });
        Ok(1)
    }
}

struct DbBackupHandler;

impl CommandHandler for DbBackupHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let db_type = match args.get_value("db_type") {
            Arg::Simple(s) => s.to_string(),
            _ => "sqlite".into(),
        };
        let _ = with_runtime(|rt| {
            queue_desktop_plan_request(
                rt,
                DesktopPlanRequest::DbBackup {
                    label: db_type.clone(),
                },
            );
            sync_plan_export(rt, server.get_player_count());
        });
        Ok(1)
    }
}

struct DbRemovePlayerHandler;

impl CommandHandler for DbRemovePlayerHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let player = match args.get_value("player") {
            Arg::Simple(s) => s.to_string(),
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan db remove <player>"),
                ));
            }
        };
        let _ = with_runtime(|rt| {
            rt.pending_db_op = Some(PendingDbOp::RemovePlayer { target: player });
        });
        Ok(1)
    }
}

struct DbClearPromptHandler;

impl CommandHandler for DbClearPromptHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let _ = with_runtime(|rt| {
            rt.pending_db_op = Some(PendingDbOp::ClearNetwork);
        });
        Ok(1)
    }
}

struct AcceptDbHandler;

impl CommandHandler for AcceptDbHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let _ = with_runtime(|rt| {
            let op = rt.pending_db_op.take();
            if let Some(op) = op {
                match op {
                    PendingDbOp::ClearNetwork => {
                        queue_desktop_plan_request(rt, DesktopPlanRequest::DbClearNetwork);
                    }
                    PendingDbOp::RemovePlayer { target } => {
                        queue_desktop_plan_request(
                            rt,
                            DesktopPlanRequest::DbRemovePlayer { target },
                        );
                    }
                }
                sync_plan_export(rt, server.get_player_count());
            }
        });
        Ok(1)
    }
}

struct CancelDbHandler;

impl CommandHandler for CancelDbHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let _ = with_runtime(|rt| {
            rt.pending_db_op = None;
        });
        Ok(1)
    }
}

struct DbRestoreHandler;

impl CommandHandler for DbRestoreHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let path = match args.get_value("file") {
            Arg::Simple(s) => s.to_string(),
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan db restore <backup.db>"),
                ));
            }
        };
        let _ = with_runtime(|rt| {
            queue_desktop_plan_request(
                rt,
                DesktopPlanRequest::DbRestore {
                    backup_path: path,
                },
            );
            sync_plan_export(rt, server.get_player_count());
        });
        Ok(1)
    }
}

struct DbUninstalledHandler;

impl CommandHandler for DbUninstalledHandler {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let target = match args.get_value("server") {
            Arg::Simple(s) => s.to_string(),
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan db uninstalled <server>"),
                ));
            }
        };
        let _ = with_runtime(|rt| {
            queue_desktop_plan_request(
                rt,
                DesktopPlanRequest::DbMarkServerUninstalled { server: target },
            );
            sync_plan_export(rt, server.get_player_count());
        });
        Ok(1)
    }
}

struct ImportHandler;

impl CommandHandler for ImportHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        require_console(&sender)?;
        let _ = sender;
        Ok(1)
    }
}

struct PlayerSelfLinkHandler;

impl CommandHandler for PlayerSelfLinkHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let id = player_id(&player);
        player.send_system_message(
            TextComponent::text(&format!(
                "§6Plan profile§r — open Pumpkin Patch Analytics → player §f{}§7 (§f{}§7)",
                player.get_display_name().get_text(),
                id
            )),
            false,
        );
        Ok(1)
    }
}

fn send_ingame_summary(
    player: &pumpkin_plugin_api::player::Player,
    target_uuid: &str,
    online: u32,
) {
    let (summary, sessions_hint) = with_runtime(|rt| {
        let entry = rt.players.get(target_uuid)?;
        let now_ms = unix_now() * 1000;
        let triples: Vec<(i64, i64, u64)> = entry
            .completed_sessions
            .iter()
            .map(|s| {
                (
                    s.start_unix * 1000,
                    s.end_unix * 1000,
                    s.afk_secs * 1000,
                )
            })
            .collect();
        let (w1, w2, w3) = active_playtime_in_weeks(&triples, now_ms);
        let ai = ActivityIndex::calculate(w1, w2, w3, DEFAULT_PLAYTIME_THRESHOLD_MS);
        let geo = if entry.geolocation.is_empty() {
            String::new()
        } else {
            format!(" | Geo {}", entry.geolocation)
        };
        let summary = format!(
            "§6{}§r — Playtime {}s | AFK {}s | Deaths {} | PvP {} | Mob {} | PAI {:.2} ({}){}",
            entry.name,
            entry.playtime_secs,
            entry.afk_time_secs,
            entry.deaths,
            entry.player_kills,
            entry.mob_kills,
            ai.value,
            ai.group.label(),
            geo
        );
        let sessions_hint = if entry.completed_sessions.is_empty() {
            "Sessions: none closed yet this export cycle.".into()
        } else {
            let last = entry.completed_sessions.last().unwrap();
            format!(
                "Sessions: {} closed | last {}s (AFK {}s)",
                entry.completed_sessions.len(),
                last.end_unix - last.start_unix,
                last.afk_secs
            )
        };
        Some((summary, sessions_hint))
    })
    .flatten()
    .unwrap_or_else(|| {
        (
            "No stats recorded yet.".into(),
            "Play on this server with Patch Plan deployed.".into(),
        )
    });
    let _ = online;
    player.send_system_message(TextComponent::text(&summary), false);
    player.send_system_message(TextComponent::text(&sessions_hint), false);
}

struct StatsHandler;

impl CommandHandler for StatsHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let id = player_id(&player);
        send_ingame_summary(&player, &id, _server.get_player_count());
        Ok(1)
    }
}

fn require_player(sender: &CommandSender) -> Result<pumpkin_plugin_api::player::Player, CommandError> {
    sender.as_player().ok_or_else(|| {
        CommandError::CommandFailed(TextComponent::text("Only players can use this command"))
    })
}
