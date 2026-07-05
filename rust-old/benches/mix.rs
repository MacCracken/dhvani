use criterion::{Criterion, criterion_group, criterion_main};
use dhvani::buffer::{AudioBuffer, mix};

fn bench_mix_stereo_1s(c: &mut Criterion) {
    let a = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    let b = AudioBuffer::from_interleaved(vec![0.3; 88200], 2, 44100).unwrap();
    c.bench_function("mix_2_stereo_1s", |bench| {
        bench.iter(|| mix(&[&a, &b]).unwrap())
    });
}

fn bench_mix_8_sources(c: &mut Criterion) {
    let bufs: Vec<AudioBuffer> = (0..8)
        .map(|_| AudioBuffer::from_interleaved(vec![0.1; 88200], 2, 44100).unwrap())
        .collect();
    let refs: Vec<&AudioBuffer> = bufs.iter().collect();
    c.bench_function("mix_8_stereo_1s", |bench| {
        bench.iter(|| mix(&refs).unwrap())
    });
}

criterion_group!(benches, bench_mix_stereo_1s, bench_mix_8_sources);
criterion_main!(benches);
