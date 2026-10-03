use super::Result;
use crate::bundled_plugins::provision_server_plugin;
use crate::state::AppStateHandle;
use patch_core::{
    list_installed_plugins, read_hub_settings, read_server_settings, remove_installed_plugin,
    write_hub_settings, write_server_settings, AddBackendRequest, HubSettings, HubSettingsPatch,
    InstalledPlugin, ServerRecord, ServerSettings, UpdateHubSettingsRequest,
    UpdateServerSettingsRequest,
};
use uuid::Uuid;

#[tauri::command]
pub async fn hub_get_settings(
    state: tauri::State<'_, AppStateHandle>,
    network_id: uuid::Uuid,
) -> Result<HubSettings> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let hub_path = std::path::Path::new(&network.data_path).join("hub");
    Ok(read_hub_settings(&hub_path)?)
}

#[tauri::command]
pub async fn hub_update_settings(
    state: tauri::State<'_, AppStateHandle>,
    network_id: uuid::Uuid,
    patch: UpdateHubSettingsRequest,
) -> Result<HubSettings> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let hub_path = std::path::Path::new(&network.data_path).join("hub");
    let hub_patch = HubSettingsPatch {
        bind: patch.bind,
        motd: patch.motd,
        online_mode: patch.online_mode,
        show_max_players: patch.show_max_players,
        player_info_forwarding: patch.player_info_forwarding,
    };
    Ok(write_hub_settings(&hub_path, &hub_patch)?)
}

#[tauri::command]
pub async fn server_get(
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
) -> Result<ServerRecord> {
    let inner = state.get().await?;
    inner.db.get_server(id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn server_get_settings(
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
) -> Result<ServerSettings> {
    let inner = state.get().await?;
    let server = inner.db.get_server(id).await?;
    let path = std::path::Path::new(&server.data_path);
    let network = inner.db.get_network(server.network_id).await?;
    patch_core::sync_server_instance_config(path, server.game_port, &network.forwarding_secret)?;
    Ok(read_server_settings(path, server.game_port)?)
}

#[tauri::command]
pub async fn server_update_settings(
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
    patch: UpdateServerSettingsRequest,
) -> Result<ServerSettings> {
    let inner = state.get().await?;
    let server = inner.db.get_server(id).await?;
    let network = inner.db.get_network(server.network_id).await?;
    let path = std::path::Path::new(&server.data_path);
    let mut settings = read_server_settings(path, server.game_port)?;
    if let Some(v) = patch.bind_address {
        settings.bind_address = v;
    }
    settings.bind_address = patch_core::bind_address_for_port(server.game_port);
    settings.online_mode = false;
    if let Some(v) = patch.online_mode {
        settings.online_mode = v;
    }
    if let Some(v) = patch.max_players {
        settings.max_players = v;
    }
    if let Some(v) = patch.view_distance {
        settings.view_distance = v;
    }
    if let Some(v) = patch.simulation_distance {
        settings.simulation_distance = v;
    }
    if let Some(v) = patch.motd {
        settings.motd = v;
    }
    if let Some(v) = patch.commands_enabled {
        settings.commands_enabled = v;
    }
    write_server_settings(path, server.game_port, &network.forwarding_secret, &settings)?;
    Ok(read_server_settings(path, server.game_port)?)
}

#[tauri::command]
pub async fn server_set_minecraft_version(
    state: tauri::State<'_, AppStateHandle>,
    id: Uuid,
    minecraft_version: String,
) -> Result<ServerRecord> {
    let inner = state.get().await?;
    inner
        .db
        .update_server_minecraft_version(id, &minecraft_version)
        .await?;
    inner.db.get_server(id).await.map_err(Into::into)
}

#[tauri::command]
pub async fn server_add_backend(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    request: AddBackendRequest,
) -> Result<ServerRecord> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let settings = inner.settings().await;
    let mc = request
        .minecraft_version
        .unwrap_or_else(|| "1.21.4".to_string());
    let backend = inner
        .networks
        .add_backend(&inner.db, &network, &settings, &request.name, &mc)
        .await?;
    provision_server_plugin(&state, &inner, &network, &backend).await;
    Ok(backend)
}

#[tauri::command]
pub async fn server_remove(
    state: tauri::State<'_, AppStateHandle>,
    network_id: Uuid,
    server_id: Uuid,
) -> Result<()> {
    let inner = state.get().await?;
    let network = inner.db.get_network(network_id).await?;
    let settings = inner.settings().await;
    inner
        .networks
        .remove_server(&inner.db, &network, &settings, server_id)
        .await?;
    Ok(())
}

#[tauri::command]
pub async fn server_list_plugins(
    state: tauri::State<'_, AppStateHandle>,
    server_id: Uuid,
) -> Result<Vec<InstalledPlugin>> {
    let inner = state.get().await?;
    let server = inner.db.get_server(server_id).await?;
    Ok(list_installed_plugins(std::path::Path::new(&server.data_path))?)
}

#[tauri::command]
pub async fn server_remove_plugin(
    state: tauri::State<'_, AppStateHandle>,
    server_id: Uuid,
    filename: String,
) -> Result<()> {
    let inner = state.get().await?;
    let server = inner.db.get_server(server_id).await?;
    remove_installed_plugin(std::path::Path::new(&server.data_path), &filename)?;
    Ok(())
}
