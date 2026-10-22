//! Plan public developer API surface (indepth.md §8 APIv5) — capability advertisement.

use serde_json::{json, Value};

/// JSON payload for `GET /v1/capabilities` (CapabilityService subset).
pub fn plan_capabilities_v1() -> Value {
    json!({
        "version": 5,
        "capabilities": [
            "DATA_EXTENSION_VALUES",
            "DATA_EXTENSION_TABLES",
            "QUERY_SERVICE_READ",
            "QUERY_SERVICE_WRITE",
            "PAGE_EXTENSIONS",
            "PLAYER_DATA_CLEAR_EVENTS",
            "MANUAL_EXTENSION_REFRESH"
        ],
        "services": {
            "CapabilityService": { "available": true },
            "ExtensionService": { "available": true, "call_events": [
                "PLAYER_JOIN",
                "PLAYER_LEAVE",
                "SERVER_EXTENSION_REGISTER",
                "SERVER_PERIODICAL",
                "MANUAL"
            ]},
            "QueryService": { "available": true, "write_enabled": true, "write_tables": [
                "plan_statistic_totals",
                "plan_user_statistic_totals",
                "plan_commandusages",
                "plan_extension_user_values",
                "plan_extension_server_values",
                "plan_extension_groups",
                "plan_extension_tables",
                "plan_extension_table_rows"
            ]},
            "ResolverService": { "available": true }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertises_data_extension_capability() {
        let v = plan_capabilities_v1();
        let caps = v["capabilities"].as_array().expect("capabilities array");
        assert!(caps.iter().any(|c| c == "DATA_EXTENSION_VALUES"));
        assert!(caps.iter().any(|c| c == "QUERY_SERVICE_READ"));
        assert_eq!(v["services"]["QueryService"]["write_enabled"], true);
    }
}
