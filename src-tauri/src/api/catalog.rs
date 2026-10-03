use super::Result;
use patch_core::{
    fetch_minecraft_versions, ModrinthClient, ModrinthSearchResult, ModrinthVersion,
    MinecraftVersionInfo, PumpkinMarketClient, PumpkinMarketListResult, PumpkinMarketPluginDetail,
};

#[tauri::command]
pub async fn minecraft_list_versions() -> Result<Vec<MinecraftVersionInfo>> {
    Ok(fetch_minecraft_versions().await?)
}

#[tauri::command]
pub async fn modrinth_search_plugins(
    query: String,
    minecraft_version: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
) -> Result<ModrinthSearchResult> {
    let client = ModrinthClient::new();
    Ok(client
        .search_plugins(
            &query,
            minecraft_version.as_deref(),
            limit.unwrap_or(20),
            offset.unwrap_or(0),
        )
        .await?)
}

#[tauri::command]
pub async fn modrinth_project_versions(
    project_id: String,
    minecraft_version: Option<String>,
) -> Result<Vec<ModrinthVersion>> {
    let client = ModrinthClient::new();
    Ok(client
        .project_versions(&project_id, minecraft_version.as_deref())
        .await?)
}

#[tauri::command]
pub async fn modrinth_install_plugin(
    _project_id: String,
    version_id: String,
    instance_path: String,
) -> Result<String> {
    let client = ModrinthClient::new();
    let version = client.get_version(&version_id).await?;
    Ok(client
        .install_version_to_instance(&version, std::path::Path::new(&instance_path))
        .await?)
}

#[tauri::command]
pub async fn pumpkin_market_list_plugins(
    search: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<PumpkinMarketListResult> {
    let client = PumpkinMarketClient::new();
    Ok(client
        .list_plugins(
            search.as_deref(),
            limit.unwrap_or(12),
            cursor.as_deref(),
        )
        .await?)
}

#[tauri::command]
pub async fn pumpkin_market_get_plugin(plugin_id: u64) -> Result<PumpkinMarketPluginDetail> {
    let client = PumpkinMarketClient::new();
    Ok(client.get_plugin(plugin_id).await?)
}

#[tauri::command]
pub async fn pumpkin_market_install_plugin(
    plugin_id: u64,
    instance_path: String,
) -> Result<String> {
    let client = PumpkinMarketClient::new();
    Ok(client
        .install_plugin_to_instance(plugin_id, std::path::Path::new(&instance_path))
        .await?)
}
