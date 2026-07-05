use criterion::{Criterion, criterion_group, criterion_main};
use dhvani::buffer::resample::ResampleQuality;
use dhvani::buffer::{AudioBuffer, resample_linear};

fn bench_resample_linear_44100_to_48000(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 44100], 1, 44100).unwrap();
    c.bench_function("resample_linear_44100_48000_mono_1s", |bench| {
        bench.iter(|| resample_linear(&buf, 48000).unwrap())
    });
}

fn bench_resample_linear_48000_to_44100(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 96000], 2, 48000).unwrap();
    c.bench_function("resample_linear_48000_44100_stereo_1s", |bench| {
        bench.iter(|| resample_linear(&buf, 44100).unwrap())
    });
}

fn bench_resample_sinc_draft(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    c.bench_function("resample_sinc_draft_stereo_1s", |bench| {
        bench.iter(|| {
            dhvani::buffer::resample::resample_sinc(&buf, 48000, ResampleQuality::Draft).unwrap()
        })
    });
}

fn bench_resample_sinc_good(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    c.bench_function("resample_sinc_good_stereo_1s", |bench| {
        bench.iter(|| {
            dhvani::buffer::resample::resample_sinc(&buf, 48000, ResampleQuality::Good).unwrap()
        })
    });
}

fn bench_resample_sinc_best(c: &mut Criterion) {
    let buf = AudioBuffer::from_interleaved(vec![0.5; 88200], 2, 44100).unwrap();
    c.bench_function("resample_sinc_best_stereo_1s", |bench| {
        bench.iter(|| {
            dhvani::buffer::resample::resample_sinc(&buf, 48000, ResampleQuality::Best).unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_resample_linear_44100_to_48000,
    bench_resample_linear_48000_to_44100,
    bench_resample_sinc_draft,
    bench_resample_sinc_good,
    bench_resample_sinc_best,
);
criterion_main!(benches);
