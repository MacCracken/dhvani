#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::{AudioBuffer, resample_linear};
use dhvani::buffer::resample::{ResampleQuality, resample_sinc};

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let channels = (data[0] % 4).max(1) as u32;
    let target_rate = match data[1] % 4 {
        0 => 22050u32,
        1 => 44100,
        2 => 48000,
        _ => 96000,
    };

    let sample_bytes = &data[2..];
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

    if let Ok(buf) = AudioBuffer::from_interleaved(truncated, channels, 44100) {
        let _ = resample_linear(&buf, target_rate);
        let _ = resample_sinc(&buf, target_rate, ResampleQuality::Draft);
    }
});
