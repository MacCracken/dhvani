use criterion::{Criterion, criterion_group, criterion_main};
use nada::buffer::{AudioBuffer, resample_linear};

fn bench_resample_44100_to_48000(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 44100], 1, 44100).unwrap();
    c.bench_function("resample_44100_48000_mono_1s", |bench| {
        bench.iter(|| resample_linear(&buf, 48000).unwrap())
    });
}

fn bench_resample_48000_to_44100(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 96000], 2, 48000).unwrap();
    c.bench_function("resample_48000_44100_stereo_1s", |bench| {
        bench.iter(|| resample_linear(&buf, 44100).unwrap())
    });
}

criterion_group!(
    benches,
    bench_resample_44100_to_48000,
    bench_resample_48000_to_44100
);
criterion_main!(benches);
