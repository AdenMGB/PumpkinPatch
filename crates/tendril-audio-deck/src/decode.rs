use opus_pure::{MAX_PACKET_SAMPLES, OpusDecoder};

/// Decode one Opus packet (48 kHz mono) into interleaved f32 samples.
pub fn decode_opus_mono_48k(packet: &[u8]) -> Result<Vec<f32>, String> {
    if packet.is_empty() {
        return Ok(Vec::new());
    }
    let mut decoder = OpusDecoder::new(48_000, 1).map_err(|e| e.to_string())?;
    let mut out = vec![0f32; MAX_PACKET_SAMPLES];
    let samples = decoder
        .decode(packet, 960, &mut out)
        .map_err(|e| e.to_string())?;
    out.truncate(samples);
    Ok(out)
}

/// Encode silence for tests / calibration tones.
#[cfg(test)]
pub fn encode_silence_packet() -> Vec<u8> {
    use opus_pure::{Application, MAX_PACKET_BYTES};
    let mut enc = opus_pure::OpusEncoder::new(48_000, 1, Application::Audio).unwrap();
    let pcm = vec![0.0f32; 960];
    let mut packet = vec![0u8; MAX_PACKET_BYTES];
    let n = enc.encode(&pcm, 960, &mut packet).unwrap();
    packet.truncate(n);
    packet
}
