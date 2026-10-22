//! Plan `/v1/extensionData` envelope (§8 stub until DataExtension trait lands).

use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// Returns registered extension plugins for the server/network (empty until extensions register).
pub fn extension_data_v1() -> PlanExtensionDataV1 {
    PlanExtensionDataV1 {
        extensions: Vec::new(),
    }
}
