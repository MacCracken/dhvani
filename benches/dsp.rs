use criterion::{Criterion, criterion_group, criterion_main};
use nada::buffer::AudioBuffer;
use nada::dsp;

fn bench_noise_gate_1s(c: &mut Criterion) {
    let mut buf = AudioBuffer::from_interleaved(vec![0.01; 88200], 2, 44100).unwrap();
    c.bench_function("noise_gate_stereo_1s", |bench| {
        bench.iter(|| dsp::noise_gate(&mut buf, 0.05))
    });
}

fn bench_compress_1s(c: &mut Criterion) {
    let mut buf = AudioBuffer::from_interleaved(vec![0.8; 88200], 2, 44100).unwrap();
    c.bench_function("compress_stereo_1s", |bench| {
        bench.iter(|| dsp::compress(&mut buf, 0.5, 4.0))
    });
}

fn bench_normalize_1s(c: &mut Criterion) {
    let mut buf = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    c.bench_function("normalize_stereo_1s", |bench| {
        bench.iter(|| dsp::normalize(&mut buf, 0.95))
    });
}

criterion_group!(
    benches,
    bench_noise_gate_1s,
    bench_compress_1s,
    bench_normalize_1s
);
criterion_main!(benches);
