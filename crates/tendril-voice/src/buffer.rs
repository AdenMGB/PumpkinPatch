//! Rolling per-speaker Opus capture (plangem.md §5.1 — 60s RAM ring).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

const DEFAULT_RETENTION: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct SpeakerRingBuffer {
    chunks: VecDeque<(Instant, Vec<u8>)>,
    retention: Duration,
}

impl SpeakerRingBuffer {
    pub fn new() -> Self {
        Self {
            chunks: VecDeque::new(),
            retention: DEFAULT_RETENTION,
        }
    }

    pub fn push(&mut self, frame: Vec<u8>) {
        let now = Instant::now();
        self.chunks.push_back((now, frame));
        while let Some((t, _)) = self.chunks.front() {
            if now.duration_since(*t) > self.retention {
                self.chunks.pop_front();
            } else {
                break;
            }
        }
    }

    pub fn flush(&mut self) -> Vec<Vec<u8>> {
        self.chunks.iter().map(|(_, b)| b.clone()).collect()
    }

    pub fn packet_count(&self) -> u32 {
        self.chunks.len() as u32
    }

    pub fn byte_count(&self) -> u64 {
        self.chunks.iter().map(|(_, b)| b.len() as u64).sum()
    }
}

#[cfg(test)]
impl SpeakerRingBuffer {
    fn with_retention(retention: Duration) -> Self {
        Self {
            chunks: VecDeque::new(),
            retention,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_drops_old_chunks() {
        let mut ring = SpeakerRingBuffer::with_retention(Duration::from_millis(50));
        ring.push(vec![1]);
        std::thread::sleep(Duration::from_millis(60));
        ring.push(vec![2]);
        let flushed = ring.flush();
        assert_eq!(flushed.len(), 1);
        assert_eq!(flushed[0], vec![2]);
    }
}
