//! Plan JSON theme payloads (`GET /v1/theme`, indepth.md §6).

use crate::error::{Error, Result};
use serde_json::{json, Value};
use sqlx::{Row, SqlitePool};

pub async fn plan_theme_json_db(pool: &SqlitePool, name: &str) -> Result<Value> {
    let row = sqlx::query("SELECT json FROM plan_custom_themes WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    if let Some(row) = row {
        let raw: String = row.get("json");
        return serde_json::from_str(&raw).map_err(Error::from);
    }
    Ok(plan_theme_json(name))
}

pub async fn save_plan_theme_json(
    pool: &SqlitePool,
    name: &str,
    theme: &Value,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    let json = serde_json::to_string(theme)?;
    sqlx::query(
        "INSERT INTO plan_custom_themes (name, json, updated_at_ms) VALUES (?, ?, ?)
         ON CONFLICT(name) DO UPDATE SET json = excluded.json, updated_at_ms = excluded.updated_at_ms",
    )
    .bind(name)
    .bind(json)
    .bind(now)
    .execute(pool)
    .await?;
    Ok(())
}

pub fn plan_theme_json(name: &str) -> Value {
    match name {
        "night" | "dracula" => json!({
            "name": name,
            "colors": {
                "primary": "#a78bfa",
                "bg": "#0f172a",
                "card": "#1e293b",
                "text": "#f8fafc"
            },
            "charts": {
                "palette": ["#a78bfa", "#38bdf8", "#34d399", "#fbbf24", "#f87171"]
            },
            "night_mode": true
        }),
        "cyberpunk" => json!({
            "name": "cyberpunk",
            "colors": {
                "primary": "#ff00ff",
                "bg": "#05010a",
                "card": "#120018",
                "text": "#e2e8f0"
            },
            "charts": {
                "palette": ["#ff00ff", "#00ffff", "#ffff00", "#ff6b6b", "#4ade80"]
            },
            "night_mode": true
        }),
        _ => json!({
            "name": "default",
            "colors": {
                "primary": "#e67e22",
                "bg": "#111827",
                "card": "#1f2937",
                "text": "#f9fafb"
            },
            "charts": {
                "palette": ["#e67e22", "#3498db", "#2ecc71", "#9b59b6", "#e74c3c"]
            },
            "night_mode": false
        }),
    }
}
