#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::{AudioBuffer, mix};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let channels = (data[0] % 8).max(1) as u32;
    let sample_rate = 44100u32;

    // Interpret remaining bytes as f32 samples
    let sample_bytes = &data[1..];
    let num_f32 = sample_bytes.len() / 4;
    if num_f32 < channels as usize {
        return;
    }

    let samples: Vec<f32> = sample_bytes
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|f| f.is_finite())
        .collect();

    if samples.len() < channels as usize {
        return;
    }

    let frames = samples.len() / channels as usize;
    let truncated = samples[..frames * channels as usize].to_vec();

    if let Ok(buf) = AudioBuffer::from_interleaved(truncated.clone(), channels, sample_rate) {
        // Test mix with self
        let _ = mix(&[&buf, &buf]);

        // Test buffer operations
        let mut buf2 = buf.clone();
        buf2.apply_gain(0.5);
        buf2.clamp();
        let _ = buf2.peak();
        let _ = buf2.rms();
    }
});
