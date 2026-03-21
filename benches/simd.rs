//! SIMD kernel benchmarks.
//!
//! Run with:
//!   cargo bench --bench simd               # SIMD path (default features)
//!   cargo bench --bench simd --no-default-features  # scalar fallback
//!
//! Compare results to measure SIMD speedup.

use criterion::{Criterion, criterion_group, criterion_main};
use nada::buffer::AudioBuffer;
use nada::buffer::convert;

fn make_stereo_1s() -> AudioBuffer {
    let samples: Vec<f32> = (0..88200)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / 44100.0).sin() * 0.8)
        .collect();
    AudioBuffer::from_interleaved(samples, 2, 44100).unwrap()
}

fn bench_apply_gain(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    c.bench_function("simd_apply_gain_stereo_1s", |b| {
        b.iter(|| buf.apply_gain(0.9))
    });
}

fn bench_clamp(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    // Push some samples over 1.0
    buf.apply_gain(1.5);
    c.bench_function("simd_clamp_stereo_1s", |b| {
        b.iter(|| buf.clamp())
    });
}

fn bench_peak(c: &mut Criterion) {
    let buf = make_stereo_1s();
    c.bench_function("simd_peak_stereo_1s", |b| {
        b.iter(|| buf.peak())
    });
}

fn bench_rms(c: &mut Criterion) {
    let buf = make_stereo_1s();
    c.bench_function("simd_rms_stereo_1s", |b| {
        b.iter(|| buf.rms())
    });
}

fn bench_mix_2(c: &mut Criterion) {
    let a = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    let b = AudioBuffer::from_interleaved(vec![0.3; 88200], 2, 44100).unwrap();
    c.bench_function("simd_mix_2_stereo_1s", |b_| {
        b_.iter(|| nada::buffer::mix(&[&a, &b]).unwrap())
    });
}

fn bench_noise_gate(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    c.bench_function("simd_noise_gate_stereo_1s", |b| {
        b.iter(|| nada::dsp::noise_gate(&mut buf, 0.01))
    });
}

fn bench_i16_to_f32(c: &mut Criterion) {
    let i16_data: Vec<i16> = (0..88200).map(|i| ((i % 65536) as i16)).collect();
    c.bench_function("simd_i16_to_f32_stereo_1s", |b| {
        b.iter(|| convert::i16_to_f32(&i16_data))
    });
}

fn bench_f32_to_i16(c: &mut Criterion) {
    let f32_data: Vec<f32> = (0..88200).map(|i| (i as f32 / 88200.0) * 2.0 - 1.0).collect();
    c.bench_function("simd_f32_to_i16_stereo_1s", |b| {
        b.iter(|| convert::f32_to_i16(&f32_data))
    });
}

fn bench_sinc_resample(c: &mut Criterion) {
    use nada::buffer::resample::{ResampleQuality, resample_sinc};
    let buf = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    c.bench_function("simd_sinc_resample_good_stereo_1s", |b| {
        b.iter(|| resample_sinc(&buf, 48000, ResampleQuality::Good).unwrap())
    });
}

criterion_group!(
    benches,
    bench_apply_gain,
    bench_clamp,
    bench_peak,
    bench_rms,
    bench_mix_2,
    bench_noise_gate,
    bench_i16_to_f32,
    bench_f32_to_i16,
    bench_sinc_resample,
);
criterion_main!(benches);
