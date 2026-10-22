//! Host-side performance samples for plan_tps (§2.5 partial — CPU/RAM/disk + live player counts).

use crate::error::Result;
use sqlx::SqlitePool;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn sample_server_performance(
    pool: &SqlitePool,
    plan_server_id: i64,
    players_online: u32,
) -> Result<()> {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);

    let (cpu, ram_mb, free_disk_mb) = host_gauges();

    sqlx::query(
        "INSERT INTO plan_tps
         (server_id, date, ticks_per_second, players_online, cpu_usage, ram_usage, free_disk_space)
         VALUES (?, ?, 20.0, ?, ?, ?, ?)",
    )
    .bind(plan_server_id)
    .bind(now_ms)
    .bind(players_online as i64)
    .bind(cpu)
    .bind(ram_mb)
    .bind(free_disk_mb)
    .execute(pool)
    .await?;

    Ok(())
}

fn host_gauges() -> (f64, i64, i64) {
    let mut sys = sysinfo::System::new();
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let cpu = sys.global_cpu_usage() as f64;
    let ram_mb = (sys.used_memory() / (1024 * 1024)) as i64;
    let free_disk_mb = sysinfo::Disks::new_with_refreshed_list()
        .iter()
        .map(|d| d.available_space() / (1024 * 1024))
        .max()
        .unwrap_or(0) as i64;
    (cpu, ram_mb, free_disk_mb)
}
