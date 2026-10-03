use crate::error::{Error, Result};
use crate::plugin_install::{
    cache_bundled_plugin, copy_plugin_artifact, resolve_bundled_artifact,
};
use crate::server_settings::instance_plugins_dir;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifestEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub artifact: String,
    pub targets: Vec<String>,
    #[serde(default)]
    pub auto_deploy: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginManifest {
    pub plugins: Vec<PluginManifestEntry>,
}

impl PluginManifest {
    pub fn load_from_repo(plugins_root: &Path) -> Result<Self> {
        let path = plugins_root.join("manifest.json");
        if !path.exists() {
            return Ok(Self { plugins: vec![] });
        }
        let data = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&data)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn get(&self, id: &str) -> Option<&PluginManifestEntry> {
        self.plugins.iter().find(|p| p.id == id)
    }

    fn validate(&self) -> Result<()> {
        let mut seen = std::collections::HashSet::new();
        for entry in &self.plugins {
            if entry.id.trim().is_empty() {
                return Err(Error::InvalidState(
                    "plugin manifest entry missing id".into(),
                ));
            }
            if !seen.insert(entry.id.clone()) {
                return Err(Error::InvalidState(format!(
                    "duplicate plugin id in manifest: {}",
                    entry.id
                )));
            }
            if entry.artifact.trim().is_empty() {
                return Err(Error::InvalidState(format!(
                    "plugin {} missing artifact path",
                    entry.id
                )));
            }
        }
        Ok(())
    }
}

pub fn deploy_plugin(
    plugins_root: &Path,
    entry: &PluginManifestEntry,
    instance_path: &Path,
    cache_dir: &Path,
    target_role: Option<&str>,
) -> Result<PathBuf> {
    if let Some(role) = target_role {
        if !entry.targets.is_empty() && !entry.targets.iter().any(|t| t == role) {
            return Err(Error::InvalidState(format!(
                "plugin {} is not configured for role '{role}'",
                entry.id
            )));
        }
    }

    let source = resolve_bundled_artifact(plugins_root, &entry.artifact)?;
    if !source.exists() {
        return Err(Error::NotFound(format!(
            "plugin artifact missing: {}",
            source.display()
        )));
    }

    let filename = source
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| format!("{}.wasm", entry.id));

    let plugins_dir = instance_plugins_dir(instance_path);
    let dest = copy_plugin_artifact(&source, &plugins_dir, &filename)?;
    cache_bundled_plugin(cache_dir, &entry.id, &entry.version, &source)?;
    Ok(dest)
}

pub fn deploy_auto_plugins_for_server(
    plugins_root: &Path,
    manifest: &PluginManifest,
    server: &crate::models::ServerRecord,
    instance_path: &Path,
    cache_dir: &Path,
) -> Result<Vec<PathBuf>> {
    let mut deployed = Vec::new();
    for entry in &manifest.plugins {
        if !entry.auto_deploy {
            continue;
        }
        let dest = deploy_plugin(
            plugins_root,
            entry,
            instance_path,
            cache_dir,
            Some(server.role.as_str()),
        )?;
        deployed.push(dest);
    }
    Ok(deployed)
}
