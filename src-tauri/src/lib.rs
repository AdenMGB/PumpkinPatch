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
            api::network::network_logs_get,
            api::console::network_console_send,
            api::network::plugin_list_available,
            api::network::plugin_deploy,
            api::analytics::analytics_network_overview,
            api::analytics::analytics_plan_sync,
            api::analytics::analytics_install_plan_on_backends,
            api::analytics::analytics_plan_sessions,
            api::analytics::analytics_plan_players_v1,
            api::analytics::analytics_plan_players_table_v1,
            api::analytics::analytics_plan_sessions_v1,
            api::analytics::analytics_plan_commands,
            api::analytics::analytics_plan_players_table,
            api::analytics::analytics_plan_graph_players_online,
            api::analytics::analytics_plan_server_overview,
            api::analytics::analytics_plan_kills,
            api::analytics::analytics_plan_player_versus,
            api::analytics::analytics_plan_network_overview,
            api::analytics::analytics_plan_network_playerbase_overview,
            api::analytics::analytics_plan_network_sessions_overview,
            api::analytics::analytics_plan_network_ping_table,
            api::analytics::analytics_plan_player,
            api::analytics::analytics_plan_graph,
            api::analytics::analytics_plan_sessions_overview,
            api::analytics::analytics_plan_join_addresses,
            api::analytics::analytics_plan_filters,
            api::analytics::analytics_plan_query,
            api::analytics::analytics_plan_query_by_token,
            api::analytics::analytics_plan_online_overview,
            api::analytics::analytics_plan_playerbase_overview,
            api::analytics::analytics_plan_performance_overview,
            api::analytics::analytics_plan_retention,
            api::analytics::analytics_plan_ping_table,
            api::analytics::analytics_plan_extension_data,
            api::settings::settings_get,
            api::settings::settings_set,
            api::settings::settings_detect_java,
            api::ping::server_ping,
            api::server::hub_get_settings,
            api::server::hub_update_settings,
            api::server::server_get,
            api::server::server_get_settings,
            api::server::server_update_settings,
            api::server::server_set_minecraft_version,
            api::server::server_set_pumpkin_channel,
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
            api::orchestration::network_topology_get,
            api::orchestration::network_hot_wire_server_to_proxy,
            api::orchestration::analytics_plan_export_static,
            api::orchestration::analytics_harvest_sync,
            api::plan_http::plan_http_start,
            api::telemetry::subscribe_live_telemetry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
