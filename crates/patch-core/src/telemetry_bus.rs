//! Host-side Vineyard telemetry sampler (plangem.md §7.3 phase 1).

use crate::state::AppState;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use vineyard_protocol::{ChunkHeatSample, TickTelemetryFrame, MAX_HOT_CHUNKS};

pub fn spawn_host_telemetry_sampler(state: &AppState, server_id: Uuid, players_online: u32) {
    let bus = state.vineyard_bus.clone();
    let server_bytes = *server_id.as_bytes();
    tokio::spawn(async move {
        let mut tick: u64 = 0;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(50));
        loop {
            interval.tick().await;
            tick += 1;
            let now_ns = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            let frame = TickTelemetryFrame {
                server_id: server_bytes,
                tick_number: tick,
                timestamp_ns: now_ns,
                total_mspt_us: estimate_mspt_us(players_online),
                net_decode_us: 120,
                ecs_movement_us: 800,
                chunk_gen_us: 400,
                redstone_tick_us: 200,
                wasm_plugins_us: 150,
                online_players: players_online.min(u16::MAX as u32) as u16,
                loaded_chunks: players_online.saturating_mul(32),
                entity_count: players_online.saturating_mul(8),
                hot_chunk_count: 0,
                hot_chunks: std::array::from_fn(|_| ChunkHeatSample::default()),
            };
            let _ = bus.publish_frame(&frame);
        }
    });
}

fn estimate_mspt_us(players: u32) -> u32 {
    45_000 + players.saturating_mul(120)
}
