//! Velocity `MODERN` player info forwarding (HMAC-SHA256).

use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Builds the signed `velocity:player_info` payload (version 1).
pub fn modern_player_info(secret: &str, address: &str, uuid: Uuid, username: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.push(0x01); // version
    write_uuid(&mut data, uuid);
    write_string(&mut data, username);
    write_properties(&mut data, address);
    let sig = sign(secret, &data);
    let mut out = Vec::with_capacity(1 + data.len() + sig.len());
    out.push(0x01);
    out.extend_from_slice(&data);
    out.extend_from_slice(&sig);
    out
}

fn sign(secret: &str, data: &[u8]) -> [u8; 32] {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

fn write_uuid(buf: &mut Vec<u8>, uuid: Uuid) {
    buf.extend_from_slice(uuid.as_bytes());
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_var_int(buf, bytes.len() as i32);
    buf.extend_from_slice(bytes);
}

fn write_properties(buf: &mut Vec<u8>, address: &str) {
    write_var_int(buf, 1);
    write_string(buf, "forwarding");
    write_string(buf, address);
    write_var_int(buf, 0);
}

fn write_var_int(buf: &mut Vec<u8>, value: i32) {
    let mut val = value as u32;
    loop {
        let mut temp = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            temp |= 0x80;
        }
        buf.push(temp);
        if val == 0 {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_packet_has_signature() {
        let packet = modern_player_info(
            "test-secret",
            "127.0.0.1",
            Uuid::nil(),
            "Steve",
        );
        assert!(packet.len() > 40);
        assert_eq!(packet[0], 0x01);
    }
}
