//! Plan web RBAC + session cookies (indepth.md §4.2, §5.2).

use crate::error::{Error, Result};
use sqlx::{Row, SqlitePool};

const DEFAULT_PERMISSIONS: &[&str] = &[
    "page.network",
    "page.server",
    "page.player.self",
    "page.player.other",
    "access.query",
    "manage.groups",
    "access.theme.editor",
    "manage.themes",
];

pub async fn ensure_plan_web_security(pool: &SqlitePool) -> Result<()> {
    for perm in DEFAULT_PERMISSIONS {
        sqlx::query("INSERT OR IGNORE INTO plan_web_permission (permission) VALUES (?)")
            .bind(perm)
            .execute(pool)
            .await?;
    }
    sqlx::query("INSERT OR IGNORE INTO plan_web_group (group_name) VALUES ('admin')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO plan_web_group (group_name) VALUES ('staff')")
        .execute(pool)
        .await?;
    sqlx::query("INSERT OR IGNORE INTO plan_web_group (group_name) VALUES ('player')")
        .execute(pool)
        .await?;

    let admin_id: i64 = sqlx::query_scalar("SELECT id FROM plan_web_group WHERE group_name = 'admin'")
        .fetch_one(pool)
        .await?;
    for perm in DEFAULT_PERMISSIONS {
        let pid: i64 = sqlx::query_scalar("SELECT id FROM plan_web_permission WHERE permission = ?")
            .bind(perm)
            .fetch_one(pool)
            .await?;
        sqlx::query(
            "INSERT OR IGNORE INTO plan_web_group_to_permission (group_id, permission_id) VALUES (?, ?)",
        )
        .bind(admin_id)
        .bind(pid)
        .execute(pool)
        .await?;
    }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM plan_security")
        .fetch_one(pool)
        .await?;
    if count == 0 {
        let hash = hash_password("admin")?;
        sqlx::query(
            "INSERT INTO plan_security (username, salted_pass_hash, linked_to_uuid, group_id) VALUES (?, ?, '', ?)",
        )
        .bind("admin")
        .bind(hash)
        .bind(admin_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub fn hash_password(password: &str) -> Result<String> {
    bcrypt::hash(password, bcrypt::DEFAULT_COST)
        .map_err(|e| crate::error::Error::Other(format!("password hash failed: {e}")))
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    if stored.starts_with("bcrypt-stub:") {
        return legacy_stub_verify(password, stored);
    }
    bcrypt::verify(password, stored).unwrap_or(false)
}

fn legacy_stub_verify(password: &str, stored: &str) -> bool {
    let expected = format!("bcrypt-stub:{:x}", stable_hash(password));
    expected == stored
}

fn stable_hash(password: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    password.hash(&mut hasher);
    hasher.finish()
}

pub async fn login(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> Result<Option<String>> {
    let row = sqlx::query(
        "SELECT salted_pass_hash FROM plan_security WHERE username = ? COLLATE NOCASE",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let hash: String = row.get("salted_pass_hash");
    if !verify_password(password, &hash) {
        return Ok(None);
    }
    let cookie = new_session_cookie();
    let expires = chrono::Utc::now().timestamp_millis() + 7 * 24 * 60 * 60 * 1000;
    sqlx::query(
        "INSERT INTO plan_cookies (web_username, expires, cookie) VALUES (?, ?, ?)",
    )
    .bind(username)
    .bind(expires)
    .bind(&cookie)
    .execute(pool)
    .await?;
    Ok(Some(cookie))
}

pub async fn validate_cookie(pool: &SqlitePool, cookie: &str) -> Result<bool> {
    let now = chrono::Utc::now().timestamp_millis();
    let row = sqlx::query(
        "SELECT expires FROM plan_cookies WHERE cookie = ? AND expires > ?",
    )
    .bind(cookie)
    .bind(now)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

pub async fn log_access(
    pool: &SqlitePool,
    from_ip: Option<&str>,
    method: &str,
    uri: &str,
    code: i32,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp_millis();
    sqlx::query(
        "INSERT INTO plan_access_log (time, from_ip, request_method, request_uri, response_code) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(now)
    .bind(from_ip)
    .bind(method)
    .bind(uri)
    .bind(code)
    .execute(pool)
    .await?;
    Ok(())
}

fn new_session_cookie() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RegistrationStart {
    pub code: String,
    pub expires_ms: i64,
}

/// Step 1 of Plan registration (indepth.md §5.2): browser submits username/password.
pub async fn start_web_registration(
    pool: &SqlitePool,
    username: &str,
    password: &str,
) -> Result<RegistrationStart> {
    if username.len() < 3 || password.len() < 6 {
        return Err(Error::InvalidState(
            "username must be at least 3 characters and password at least 6".into(),
        ));
    }
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM plan_security WHERE username = ? COLLATE NOCASE",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;
    if existing.is_some() {
        return Err(Error::InvalidState("username already registered".into()));
    }
    let hash = hash_password(password)?;
    let code = registration_code();
    let now = chrono::Utc::now().timestamp_millis();
    let expires = now + 15 * 60 * 1000;
    sqlx::query(
        "INSERT INTO plan_pending_registration (code, username, salted_pass_hash, created_ms, expires_ms)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&code)
    .bind(username)
    .bind(&hash)
    .bind(now)
    .bind(expires)
    .execute(pool)
    .await?;
    Ok(RegistrationStart {
        code,
        expires_ms: expires,
    })
}

/// Step 2: link Minecraft UUID after `/plan register <code>` (player) or console (admin group).
pub async fn complete_web_registration(
    pool: &SqlitePool,
    code: &str,
    linked_uuid: &str,
    grant_admin_group: bool,
) -> Result<String> {
    let row = sqlx::query(
        "SELECT username, salted_pass_hash, expires_ms FROM plan_pending_registration WHERE code = ?",
    )
    .bind(code)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Err(Error::InvalidState("invalid or expired registration code".into()));
    };
    let expires: i64 = row.get("expires_ms");
    if chrono::Utc::now().timestamp_millis() > expires {
        sqlx::query("DELETE FROM plan_pending_registration WHERE code = ?")
            .bind(code)
            .execute(pool)
            .await?;
        return Err(Error::InvalidState("registration code expired".into()));
    }
    let username: String = row.get("username");
    let hash: String = row.get("salted_pass_hash");
    let group_name = if grant_admin_group { "admin" } else { "player" };
    let group_id: i64 = sqlx::query_scalar("SELECT id FROM plan_web_group WHERE group_name = ?")
        .bind(group_name)
        .fetch_one(pool)
        .await?;
    sqlx::query(
        "INSERT INTO plan_security (username, salted_pass_hash, linked_to_uuid, group_id) VALUES (?, ?, ?, ?)",
    )
    .bind(&username)
    .bind(&hash)
    .bind(linked_uuid)
    .bind(group_id)
    .execute(pool)
    .await?;
    sqlx::query("DELETE FROM plan_pending_registration WHERE code = ?")
        .bind(code)
        .execute(pool)
        .await?;
    Ok(username)
}

pub async fn process_registration_completions(
    pool: &SqlitePool,
    completions: &[patch_analytics_protocol::RegistrationCompletion],
) -> Result<u32> {
    let mut ok = 0u32;
    for item in completions {
        if complete_web_registration(pool, &item.code, &item.player_uuid, item.grant_admin)
            .await
            .is_ok()
        {
            ok += 1;
        }
    }
    Ok(ok)
}

fn registration_code() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..12)
        .map(|_| format!("{:x}", rng.random_range(0u8..16)))
        .collect()
}

#[cfg(test)]
mod registration_tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn registration_round_trip() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        let start = start_web_registration(&pool, "steve", "secret123").await.unwrap();
        let username = complete_web_registration(&pool, &start.code, "uuid-steve", false)
            .await
            .unwrap();
        assert_eq!(username, "steve");
        let linked: String = sqlx::query_scalar(
            "SELECT linked_to_uuid FROM plan_security WHERE username = 'steve'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(linked, "uuid-steve");
    }
}
