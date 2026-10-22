//! TendrilVoice UDP spatial SFU — SVC/Plasmo opaque passthrough + TV framing, 60s evidence rings.

mod buffer;
mod engine;
mod protocol;

pub use buffer::SpeakerRingBuffer;
pub use engine::{
    DEFAULT_PROXIMITY_BLOCKS, MonitorEar, TendrilVoiceEngine, VoicePeerInfo, VoicePosition,
    VoiceReportClip,
};
pub use protocol::{
    encode_handshake, encode_sound, parse_packet, uuid_bytes_from_str, uuid_bytes_to_string,
    ParsedPacket, PKT_HANDSHAKE, PKT_SOUND,
};

use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::net::UdpSocket;

pub struct TendrilVoiceConfig {
    pub bind: SocketAddr,
    /// Minecraft block radius for proximity fan-out (default 48).
    pub proximity_blocks: f64,
}

impl Default for TendrilVoiceConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:24454".parse().expect("valid default bind"),
            proximity_blocks: DEFAULT_PROXIMITY_BLOCKS,
        }
    }
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
        TendrilVoiceSfu::serve_with_engine(config, None, None).await
    }

    pub async fn serve_with_stats(
        config: TendrilVoiceConfig,
        stats: Option<Arc<TendrilVoiceStats>>,
    ) -> Result<(), std::io::Error> {
        TendrilVoiceSfu::serve_with_engine(config, stats, None).await
    }

    pub async fn serve_with_engine(
        config: TendrilVoiceConfig,
        stats: Option<Arc<TendrilVoiceStats>>,
        engine: Option<Arc<TendrilVoiceEngine>>,
    ) -> Result<(), std::io::Error> {
        let socket = UdpSocket::bind(config.bind).await?;
        let engine = engine.unwrap_or_else(|| TendrilVoiceEngine::new(config.proximity_blocks));
        let mut buf = [0u8; 2048];
        loop {
            let (len, peer) = socket.recv_from(&mut buf).await?;
            if let Some(st) = &stats {
                st.packets_in.fetch_add(1, Ordering::Relaxed);
                st.bytes_in.fetch_add(len as u64, Ordering::Relaxed);
            }
            let payload = &buf[..len];
            let parsed = parse_packet(payload);
            engine.handle_parsed(peer, &parsed);

            let (speaker, wire): ([u8; 16], Vec<u8>) = match parsed {
                ParsedPacket::Sound { speaker, opus } => (speaker, encode_sound(&speaker, opus)),
                ParsedPacket::OpaqueSvc(p) => match engine.player_for_peer(peer) {
                    Some(sp) => (sp, p.to_vec()),
                    None => continue,
                },
                _ => continue,
            };

            for target in engine.proximity_targets(&speaker, peer) {
                if socket.send_to(&wire, target).await.is_ok() {
                    engine.forwards.fetch_add(1, Ordering::Relaxed);
                    if let Some(st) = &stats {
                        st.packets_out.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }
    }
}
