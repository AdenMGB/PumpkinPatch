//! Patch Plan — Plan-inspired player analytics export format for Pumpkin Patch networks.
//!
//! Inspired by [Plan](https://github.com/plan-player-analytics/Plan) (LGPL-3). This is a
//! native Pumpkin WASM implementation, not a port of Plan's Java codebase.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Must match [`PluginMetadata::name`] in the PatchPlan Pumpkin plugin.
pub const PLUGIN_DATA_DIR_NAME: &str = "PatchPlan";
pub const CONFIG_FILENAME: &str = "patch-config.json";
pub const EXPORT_FILENAME: &str = "export.json";
pub const STATE_FILENAME: &str = "state.json";
pub const EXPORT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchPlanConfig {
    pub schema_version: u32,
    pub server_id: String,
    pub server_name: String,
    pub network_id: String,
    pub export_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerStats {
    pub uuid: String,
    pub name: String,
    pub first_seen_unix: i64,
    pub last_seen_unix: i64,
    pub join_count: u64,
    pub playtime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySample {
    /// Unix timestamp (seconds).
    pub t: i64,
    pub online: u32,
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
            schema_version: 1,
            server_id: "s".into(),
            server_name: "Lobby".into(),
            network_id: "n".into(),
            generated_at_unix: 1,
            online_now: 2,
            peak_online: 5,
            total_joins: 10,
            unique_players: 3,
            total_playtime_secs: 100,
            players: vec![],
            activity_samples: vec![],
        };
        let sig = sign_payload(&payload, "test-secret").unwrap();
        let export = SignedExport {
            payload,
            signature: sig,
        };
        verify_signed_export(&export, "test-secret").unwrap();
        assert!(verify_signed_export(&export, "wrong").is_err());
    }
}
