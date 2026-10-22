//! TendrilVoice UDP SFU scaffold — accepts Opus frames and fans out to proximity groups.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;

pub struct TendrilVoiceConfig {
    pub bind: SocketAddr,
}

#[derive(Debug, Default)]
pub struct TendrilVoiceStats {
    pub packets_in: AtomicU64,
    pub packets_out: AtomicU64,
    pub bytes_in: AtomicU64,
}

pub struct TendrilVoiceSfu;

impl TendrilVoiceSfu {
    pub async fn serve(config: TendrilVoiceConfig) -> Result<(), std::io::Error> {
        TendrilVoiceSfu::serve_with_stats(config, None).await
    }

    pub async fn serve_with_stats(
        config: TendrilVoiceConfig,
        stats: Option<Arc<TendrilVoiceStats>>,
    ) -> Result<(), std::io::Error> {
        let socket = UdpSocket::bind(config.bind).await?;
        let mut buf = [0u8; 2048];
        loop {
            let (len, peer) = socket.recv_from(&mut buf).await?;
            if let Some(st) = &stats {
                st.packets_in.fetch_add(1, Ordering::Relaxed);
                st.bytes_in.fetch_add(len as u64, Ordering::Relaxed);
            }
            let _ = socket.send_to(&buf[..len], peer).await?;
            if let Some(st) = &stats {
                st.packets_out.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}
