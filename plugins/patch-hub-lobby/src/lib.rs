//! Pumpkin Patch hub lobby — server list + `/join` helper (Velocity `/server`).

mod handlers;
mod state;

use handlers::{CommandHandler, JoinHandler};
use patch_network_protocol::ServerListPayload;
use pumpkin_plugin_api::events::EventPriority;
use pumpkin_plugin_api::permissions::{FS_READ_DATA, FS_WRITE_DATA};
use pumpkin_plugin_api::{Context, Plugin, PluginMetadata, Result};
use state::{load_server_list, SERVER_LIST_FILENAME, STATE};

pub const PLUGIN_DATA_DIR_NAME: &str = "patch-hub-lobby";

struct HubLobbyPlugin;

impl Plugin for HubLobbyPlugin {
    fn new() -> Self {
        HubLobbyPlugin
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: PLUGIN_DATA_DIR_NAME.into(),
            version: env!("CARGO_PKG_VERSION").into(),
            authors: vec!["Pumpkin Patch".into()],
            description: "Lobby hub server selector (Pumpkin Patch)".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<()> {
        let data_folder = context.get_data_folder();
        std::fs::create_dir_all(&data_folder).map_err(|e| e.to_string())?;
        let list_path = format!("{data_folder}/{SERVER_LIST_FILENAME}");
        let list: ServerListPayload = if std::path::Path::new(&list_path).exists() {
            load_server_list(&list_path)?
        } else {
            ServerListPayload { servers: vec![] }
        };
        let _ = STATE.set(std::sync::Mutex::new(state::RuntimeState {
            data_folder,
            list,
        }));

        context
            .register_event_handler(JoinHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        context
            .register_event_handler(CommandHandler, EventPriority::Normal, false)
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

pumpkin_plugin_api::register_plugin!(HubLobbyPlugin);
