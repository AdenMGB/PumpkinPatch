use crate::error::{Error, Result};
use crate::plugins::{deploy_plugin, PluginManifest};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct PluginProfileEntry {
    pub bundled_id: String,
    pub targets: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PluginProfile {
    pub id: String,
    pub name: String,
    pub plugins: Vec<PluginProfileEntry>,
}

pub fn load_plugin_profile(profiles_root: &Path, profile_id: &str) -> Result<PluginProfile> {
    let path = profiles_root.join(format!("{profile_id}.json"));
    let raw = std::fs::read_to_string(&path)
        .map_err(|_| Error::NotFound(format!("profile {profile_id}")))?;
    serde_json::from_str(&raw).map_err(|e| Error::Other(e.to_string()))
}

pub fn apply_plugin_profile(
    plugins_root: &Path,
    plugins_cache: &Path,
    profile: &PluginProfile,
    instance_path: &Path,
    server_role: &str,
) -> Result<Vec<PathBuf>> {
    let manifest = PluginManifest::load_from_repo(plugins_root)?;
    let mut deployed = Vec::new();
    for entry in &profile.plugins {
        let manifest_entry = manifest
            .get(&entry.bundled_id)
            .ok_or_else(|| Error::NotFound(entry.bundled_id.clone()))?;
        if let Some(targets) = &entry.targets {
            if !targets.iter().any(|t| t == server_role) {
                continue;
            }
        } else if !manifest_entry.targets.iter().any(|t| t == server_role) {
            continue;
        }
        let dest = deploy_plugin(
            plugins_root,
            manifest_entry,
            instance_path,
            plugins_cache,
            Some(server_role),
        )?;
        deployed.push(dest);
    }
    Ok(deployed)
}
