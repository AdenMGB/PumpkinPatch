//! Patch Plan — Plan-inspired player analytics export format for Pumpkin Patch networks.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

pub const PLUGIN_DATA_DIR_NAME: &str = "PatchPlan";
pub const CONFIG_FILENAME: &str = "patch-config.json";
pub const EXPORT_FILENAME: &str = "export.json";
pub const STATE_FILENAME: &str = "state.json";
pub const EXPORT_SCHEMA_VERSION: u32 = 2;

pub const MAX_RECENT_DEATHS: usize = 120;
pub const MAX_RECENT_EVENTS: usize = 400;
pub const MAX_RECENT_ADVANCEMENTS_PER_PLAYER: usize = 64;
pub const MAX_EXPORT_PLAYERS: usize = 512;
pub const MAX_ACTIVITY_SAMPLES: usize = 576;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchPlanConfig {
    pub schema_version: u32,
    pub server_id: String,
    pub server_name: String,
    pub network_id: String,
    pub export_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerStats {
    pub uuid: String,
    pub name: String,
    pub first_seen_unix: i64,
    pub last_seen_unix: i64,
    pub join_count: u64,
    pub login_count: u64,
    pub kick_count: u64,
    pub playtime_secs: u64,
    pub deaths: u64,
    pub mob_kills: u64,
    pub player_kills: u64,
    pub advancements: u64,
    pub recipes_discovered: u64,
    pub items_consumed: u64,
    pub items_broken: u64,
    pub blocks_harvested: u64,
    pub teleports: u64,
    pub world_changes: u64,
    pub chat_messages: u64,
    pub commands_used: u64,
    pub fish_caught: u64,
    pub portal_uses: u64,
    pub items_dropped: u64,
    pub beds_entered: u64,
    pub items_damaged: u64,
    pub xp_gained: i64,
    pub xp_level: i32,
    pub current_gamemode: String,
    pub last_world: String,
    #[serde(default)]
    pub statistic_totals: HashMap<String, u64>,
    #[serde(default)]
    pub recent_advancements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySample {
    pub t: i64,
    pub online: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServerTotals {
    pub total_logins: u64,
    pub total_kicks: u64,
    pub total_deaths: u64,
    pub total_advancements: u64,
    pub total_mob_kills: u64,
    pub total_player_kills: u64,
    pub total_recipes_discovered: u64,
    pub total_blocks_harvested: u64,
    pub total_chat_messages: u64,
    pub total_commands: u64,
    #[serde(default)]
    pub statistic_totals: HashMap<String, u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeathRecord {
    pub t: i64,
    pub player_uuid: String,
    pub player_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub t: i64,
    pub kind: String,
    pub player_uuid: String,
    pub player_name: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsPayload {
    pub schema_version: u32,
    pub server_id: String,
    pub server_name: String,
    pub network_id: String,
    pub generated_at_unix: i64,
    pub online_now: u32,
    pub peak_online: u32,
    pub total_joins: u64,
    pub unique_players: u64,
    pub total_playtime_secs: u64,
    #[serde(default)]
    pub server_totals: ServerTotals,
    #[serde(default)]
    pub recent_deaths: Vec<DeathRecord>,
    #[serde(default)]
    pub recent_events: Vec<AnalyticsEvent>,
    pub players: Vec<PlayerStats>,
    pub activity_samples: Vec<ActivitySample>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedExport {
    pub payload: AnalyticsPayload,
    pub signature: String,
}

pub fn export_data_dir(instance_path: &std::path::Path) -> std::path::PathBuf {
    instance_path
        .join("plugins")
        .join("data")
        .join(PLUGIN_DATA_DIR_NAME)
}

pub fn export_file_path(instance_path: &std::path::Path) -> std::path::PathBuf {
    export_data_dir(instance_path).join(EXPORT_FILENAME)
}

pub fn config_file_path(instance_path: &std::path::Path) -> std::path::PathBuf {
    export_data_dir(instance_path).join(CONFIG_FILENAME)
}

pub fn mirror_vanilla_stat_counters(player: &mut PlayerStats) {
    for (id, total) in &player.statistic_totals {
        let key = id.as_str();
        if key.contains("deaths") {
            player.deaths = player.deaths.max(*total);
        } else if key.contains("player_kills") {
            player.player_kills = player.player_kills.max(*total);
        } else if key.contains("mob_kills") {
            player.mob_kills = player.mob_kills.max(*total);
        }
    }
}

pub fn apply_stat_increment(player: &mut PlayerStats, statistic_id: &str, amount: i32) {
    if amount <= 0 {
        return;
    }
    let add = amount as u64;
    *player
        .statistic_totals
        .entry(statistic_id.to_string())
        .or_insert(0) += add;
    mirror_vanilla_stat_counters(player);
}

pub fn sign_payload(payload: &AnalyticsPayload, secret: &str) -> Result<String, String> {
    let canonical = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).map_err(|e| e.to_string())?;
    mac.update(canonical.as_bytes());
    Ok(hex_encode(mac.finalize().into_bytes().as_slice()))
}

pub fn verify_signed_export(export: &SignedExport, secret: &str) -> Result<(), String> {
    let expected = sign_payload(&export.payload, secret)?;
    if constant_time_eq(expected.as_bytes(), export.signature.as_bytes()) {
        Ok(())
    } else {
        Err("analytics export signature mismatch".into())
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let payload = AnalyticsPayload {
            schema_version: 2,
            server_id: "s".into(),
            server_name: "Lobby".into(),
            network_id: "n".into(),
            generated_at_unix: 1,
            online_now: 2,
            peak_online: 5,
            total_joins: 10,
            unique_players: 3,
            total_playtime_secs: 100,
            server_totals: ServerTotals::default(),
            recent_deaths: vec![],
            recent_events: vec![],
            players: vec![],
            activity_samples: vec![],
        };
        let sig = sign_payload(&payload, "test-secret").unwrap();
        let export = SignedExport {
            payload,
            signature: sig,
        };
        verify_signed_export(&export, "test-secret").unwrap();
    }

    #[test]
    fn stat_increment_updates_map() {
        let mut p = PlayerStats::default();
        apply_stat_increment(&mut p, "minecraft:custom:deaths", 1);
        assert_eq!(p.statistic_totals.get("minecraft:custom:deaths"), Some(&1));
    }
}
