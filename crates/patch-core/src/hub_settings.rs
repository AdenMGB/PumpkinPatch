use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSettings {
    pub bind: String,
    pub motd: String,
    pub online_mode: bool,
    pub show_max_players: u32,
    pub player_info_forwarding: String,
}

impl Default for HubSettings {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:25565".into(),
            motd: "Pumpkin Patch Network".into(),
            online_mode: true,
            show_max_players: 500,
            player_info_forwarding: "modern".into(),
        }
    }
}

pub fn read_hub_settings(hub_path: &Path) -> Result<HubSettings> {
    let file = hub_path.join("velocity.toml");
    if !file.exists() {
        return Ok(HubSettings::default());
    }
    let text = std::fs::read_to_string(&file)?;
    let value: toml::Value = toml::from_str(&text)?;
    let mut settings = HubSettings::default();
    if let Some(bind) = value.get("bind").and_then(|v| v.as_str()) {
        settings.bind = bind.to_string();
    }
    if let Some(motd) = value.get("motd").and_then(|v| v.as_str()) {
        settings.motd = motd.to_string();
    }
    if let Some(online) = value.get("online-mode").and_then(|v| v.as_bool()) {
        settings.online_mode = online;
    }
    if let Some(max) = value.get("show-max-players").and_then(|v| v.as_integer()) {
        settings.show_max_players = max as u32;
    }
    if let Some(mode) = value
        .get("forwarding")
        .and_then(|f| f.get("mode"))
        .and_then(|v| v.as_str())
    {
        settings.player_info_forwarding = mode.to_string();
    }
    Ok(settings)
}

pub fn write_hub_settings(hub_path: &Path, patch: &HubSettingsPatch) -> Result<HubSettings> {
    let file = hub_path.join("velocity.toml");
    if !file.exists() {
        return Err(Error::NotFound("velocity.toml".into()));
    }
    let text = std::fs::read_to_string(&file)?;
    let mut value: toml::Value = toml::from_str(&text)?;
    let table = value
        .as_table_mut()
        .ok_or_else(|| Error::Other("invalid velocity.toml".into()))?;

    if let Some(bind) = &patch.bind {
        table.insert("bind".into(), toml::Value::String(bind.clone()));
    }
    if let Some(motd) = &patch.motd {
        table.insert("motd".into(), toml::Value::String(motd.clone()));
    }
    if let Some(online) = patch.online_mode {
        table.insert("online-mode".into(), toml::Value::Boolean(online));
    }
    if let Some(max) = patch.show_max_players {
        table.insert(
            "show-max-players".into(),
            toml::Value::Integer(max as i64),
        );
    }
    if let Some(mode) = &patch.player_info_forwarding {
        let forwarding = table
            .entry("forwarding")
            .or_insert(toml::Value::Table(toml::map::Map::new()));
        if let Some(ft) = forwarding.as_table_mut() {
            ft.insert("mode".into(), toml::Value::String(mode.clone()));
        }
    }

    let out = toml::to_string_pretty(&value)?;
    std::fs::write(file, out)?;
    read_hub_settings(hub_path)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HubSettingsPatch {
    pub bind: Option<String>,
    pub motd: Option<String>,
    pub online_mode: Option<bool>,
    pub show_max_players: Option<u32>,
    pub player_info_forwarding: Option<String>,
}
