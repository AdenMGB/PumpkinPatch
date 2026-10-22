use crate::decode::decode_opus_mono_48k;
use crate::DeckError;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, SampleFormat, StreamConfig};
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Serialize)]
pub struct AudioOutputDevice {
    pub id: String,
    pub name: String,
}

pub fn list_output_devices() -> Vec<AudioOutputDevice> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    if let Ok(devices) = host.devices() {
        for device in devices {
            if device.default_output_config().is_err() {
                continue;
            }
            let name = device.name().unwrap_or_else(|_| "output".into());
            out.push(AudioOutputDevice {
                id: name.clone(),
                name,
            });
        }
    }
    out
}

struct SampleQueue {
    samples: VecDeque<f32>,
    max_len: usize,
}

impl SampleQueue {
    fn new(max_len: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(4096),
            max_len,
        }
    }

    fn push_pcm(&mut self, pcm: &[f32]) {
        for &s in pcm {
            if self.samples.len() >= self.max_len {
                self.samples.pop_front();
            }
            self.samples.push_back(s.clamp(-1.0, 1.0));
        }
    }

    fn pop_frame(&mut self, out: &mut [f32]) {
        for slot in out.iter_mut() {
            *slot = self.samples.pop_front().unwrap_or(0.0);
        }
    }
}

struct PlaybackEngine {
    stream: cpal::Stream,
    queue: Arc<Mutex<SampleQueue>>,
}

impl PlaybackEngine {
    fn open(device_id: Option<&str>) -> Result<Self, DeckError> {
        let host = cpal::default_host();
        let device = if let Some(id) = device_id {
            host.devices()
                .map_err(|e| DeckError::Audio(e.to_string()))?
                .find(|d| d.name().map(|n| n == id).unwrap_or(false))
                .ok_or_else(|| DeckError::DeviceNotFound(id.to_string()))?
        } else {
            host.default_output_device().ok_or(DeckError::NoDevices)?
        };

        let config = device
            .default_output_config()
            .map_err(|e| DeckError::Audio(e.to_string()))?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels() as usize;
        let queue = Arc::new(Mutex::new(SampleQueue::new(sample_rate as usize * channels * 2)));
        let stream = build_stream(&device, &config, queue.clone())?;
        stream
            .play()
            .map_err(|e| DeckError::Audio(e.to_string()))?;
        Ok(Self { stream, queue })
    }

    fn run_loop(self, opus_rx: Receiver<Vec<u8>>, stop: Arc<AtomicBool>) {
        while !stop.load(Ordering::Relaxed) {
            match opus_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(packet) => {
                    if let Ok(pcm) = decode_opus_mono_48k(&packet) {
                        if let Ok(mut q) = self.queue.lock() {
                            q.push_pcm(&pcm);
                        }
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        drop(self.stream);
    }
}

pub fn run_playback_thread(
    device_id: Option<&str>,
    opus_rx: Receiver<Vec<u8>>,
    stop: Arc<AtomicBool>,
) {
    if let Ok(engine) = PlaybackEngine::open(device_id) {
        engine.run_loop(opus_rx, stop);
    }
}

fn build_stream(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    queue: Arc<Mutex<SampleQueue>>,
) -> Result<cpal::Stream, DeckError> {
    let channels = config.channels() as usize;
    let sample_format = config.sample_format();
    let stream_config = StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: BufferSize::Default,
    };

    let err_fn = |e| eprintln!("[tendril-audio-deck] stream error: {e}");

    let stream = match sample_format {
        SampleFormat::F32 => device
            .build_output_stream(
                &stream_config,
                move |data: &mut [f32], _| fill_interleaved(data, channels, &queue),
                err_fn,
                None,
            )
            .map_err(|e| DeckError::Audio(e.to_string()))?,
        SampleFormat::I16 => device
            .build_output_stream(
                &stream_config,
                move |data: &mut [i16], _| {
                    let mut tmp = vec![0f32; data.len()];
                    fill_interleaved(&mut tmp, channels, &queue);
                    for (out, s) in data.iter_mut().zip(tmp.iter()) {
                        *out = f32_to_i16(*s);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| DeckError::Audio(e.to_string()))?,
        SampleFormat::U16 => device
            .build_output_stream(
                &stream_config,
                move |data: &mut [u16], _| {
                    let mut tmp = vec![0f32; data.len()];
                    fill_interleaved(&mut tmp, channels, &queue);
                    for (out, s) in data.iter_mut().zip(tmp.iter()) {
                        *out = f32_to_u16(*s);
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| DeckError::Audio(e.to_string()))?,
        _ => {
            return Err(DeckError::Audio(format!(
                "unsupported sample format {sample_format}"
            )));
        }
    };
    Ok(stream)
}

fn f32_to_i16(s: f32) -> i16 {
    (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

fn f32_to_u16(s: f32) -> u16 {
    let x = (s.clamp(-1.0, 1.0) + 1.0) * 0.5;
    (x * u16::MAX as f32) as u16
}

fn fill_interleaved(data: &mut [f32], channels: usize, queue: &Arc<Mutex<SampleQueue>>) {
    if channels == 0 {
        return;
    }
    let frames = data.len() / channels;
    let mut mono = vec![0f32; frames];
    if let Ok(mut q) = queue.lock() {
        q.pop_frame(&mut mono);
    }
    for (i, frame) in mono.iter().enumerate() {
        for ch in 0..channels {
            data[i * channels + ch] = *frame;
        }
    }
}
