//! Plan DataExtension trait surface (indepth.md §8) for future Pumpkin plugins.

use serde_json::Value;

/// Icon metadata for extension tabs in the Plan UI.
#[derive(Debug, Clone)]
pub struct ExtensionIcon {
    pub material: String,
    pub url: Option<String>,
}

/// A single scalar extension value shown on player or server pages.
#[derive(Debug, Clone)]
pub struct ExtensionValue {
    pub key: String,
    pub value: Value,
    pub show_in_players_table: bool,
}

/// Plugin-provided analytics extension (tabs, tables, player/server values).
pub trait PlanDataExtension: Send + Sync {
    fn plugin_name(&self) -> &'static str;

    fn plugin_icon(&self) -> ExtensionIcon;

    fn provide_player_data(&self, _player_uuid: &str) -> Vec<ExtensionValue> {
        Vec::new()
    }

    fn provide_server_data(&self) -> Vec<ExtensionValue> {
        Vec::new()
    }
}
