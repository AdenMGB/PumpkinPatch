//! Host-side Vineyard telemetry sampler (plangem.md §7.3 phase 1).

use crate::state::AppState;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use vineyard_bus::VineyardBus;
use vineyard_protocol::{packed_frame_byte_length, ChunkHeatSample, TickTelemetryFrame};

/// Tails `vineyard-telemetry.bin` written by `pumpkin-hook-sdk` on Pumpkin instances.
pub fn spawn_instance_telemetry_tail(bus: VineyardBus, _server_id: Uuid, file: PathBuf) {
    let frame_len = packed_frame_byte_length();
    tokio::spawn(async move {
        let mut offset: u64 = 0;
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(75));
        loop {
            interval.tick().await;
            let meta = tokio::fs::metadata(&file).await;
            if meta.is_err() {
                continue;
            }
            let len = meta.unwrap().len();
            if len < offset + frame_len as u64 {
                continue;
            }
            let mut file_handle = match tokio::fs::File::open(&file).await {
                Ok(f) => f,
                Err(_) => continue,
            };
            use tokio::io::{AsyncReadExt, AsyncSeekExt};
            if file_handle
                .seek(std::io::SeekFrom::Start(offset))
                .await
                .is_err()
            {
                continue;
            }
            while offset + frame_len as u64 <= len {
                let mut buf = vec![0u8; frame_len];
                if file_handle.read_exact(&mut buf).await.is_err() {
                    break;
                }
                offset += frame_len as u64;
                let _ = bus.publish_raw(buf);
            }
        }
    });
}

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
            let hot_chunk_count = 3u8;
            let mut hot_chunks =
                std::array::from_fn(|_| ChunkHeatSample::default());
            for (i, sample) in hot_chunks.iter_mut().take(hot_chunk_count as usize).enumerate() {
                *sample = ChunkHeatSample {
                    dimension_id: 0,
                    chunk_x: (i as i32) - 1,
                    chunk_z: (tick as i32 % 5) - 2,
                    tick_cost_us: 2_000 + (i as u32) * 1_500 + (tick % 7) as u32 * 200,
                    entity_count: (players_online as u16).min(64),
                    packet_bytes_emitted: 512 + i as u32 * 128,
                };
            }
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
                hot_chunk_count,
                hot_chunks,
            };
            let _ = bus.publish_frame(&frame);
        }
    });
}

fn estimate_mspt_us(players: u32) -> u32 {
    45_000 + players.saturating_mul(120)
}
