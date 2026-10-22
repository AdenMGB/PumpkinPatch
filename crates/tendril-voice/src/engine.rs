//! Spatial SFU state: peer bindings, positions, evidence buffers.

use crate::buffer::SpeakerRingBuffer;
use crate::protocol::{uuid_bytes_to_string, ParsedPacket};
use dashmap::DashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

pub const DEFAULT_PROXIMITY_BLOCKS: f64 = 48.0;

#[derive(Debug, Clone, serde::Serialize)]
pub struct VoicePosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dimension: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VoicePeerInfo {
    pub player_uuid: String,
    pub addr: String,
    pub dimension: String,
    pub packets_buffered: u32,
}

/// Admin "drop an ear" coordinate for live moderation (plangem §5.1).
#[derive(Debug, Clone, serde::Serialize)]
pub struct MonitorEar {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub dimension: String,
}

pub struct VoiceReportClip {
    pub speaker_uuid: String,
    pub captured_packets: u32,
    pub total_bytes: u64,
    pub frames: Vec<Vec<u8>>,
}

#[derive(Debug)]
struct PeerState {
    player: [u8; 16],
}

#[derive(Debug)]
pub struct TendrilVoiceEngine {
    proximity_blocks: f64,
    peers: DashMap<SocketAddr, PeerState>,
    positions: DashMap<[u8; 16], VoicePosition>,
    rings: DashMap<[u8; 16], SpeakerRingBuffer>,
    monitor_ear: Mutex<Option<MonitorEar>>,
    monitor_sink: Mutex<Option<Sender<Vec<u8>>>>,
    pub spatial_drops: AtomicU64,
    pub forwards: AtomicU64,
}

impl TendrilVoiceEngine {
    pub fn new(proximity_blocks: f64) -> Arc<Self> {
        Arc::new(Self {
            proximity_blocks,
            peers: DashMap::new(),
            positions: DashMap::new(),
            rings: DashMap::new(),
            monitor_ear: Mutex::new(None),
            monitor_sink: Mutex::new(None),
            spatial_drops: AtomicU64::new(0),
            forwards: AtomicU64::new(0),
        })
    }

    pub fn set_monitor(&self, ear: Option<MonitorEar>, sink: Option<Sender<Vec<u8>>>) {
        if let Ok(mut slot) = self.monitor_ear.lock() {
            *slot = ear;
        }
        if let Ok(mut slot) = self.monitor_sink.lock() {
            *slot = sink;
        }
    }

    fn tap_monitor(&self, speaker: &[u8; 16], opus: &[u8]) {
        let ear = self.monitor_ear.lock().ok().and_then(|g| g.clone());
        let sink = self.monitor_sink.lock().ok().and_then(|g| g.clone());
        let (Some(ear), Some(sink)) = (ear, sink) else {
            return;
        };
        if let Some(sp) = self.positions.get(speaker) {
            if sp.dimension != ear.dimension {
                return;
            }
            let dx = sp.x - ear.x;
            let dy = sp.y - ear.y;
            let dz = sp.z - ear.z;
            let dist_sq = dx * dx + dy * dy + dz * dz;
            if dist_sq > self.proximity_blocks * self.proximity_blocks {
                return;
            }
        }
        let _ = sink.send(opus.to_vec());
    }

    pub fn set_position(&self, player_uuid: &str, pos: VoicePosition) {
        if let Ok(id) = uuid::Uuid::parse_str(player_uuid) {
            self.positions.insert(*id.as_bytes(), pos);
        }
    }

    pub fn bind_peer(&self, addr: SocketAddr, player: [u8; 16]) {
        self.peers.insert(addr, PeerState { player });
        self.rings.entry(player).or_insert_with(SpeakerRingBuffer::new);
    }

    pub fn player_for_peer(&self, addr: SocketAddr) -> Option<[u8; 16]> {
        self.peers.get(&addr).map(|p| p.player)
    }

    pub fn record_sound(&self, speaker: &[u8; 16], frame: &[u8]) {
        let mut entry = self
            .rings
            .entry(*speaker)
            .or_insert_with(SpeakerRingBuffer::new);
        entry.push(frame.to_vec());
        self.tap_monitor(speaker, frame);
    }

    pub fn handle_parsed(&self, peer: SocketAddr, parsed: &ParsedPacket<'_>) {
        match parsed {
            ParsedPacket::Handshake { player_uuid } => self.bind_peer(peer, *player_uuid),
            ParsedPacket::Sound { speaker, opus } => {
                self.bind_peer(peer, *speaker);
                self.record_sound(speaker, opus);
            }
            ParsedPacket::OpaqueSvc(payload) => {
                if let Some(player) = self.player_for_peer(peer) {
                    self.record_sound(&player, payload);
                }
            }
            ParsedPacket::Unknown => {}
        }
    }

    pub fn proximity_targets(&self, speaker: &[u8; 16], from: SocketAddr) -> Vec<SocketAddr> {
        let speaker_pos = self.positions.get(speaker);
        let mut out = Vec::new();
        for entry in self.peers.iter() {
            let addr = *entry.key();
            if addr == from {
                continue;
            }
            let other = entry.value().player;
            if speaker_pos.is_none() {
                out.push(addr);
                continue;
            }
            let sp = speaker_pos.as_ref().unwrap();
            match self.positions.get(&other) {
                Some(op) => {
                    if op.dimension != sp.dimension {
                        self.spatial_drops.fetch_add(1, Ordering::Relaxed);
                        continue;
                    }
                    let dx = op.x - sp.x;
                    let dy = op.y - sp.y;
                    let dz = op.z - sp.z;
                    let dist_sq = dx * dx + dy * dy + dz * dz;
                    if dist_sq <= self.proximity_blocks * self.proximity_blocks {
                        out.push(addr);
                    } else {
                        self.spatial_drops.fetch_add(1, Ordering::Relaxed);
                    }
                }
                None => out.push(addr),
            }
        }
        out
    }

    pub fn peer_count(&self) -> u64 {
        self.peers.len() as u64
    }

    pub fn list_peers(&self) -> Vec<VoicePeerInfo> {
        self.peers
            .iter()
            .map(|entry| {
                let addr = *entry.key();
                let player = entry.value().player;
                let packets = self
                    .rings
                    .get(&player)
                    .map(|r| r.packet_count())
                    .unwrap_or(0);
                let dimension = self
                    .positions
                    .get(&player)
                    .map(|p| p.dimension.clone())
                    .unwrap_or_else(|| "minecraft:overworld".into());
                VoicePeerInfo {
                    player_uuid: uuid_bytes_to_string(&player),
                    addr: addr.to_string(),
                    dimension,
                    packets_buffered: packets,
                }
            })
            .collect()
    }

    pub fn forwards_count(&self) -> u64 {
        self.forwards.load(Ordering::Relaxed)
    }

    pub fn spatial_drops_count(&self) -> u64 {
        self.spatial_drops.load(Ordering::Relaxed)
    }

    pub fn flush_report(&self, player_uuid: &str) -> Option<VoiceReportClip> {
        let id = uuid::Uuid::parse_str(player_uuid).ok()?;
        let key = *id.as_bytes();
        let mut ring = self.rings.get_mut(&key)?;
        let frames = ring.flush();
        let total_bytes = frames.iter().map(|f| f.len() as u64).sum();
        Some(VoiceReportClip {
            speaker_uuid: player_uuid.to_string(),
            captured_packets: frames.len() as u32,
            total_bytes,
            frames,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::uuid_bytes_to_string;
    use std::net::{IpAddr, Ipv4Addr};

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    #[test]
    fn monitor_tap_only_near_ear() {
        let engine = TendrilVoiceEngine::new(10.0);
        let speaker = [9u8; 16];
        let (tx, rx) = std::sync::mpsc::channel();
        engine.set_monitor(
            Some(MonitorEar {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                dimension: "minecraft:overworld".into(),
            }),
            Some(tx),
        );
        engine.set_position(
            &uuid_bytes_to_string(&speaker),
            VoicePosition {
                x: 3.0,
                y: 0.0,
                z: 0.0,
                dimension: "minecraft:overworld".into(),
            },
        );
        engine.record_sound(&speaker, &[1, 2, 3]);
        assert_eq!(rx.try_recv().unwrap(), vec![1, 2, 3]);

        engine.set_position(
            &uuid_bytes_to_string(&speaker),
            VoicePosition {
                x: 50.0,
                y: 0.0,
                z: 0.0,
                dimension: "minecraft:overworld".into(),
            },
        );
        engine.record_sound(&speaker, &[4]);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn proximity_respects_dimension_and_distance() {
        let engine = TendrilVoiceEngine::new(10.0);
        let a = [1u8; 16];
        let b = [2u8; 16];
        engine.bind_peer(addr(1), a);
        engine.bind_peer(addr(2), b);
        engine.set_position(
            &uuid_bytes_to_string(&a),
            VoicePosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                dimension: "minecraft:overworld".into(),
            },
        );
        engine.set_position(
            &uuid_bytes_to_string(&b),
            VoicePosition {
                x: 5.0,
                y: 0.0,
                z: 0.0,
                dimension: "minecraft:overworld".into(),
            },
        );
        let near = engine.proximity_targets(&a, addr(1));
        assert!(near.contains(&addr(2)));

        engine.set_position(
            &uuid_bytes_to_string(&b),
            VoicePosition {
                x: 100.0,
                y: 0.0,
                z: 0.0,
                dimension: "minecraft:overworld".into(),
            },
        );
        let far = engine.proximity_targets(&a, addr(1));
        assert!(!far.contains(&addr(2)));

        engine.set_position(
            &uuid_bytes_to_string(&b),
            VoicePosition {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                dimension: "minecraft:the_nether".into(),
            },
        );
        let other_dim = engine.proximity_targets(&a, addr(1));
        assert!(!other_dim.contains(&addr(2)));
    }
}
