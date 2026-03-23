use criterion::{Criterion, criterion_group, criterion_main};
use dhvani::buffer::AudioBuffer;
use dhvani::dsp;
use dhvani::dsp::{
    BandType, BiquadFilter, Compressor, CompressorParams, EqBandConfig, FilterType, ParametricEq,
    Reverb, ReverbParams,
};

fn make_stereo_1s() -> AudioBuffer {
    let samples: Vec<f32> = (0..88200)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / 44100.0).sin() * 0.8)
        .collect();
    AudioBuffer::from_interleaved(samples, 2, 44100).unwrap()
}

fn bench_noise_gate_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    c.bench_function("noise_gate_stereo_1s", |bench| {
        bench.iter(|| dsp::noise_gate(&mut buf, 0.05))
    });
}

fn bench_compress_legacy_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let mut comp = Compressor::new(CompressorParams::default(), 44100).unwrap();
    c.bench_function("compress_stereo_1s", |bench| {
        bench.iter(|| comp.process(&mut buf))
    });
}

fn bench_normalize_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    c.bench_function("normalize_stereo_1s", |bench| {
        bench.iter(|| dsp::normalize(&mut buf, 0.95))
    });
}

fn bench_biquad_lp_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let mut filt = BiquadFilter::new(FilterType::LowPass, 5000.0, 0.707, 44100, 2);
    c.bench_function("biquad_lp_stereo_1s", |bench| {
        bench.iter(|| filt.process(&mut buf))
    });
}

fn bench_parametric_eq_3band_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let bands = vec![
        EqBandConfig {
            band_type: BandType::HighPass,
            freq_hz: 80.0,
            gain_db: 0.0,
            q: 0.707,
            enabled: true,
        },
        EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 3000.0,
            gain_db: 3.0,
            q: 1.5,
            enabled: true,
        },
        EqBandConfig {
            band_type: BandType::HighShelf,
            freq_hz: 10000.0,
            gain_db: -2.0,
            q: 0.707,
            enabled: true,
        },
    ];
    let mut eq = ParametricEq::new(bands, 44100, 2);
    c.bench_function("parametric_eq_3band_stereo_1s", |bench| {
        bench.iter(|| eq.process(&mut buf))
    });
}

fn bench_parametric_eq_10band_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let freqs = [
        31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
    ];
    let bands: Vec<EqBandConfig> = freqs
        .iter()
        .map(|&f| EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: f,
            gain_db: 3.0,
            q: 1.4,
            enabled: true,
        })
        .collect();
    let mut eq = ParametricEq::new(bands, 44100, 2);
    c.bench_function("parametric_eq_10band_stereo_1s", |bench| {
        bench.iter(|| eq.process(&mut buf))
    });
}

fn bench_compressor_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let mut comp = Compressor::new(
        CompressorParams {
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 3.0,
            knee_db: 6.0,
            ..Default::default()
        },
        44100,
    )
    .unwrap();
    c.bench_function("compressor_stereo_1s", |bench| {
        bench.iter(|| comp.process(&mut buf))
    });
}

fn bench_reverb_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let mut reverb = Reverb::new(
        ReverbParams {
            room_size: 0.6,
            damping: 0.4,
            mix: 0.3,
        },
        44100,
    )
    .unwrap();
    c.bench_function("reverb_stereo_1s", |bench| {
        bench.iter(|| reverb.process(&mut buf))
    });
}

fn bench_panner_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let panner = dhvani::dsp::StereoPanner::new(0.3);
    c.bench_function("panner_stereo_1s", |bench| {
        bench.iter(|| panner.process(&mut buf))
    });
}

fn bench_limiter_1s(c: &mut Criterion) {
    let mut buf = make_stereo_1s();
    let mut limiter = dhvani::dsp::EnvelopeLimiter::new(
        dhvani::dsp::LimiterParams {
            ceiling_db: -1.0,
            release_ms: 50.0,
            knee_db: 3.0,
            ..Default::default()
        },
        44100,
    )
    .unwrap();
    c.bench_function("limiter_stereo_1s", |bench| {
        bench.iter(|| limiter.process(&mut buf))
    });
}

criterion_group!(
    benches,
    bench_noise_gate_1s,
    bench_compress_legacy_1s,
    bench_normalize_1s,
    bench_biquad_lp_1s,
    bench_parametric_eq_3band_1s,
    bench_parametric_eq_10band_1s,
    bench_compressor_1s,
    bench_reverb_1s,
    bench_panner_1s,
    bench_limiter_1s,
);
criterion_main!(benches);
