//! Scarecrow Shield — lightweight handshake validation (plangem.md §6.7).

use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct ScarecrowStats {
    pub allowed: AtomicU64,
    pub dropped: AtomicU64,
}

impl ScarecrowStats {
    pub fn record_allowed(&self) {
        self.allowed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_dropped(&self) {
        self.dropped.fetch_add(1, Ordering::Relaxed);
    }
}

/// Returns false when the handshake should be dropped before backend connect.
pub fn validate_handshake(
    protocol_version: i32,
    server_address: &str,
    next_state: u8,
    max_packet_bytes: usize,
    packet_len: usize,
) -> bool {
    if packet_len > max_packet_bytes {
        return false;
    }
    if protocol_version < 47 || protocol_version > 1_000 {
        return false;
    }
    if server_address.is_empty() || server_address.len() > 255 {
        return false;
    }
    if next_state > 2 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_hostname() {
        let long = "a".repeat(300);
        assert!(!validate_handshake(767, &long, 1, 32_768, 100));
    }

    #[test]
    fn accepts_status_ping() {
        assert!(validate_handshake(767, "localhost", 1, 32_768, 50));
    }
}
