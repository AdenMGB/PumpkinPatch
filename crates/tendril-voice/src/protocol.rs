//! TendrilVoice wire framing + Simple Voice Chat opaque passthrough detection.

pub const HEADER_TV: [u8; 2] = [0x54, 0x56]; // "TV"

pub const PKT_HANDSHAKE: u8 = 0x00;
pub const PKT_SOUND: u8 = 0x01;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedPacket<'a> {
    Handshake { player_uuid: [u8; 16] },
    Sound { speaker: [u8; 16], opus: &'a [u8] },
    /// Unrecognized but likely encrypted SVC / Plasmo payload — forward verbatim.
    OpaqueSvc(&'a [u8]),
    Unknown,
}

pub fn parse_packet(buf: &[u8]) -> ParsedPacket<'_> {
    if buf.len() >= 19 && buf[0..2] == HEADER_TV {
        match buf[2] {
            PKT_HANDSHAKE if buf.len() >= 18 => {
                let mut id = [0u8; 16];
                id.copy_from_slice(&buf[3..19]);
                return ParsedPacket::Handshake { player_uuid: id };
            }
            PKT_SOUND if buf.len() > 19 => {
                let mut id = [0u8; 16];
                id.copy_from_slice(&buf[3..19]);
                return ParsedPacket::Sound {
                    speaker: id,
                    opus: &buf[19..],
                };
            }
            _ => {}
        }
    }
    // Heuristic: SVC/Plasmo voice frames are typically 20–1400 bytes UDP payloads.
    if buf.len() >= 20 && buf.len() <= 2048 {
        return ParsedPacket::OpaqueSvc(buf);
    }
    ParsedPacket::Unknown
}

pub fn encode_sound(speaker: &[u8; 16], opus: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 + 16 + opus.len());
    out.extend_from_slice(&HEADER_TV);
    out.push(PKT_SOUND);
    out.extend_from_slice(speaker);
    out.extend_from_slice(opus);
    out
}

pub fn encode_handshake(player: &[u8; 16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(19);
    out.extend_from_slice(&HEADER_TV);
    out.push(PKT_HANDSHAKE);
    out.extend_from_slice(player);
    out
}

pub fn uuid_bytes_from_str(s: &str) -> Option<[u8; 16]> {
    uuid::Uuid::parse_str(s).ok().map(|u| *u.as_bytes())
}

pub fn uuid_bytes_to_string(id: &[u8; 16]) -> String {
    uuid::Uuid::from_bytes(*id).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tv_handshake_roundtrip() {
        let id = [7u8; 16];
        let pkt = encode_handshake(&id);
        let parsed = parse_packet(&pkt);
        assert_eq!(
            parsed,
            ParsedPacket::Handshake {
                player_uuid: id
            }
        );
    }

    #[test]
    fn tv_sound_carries_opus_tail() {
        let id = [1u8; 16];
        let opus = [0xde, 0xad, 0xbe, 0xef];
        let pkt = encode_sound(&id, &opus);
        match parse_packet(&pkt) {
            ParsedPacket::Sound { speaker, opus: tail } => {
                assert_eq!(speaker, id);
                assert_eq!(tail, opus);
            }
            other => panic!("expected sound, got {other:?}"),
        }
    }

    #[test]
    fn opaque_svc_heuristic() {
        let blob = vec![0u8; 64];
        assert!(matches!(parse_packet(&blob), ParsedPacket::OpaqueSvc(_)));
        assert!(matches!(parse_packet(&[1, 2, 3]), ParsedPacket::Unknown));
    }
}
