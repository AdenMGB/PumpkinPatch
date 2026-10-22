//! Plan JSON theme payloads (`GET /v1/theme`, indepth.md §6).

use serde_json::{json, Value};

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
