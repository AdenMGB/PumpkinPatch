use super::Result;
use crate::state::AppStateHandle;
use serde::Serialize;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tendril_voice::{
    TendrilVoiceConfig, TendrilVoiceEngine, TendrilVoiceSfu, TendrilVoiceStats, VoicePeerInfo,
    VoicePosition,
};
use tokio::sync::Mutex;
use uuid::Uuid;
use vineyard_bus::VineyardBus;
use vineyard_protocol::{try_unpack_voice_position, VoicePositionFrame};

struct TendrilRuntime {
    stats: Arc<TendrilVoiceStats>,
    engine: Arc<TendrilVoiceEngine>,
}

static TENDRIL: Mutex<Option<TendrilRuntime>> = Mutex::const_new(None);

#[derive(Serialize)]
pub struct TendrilVoiceStatsDto {
    pub packets_in: u64,
    pub packets_out: u64,
    pub bytes_in: u64,
    pub forwards: u64,
    pub spatial_drops: u64,
    pub peers: u64,
}

#[derive(Serialize)]
pub struct TendrilVoiceFlushResult {
    pub speaker_uuid: String,
    pub captured_packets: u32,
    pub total_bytes: u64,
    pub output_dir: String,
}

pub async fn ensure_tendril_voice_running(
    state: &AppStateHandle,
    bind: String,
    proximity_blocks: Option<f64>,
) -> Result<String> {
    let inner = state.get().await?;
    let addr: SocketAddr = bind
        .parse()
        .map_err(|e| patch_core::Error::InvalidState(format!("invalid bind: {e}")))?;
    let proximity = proximity_blocks.unwrap_or(tendril_voice::DEFAULT_PROXIMITY_BLOCKS);
    let stats = Arc::new(TendrilVoiceStats::default());
    let engine = TendrilVoiceEngine::new(proximity);
    spawn_vineyard_voice_bridge(inner.vineyard_bus.clone(), engine.clone());
    {
        let mut guard = TENDRIL.lock().await;
        *guard = Some(TendrilRuntime {
            stats: stats.clone(),
            engine: engine.clone(),
        });
    }
    let config = TendrilVoiceConfig {
        bind: addr,
        proximity_blocks: proximity,
    };
    tokio::spawn(async move {
        let _ = TendrilVoiceSfu::serve_with_engine(config, Some(stats), Some(engine)).await;
    });
    Ok(format!(
        "TendrilVoice SFU listening on {bind} (proximity {proximity} blocks); Vineyard position stream attached"
    ))
}

fn spawn_vineyard_voice_bridge(bus: VineyardBus, engine: Arc<TendrilVoiceEngine>) {
    tokio::spawn(async move {
        let mut rx = bus.subscribe_voice_positions();
        while let Ok(bytes) = rx.recv().await {
            if let Some(frame) = try_unpack_voice_position(&bytes) {
                let player_uuid = Uuid::from_bytes(frame.player_uuid).to_string();
                engine.set_position(
                    &player_uuid,
                    VoicePosition {
                        x: frame.x,
                        y: frame.y,
                        z: frame.z,
                        dimension: frame.dimension,
                    },
                );
            }
        }
    });
}

#[tauri::command]
pub async fn tendril_voice_start(
    state: tauri::State<'_, AppStateHandle>,
    bind: String,
    proximity_blocks: Option<f64>,
) -> Result<String> {
    ensure_tendril_voice_running(&state, bind, proximity_blocks).await
}

/// Publish a player position on the Vineyard bus (Gourd backends / Pumpkin hook → SFU).
#[tauri::command]
pub async fn vineyard_publish_voice_position(
    state: tauri::State<'_, AppStateHandle>,
    player_uuid: String,
    x: f64,
    y: f64,
    z: f64,
    dimension: String,
    yaw: Option<f32>,
    pitch: Option<f32>,
) -> Result<()> {
    let inner = state.get().await?;
    let id = Uuid::parse_str(&player_uuid)
        .map_err(|e| patch_core::Error::InvalidState(format!("invalid uuid: {e}")))?;
    let frame = VoicePositionFrame {
        player_uuid: *id.as_bytes(),
        x,
        y,
        z,
        yaw: yaw.unwrap_or(0.0),
        pitch: pitch.unwrap_or(0.0),
        dimension,
    };
    inner.vineyard_bus.publish_voice_position_best_effort(&frame);
    Ok(())
}

#[tauri::command]
pub async fn tendril_voice_stats() -> Result<TendrilVoiceStatsDto> {
    let guard = TENDRIL.lock().await;
    let Some(rt) = guard.as_ref() else {
        return Ok(TendrilVoiceStatsDto {
            packets_in: 0,
            packets_out: 0,
            bytes_in: 0,
            forwards: 0,
            spatial_drops: 0,
            peers: 0,
        });
    };
    Ok(TendrilVoiceStatsDto {
        packets_in: rt.stats.packets_in.load(std::sync::atomic::Ordering::Relaxed),
        packets_out: rt.stats.packets_out.load(std::sync::atomic::Ordering::Relaxed),
        bytes_in: rt.stats.bytes_in.load(std::sync::atomic::Ordering::Relaxed),
        forwards: rt.engine.forwards_count(),
        spatial_drops: rt.engine.spatial_drops_count(),
        peers: rt.engine.peer_count(),
    })
}

#[tauri::command]
pub async fn tendril_voice_list_peers() -> Result<Vec<VoicePeerInfo>> {
    let guard = TENDRIL.lock().await;
    Ok(guard
        .as_ref()
        .map(|rt| rt.engine.list_peers())
        .unwrap_or_default())
}

#[tauri::command]
pub async fn tendril_voice_set_position(
    state: tauri::State<'_, AppStateHandle>,
    player_uuid: String,
    x: f64,
    y: f64,
    z: f64,
    dimension: String,
) -> Result<()> {
    vineyard_publish_voice_position(
        state,
        player_uuid.clone(),
        x,
        y,
        z,
        dimension.clone(),
        None,
        None,
    )
    .await?;
    let guard = TENDRIL.lock().await;
    if let Some(rt) = guard.as_ref() {
        rt.engine.set_position(
            &player_uuid,
            VoicePosition {
                x,
                y,
                z,
                dimension,
            },
        );
    }
    Ok(())
}

#[tauri::command]
pub async fn tendril_voice_flush_report(
    player_uuid: String,
    output_dir: Option<String>,
) -> Result<TendrilVoiceFlushResult> {
    let guard = TENDRIL.lock().await;
    let Some(rt) = guard.as_ref() else {
        return Err(patch_core::Error::InvalidState("TendrilVoice is not running".into()).into());
    };
    let clip = rt
        .engine
        .flush_report(&player_uuid)
        .ok_or_else(|| patch_core::Error::InvalidState("no buffered audio for player".into()))?;

    let base: PathBuf = match output_dir {
        Some(p) => PathBuf::from(p),
        None => {
            let mut dir = std::env::temp_dir();
            dir.push("pumpkinpatch-voice-evidence");
            dir.push(&clip.speaker_uuid);
            dir
        }
    };
    fs::create_dir_all(&base).map_err(patch_core::Error::Io)?;
    for (i, frame) in clip.frames.iter().enumerate() {
        let path = base.join(format!("frame_{:04}.opus", i));
        fs::write(path, frame).map_err(patch_core::Error::Io)?;
    }
    let manifest = serde_json::json!({
        "speaker_uuid": clip.speaker_uuid,
        "captured_packets": clip.captured_packets,
        "total_bytes": clip.total_bytes,
        "frame_count": clip.frames.len(),
    });
    fs::write(
        base.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| {
            patch_core::Error::InvalidState(format!("manifest json: {e}"))
        })?,
    )
    .map_err(patch_core::Error::Io)?;

    Ok(TendrilVoiceFlushResult {
        speaker_uuid: clip.speaker_uuid,
        captured_packets: clip.captured_packets,
        total_bytes: clip.total_bytes,
        output_dir: base.to_string_lossy().into_owned(),
    })
}
