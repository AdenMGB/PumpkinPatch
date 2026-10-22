//! Plan `/v1/extensionData` envelope + in-process DataExtension registry (indepth.md §8).

use crate::plan_store::PlanPlayerTableRow;
use patch_plan_core::{ExtensionIcon, ExtensionValue, PlanDataExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginBooleanGroupFilter {
    #[serde(default)]
    pub plugin: String,
    #[serde(default, alias = "providerName")]
    pub provider: String,
    pub value: Option<bool>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginNumberFilter {
    #[serde(default)]
    pub plugin: String,
    #[serde(default, alias = "providerName")]
    pub provider: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

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

/// Invokes `f` for each registered native extension (SQL sync, exports).
pub fn for_each_extension(mut f: impl FnMut(&dyn PlanDataExtension)) {
    ensure_builtin_extensions();
    let reg = registry().read().expect("extension registry lock");
    for ext in &reg.extensions {
        f(ext.as_ref());
    }
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

    fn provide_player_data(&self, player_uuid: &str) -> Vec<ExtensionValue> {
        vec![ExtensionValue {
            key: "profile_linked".into(),
            value: Value::Bool(!player_uuid.is_empty()),
            show_in_players_table: true,
        }]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionQueryProvider {
    pub plugin: String,
    pub provider: String,
    pub value_type: String,
}

fn value_type_label(value: &Value) -> &'static str {
    if value.is_boolean() {
        "boolean"
    } else if value.is_number() {
        "number"
    } else {
        "string"
    }
}

/// Providers exposed to Plan `/query` extension filter UI (`pluginsBooleanGroups` / `pluginNumbers`).
pub fn extension_filter_provider_catalog() -> Vec<ExtensionQueryProvider> {
    ensure_builtin_extensions();
    let reg = registry().read().expect("extension registry lock");
    let mut out = Vec::new();
    for ext in &reg.extensions {
        let plugin = ext.plugin_name().to_string();
        for v in ext.provide_server_data() {
            out.push(ExtensionQueryProvider {
                plugin: plugin.clone(),
                provider: v.key.clone(),
                value_type: value_type_label(&v.value).to_string(),
            });
        }
        for v in ext.provide_player_data("00000000-0000-0000-0000-000000000001") {
            out.push(ExtensionQueryProvider {
                plugin: plugin.clone(),
                provider: v.key.clone(),
                value_type: value_type_label(&v.value).to_string(),
            });
        }
    }
    out
}

pub fn ensure_builtin_extensions() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        register_plan_extension(Box::new(HarvestMetricsExtension));
    });
}

/// Applies Plan `/query` DataExtension boolean/group and numeric filters (indepth.md §5.3).
pub fn filter_players_by_extension_query(
    players: &mut Vec<PlanPlayerTableRow>,
    boolean_groups: &[PluginBooleanGroupFilter],
    numbers: &[PluginNumberFilter],
) {
    if boolean_groups.is_empty() && numbers.is_empty() {
        return;
    }
    ensure_builtin_extensions();
    let reg = registry().read().expect("extension registry lock");
    let extensions: Vec<&dyn PlanDataExtension> = reg
        .extensions
        .iter()
        .map(|b| b.as_ref())
        .collect();
    players.retain(|p| {
        player_matches_extension_filters(&p.uuid, boolean_groups, numbers, &extensions)
    });
}

fn player_matches_extension_filters(
    player_uuid: &str,
    boolean_groups: &[PluginBooleanGroupFilter],
    numbers: &[PluginNumberFilter],
    extensions: &[&dyn PlanDataExtension],
) -> bool {
    for filter in boolean_groups {
        if !boolean_group_filter_matches(player_uuid, filter, extensions) {
            return false;
        }
    }
    for filter in numbers {
        if !number_filter_matches(player_uuid, filter, extensions) {
            return false;
        }
    }
    true
}

fn boolean_group_filter_matches(
    player_uuid: &str,
    filter: &PluginBooleanGroupFilter,
    extensions: &[&dyn PlanDataExtension],
) -> bool {
    let plugin = filter.plugin.to_ascii_lowercase();
    let provider = filter.provider.to_ascii_lowercase();
    for ext in extensions {
        if ext.plugin_name().to_ascii_lowercase() != plugin {
            continue;
        }
        for value in ext.provide_player_data(player_uuid) {
            if value.key.to_ascii_lowercase() != provider {
                continue;
            }
            if let Some(expected) = filter.value {
                return value.value.as_bool() == Some(expected);
            }
            if let Some(group) = &filter.group {
                return value
                    .value
                    .as_str()
                    .map(|s| s.eq_ignore_ascii_case(group))
                    .unwrap_or(false);
            }
            return value.value.as_bool().unwrap_or(false);
        }
    }
    false
}

fn number_filter_matches(
    player_uuid: &str,
    filter: &PluginNumberFilter,
    extensions: &[&dyn PlanDataExtension],
) -> bool {
    let plugin = filter.plugin.to_ascii_lowercase();
    let provider = filter.provider.to_ascii_lowercase();
    for ext in extensions {
        if ext.plugin_name().to_ascii_lowercase() != plugin {
            continue;
        }
        for value in ext.provide_player_data(player_uuid) {
            if value.key.to_ascii_lowercase() != provider {
                continue;
            }
            let n = value
                .value
                .as_f64()
                .or_else(|| value.value.as_i64().map(|i| i as f64))
                .or_else(|| value.value.as_u64().map(|u| u as f64));
            let Some(n) = n else {
                return false;
            };
            if let Some(min) = filter.min {
                if n < min {
                    return false;
                }
            }
            if let Some(max) = filter.max {
                if n > max {
                    return false;
                }
            }
            return true;
        }
    }
    false
}

#[cfg(test)]
mod extension_query_tests {
    use super::*;
    use patch_plan_core::{ExtensionIcon, ExtensionValue};

    struct NumericTestExtension;

    impl PlanDataExtension for NumericTestExtension {
        fn plugin_name(&self) -> &'static str {
            "TestPlugin"
        }

        fn plugin_icon(&self) -> ExtensionIcon {
            ExtensionIcon {
                material: "test".into(),
                url: None,
            }
        }

        fn provide_player_data(&self, _player_uuid: &str) -> Vec<ExtensionValue> {
            vec![ExtensionValue {
                key: "balance".into(),
                value: serde_json::json!(42.0),
                show_in_players_table: true,
            }]
        }
    }

    #[test]
    fn boolean_extension_filter_matches_harvest_profile_linked() {
        let ext: &dyn PlanDataExtension = &HarvestMetricsExtension;
        let filter = PluginBooleanGroupFilter {
            plugin: "HarvestMetrics".into(),
            provider: "profile_linked".into(),
            value: Some(true),
            group: None,
        };
        assert!(boolean_group_filter_matches("uuid-1", &filter, &[ext]));
        assert!(!boolean_group_filter_matches("", &filter, &[ext]));
    }

    #[test]
    fn catalog_lists_harvest_metrics_providers() {
        let providers = extension_filter_provider_catalog();
        assert!(
            providers
                .iter()
                .any(|p| p.plugin == "HarvestMetrics" && p.provider == "profile_linked")
        );
    }

    #[test]
    fn number_extension_filter_respects_min_max() {
        let ext: &dyn PlanDataExtension = &NumericTestExtension;
        let ok = PluginNumberFilter {
            plugin: "TestPlugin".into(),
            provider: "balance".into(),
            min: Some(40.0),
            max: Some(50.0),
        };
        let bad = PluginNumberFilter {
            plugin: "TestPlugin".into(),
            provider: "balance".into(),
            min: Some(100.0),
            max: None,
        };
        assert!(number_filter_matches("p", &ok, &[ext]));
        assert!(!number_filter_matches("p", &bad, &[ext]));
    }
}
