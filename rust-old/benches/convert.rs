use criterion::{Criterion, criterion_group, criterion_main};
use dhvani::buffer::AudioBuffer;
use dhvani::buffer::convert::{
    f32_to_i16, i16_to_f32, interleaved_to_planar, mono_to_stereo, stereo_to_mono,
};

fn bench_i16_f32_roundtrip(c: &mut Criterion) {
    let i16_samples: Vec<i16> = (0..88200).map(|i| (i % 65536 - 32768) as i16).collect();
    c.bench_function("i16_f32_roundtrip_stereo_1s", |bench| {
        bench.iter(|| {
            let f32s = i16_to_f32(&i16_samples);
            f32_to_i16(&f32s)
        })
    });
}

fn bench_interleaved_to_planar(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    c.bench_function("interleaved_to_planar_stereo_1s", |bench| {
        bench.iter(|| interleaved_to_planar(&buf))
    });
}

fn bench_mono_to_stereo(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 44100], 1, 44100).unwrap();
    c.bench_function("mono_to_stereo_1s", |bench| {
        bench.iter(|| mono_to_stereo(&buf).unwrap())
    });
}

fn bench_stereo_to_mono(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    c.bench_function("stereo_to_mono_1s", |bench| {
        bench.iter(|| stereo_to_mono(&buf).unwrap())
    });
}

criterion_group!(
    benches,
    bench_i16_f32_roundtrip,
    bench_interleaved_to_planar,
    bench_mono_to_stereo,
    bench_stereo_to_mono,
);
criterion_main!(benches);
