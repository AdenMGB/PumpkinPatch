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
