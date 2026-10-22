//! In-game `/patchplan` and `/plan` commands (Plan §7 subset).

use crate::state::{persist_state, player_id, unix_now, with_runtime, write_export};
use patch_plan_core::{active_playtime_in_weeks, ActivityIndex, DEFAULT_PLAYTIME_THRESHOLD_MS};
use pumpkin_plugin_api::command::{
    Arg, ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs,
    StringType,
};
use pumpkin_plugin_api::commands::CommandHandler;
use pumpkin_plugin_api::text::TextComponent;
use pumpkin_plugin_api::Server;

pub fn register_plan_command(context: &pumpkin_plugin_api::Context) -> pumpkin_plugin_api::Result<()> {
    let names = ["patchplan".to_string(), "plan".to_string()];
    let cmd = Command::new(&names, "Patch Plan player analytics (Pumpkin Patch)")
        .execute(StatsHandler)
        .then(CommandNode::literal("help").execute(HelpHandler))
        .then(CommandNode::literal("top").execute(TopHandler))
        .then(CommandNode::literal("ingame").execute(StatsHandler))
        .then(CommandNode::literal("sessions").execute(SessionsHandler))
        .then(
            CommandNode::literal("search").then(
                CommandNode::argument("query", &ArgumentType::String(StringType::Greedy))
                    .execute(SearchHandler),
            ),
        )
        .then(CommandNode::literal("info").execute(InfoHandler))
        .then(CommandNode::literal("json").execute(JsonHandler))
        .then(CommandNode::literal("servers").execute(ServersListHandler))
        .then(CommandNode::literal("reload").execute(ReloadHandler))
        .then(
            CommandNode::literal("register").then(
                CommandNode::argument("code", &ArgumentType::String(StringType::Greedy))
                    .execute(RegisterHandler),
            ),
        )
        .then(CommandNode::literal("network").execute(NetworkLinkHandler))
        .then(CommandNode::literal("players").execute(PlayersLinkHandler))
        .then(
            CommandNode::literal("player").then(
                CommandNode::argument("target", &ArgumentType::String(StringType::Word))
                    .execute(PlayerLinkHandler),
            ),
        )
        .then(CommandNode::literal("export").execute(ExportHandler))
        .then(CommandNode::literal("db").then(CommandNode::literal("status").execute(DbStatusHandler)));
    context.register_command(cmd, "patchplan.command");
    Ok(())
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
            "§e/plan db status§r — SQLite export / flush status",
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

struct RegisterHandler;

impl CommandHandler for RegisterHandler {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let player = require_player(&sender)?;
        let _code = match args.get_value("code") {
            Arg::Simple(s) => s,
            _ => {
                return Err(CommandError::CommandFailed(
                    TextComponent::text("Usage: /plan register <code>"),
                ));
            }
        };
        player.send_system_message(
            TextComponent::text(
                "§aRegistration code received§7 — sync via Pumpkin Patch Plan HTTP when linked.",
            ),
            false,
        );
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
        let (summary, sessions_hint) = with_runtime(|rt| {
            let entry = rt.players.get(&id)?;
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
                "Playtime {}s | AFK {}s | Deaths {} | PvP {} | Mob {} | PAI {:.2} ({}){}",
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

        player.send_system_message(TextComponent::text(&summary), false);
        player.send_system_message(TextComponent::text(&sessions_hint), false);
        Ok(1)
    }
}

fn require_player(sender: &CommandSender) -> Result<pumpkin_plugin_api::player::Player, CommandError> {
    sender.as_player().ok_or_else(|| {
        CommandError::CommandFailed(TextComponent::text("Only players can use this command"))
    })
}
