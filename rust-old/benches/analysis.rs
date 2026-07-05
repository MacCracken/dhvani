use criterion::{Criterion, criterion_group, criterion_main};
use dhvani::analysis;
use dhvani::buffer::AudioBuffer;

fn make_mono_1s() -> AudioBuffer {
    let samples: Vec<f32> = (0..44100)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.8)
        .collect();
    AudioBuffer::from_interleaved(samples, 1, 44100).unwrap()
}

fn make_stereo_1s() -> AudioBuffer {
    let samples: Vec<f32> = (0..88200)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / 44100.0).sin() * 0.8)
        .collect();
    AudioBuffer::from_interleaved(samples, 2, 44100).unwrap()
}

fn bench_fft_4096(c: &mut Criterion) {
    let buf = make_mono_1s();
    c.bench_function("fft_4096_mono", |b| {
        b.iter(|| analysis::spectrum_fft(&buf, 4096).unwrap())
    });
}

fn bench_dft_4096(c: &mut Criterion) {
    let buf = make_mono_1s();
    c.bench_function("dft_4096_mono", |b| {
        b.iter(|| analysis::spectrum_dft(&buf, 4096).unwrap())
    });
}

fn bench_stft_1s(c: &mut Criterion) {
    let buf = make_mono_1s();
    c.bench_function("stft_2048_512_mono_1s", |b| {
        b.iter(|| analysis::compute_stft(&buf, 2048, 512).unwrap())
    });
}

fn bench_r128_stereo_1s(c: &mut Criterion) {
    let buf = make_stereo_1s();
    c.bench_function("r128_stereo_1s", |b| {
        b.iter(|| analysis::measure_r128(&buf).unwrap())
    });
}

fn bench_dynamics_1s(c: &mut Criterion) {
    let buf = make_mono_1s();
    c.bench_function("dynamics_mono_1s", |b| {
        b.iter(|| analysis::analyze_dynamics(&buf))
    });
}

fn bench_chromagram(c: &mut Criterion) {
    let buf = make_mono_1s();
    c.bench_function("chromagram_4096_mono", |b| {
        b.iter(|| analysis::chromagram(&buf, 4096).unwrap())
    });
}

fn bench_onset_detection(c: &mut Criterion) {
    let buf = make_mono_1s();
    c.bench_function("onset_2048_512_mono_1s", |b| {
        b.iter(|| analysis::detect_onsets(&buf, 2048, 512, 0.3).unwrap())
    });
}

criterion_group!(
    benches,
    bench_fft_4096,
    bench_dft_4096,
    bench_stft_1s,
    bench_r128_stereo_1s,
    bench_dynamics_1s,
    bench_chromagram,
    bench_onset_detection,
);
criterion_main!(benches);
