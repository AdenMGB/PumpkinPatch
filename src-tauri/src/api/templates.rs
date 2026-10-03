use super::Result;
use crate::state::AppStateHandle;
use patch_core::CreateNetworkRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTemplate {
    pub id: String,
    pub name: String,
    pub description: String,
    pub backend_names: Vec<String>,
    pub pumpkin_channel: Option<String>,
    pub minecraft_version: Option<String>,
}

fn bundled_templates() -> Vec<NetworkTemplate> {
    vec![
        NetworkTemplate {
            id: "survival".into(),
            name: "Survival network".into(),
            description: "Lobby plus survival and creative backends".into(),
            backend_names: vec!["Survival".into(), "Creative".into()],
            pumpkin_channel: Some("nightly".into()),
            minecraft_version: Some("1.21.4".into()),
        },
        NetworkTemplate {
            id: "minigames".into(),
            name: "Minigames hub".into(),
            description: "Lobby with two minigame backends".into(),
            backend_names: vec!["SkyWars".into(), "BedWars".into()],
            pumpkin_channel: Some("nightly".into()),
            minecraft_version: Some("1.21.4".into()),
        },
    ]
}

#[tauri::command]
pub fn network_templates_list() -> Vec<NetworkTemplate> {
    bundled_templates()
}

#[tauri::command]
pub async fn network_create_from_template(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppStateHandle>,
    template_id: String,
    name: String,
    hub_port: Option<u16>,
) -> Result<patch_core::NetworkSummary> {
    let template = bundled_templates()
        .into_iter()
        .find(|t| t.id == template_id)
        .ok_or_else(|| patch_core::Error::NotFound(template_id))?;
    let request = CreateNetworkRequest {
        name,
        hub_port,
        hub_type: Some(patch_core::HubType::Velocity),
        backend_names: template.backend_names,
        pumpkin_channel: template.pumpkin_channel,
        minecraft_version: template.minecraft_version,
    };
    crate::api::network::network_create(app, state, request).await
}
