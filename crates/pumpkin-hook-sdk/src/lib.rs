//! Embed in PumpkinMC builds to emit `TickTelemetryFrame` into Vineyard shared memory / files.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use vineyard_protocol::{pack_frame, TickTelemetryFrame};

pub struct TelemetrySink {
    path: std::path::PathBuf,
    server_id: [u8; 16],
    tick: u64,
}

impl TelemetrySink {
    pub fn open(instance_id: &[u8; 16], dir: &Path) -> Self {
        let path = dir.join("vineyard-telemetry.bin");
        Self {
            path,
            server_id: *instance_id,
            tick: 0,
        }
    }

    pub fn emit(&mut self, frame: TickTelemetryFrame) -> std::io::Result<()> {
        let bytes = pack_frame(&frame);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    pub fn emit_host_sample(&mut self, players_online: u16, mspt_us: u32) -> std::io::Result<()> {
        self.tick += 1;
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        self.emit(TickTelemetryFrame {
            server_id: self.server_id,
            tick_number: self.tick,
            timestamp_ns: now_ns,
            total_mspt_us: mspt_us,
            net_decode_us: 0,
            ecs_movement_us: 0,
            chunk_gen_us: 0,
            redstone_tick_us: 0,
            wasm_plugins_us: 0,
            online_players: players_online,
            loaded_chunks: 0,
            entity_count: 0,
            hot_chunk_count: 0,
            hot_chunks: std::array::from_fn(|_| vineyard_protocol::ChunkHeatSample::default()),
        })
    }
}
