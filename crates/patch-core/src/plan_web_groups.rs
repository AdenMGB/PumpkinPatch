//! Plan web RBAC group editor API (React `/manage` → GroupsView).

use crate::error::{Error, Result};
use sqlx::{Row, SqlitePool};

pub async fn list_web_group_names(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<String> =
        sqlx::query_scalar("SELECT group_name FROM plan_web_group ORDER BY group_name COLLATE NOCASE")
            .fetch_all(pool)
            .await?;
    Ok(rows)
}

pub async fn list_all_web_permissions(pool: &SqlitePool) -> Result<Vec<String>> {
    let rows: Vec<String> = sqlx::query_scalar(
        "SELECT permission FROM plan_web_permission ORDER BY permission COLLATE NOCASE",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn list_permissions_for_group(pool: &SqlitePool, group_name: &str) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT p.permission FROM plan_web_group g
         JOIN plan_web_group_to_permission gp ON gp.group_id = g.id
         JOIN plan_web_permission p ON p.id = gp.permission_id
         WHERE g.group_name = ?
         ORDER BY p.permission COLLATE NOCASE",
    )
    .bind(group_name)
    .fetch_all(pool)
    .await?;
    Ok(rows.iter().map(|r| r.get("permission")).collect())
}

async fn group_id(pool: &SqlitePool, group_name: &str) -> Result<i64> {
    sqlx::query_scalar("SELECT id FROM plan_web_group WHERE group_name = ?")
        .bind(group_name)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| Error::NotFound(format!("web group '{group_name}'")))
}

/// Replaces permissions for a group; creates the group when `permissions` is empty (Plan `addGroup`).
pub async fn save_permissions_for_group(
    pool: &SqlitePool,
    group_name: &str,
    permissions: &[String],
) -> Result<()> {
    if group_name.is_empty() {
        return Err(Error::InvalidState("group name required".into()));
    }
    let gid = match group_id(pool, group_name).await {
        Ok(id) => id,
        Err(Error::NotFound(_)) if permissions.is_empty() => {
            sqlx::query("INSERT INTO plan_web_group (group_name) VALUES (?)")
                .bind(group_name)
                .execute(pool)
                .await?;
            group_id(pool, group_name).await?
        }
        Err(e) => return Err(e),
    };

    sqlx::query("DELETE FROM plan_web_group_to_permission WHERE group_id = ?")
        .bind(gid)
        .execute(pool)
        .await?;

    for perm in permissions {
        let pid: Option<i64> =
            sqlx::query_scalar("SELECT id FROM plan_web_permission WHERE permission = ?")
                .bind(perm)
                .fetch_optional(pool)
                .await?;
        let pid = if let Some(id) = pid {
            id
        } else {
            sqlx::query("INSERT INTO plan_web_permission (permission) VALUES (?)")
                .bind(perm)
                .execute(pool)
                .await?;
            sqlx::query_scalar("SELECT id FROM plan_web_permission WHERE permission = ?")
                .bind(perm)
                .fetch_one(pool)
                .await?
        };
        sqlx::query(
            "INSERT OR IGNORE INTO plan_web_group_to_permission (group_id, permission_id) VALUES (?, ?)",
        )
        .bind(gid)
        .bind(pid)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn delete_web_group(pool: &SqlitePool, group_name: &str, move_to: &str) -> Result<()> {
    if group_name == move_to {
        return Err(Error::InvalidState("moveTo must differ from group".into()));
    }
    let from_id = group_id(pool, group_name).await?;
    let to_id = group_id(pool, move_to).await?;

    sqlx::query("UPDATE plan_security SET group_id = ? WHERE group_id = ?")
        .bind(to_id)
        .bind(from_id)
        .execute(pool)
        .await?;

    sqlx::query("DELETE FROM plan_web_group WHERE id = ?")
        .bind(from_id)
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn save_and_list_group_permissions() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();
        crate::plan_store::ensure_plan_schema(&pool).await.unwrap();
        save_permissions_for_group(&pool, "moderators", &[]).await.unwrap();
        save_permissions_for_group(
            &pool,
            "moderators",
            &["access.query".into(), "page.server".into()],
        )
        .await
        .unwrap();
        let perms = list_permissions_for_group(&pool, "moderators").await.unwrap();
        assert!(perms.contains(&"access.query".to_string()));
        let groups = list_web_group_names(&pool).await.unwrap();
        assert!(groups.iter().any(|g| g == "moderators"));
    }
}
