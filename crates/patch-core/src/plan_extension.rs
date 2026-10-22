//! Plan `/v1/extensionData` envelope + in-process DataExtension registry (indepth.md §8).

use patch_plan_core::{ExtensionIcon, ExtensionValue, PlanDataExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExtensionDataV1 {
    pub extensions: Vec<PlanExtensionPlugin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExtensionPlugin {
    pub plugin_name: String,
    pub icon: String,
    pub last_updated: i64,
    pub tabs: Vec<PlanExtensionTab>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExtensionTab {
    pub tab_name: String,
    pub values: Vec<PlanExtensionValue>,
    pub tables: Vec<PlanExtensionTable>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExtensionValue {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanExtensionTable {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
}

struct Registry {
    extensions: Vec<Box<dyn PlanDataExtension>>,
}

fn registry() -> &'static RwLock<Registry> {
    static REG: OnceLock<RwLock<Registry>> = OnceLock::new();
    REG.get_or_init(|| {
        RwLock::new(Registry {
            extensions: Vec::new(),
        })
    })
}

/// Register a native analytics extension (desktop-side or tests).
pub fn register_plan_extension(ext: Box<dyn PlanDataExtension>) {
    registry()
        .write()
        .expect("extension registry lock")
        .extensions
        .push(ext);
}

/// Returns registered extension plugins for the server/network.
pub fn extension_data_v1() -> PlanExtensionDataV1 {
    let now = chrono::Utc::now().timestamp_millis();
    let reg = registry().read().expect("extension registry lock");
    let extensions = reg
        .extensions
        .iter()
        .map(|ext| {
            let icon = ext.plugin_icon();
            let server_values = ext.provide_server_data();
            PlanExtensionPlugin {
                plugin_name: ext.plugin_name().to_string(),
                icon: icon.material.clone(),
                last_updated: now,
                tabs: vec![PlanExtensionTab {
                    tab_name: "Overview".into(),
                    values: server_values
                        .into_iter()
                        .map(|v| PlanExtensionValue {
                            name: v.key,
                            value: v.value,
                        })
                        .collect(),
                    tables: Vec::new(),
                }],
            }
        })
        .collect();
    PlanExtensionDataV1 { extensions }
}

/// Built-in HarvestMetrics tab for the desktop app.
pub struct HarvestMetricsExtension;

impl PlanDataExtension for HarvestMetricsExtension {
    fn plugin_name(&self) -> &'static str {
        "HarvestMetrics"
    }

    fn plugin_icon(&self) -> ExtensionIcon {
        ExtensionIcon {
            material: "chart-bar".into(),
            url: None,
        }
    }

    fn provide_server_data(&self) -> Vec<ExtensionValue> {
        vec![ExtensionValue {
            key: "engine".into(),
            value: Value::String("DuckDB + Plan SQLite ingest".into()),
            show_in_players_table: false,
        }]
    }
}

pub fn ensure_builtin_extensions() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        register_plan_extension(Box::new(HarvestMetricsExtension));
    });
}
