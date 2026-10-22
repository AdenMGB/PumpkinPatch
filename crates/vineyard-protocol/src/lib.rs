//! Zero-copy packed telemetry frames for Vineyard (plangem.md MODULE 7.2).

mod voice_position;

pub use voice_position::{
    pack_voice_position, try_unpack_voice_position, VoicePositionFrame, VOICE_POSITION_MAGIC,
};

use serde::{Deserialize, Serialize};

pub const MAX_HOT_CHUNKS: usize = 8;

/// Logical telemetry frame (packed manually for Tauri binary IPC).
#[derive(Clone, Debug, PartialEq)]
pub struct TickTelemetryFrame {
    pub server_id: [u8; 16],
    pub tick_number: u64,
    pub timestamp_ns: u64,
    pub total_mspt_us: u32,
    pub net_decode_us: u32,
    pub ecs_movement_us: u32,
    pub chunk_gen_us: u32,
    pub redstone_tick_us: u32,
    pub wasm_plugins_us: u32,
    pub online_players: u16,
    pub loaded_chunks: u32,
    pub entity_count: u32,
    pub hot_chunk_count: u8,
    pub hot_chunks: [ChunkHeatSample; MAX_HOT_CHUNKS],
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ChunkHeatSample {
    pub dimension_id: u8,
    pub chunk_x: i32,
    pub chunk_z: i32,
    pub tick_cost_us: u32,
    pub entity_count: u16,
    pub packet_bytes_emitted: u32,
}

/// CRDT routing mutation published on the Vineyard mesh (plangem.md §7.5).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteWireSpec {
    pub proxy_id: uuid::Uuid,
    pub backend_server_id: uuid::Uuid,
    pub server_alias: String,
    pub forced_hosts: Vec<String>,
    pub priority_tier: i32,
    pub permission_gate: Option<String>,
    /// Gourd pool lifecycle: `active`, `warming_up`, `hidden_ready`, `in_game_locked`, `draining`.
    #[serde(default)]
    pub pool_state: Option<String>,
}

/// Byte length of every `pack_frame` payload (always serializes `MAX_HOT_CHUNKS` slots).
pub fn packed_frame_byte_length() -> usize {
    70 + MAX_HOT_CHUNKS * 21
}

pub fn pack_frame(frame: &TickTelemetryFrame) -> Vec<u8> {
    let mut buf = Vec::with_capacity(128);
    buf.extend_from_slice(&frame.server_id);
    buf.extend_from_slice(&frame.tick_number.to_le_bytes());
    buf.extend_from_slice(&frame.timestamp_ns.to_le_bytes());
    buf.extend_from_slice(&frame.total_mspt_us.to_le_bytes());
    buf.extend_from_slice(&frame.net_decode_us.to_le_bytes());
    buf.extend_from_slice(&frame.ecs_movement_us.to_le_bytes());
    buf.extend_from_slice(&frame.chunk_gen_us.to_le_bytes());
    buf.extend_from_slice(&frame.redstone_tick_us.to_le_bytes());
    buf.extend_from_slice(&frame.wasm_plugins_us.to_le_bytes());
    buf.extend_from_slice(&frame.online_players.to_le_bytes());
    buf.extend_from_slice(&frame.loaded_chunks.to_le_bytes());
    buf.extend_from_slice(&frame.entity_count.to_le_bytes());
    buf.push(frame.hot_chunk_count);
    buf.extend_from_slice(&[0u8; 3]);
    for chunk in &frame.hot_chunks {
        buf.push(chunk.dimension_id);
        buf.extend_from_slice(&chunk.chunk_x.to_le_bytes());
        buf.extend_from_slice(&chunk.chunk_z.to_le_bytes());
        buf.extend_from_slice(&chunk.tick_cost_us.to_le_bytes());
        buf.extend_from_slice(&chunk.entity_count.to_le_bytes());
        buf.extend_from_slice(&chunk.packet_bytes_emitted.to_le_bytes());
    }
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_has_expected_header_size() {
        let frame = TickTelemetryFrame {
            server_id: [1u8; 16],
            tick_number: 9,
            timestamp_ns: 2,
            total_mspt_us: 50_000,
            net_decode_us: 1,
            ecs_movement_us: 2,
            chunk_gen_us: 3,
            redstone_tick_us: 4,
            wasm_plugins_us: 5,
            online_players: 2,
            loaded_chunks: 10,
            entity_count: 4,
            hot_chunk_count: 0,
            hot_chunks: [ChunkHeatSample::default(); MAX_HOT_CHUNKS],
        };
        let bytes = pack_frame(&frame);
        assert!(bytes.len() >= 68);
    }
}
