use crate::error::{Error, Result};
use crate::plugin_install::is_plugin_artifact_filename;
use crate::server_settings::instance_plugins_dir;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
}

pub fn list_installed_plugins(instance_path: &Path) -> Result<Vec<InstalledPlugin>> {
    let dir = instance_plugins_dir(instance_path);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !is_plugin_artifact_filename(&name) {
            continue;
        }
        let size = entry.metadata()?.len();
        out.push(InstalledPlugin {
            filename: name,
            path: path.to_string_lossy().into(),
            size_bytes: size,
        });
    }
    out.sort_by(|a, b| a.filename.cmp(&b.filename));
    Ok(out)
}

pub fn remove_installed_plugin(instance_path: &Path, filename: &str) -> Result<()> {
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Err(Error::InvalidState("invalid plugin filename".into()));
    }
    let path = instance_plugins_dir(instance_path).join(filename);
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

