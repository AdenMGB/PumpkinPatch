//! Plan DataExtension SQL persistence (indepth.md §4 extension tables subset).

use crate::error::Result;
use crate::plan_extension::for_each_extension;
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanExtensionColumn {
    pub key: String,
    pub label: String,
    pub plugin_name: String,
}

const EXTENSION_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS plan_extension_plugins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    network_id TEXT NOT NULL,
    plugin_name TEXT NOT NULL,
    icon TEXT NOT NULL DEFAULT '',
    UNIQUE (network_id, plugin_name)
);

CREATE TABLE IF NOT EXISTS plan_extension_providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id INTEGER NOT NULL,
    provider_name TEXT NOT NULL,
    value_type TEXT NOT NULL DEFAULT 'string',
    show_in_players_table INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (plugin_id) REFERENCES plan_extension_plugins(id) ON DELETE CASCADE,
    UNIQUE (plugin_id, provider_name)
);

CREATE TABLE IF NOT EXISTS plan_extension_user_values (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    provider_id INTEGER NOT NULL,
    boolean_value INTEGER,
    double_value REAL,
    string_value TEXT,
    group_value TEXT,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES plan_extension_providers(id) ON DELETE CASCADE,
    UNIQUE (user_id, provider_id)
);

CREATE TABLE IF NOT EXISTS plan_extension_server_values (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    server_id INTEGER NOT NULL,
    provider_id INTEGER NOT NULL,
    boolean_value INTEGER,
    double_value REAL,
    string_value TEXT,
    FOREIGN KEY (server_id) REFERENCES plan_servers(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES plan_extension_providers(id) ON DELETE CASCADE,
    UNIQUE (server_id, provider_id)
);

CREATE TABLE IF NOT EXISTS plan_extension_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    provider_id INTEGER NOT NULL,
    group_name TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES plan_users(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES plan_extension_providers(id) ON DELETE CASCADE,
    UNIQUE (user_id, provider_id, group_name)
);

CREATE TABLE IF NOT EXISTS plan_extension_tabs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    plugin_id INTEGER NOT NULL,
    tab_name TEXT NOT NULL,
    element_order INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (plugin_id) REFERENCES plan_extension_plugins(id) ON DELETE CASCADE,
    UNIQUE (plugin_id, tab_name)
);
";

pub async fn ensure_extension_sql_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::raw_sql(EXTENSION_SCHEMA).execute(pool).await?;
    Ok(())
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

async fn upsert_extension_tab(pool: &SqlitePool, plugin_id: i64, tab_name: &str) -> Result<()> {
    sqlx::query(
        "INSERT INTO plan_extension_tabs (plugin_id, tab_name, element_order) VALUES (?, ?, 0)
         ON CONFLICT(plugin_id, tab_name) DO NOTHING",
    )
    .bind(plugin_id)
    .bind(tab_name)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_plugin(
    pool: &SqlitePool,
    network_id: &str,
    plugin_name: &str,
    icon: &str,
) -> Result<i64> {
    sqlx::query(
        "INSERT INTO plan_extension_plugins (network_id, plugin_name, icon) VALUES (?, ?, ?)
         ON CONFLICT(network_id, plugin_name) DO UPDATE SET icon = excluded.icon",
    )
    .bind(network_id)
    .bind(plugin_name)
    .bind(icon)
    .execute(pool)
    .await?;
    let id: i64 = sqlx::query_scalar(
        "SELECT id FROM plan_extension_plugins WHERE network_id = ? AND plugin_name = ?",
    )
    .bind(network_id)
    .bind(plugin_name)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

async fn upsert_provider(
    pool: &SqlitePool,
    plugin_id: i64,
    provider_name: &str,
    value_type: &str,
    show_in_players_table: bool,
) -> Result<i64> {
    sqlx::query(
        "INSERT INTO plan_extension_providers (plugin_id, provider_name, value_type, show_in_players_table)
         VALUES (?, ?, ?, ?)
         ON CONFLICT(plugin_id, provider_name) DO UPDATE SET
           value_type = excluded.value_type,
           show_in_players_table = excluded.show_in_players_table",
    )
    .bind(plugin_id)
    .bind(provider_name)
    .bind(value_type)
    .bind(show_in_players_table as i32)
    .execute(pool)
    .await?;
    let id: i64 = sqlx::query_scalar(
        "SELECT id FROM plan_extension_providers WHERE plugin_id = ? AND provider_name = ?",
    )
    .bind(plugin_id)
    .bind(provider_name)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

async fn upsert_user_value(
    pool: &SqlitePool,
    user_id: i64,
    provider_id: i64,
    value: &Value,
) -> Result<()> {
    let (b, d, s, g) = split_value(value);
    sqlx::query(
        "INSERT INTO plan_extension_user_values
         (user_id, provider_id, boolean_value, double_value, string_value, group_value)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(user_id, provider_id) DO UPDATE SET
           boolean_value = excluded.boolean_value,
           double_value = excluded.double_value,
           string_value = excluded.string_value,
           group_value = excluded.group_value",
    )
    .bind(user_id)
    .bind(provider_id)
    .bind(b)
    .bind(d)
    .bind(s)
    .bind(g)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_server_value(
    pool: &SqlitePool,
    server_id: i64,
    provider_id: i64,
    value: &Value,
) -> Result<()> {
    let (b, d, s, _) = split_value(value);
    sqlx::query(
        "INSERT INTO plan_extension_server_values
         (server_id, provider_id, boolean_value, double_value, string_value)
         VALUES (?, ?, ?, ?, ?)
         ON CONFLICT(server_id, provider_id) DO UPDATE SET
           boolean_value = excluded.boolean_value,
           double_value = excluded.double_value,
           string_value = excluded.string_value",
    )
    .bind(server_id)
    .bind(provider_id)
    .bind(b)
    .bind(d)
    .bind(s)
    .execute(pool)
    .await?;
    Ok(())
}

fn split_value(value: &Value) -> (Option<i32>, Option<f64>, Option<String>, Option<String>) {
    if let Some(b) = value.as_bool() {
        return (Some(if b { 1 } else { 0 }), None, None, None);
    }
    if let Some(n) = value.as_f64() {
        return (None, Some(n), None, None);
    }
    if let Some(s) = value.as_str() {
        return (None, None, Some(s.to_string()), Some(s.to_string()));
    }
    if value.is_string() || value.is_object() {
        return (
            None,
            None,
            Some(value.to_string()),
            value.as_str().map(|s| s.to_string()),
        );
    }
    (None, None, Some(value.to_string()), None)
}

/// Mirrors in-process `PlanDataExtension` values into SQLite for QueryService SQL & exports.
pub async fn sync_registered_extensions_to_sql(
    pool: &SqlitePool,
    network_id: &str,
    server_id: i64,
) -> Result<u32> {
    let user_rows = sqlx::query_as::<_, (i64, String)>(
        "SELECT u.id, u.uuid FROM plan_users u
         JOIN plan_user_info ui ON ui.user_id = u.id
         WHERE ui.server_id = ?",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?;

    let plugin_jobs: Vec<(
        String,
        String,
        Vec<(String, Value, bool)>,
        Vec<(i64, String, Value, bool)>,
    )> = {
        let mut jobs = Vec::new();
        for_each_extension(|ext| {
            let icon = ext.plugin_icon().material.clone();
            let server_vals = ext
                .provide_server_data()
                .into_iter()
                .map(|v| (v.key, v.value, v.show_in_players_table))
                .collect();
            let mut player_vals = Vec::new();
            for (user_id, uuid) in &user_rows {
                for pv in ext.provide_player_data(uuid) {
                    player_vals.push((
                        *user_id,
                        pv.key,
                        pv.value,
                        pv.show_in_players_table,
                    ));
                }
            }
            jobs.push((ext.plugin_name().to_string(), icon, server_vals, player_vals));
        });
        jobs
    };

    let mut rows = 0u32;
    for (plugin_name, icon, server_vals, player_vals) in plugin_jobs {
        let plugin_id = upsert_plugin(pool, network_id, &plugin_name, &icon).await?;
        upsert_extension_tab(pool, plugin_id, "Overview").await?;
        rows += 1;
        for (key, value, show) in server_vals {
            let provider_id =
                upsert_provider(pool, plugin_id, &key, value_type_label(&value), show).await?;
            upsert_server_value(pool, server_id, provider_id, &value).await?;
            rows += 1;
        }
        for (user_id, key, value, show) in player_vals {
            let provider_id =
                upsert_provider(pool, plugin_id, &key, value_type_label(&value), show).await?;
            upsert_user_value(pool, user_id, provider_id, &value).await?;
            rows += 1;
        }
    }

    Ok(rows)
}

fn decode_extension_cell(
    boolean_value: Option<i32>,
    double_value: Option<f64>,
    string_value: Option<String>,
    group_value: Option<String>,
) -> Value {
    if let Some(b) = boolean_value {
        return json!(b != 0);
    }
    if let Some(d) = double_value {
        return json!(d);
    }
    if let Some(g) = group_value.filter(|s| !s.is_empty()) {
        return Value::String(g);
    }
    if let Some(s) = string_value {
        return Value::String(s);
    }
    Value::Null
}

/// Columns for `/v1/playersTable` (`show_in_players_table = 1`).
pub async fn extension_columns_for_table(
    pool: &SqlitePool,
    network_id: &str,
) -> Result<Vec<PlanExtensionColumn>> {
    let rows = sqlx::query(
        "SELECT pl.plugin_name, p.provider_name
         FROM plan_extension_providers p
         JOIN plan_extension_plugins pl ON pl.id = p.plugin_id
         WHERE pl.network_id = ? AND p.show_in_players_table = 1
         ORDER BY pl.plugin_name, p.provider_name",
    )
    .bind(network_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .iter()
        .map(|r| {
            let plugin_name: String = r.get("plugin_name");
            let provider_name: String = r.get("provider_name");
            PlanExtensionColumn {
                key: provider_name.clone(),
                label: provider_name,
                plugin_name,
            }
        })
        .collect())
}

/// Per-player nested extension map: `{ "PluginName": { "provider": value } }`.
pub async fn extension_values_by_uuid(
    pool: &SqlitePool,
    network_id: &str,
    uuids: &[String],
) -> Result<HashMap<String, Value>> {
    if uuids.is_empty() {
        return Ok(HashMap::new());
    }
    let placeholders = uuids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT u.uuid, pl.plugin_name, p.provider_name,
                uv.boolean_value, uv.double_value, uv.string_value, uv.group_value
         FROM plan_extension_user_values uv
         JOIN plan_users u ON u.id = uv.user_id
         JOIN plan_extension_providers p ON p.id = uv.provider_id
         JOIN plan_extension_plugins pl ON pl.id = p.plugin_id
         WHERE pl.network_id = ? AND u.uuid IN ({placeholders})"
    );
    let mut query = sqlx::query(&sql).bind(network_id);
    for uuid in uuids {
        query = query.bind(uuid);
    }
    let rows = query.fetch_all(pool).await?;
    let mut out: HashMap<String, serde_json::Map<String, Value>> = HashMap::new();
    for row in rows {
        let uuid: String = row.get("uuid");
        let plugin: String = row.get("plugin_name");
        let provider: String = row.get("provider_name");
        let value = decode_extension_cell(
            row.get("boolean_value"),
            row.get("double_value"),
            row.get("string_value"),
            row.get("group_value"),
        );
        let plugin_obj = out.entry(uuid).or_default();
        let nested = plugin_obj
            .entry(plugin)
            .or_insert_with(|| Value::Object(serde_json::Map::new()));
        if let Some(map) = nested.as_object_mut() {
            map.insert(provider, value);
        }
    }
    Ok(out
        .into_iter()
        .map(|(uuid, plugins)| (uuid, Value::Object(plugins)))
        .collect())
}

/// Re-sync all servers in a network (APIv5 `MANUAL_EXTENSION_REFRESH`).
pub async fn sync_all_servers_extensions(pool: &SqlitePool, network_id: &str) -> Result<u32> {
    let servers: Vec<(i64,)> = sqlx::query_as(
        "SELECT id FROM plan_servers WHERE network_id = ? AND is_installed = 1",
    )
    .bind(network_id)
    .fetch_all(pool)
    .await?;
    let mut total = 0u32;
    for (server_id,) in servers {
        total += sync_registered_extensions_to_sql(pool, network_id, server_id).await?;
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn sync_writes_harvest_metrics_provider_row() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO plan_servers (uuid, network_id, name) VALUES ('srv', 'net', 'S')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let server_id: i64 = sqlx::query_scalar("SELECT id FROM plan_servers LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO plan_users (uuid, registered, name) VALUES ('u1', 1, 'Steve')",
        )
        .execute(&pool)
        .await
        .unwrap();
        let user_id: i64 = sqlx::query_scalar("SELECT id FROM plan_users LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO plan_user_info (user_id, server_id, join_address, registered)
             VALUES (?, ?, '', 1)",
        )
        .bind(user_id)
        .bind(server_id)
        .execute(&pool)
        .await
        .unwrap();

        let n = sync_registered_extensions_to_sql(&pool, "net", server_id)
            .await
            .expect("sync");
        assert!(n > 0);

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM plan_extension_user_values uv
             JOIN plan_extension_providers p ON p.id = uv.provider_id
             JOIN plan_extension_plugins pl ON pl.id = p.plugin_id
             WHERE pl.plugin_name = 'HarvestMetrics' AND p.provider_name = 'profile_linked'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(count >= 1);
    }
}
