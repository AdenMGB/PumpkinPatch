use super::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tendril_voice::{TendrilVoiceConfig, TendrilVoiceStats, TendrilVoiceSfu};
use tokio::sync::Mutex;

static TENDRIL: Mutex<Option<Arc<TendrilVoiceStats>>> = Mutex::const_new(None);

#[tauri::command]
pub async fn tendril_voice_start(bind: String) -> Result<String> {
    let addr: SocketAddr = bind
        .parse()
        .map_err(|e| patch_core::Error::InvalidState(format!("invalid bind: {e}")))?;
    let stats = Arc::new(TendrilVoiceStats::default());
    {
        let mut guard = TENDRIL.lock().await;
        *guard = Some(stats.clone());
    }
    tokio::spawn(async move {
        let _ = TendrilVoiceSfu::serve_with_stats(
            TendrilVoiceConfig { bind: addr },
            Some(stats),
        )
        .await;
    });
    Ok(format!("TendrilVoice SFU listening on {bind}"))
}

#[tauri::command]
pub async fn tendril_voice_stats() -> Result<(u64, u64, u64)> {
    let guard = TENDRIL.lock().await;
    let Some(stats) = guard.as_ref() else {
        return Ok((0, 0, 0));
    };
    Ok((
        stats.packets_in.load(std::sync::atomic::Ordering::Relaxed),
        stats.packets_out.load(std::sync::atomic::Ordering::Relaxed),
        stats.bytes_in.load(std::sync::atomic::Ordering::Relaxed),
    ))
}
