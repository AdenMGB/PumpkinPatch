use crate::error::{Error, Result};
use crate::instance_config::sync_server_instance_config;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    pub bind_address: String,
    pub online_mode: bool,
    pub max_players: u32,
    pub view_distance: u8,
    pub simulation_distance: u8,
    pub motd: String,
    pub commands_enabled: bool,
}

impl ServerSettings {
    pub fn for_game_port(port: u16) -> Self {
        Self {
            bind_address: bind_address_for_port(port),
            online_mode: false,
            max_players: 100,
            view_distance: 12,
            simulation_distance: 10,
            motd: "Pumpkin Patch Network".into(),
            commands_enabled: true,
        }
    }
}

pub fn read_server_settings(instance_path: &Path, game_port: u16) -> Result<ServerSettings> {
    let patch = instance_path.join("pumpkin.patch.toml");
    let main = instance_path.join("pumpkin.toml");
    let mut settings = ServerSettings::for_game_port(game_port);
    if patch.exists() {
        merge_from_toml(&mut settings, &std::fs::read_to_string(&patch)?)?;
    }
    if main.exists() {
        merge_from_toml(&mut settings, &std::fs::read_to_string(&main)?)?;
    }
    let expected = bind_address_for_port(game_port);
    if port_from_bind_address(&settings.bind_address).ok() != Some(game_port) {
        settings.bind_address = expected;
    }
    Ok(settings)
}

pub fn write_server_settings(
    instance_path: &Path,
    game_port: u16,
    forwarding_secret: &str,
    settings: &ServerSettings,
) -> Result<()> {
    std::fs::create_dir_all(instance_path)?;
    let mut settings = settings.clone();
    settings.bind_address = bind_address_for_port(game_port);
    settings.online_mode = false;

    let patch_path = instance_path.join("pumpkin.patch.toml");
    let mut existing = if patch_path.exists() {
        std::fs::read_to_string(&patch_path)?
    } else {
        String::new()
    };
    let block = format_settings_patch(&settings);
    replace_or_append_section(&mut existing, "[networking.java]", &block);
    if !existing.contains("[networking.proxy]") {
        existing.push_str(&format_proxy_block(forwarding_secret));
    }
    if !existing.contains("[commands]") {
        existing.push_str(&format!(
            "\n[commands]\nenabled = {}\n",
            settings.commands_enabled
        ));
    } else {
        existing = upsert_commands(&existing, settings.commands_enabled);
    }
    std::fs::write(patch_path, existing)?;

    sync_server_instance_config(instance_path, game_port, forwarding_secret)?;

    if let Ok(mut synced) = read_server_settings(instance_path, game_port) {
        synced.max_players = settings.max_players;
        synced.view_distance = settings.view_distance;
        synced.simulation_distance = settings.simulation_distance;
        synced.motd = settings.motd.clone();
        synced.commands_enabled = settings.commands_enabled;
        let pumpkin_path = instance_path.join("pumpkin.toml");
        if pumpkin_path.exists() {
            let text = std::fs::read_to_string(&pumpkin_path)?;
            let mut root: toml::Value = toml::from_str(&text)?;
            apply_gameplay_patch(&mut root, &synced);
            std::fs::write(pumpkin_path, toml::to_string_pretty(&root)?)?;
        }
    }
    Ok(())
}

fn format_proxy_block(secret: &str) -> String {
    format!(
        r#"
[networking.proxy]
enabled = true

[networking.proxy.velocity]
enabled = true
secret = "{secret}"
"#,
        secret = secret,
    )
}

fn replace_or_append_section(existing: &mut String, header: &str, block: &str) {
    if let Some(start) = existing.find(header) {
        if let Some(next) = existing[start + 1..].find("\n[") {
            let end = start + 1 + next;
            existing.replace_range(start..end, block.trim_end());
        } else {
            existing.replace_range(start.., block.trim_end());
        }
    } else if existing.trim().is_empty() {
        *existing = block.to_string();
    } else {
        existing.push('\n');
        existing.push_str(block);
    }
}

fn format_settings_patch(settings: &ServerSettings) -> String {
    format!(
        r#"[networking.java]
address = "{address}"
online_mode = {online_mode}
max-players = {max_players}
view-distance = {view_distance}
simulation-distance = {simulation_distance}
motd = {motd}
"#,
        address = settings.bind_address,
        online_mode = settings.online_mode,
        max_players = settings.max_players,
        view_distance = settings.view_distance,
        simulation_distance = settings.simulation_distance,
        motd = toml_string(&settings.motd),
    )
}

fn apply_gameplay_patch(root: &mut toml::Value, settings: &ServerSettings) {
    set_java_field(root, "max-players", settings.max_players as i64);
    set_java_field(root, "view-distance", settings.view_distance as i64);
    set_java_field(root, "simulation-distance", settings.simulation_distance as i64);
    set_java_field_str(root, "motd", &settings.motd);
    if let Some(table) = root.as_table_mut() {
        let commands = table
            .entry("commands")
            .or_insert(toml::Value::Table(toml::map::Map::new()));
        if let Some(ct) = commands.as_table_mut() {
            ct.insert("enabled".into(), toml::Value::Boolean(settings.commands_enabled));
        }
    }
}

fn set_java_field(root: &mut toml::Value, key: &str, value: i64) {
    if let Some(table) = root.as_table_mut() {
        let networking = table
            .entry("networking")
            .or_insert(toml::Value::Table(toml::map::Map::new()));
        if let Some(nt) = networking.as_table_mut() {
            let java = nt
                .entry("java")
                .or_insert(toml::Value::Table(toml::map::Map::new()));
            if let Some(jt) = java.as_table_mut() {
                jt.insert(key.into(), toml::Value::Integer(value));
            }
        }
    }
}

fn set_java_field_str(root: &mut toml::Value, key: &str, value: &str) {
    if let Some(table) = root.as_table_mut() {
        let networking = table
            .entry("networking")
            .or_insert(toml::Value::Table(toml::map::Map::new()));
        if let Some(nt) = networking.as_table_mut() {
            let java = nt
                .entry("java")
                .or_insert(toml::Value::Table(toml::map::Map::new()));
            if let Some(jt) = java.as_table_mut() {
                jt.insert(key.into(), toml::Value::String(value.to_string()));
            }
        }
    }
}

fn upsert_commands(existing: &str, enabled: bool) -> String {
    if let Some(start) = existing.find("[commands]") {
        let rest = &existing[start..];
        let end = rest.find("\n[").map(|i| start + i).unwrap_or(existing.len());
        let mut out = existing.to_string();
        out.replace_range(
            start..end,
            &format!("[commands]\nenabled = {}\n", enabled),
        );
        out
    } else {
        format!("{existing}\n[commands]\nenabled = {enabled}\n")
    }
}

fn merge_from_toml(settings: &mut ServerSettings, toml_text: &str) -> Result<()> {
    let value: toml::Value = toml::from_str(toml_text)?;
    let java = value
        .get("networking")
        .and_then(|n| n.get("java"))
        .or_else(|| {
            value
                .get("advanced")
                .and_then(|a| a.get("networking"))
                .and_then(|n| n.get("java"))
        });
    if let Some(java) = java {
        merge_java_table(settings, java);
    }
    if let Some(cmd) = value.get("commands") {
        if let Some(e) = cmd.get("enabled").and_then(|v| v.as_bool()) {
            settings.commands_enabled = e;
        }
    }
    Ok(())
}

fn merge_java_table(settings: &mut ServerSettings, java: &toml::Value) {
    if let Some(addr) = java.get("address").and_then(|v| v.as_str()) {
        settings.bind_address = addr.to_string();
    }
    if let Some(v) = java
        .get("online-mode")
        .or_else(|| java.get("online_mode"))
    {
        if let Some(b) = v.as_bool() {
            settings.online_mode = b;
        }
    }
    if let Some(v) = java.get("max-players").or_else(|| java.get("max_players")) {
        if let Some(n) = v.as_integer() {
            settings.max_players = n as u32;
        }
    }
    if let Some(v) = java.get("view-distance").or_else(|| java.get("view_distance")) {
        if let Some(n) = v.as_integer() {
            settings.view_distance = n as u8;
        }
    }
    if let Some(v) = java
        .get("simulation-distance")
        .or_else(|| java.get("simulation_distance"))
    {
        if let Some(n) = v.as_integer() {
            settings.simulation_distance = n as u8;
        }
    }
    if let Some(m) = java.get("motd").and_then(|v| v.as_str()) {
        settings.motd = m.to_string();
    }
}

fn toml_string(s: &str) -> String {
    if s.contains('"') || s.contains('\n') {
        format!("'''{}'''", s.replace("'''", "\\'''"))
    } else {
        format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

pub fn bind_address_for_port(port: u16) -> String {
    format!("127.0.0.1:{port}")
}

pub fn port_from_bind_address(addr: &str) -> Result<u16> {
    let parsed: SocketAddr = addr
        .parse()
        .map_err(|_| Error::Other(format!("invalid bind address: {addr}")))?;
    Ok(parsed.port())
}

pub fn instance_plugins_dir(instance_path: &Path) -> PathBuf {
    instance_path.join("plugins")
}
