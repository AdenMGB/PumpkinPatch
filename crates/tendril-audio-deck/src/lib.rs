//! Native admin audio output for TendrilVoice (cpal + Opus decode).

mod decode;
mod output;

use output::run_playback_thread;
use std::path::Path;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

pub use decode::decode_opus_mono_48k;
pub use output::AudioOutputDevice;

#[derive(Debug, thiserror::Error)]
pub enum DeckError {
    #[error("no audio output devices available")]
    NoDevices,
    #[error("device not found: {0}")]
    DeviceNotFound(String),
    #[error("audio: {0}")]
    Audio(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

/// Lists cpal output devices (id is host-specific name string).
pub fn list_output_devices() -> Vec<AudioOutputDevice> {
    output::list_output_devices()
}

/// Live monitor: decodes incoming Opus frames and plays on the chosen output device.
pub struct AdminAudioDeck {
    opus_tx: Sender<Vec<u8>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl AdminAudioDeck {
    pub fn start(device_id: Option<&str>) -> Result<Self, DeckError> {
        if device_id.is_none() && list_output_devices().is_empty() {
            return Err(DeckError::NoDevices);
        }
        let (opus_tx, opus_rx) = mpsc::channel();
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let device = device_id.map(|s| s.to_string());
        let stop_flag = stop.clone();
        let thread = thread::spawn(move || {
            run_playback_thread(device.as_deref(), opus_rx, stop_flag);
        });
        Ok(Self {
            opus_tx,
            stop,
            thread: Some(thread),
        })
    }

    pub fn opus_sender(&self) -> Sender<Vec<u8>> {
        self.opus_tx.clone()
    }

    pub fn push_opus(&self, frame: &[u8]) {
        if frame.is_empty() {
            return;
        }
        let _ = self.opus_tx.send(frame.to_vec());
    }

    /// Replay flushed evidence (`frame_*.opus`) from a moderation dump directory.
    pub fn play_evidence_directory(&self, dir: &Path) -> Result<u32, DeckError> {
        let mut paths: Vec<_> = std::fs::read_dir(dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|x| x.to_str()) == Some("opus")
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with("frame_"))
                        .unwrap_or(false)
            })
            .collect();
        paths.sort();
        let mut count = 0u32;
        for path in paths {
            let bytes = std::fs::read(&path)?;
            self.push_opus(&bytes);
            count += 1;
        }
        Ok(count)
    }

    pub fn stop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

impl Drop for AdminAudioDeck {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Thread-safe handle for Tauri / SFU monitor wiring.
pub struct SharedAdminDeck {
    inner: Mutex<Option<AdminAudioDeck>>,
}

impl SharedAdminDeck {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(None),
        }
    }

    pub fn start(&self, device_id: Option<&str>) -> Result<Sender<Vec<u8>>, DeckError> {
        let mut guard = self.inner.lock().expect("deck lock");
        if let Some(deck) = guard.as_mut() {
            deck.stop();
        }
        let deck = AdminAudioDeck::start(device_id)?;
        let tx = deck.opus_sender();
        *guard = Some(deck);
        Ok(tx)
    }

    pub fn stop(&self) {
        let mut guard = self.inner.lock().expect("deck lock");
        if let Some(deck) = guard.as_mut() {
            deck.stop();
        }
        *guard = None;
    }

    pub fn push_opus(&self, frame: &[u8]) {
        let guard = self.inner.lock().expect("deck lock");
        if let Some(deck) = guard.as_ref() {
            deck.push_opus(frame);
        }
    }

    pub fn play_evidence_directory(&self, dir: &Path) -> Result<u32, DeckError> {
        let guard = self.inner.lock().expect("deck lock");
        let deck = guard
            .as_ref()
            .ok_or_else(|| DeckError::Audio("audio deck not started".into()))?;
        deck.play_evidence_directory(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opus_pure::{Application, MAX_PACKET_BYTES, OpusDecoder, OpusEncoder};

    #[test]
    fn opus_roundtrip_decode() {
        let mut enc = OpusEncoder::new(48_000, 1, Application::Audio).unwrap();
        let mut dec = OpusDecoder::new(48_000, 1).unwrap();
        let pcm = vec![0.0f32; 960];
        let mut packet = vec![0u8; MAX_PACKET_BYTES];
        let n = enc.encode(&pcm, 960, &mut packet).unwrap();
        let mut out = vec![0f32; 960];
        let samples = dec.decode(&packet[..n], 960, &mut out).unwrap();
        assert_eq!(samples, 960);
        assert!(decode_opus_mono_48k(&packet[..n]).is_ok());
    }

    #[test]
    fn list_devices_does_not_panic() {
        let _ = list_output_devices();
    }
}
