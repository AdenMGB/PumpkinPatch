use super::Result;
use patch_plugin_resolve::{PluginPackage, resolve_install_set};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ResolvePluginsRequest {
    pub root: String,
    pub root_version: String,
    pub packages: Vec<PluginPackage>,
}

#[tauri::command]
pub async fn plugins_resolve_install_order(
    request: ResolvePluginsRequest,
) -> Result<Vec<String>> {
    resolve_install_set(&request.root, &request.root_version, &request.packages).map_err(|e| {
        patch_core::Error::InvalidState(e.to_string()).into()
    })
}
