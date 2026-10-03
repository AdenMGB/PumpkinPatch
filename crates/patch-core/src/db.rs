use crate::error::{Error, Result};
use crate::models::{AppSettings, NetworkRecord, ServerRecord};
use chrono::Utc;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::Path;
use std::str::FromStr;
use uuid::Uuid;

const MIGRATION: &str = "
CREATE TABLE IF NOT EXISTS networks (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    hub_type TEXT NOT NULL,
    hub_port INTEGER NOT NULL,
    forwarding_secret TEXT NOT NULL,
    lobby_server_id TEXT,
    data_path TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS servers (
    id TEXT PRIMARY KEY NOT NULL,
    network_id TEXT NOT NULL,
    role TEXT NOT NULL,
    name TEXT NOT NULL,
    velocity_name TEXT NOT NULL,
    game_port INTEGER NOT NULL,
    data_path TEXT NOT NULL,
    pumpkin_channel TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY(network_id) REFERENCES networks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS deployed_plugins (
    id TEXT PRIMARY KEY NOT NULL,
    plugin_id TEXT NOT NULL,
    instance_path TEXT NOT NULL,
    artifact_path TEXT NOT NULL,
    deployed_at TEXT NOT NULL
);
";

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let opts = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(opts)
            .await?;
        sqlx::raw_sql(MIGRATION).execute(&pool).await?;
        let _ = sqlx::query(
            "ALTER TABLE servers ADD COLUMN minecraft_version TEXT NOT NULL DEFAULT '1.21.4'",
        )
        .execute(&pool)
        .await;
        let _ = sqlx::query(
            "ALTER TABLE networks ADD COLUMN analytics_export_token TEXT NOT NULL DEFAULT ''",
        )
        .execute(&pool)
        .await;
        Ok(Self { pool })
    }

    pub async fn insert_network(&self, network: &NetworkRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO networks (id, name, hub_type, hub_port, forwarding_secret, lobby_server_id, data_path, analytics_export_token, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(network.id.to_string())
        .bind(&network.name)
        .bind(&network.hub_type)
        .bind(network.hub_port)
        .bind(&network.forwarding_secret)
        .bind(network.lobby_server_id.map(|u| u.to_string()))
        .bind(&network.data_path)
        .bind(&network.analytics_export_token)
        .bind(network.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_server(&self, server: &ServerRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO servers (id, network_id, role, name, velocity_name, game_port, data_path, pumpkin_channel, minecraft_version, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(server.id.to_string())
        .bind(server.network_id.to_string())
        .bind(&server.role)
        .bind(&server.name)
        .bind(&server.velocity_name)
        .bind(server.game_port)
        .bind(&server.data_path)
        .bind(&server.pumpkin_channel)
        .bind(&server.minecraft_version)
        .bind(server.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_networks(&self) -> Result<Vec<NetworkRecord>> {
        let rows = sqlx::query("SELECT * FROM networks ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(|row| network_from_row(row)).collect()
    }

    pub async fn get_network(&self, id: Uuid) -> Result<NetworkRecord> {
        let row = sqlx::query("SELECT * FROM networks WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref().map(network_from_row)
            .transpose()?
            .ok_or_else(|| Error::NotFound(format!("network {id}")))
    }

    pub async fn delete_network(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM servers WHERE network_id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM networks WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_server(&self, id: Uuid) -> Result<ServerRecord> {
        let row = sqlx::query("SELECT * FROM servers WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.as_ref()
            .map(server_from_row)
            .transpose()?
            .ok_or_else(|| Error::NotFound(format!("server {id}")))
    }

    pub async fn delete_server(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM servers WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_server_minecraft_version(
        &self,
        id: Uuid,
        minecraft_version: &str,
    ) -> Result<()> {
        sqlx::query("UPDATE servers SET minecraft_version = ? WHERE id = ?")
            .bind(minecraft_version)
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_servers_for_network(&self, network_id: Uuid) -> Result<Vec<ServerRecord>> {
        let rows = sqlx::query("SELECT * FROM servers WHERE network_id = ? ORDER BY role, name")
            .bind(network_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(server_from_row).collect()
    }

    pub async fn get_settings(&self) -> Result<AppSettings> {
        let rows = sqlx::query("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await?;
        let mut settings = AppSettings::default();
        for row in rows {
            let key: String = row.get("key");
            let value: String = row.get("value");
            match key.as_str() {
                "java_path" => settings.java_path = value,
                "bind_host" => settings.bind_host = value,
                "pumpkin_channel_default" => settings.pumpkin_channel_default = value,
                _ => {}
            }
        }
        Ok(settings)
    }

    pub async fn ensure_analytics_export_token(&self, network_id: Uuid) -> Result<String> {
        let network = self.get_network(network_id).await?;
        if !network.analytics_export_token.is_empty() {
            return Ok(network.analytics_export_token);
        }
        let token = crate::analytics::generate_export_secret();
        sqlx::query("UPDATE networks SET analytics_export_token = ? WHERE id = ?")
            .bind(&token)
            .bind(network_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(token)
    }

    pub async fn save_settings(&self, settings: &AppSettings) -> Result<()> {
        for (key, value) in [
            ("java_path", settings.java_path.as_str()),
            ("bind_host", settings.bind_host.as_str()),
            (
                "pumpkin_channel_default",
                settings.pumpkin_channel_default.as_str(),
            ),
        ] {
            sqlx::query("INSERT INTO settings (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value")
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }
}

fn network_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<NetworkRecord> {
    let id = Uuid::from_str(row.get::<String, _>("id").as_str())
        .map_err(|e| Error::Other(e.to_string()))?;
    let lobby: Option<String> = row.get("lobby_server_id");
    let lobby_server_id = lobby
        .filter(|s| !s.is_empty())
        .map(|s| Uuid::from_str(&s))
        .transpose()
        .map_err(|e| Error::Other(e.to_string()))?;
    let created_at = chrono::DateTime::parse_from_rfc3339(row.get::<String, _>("created_at").as_str())
        .map_err(|e| Error::Other(e.to_string()))?
        .with_timezone(&Utc);
    let analytics_export_token: String = row
        .try_get("analytics_export_token")
        .unwrap_or_default();
    Ok(NetworkRecord {
        id,
        name: row.get("name"),
        hub_type: row.get("hub_type"),
        hub_port: row.get::<i64, _>("hub_port") as u16,
        forwarding_secret: row.get("forwarding_secret"),
        lobby_server_id,
        data_path: row.get("data_path"),
        analytics_export_token,
        created_at,
    })
}

fn server_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<ServerRecord> {
    let id = Uuid::from_str(row.get::<String, _>("id").as_str())
        .map_err(|e| Error::Other(e.to_string()))?;
    let network_id = Uuid::from_str(row.get::<String, _>("network_id").as_str())
        .map_err(|e| Error::Other(e.to_string()))?;
    let created_at = chrono::DateTime::parse_from_rfc3339(row.get::<String, _>("created_at").as_str())
        .map_err(|e| Error::Other(e.to_string()))?
        .with_timezone(&Utc);
    Ok(ServerRecord {
        id,
        network_id,
        role: row.get("role"),
        name: row.get("name"),
        velocity_name: row.get("velocity_name"),
        game_port: row.get::<i64, _>("game_port") as u16,
        data_path: row.get("data_path"),
        pumpkin_channel: row.get("pumpkin_channel"),
        minecraft_version: row
            .try_get::<String, _>("minecraft_version")
            .unwrap_or_else(|_| "1.21.4".into()),
        created_at,
    })
}
