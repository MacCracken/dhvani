#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::AudioBuffer;
use dhvani::dsp;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 {
        return;
    }

    let channels = (data[0] % 4).max(1) as u32;

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

    if let Ok(mut buf) = AudioBuffer::from_interleaved(truncated, channels, 44100) {
        dsp::noise_gate(&mut buf, 0.01);
        dsp::hard_limiter(&mut buf, 0.95);
        dsp::normalize(&mut buf, 0.9);

        // Verify no NaN/Inf
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }
});
