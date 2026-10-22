//! TendrilVoice UDP SFU scaffold — accepts Opus frames and fans out to proximity groups.

use std::net::SocketAddr;
use tokio::net::UdpSocket;

pub struct TendrilVoiceConfig {
    pub bind: SocketAddr,
}

pub struct TendrilVoiceSfu;

impl TendrilVoiceSfu {
    pub async fn serve(config: TendrilVoiceConfig) -> Result<(), std::io::Error> {
        let socket = UdpSocket::bind(config.bind).await?;
        let mut buf = [0u8; 2048];
        loop {
            let (len, peer) = socket.recv_from(&mut buf).await?;
            let _ = socket.send_to(&buf[..len], peer).await;
        }
    }
}
