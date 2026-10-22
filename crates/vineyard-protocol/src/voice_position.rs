//! Vineyard player voice coordinates for TendrilVoice SFU (plangem.md §5.1).

pub const VOICE_POSITION_MAGIC: [u8; 2] = [0x56, 0x50]; // "VP"

const MAX_DIMENSION_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub struct VoicePositionFrame {
    pub player_uuid: [u8; 16],
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub dimension: String,
}

pub fn pack_voice_position(frame: &VoicePositionFrame) -> Vec<u8> {
    let dim = frame.dimension.as_bytes();
    let dim_len = dim.len().min(MAX_DIMENSION_LEN) as u16;
    let mut out = Vec::with_capacity(2 + 16 + 8 * 3 + 4 * 2 + 2 + dim_len as usize);
    out.extend_from_slice(&VOICE_POSITION_MAGIC);
    out.extend_from_slice(&frame.player_uuid);
    out.extend_from_slice(&frame.x.to_le_bytes());
    out.extend_from_slice(&frame.y.to_le_bytes());
    out.extend_from_slice(&frame.z.to_le_bytes());
    out.extend_from_slice(&frame.yaw.to_le_bytes());
    out.extend_from_slice(&frame.pitch.to_le_bytes());
    out.extend_from_slice(&dim_len.to_le_bytes());
    out.extend_from_slice(&dim[..dim_len as usize]);
    out
}

pub fn try_unpack_voice_position(buf: &[u8]) -> Option<VoicePositionFrame> {
    if buf.len() < 2 + 16 + 24 + 8 + 2 || buf[0..2] != VOICE_POSITION_MAGIC {
        return None;
    }
    let mut off = 2;
    let mut player_uuid = [0u8; 16];
    player_uuid.copy_from_slice(&buf[off..off + 16]);
    off += 16;
    let x = f64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
    off += 8;
    let y = f64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
    off += 8;
    let z = f64::from_le_bytes(buf[off..off + 8].try_into().ok()?);
    off += 8;
    let yaw = f32::from_le_bytes(buf[off..off + 4].try_into().ok()?);
    off += 4;
    let pitch = f32::from_le_bytes(buf[off..off + 4].try_into().ok()?);
    off += 4;
    let dim_len = u16::from_le_bytes(buf[off..off + 2].try_into().ok()?) as usize;
    off += 2;
    if dim_len > MAX_DIMENSION_LEN || off + dim_len > buf.len() {
        return None;
    }
    let dimension = String::from_utf8(buf[off..off + dim_len].to_vec()).ok()?;
    Some(VoicePositionFrame {
        player_uuid,
        x,
        y,
        z,
        yaw,
        pitch,
        dimension,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_voice_position() {
        let frame = VoicePositionFrame {
            player_uuid: [3u8; 16],
            x: 12.5,
            y: 64.0,
            z: -7.25,
            yaw: 90.0,
            pitch: -10.0,
            dimension: "minecraft:overworld".into(),
        };
        let bytes = pack_voice_position(&frame);
        assert_eq!(try_unpack_voice_position(&bytes).as_ref(), Some(&frame));
    }
}
