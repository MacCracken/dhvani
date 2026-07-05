#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::AudioBuffer;
use dhvani::dsp::noise_reduce;

fuzz_target!(|data: &[u8]| {
    if data.len() < 16 { return; }

    let strength = (data[0] as f32) / 255.0;
    let samples: Vec<f32> = data[1..].chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|f| f.is_finite())
        .collect();

    if samples.len() < 4 { return; }

    if let Ok(mut buf) = AudioBuffer::from_interleaved(samples, 1, 44100) {
        noise_reduce(&mut buf, strength);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }
});
