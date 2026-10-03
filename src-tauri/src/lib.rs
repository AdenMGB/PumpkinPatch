mod api;
mod bundled_plugins;
mod plugins_root;
mod state;

use state::AppStateHandle;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_window_state::Builder::new().build())
        .manage(AppStateHandle::new())
        .setup(|app| {
            let handle = app.state::<AppStateHandle>();
            handle.set_plugins_root(plugins_root::resolve(app.handle()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::network::initialize_state,
            api::network::network_list,
            api::network::network_get,
            api::network::network_create,
            api::network::network_delete,
            api::network::network_start,
            api::network::network_stop,
            api::network::network_get_join_info,
            api::network::network_ping_hub,
            api::health::network_health_get,
            api::health::network_status_page,
            api::backup::network_export_backup,
            api::backup::network_export_backup_default,
            api::backup::network_import_backup,
            api::ops::network_proposed_ports,
            api::ops::network_set_auto_restart,
            api::ops::network_set_favorite,
            api::ops::network_export_definition,
            api::ops::java_validate,
            api::ops::network_update_binaries,
            api::ops::cache_clear,
            api::ops::plugin_apply_profile,
            api::templates::network_templates_list,
            api::templates::network_create_from_template,
            api::future::future_features_status,
            api::future::future_vine_create_network,
            api::console::network_console_send,
            api::network::plugin_list_available,
            api::network::plugin_deploy,
            api::analytics::analytics_network_overview,
            api::settings::settings_get,
            api::settings::settings_set,
            api::ping::server_ping,
            api::server::hub_get_settings,
            api::server::hub_update_settings,
            api::server::server_get,
            api::server::server_get_settings,
            api::server::server_update_settings,
            api::server::server_set_minecraft_version,
            api::server::server_add_backend,
            api::server::server_remove,
            api::server::server_list_plugins,
            api::server::server_remove_plugin,
            api::catalog::minecraft_list_versions,
            api::catalog::modrinth_search_plugins,
            api::catalog::modrinth_project_versions,
            api::catalog::modrinth_install_plugin,
            api::catalog::pumpkin_market_list_plugins,
            api::catalog::pumpkin_market_get_plugin,
            api::catalog::pumpkin_market_install_plugin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
