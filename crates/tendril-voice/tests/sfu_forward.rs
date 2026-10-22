use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tendril_voice::{
    encode_handshake, encode_sound, parse_packet, ParsedPacket, TendrilVoiceConfig,
    TendrilVoiceEngine, TendrilVoiceSfu, TendrilVoiceStats, VoicePosition,
};
use tokio::net::UdpSocket;

fn local_ephemeral() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)
}

#[tokio::test]
async fn proximity_sfu_forwards_tv_sound() {
    let probe = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let sfu_addr = probe.local_addr().unwrap();
    drop(probe);

    let engine = TendrilVoiceEngine::new(64.0);
    let speaker = [9u8; 16];
    let listener = [8u8; 16];
    engine.set_position(
        &uuid::Uuid::from_bytes(speaker).to_string(),
        VoicePosition {
            x: 0.0,
            y: 64.0,
            z: 0.0,
            dimension: "minecraft:overworld".into(),
        },
    );
    engine.set_position(
        &uuid::Uuid::from_bytes(listener).to_string(),
        VoicePosition {
            x: 2.0,
            y: 64.0,
            z: 0.0,
            dimension: "minecraft:overworld".into(),
        },
    );

    let stats = Arc::new(TendrilVoiceStats::default());
    let config = TendrilVoiceConfig {
        bind: sfu_addr,
        proximity_blocks: 64.0,
    };
    tokio::spawn(async move {
        let _ = TendrilVoiceSfu::serve_with_engine(config, Some(stats), Some(engine)).await;
    });

    let client_a = UdpSocket::bind(local_ephemeral()).await.unwrap();
    let client_b = UdpSocket::bind(local_ephemeral()).await.unwrap();
    client_a
        .send_to(&encode_handshake(&speaker), sfu_addr)
        .await
        .unwrap();
    client_b
        .send_to(&encode_handshake(&listener), sfu_addr)
        .await
        .unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let opus = [0x4f, 0x50, 0x55, 0x53];
    client_a
        .send_to(&encode_sound(&speaker, &opus), sfu_addr)
        .await
        .unwrap();

    let mut buf = [0u8; 512];
    let len = client_b
        .recv(&mut buf)
        .await
        .expect("listener should receive forwarded frame");
    match parse_packet(&buf[..len]) {
        ParsedPacket::Sound { speaker: sp, opus: tail } => {
            assert_eq!(sp, speaker);
            assert_eq!(tail, opus);
        }
        other => panic!("unexpected packet {other:?}"),
    }
}
