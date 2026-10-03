use crate::error::{Error, Result};
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
        Ok(serde_json::from_str(&data)?)
    }

    pub fn get(&self, id: &str) -> Option<&PluginManifestEntry> {
        self.plugins.iter().find(|p| p.id == id)
    }
}

pub fn deploy_plugin(
    plugins_root: &Path,
    entry: &PluginManifestEntry,
    instance_path: &Path,
    cache_dir: &Path,
) -> Result<PathBuf> {
    let source = plugins_root.join(&entry.artifact);
    if !source.exists() {
        return Err(Error::NotFound(format!(
            "plugin artifact missing: {}",
            source.display()
        )));
    }
    let plugins_dir = instance_path.join("plugins");
    std::fs::create_dir_all(&plugins_dir)?;
    let filename = source
        .file_name()
        .map(|f| f.to_owned())
        .unwrap_or_else(|| format!("{}.wasm", entry.id).into());
    let dest = plugins_dir.join(filename);
    std::fs::copy(&source, &dest)?;
    std::fs::create_dir_all(cache_dir)?;
    let cached = cache_dir.join(format!("{}-{}", entry.id, entry.version));
    std::fs::copy(&source, cached)?;
    Ok(dest)
}
