use patch_core::discover_plugins_root;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri::path::BaseDirectory;

pub fn resolve(app: &AppHandle) -> PathBuf {
    let mut candidates = Vec::new();
    if let Ok(resource) = app.path().resolve("plugins", BaseDirectory::Resource) {
        candidates.push(resource);
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("plugins"),
    );
    discover_plugins_root(&candidates).unwrap_or_else(|| {
        candidates
            .into_iter()
            .find(|p| p.join("manifest.json").exists())
            .unwrap_or_else(dev_plugins_root)
    })
}

fn dev_plugins_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("plugins")
}
