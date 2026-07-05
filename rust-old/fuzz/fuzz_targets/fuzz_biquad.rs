#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::AudioBuffer;
use dhvani::dsp::biquad::{BiquadFilter, FilterType};

fuzz_target!(|data: &[u8]| {
    if data.len() < 12 { return; }
    let freq = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let q = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    if !freq.is_finite() || !q.is_finite() || freq <= 0.0 || q <= 0.0 { return; }

    let filter_type = match data[8] % 5 {
        0 => FilterType::LowPass,
        1 => FilterType::HighPass,
        2 => FilterType::BandPass,
        3 => FilterType::Peaking { gain_db: (data[9] as f32 / 10.0) - 12.0 },
        _ => FilterType::Notch,
    };

    let sample_bytes = &data[10..];
    let samples: Vec<f32> = sample_bytes.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|f| f.is_finite())
        .collect();
    if samples.len() < 2 { return; }

    if let Ok(mut buf) = AudioBuffer::from_interleaved(samples, 1, 44100) {
        let mut filt = BiquadFilter::new(filter_type, freq.clamp(1.0, 20000.0), q.clamp(0.01, 100.0), 44100, 1);
        filt.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }
});
