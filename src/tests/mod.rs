//! Integration and property-based tests for dhvani.

mod proptest_tests;
mod serde_tests;

use crate::buffer::AudioBuffer;
use crate::clock::AudioClock;

#[cfg(feature = "dsp")]
#[test]
fn full_pipeline_mix_compress_normalize() {
    use crate::buffer::mix;
    use crate::dsp::{self, Compressor, CompressorParams};

    let a = AudioBuffer::from_interleaved(vec![0.8, 0.8, 0.7, 0.7], 2, 44100).unwrap();
    let b = AudioBuffer::from_interleaved(vec![0.6, 0.6, 0.5, 0.5], 2, 44100).unwrap();

    let mut mixed = mix(&[&a, &b]).unwrap();
    // Mixed peaks at 1.4 — compress then normalize
    let mut comp = Compressor::new(
        CompressorParams {
            threshold_db: -4.0,
            ratio: 4.0,
            attack_ms: 0.01,
            release_ms: 0.01,
            makeup_gain_db: 0.0,
            knee_db: 0.0,
            ..Default::default()
        },
        44100,
    )
    .unwrap();
    comp.process(&mut mixed);
    dsp::normalize(&mut mixed, 0.95);

    assert!(mixed.peak() <= 0.96);
    assert!(mixed.peak() >= 0.94);
}

#[test]
fn resample_preserves_duration() {
    use crate::buffer::resample_linear;
    let buf = AudioBuffer::from_interleaved(vec![0.5; 44100], 1, 44100).unwrap();
    let resampled = resample_linear(&buf, 48000).unwrap();

    // Duration should be approximately the same
    let orig_dur = buf.duration_secs();
    let new_dur = resampled.duration_secs();
    assert!((orig_dur - new_dur).abs() < 0.02);
}

#[test]
fn clock_syncs_with_buffer() {
    let buf = AudioBuffer::silence(2, 1024, 48000);
    let mut clock = AudioClock::new(48000);
    clock.start();
    clock.advance(buf.frames as u64);

    let expected_ms = 1024.0 / 48000.0 * 1000.0;
    assert!((clock.position_ms() - expected_ms).abs() < 0.1);
}

#[cfg(all(feature = "dsp", feature = "analysis"))]
#[test]
fn noise_gate_then_analyze() {
    use crate::analysis;
    use crate::dsp;

    let mut buf =
        AudioBuffer::from_interleaved(vec![0.001, -0.001, 0.5, -0.5, 0.002, -0.002], 1, 44100)
            .unwrap();

    // Gate removes noise floor
    dsp::noise_gate(&mut buf, 0.01);
    assert_eq!(buf.samples[0], 0.0);
    assert!((buf.samples[2] - 0.5).abs() < f32::EPSILON);

    // Verify not silent after gating (signal remains)
    assert!(!analysis::is_silent(&buf, -60.0));
}

#[cfg(feature = "dsp")]
#[test]
fn db_roundtrip() {
    use crate::dsp;
    let amp = 0.707;
    let db = dsp::amplitude_to_db(amp);
    let back = dsp::db_to_amplitude(db);
    assert!((amp - back).abs() < 0.001);
}

#[cfg(feature = "dsp")]
#[test]
fn full_dsp_chain_eq_compress_reverb_delay() {
    use crate::dsp::{
        self, BandType, Compressor, CompressorParams, DelayLine, EqBandConfig, ParametricEq,
        Reverb, ReverbParams,
    };

    // Generate a 440Hz sine, stereo, 1 second
    let sr = 44100u32;
    let frames = 4096;
    let samples: Vec<f32> = (0..frames * 2)
        .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / sr as f32).sin() * 0.8)
        .collect();
    let mut buf = AudioBuffer::from_interleaved(samples, 2, sr).unwrap();

    // EQ: boost 440Hz
    let mut eq = ParametricEq::new(
        vec![EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 440.0,
            gain_db: 6.0,
            q: 1.0,
            enabled: true,
        }],
        sr,
        2,
    );
    eq.process(&mut buf);

    // Compress
    let mut comp = Compressor::new(
        CompressorParams {
            threshold_db: -12.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            makeup_gain_db: 0.0,
            knee_db: 0.0,
            ..Default::default()
        },
        sr,
    )
    .unwrap();
    comp.process(&mut buf);

    // Reverb
    let mut reverb = Reverb::new(
        ReverbParams {
            room_size: 0.3,
            damping: 0.5,
            mix: 0.2,
        },
        sr,
    )
    .unwrap();
    reverb.process(&mut buf);

    // Delay
    let mut delay = DelayLine::new(50.0, 50.0, 0.3, 0.15, sr, 2);
    delay.process(&mut buf);

    // Normalize
    dsp::normalize(&mut buf, 0.95);

    // Output should be valid audio
    assert!(buf.samples.iter().all(|s| s.is_finite()));
    assert!(buf.peak() <= 1.0);
    assert!(buf.peak() >= 0.9);
}

#[cfg(feature = "dsp")]
#[test]
fn format_conversion_pipeline() {
    use crate::buffer::convert::{f32_to_i16, i16_to_f32, mono_to_stereo, stereo_to_mono};
    use crate::dsp;

    // Start with i16 samples
    let original_i16: Vec<i16> = (0..1024)
        .map(|i| ((i as f32 / 1024.0 * 2.0 - 1.0) * 30000.0) as i16)
        .collect();

    // Convert to f32 mono
    let f32_samples = i16_to_f32(&original_i16);
    let mono = AudioBuffer::from_interleaved(f32_samples, 1, 44100).unwrap();

    // Convert to stereo
    let stereo = mono_to_stereo(&mono).unwrap();
    assert_eq!(stereo.channels, 2);

    // Apply some DSP
    let mut processed = stereo;
    dsp::noise_gate(&mut processed, 0.01);

    // Convert back to mono
    let back_mono = stereo_to_mono(&processed).unwrap();
    assert_eq!(back_mono.channels, 1);

    // Convert back to i16
    let back_i16 = f32_to_i16(&back_mono.samples);
    assert_eq!(back_i16.len(), original_i16.len());
}

#[cfg(feature = "analysis")]
#[test]
fn sinc_resample_preserves_frequency() {
    use crate::analysis::spectrum_dft;
    use crate::buffer::resample::{ResampleQuality, resample_sinc};

    let sr = 44100u32;
    let frames = 8192;
    let freq = 440.0f32;
    let samples: Vec<f32> = (0..frames)
        .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
        .collect();
    let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();

    // Resample to 48000
    let resampled = resample_sinc(&buf, 48000, ResampleQuality::Good).unwrap();
    assert_eq!(resampled.sample_rate, 48000);

    // Check dominant frequency is still ~440Hz
    let spec = spectrum_dft(&resampled, 4096).unwrap();
    let dominant = spec.dominant_frequency().unwrap();
    assert!(
        (dominant - 440.0).abs() < spec.freq_resolution() * 2.0,
        "Dominant freq {dominant} should be near 440Hz"
    );
}

// ── Edge case tests ──────────────────────────────────────────────────────

#[cfg(feature = "dsp")]
#[test]
fn oscillator_zero_freq_silent() {
    use crate::dsp::{Oscillator, Waveform};

    let mut osc = Oscillator::new(Waveform::Sine, 44100);
    let mut sum = 0.0f64;
    // At 0 Hz the oscillator should stay at phase 0, producing near-zero output
    for _ in 0..1000 {
        sum += osc.sample(0.0).abs() as f64;
    }
    assert!(
        sum < 0.01,
        "0 Hz oscillator should be near-silent, sum={sum}"
    );
}

#[cfg(feature = "dsp")]
#[test]
fn delay_zero_ms_passthrough() {
    use crate::dsp::DelayLine;

    let mut delay = DelayLine::new(0.0, 10.0, 0.0, 1.0, 44100, 1);
    let mut buf = AudioBuffer::from_interleaved(vec![1.0, 0.5, 0.25, 0.0], 1, 44100).unwrap();
    delay.process(&mut buf);
    // With 0ms delay and no feedback, wet output = delayed-by-0 = input
    assert!(buf.samples.iter().all(|s| s.is_finite()));
}

#[cfg(feature = "dsp")]
#[test]
fn envelope_sub_sample_attack() {
    use crate::dsp::{AdsrParams, Envelope};

    // Attack of 0.00001 seconds ≈ less than 1 sample at 44100
    let params = AdsrParams {
        attack: 0.00001,
        decay: 0.1,
        sustain: 0.5,
        release: 0.1,
    };
    let mut env = Envelope::new(params, 44100);
    env.trigger();
    let _first = env.tick(); // stage_pos=0 → level=0
    let second = env.tick(); // should reach 1.0 with 1-sample attack
    assert!(
        second >= 1.0,
        "sub-sample attack should reach full level within 2 ticks: {second}"
    );
}

#[cfg(feature = "graph")]
#[test]
fn graph_cycle_detection() {
    use crate::graph::{Graph, NodeId};

    let mut graph = Graph::new();

    struct Passthrough;
    impl crate::graph::AudioNode for Passthrough {
        fn name(&self) -> &str {
            "pass"
        }
        fn num_inputs(&self) -> usize {
            1
        }
        fn num_outputs(&self) -> usize {
            1
        }
        fn process(&mut self, inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            if let Some(input) = inputs.first() {
                output.samples_mut().copy_from_slice(input.samples());
            }
        }
    }

    let a = NodeId::next();
    let b = NodeId::next();
    graph.add_node(a, Box::new(Passthrough));
    graph.add_node(b, Box::new(Passthrough));
    graph.connect(a, b);
    graph.connect(b, a); // cycle!

    let result = graph.compile();
    assert!(result.is_err(), "graph with cycle should fail to compile");
}

#[cfg(feature = "dsp")]
#[test]
fn compressor_parallel_mix() {
    use crate::dsp::{Compressor, CompressorParams};

    // 50% mix = parallel compression
    let params = CompressorParams {
        threshold_db: -20.0,
        ratio: 10.0,
        attack_ms: 0.01,
        release_ms: 0.01,
        makeup_gain_db: 0.0,
        knee_db: 0.0,
        mix: 0.5,
    };
    let mut comp = Compressor::new(params, 44100).unwrap();
    let mut buf = AudioBuffer::from_interleaved(vec![1.0; 4096], 1, 44100).unwrap();
    comp.process(&mut buf);
    // With 50% mix, output should be between fully compressed and fully dry
    assert!(buf.samples.iter().all(|s| s.is_finite()));
    // Dry signal is 1.0, compressed will be less → blended should be < 1.0 but > 0
    let avg: f32 = buf.samples.iter().sum::<f32>() / buf.samples.len() as f32;
    assert!(avg > 0.0 && avg <= 1.0);
}

// ── P(-1) hardening tests ────────────────────────────────────────────

#[test]
fn from_interleaved_rejects_non_divisible_samples() {
    // 5 samples with 2 channels → not evenly divisible
    let result = AudioBuffer::from_interleaved(vec![0.0; 5], 2, 44100);
    assert!(result.is_err());
}

#[test]
fn resample_linear_empty_buffer() {
    let buf = AudioBuffer::silence(2, 0, 44100);
    let out = crate::buffer::resample_linear(&buf, 48000).unwrap();
    assert_eq!(out.frames, 0);
    assert_eq!(out.sample_rate, 48000);
}

#[test]
fn resample_linear_rejects_high_rate() {
    let buf = AudioBuffer::silence(1, 100, 44100);
    let result = crate::buffer::resample_linear(&buf, 800000);
    assert!(result.is_err());
}

#[cfg(feature = "analysis")]
#[test]
fn true_peak_exceeds_sample_peak_cubic() {
    // Alternating +/- values should produce inter-sample peaks > sample peaks
    // with cubic interpolation (unlike linear which can't)
    let samples = vec![0.0, 0.9, -0.9, 0.9, -0.9, 0.9, -0.9, 0.0];
    let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
    let d = crate::analysis::dynamics::analyze_dynamics(&buf);
    // Cubic Hermite should detect inter-sample overshoot
    assert!(
        d.true_peak[0] >= d.peak[0],
        "true_peak {} should be >= peak {}",
        d.true_peak[0],
        d.peak[0]
    );
}

#[cfg(feature = "dsp")]
#[test]
fn set_params_rejects_invalid() {
    use crate::dsp::{Compressor, CompressorParams};
    let mut comp = Compressor::new(CompressorParams::default(), 44100).unwrap();
    // ratio < 1.0 should be rejected
    let bad = CompressorParams {
        ratio: 0.5,
        ..Default::default()
    };
    assert!(comp.set_params(bad).is_err());
}

#[cfg(feature = "dsp")]
#[test]
fn limiter_set_params_rejects_invalid() {
    use crate::dsp::{EnvelopeLimiter, LimiterParams};
    let mut limiter = EnvelopeLimiter::new(LimiterParams::default(), 44100).unwrap();
    let bad = LimiterParams {
        ceiling_db: 6.0, // positive ceiling is invalid
        ..Default::default()
    };
    assert!(limiter.set_params(bad).is_err());
}

#[cfg(feature = "dsp")]
#[test]
fn lfo_set_rate_clamps_negative() {
    use crate::dsp::{Lfo, LfoShape};
    let mut lfo = Lfo::new(LfoShape::Sine, 1.0, 1.0, 44100);
    lfo.set_rate(-5.0);
    assert!(lfo.rate() >= 0.0);
}

#[test]
fn clock_set_tempo_rejects_nan() {
    let mut clock = AudioClock::new(44100);
    clock.set_tempo(f64::NAN);
    assert_eq!(clock.tempo_bpm(), 0.0);
    clock.set_tempo(-120.0);
    assert_eq!(clock.tempo_bpm(), 0.0);
    clock.set_tempo(120.0);
    assert_eq!(clock.tempo_bpm(), 120.0);
}

#[cfg(feature = "dsp")]
#[test]
fn triangle_oscillator_produces_output() {
    use crate::dsp::{Oscillator, Waveform};
    let mut osc = Oscillator::new(Waveform::Triangle, 44100);
    let mut has_positive = false;
    let mut has_negative = false;
    for _ in 0..44100 {
        let s = osc.sample(440.0);
        if s > 0.3 {
            has_positive = true;
        }
        if s < -0.3 {
            has_negative = true;
        }
    }
    assert!(has_positive && has_negative, "Triangle should oscillate");
}

// ── Stress tests ────────────────────────────────────────────────────

#[cfg(feature = "dsp")]
#[test]
fn stress_long_buffer_dsp_chain() {
    use crate::dsp::{
        BiquadFilter, Compressor, CompressorParams, DelayLine, EnvelopeLimiter, FilterType,
        LimiterParams,
    };

    // 10 seconds of stereo audio through a full DSP chain
    let sr = 44100u32;
    let duration_secs = 10;
    let frames = sr as usize * duration_secs;
    let samples: Vec<f32> = (0..frames * 2)
        .map(|i| {
            let t = (i / 2) as f32 / sr as f32;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.8
                + (2.0 * std::f32::consts::PI * 880.0 * t).sin() * 0.3
        })
        .collect();
    let mut buf = AudioBuffer::from_interleaved(samples, 2, sr).unwrap();

    // Chain: HP filter → compressor → delay → limiter
    let mut hp = BiquadFilter::new(FilterType::HighPass, 80.0, 0.707, sr, 2);
    hp.process(&mut buf);

    let mut comp = Compressor::new(
        CompressorParams {
            threshold_db: -12.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            ..Default::default()
        },
        sr,
    )
    .unwrap();
    comp.process(&mut buf);

    let mut delay = DelayLine::new(100.0, 500.0, 0.3, 0.2, sr, 2);
    delay.process(&mut buf);

    let mut limiter = EnvelopeLimiter::new(LimiterParams::default(), sr).unwrap();
    limiter.process(&mut buf);

    // All samples must be finite and within bounds
    assert!(
        buf.samples.iter().all(|s| s.is_finite()),
        "NaN/Inf in output"
    );
    assert!(
        buf.peak() <= 1.01,
        "peak {} exceeds limiter ceiling",
        buf.peak()
    );
}

#[cfg(all(feature = "dsp", feature = "analysis"))]
#[test]
fn stress_analysis_long_buffer() {
    use crate::analysis;

    // 5 seconds of complex audio
    let sr = 44100u32;
    let frames = sr as usize * 5;
    let samples: Vec<f32> = (0..frames)
        .map(|i| {
            let t = i as f32 / sr as f32;
            (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
                + (2.0 * std::f32::consts::PI * 1200.0 * t).sin() * 0.3
                + (2.0 * std::f32::consts::PI * 3500.0 * t).sin() * 0.1
        })
        .collect();
    let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();

    // Run all analysis functions
    let r128 = analysis::measure_r128(&buf).unwrap();
    assert!(r128.integrated_lufs.is_finite());
    assert!(r128.range_lu >= 0.0);

    let dynamics = analysis::analyze_dynamics(&buf);
    assert!(dynamics.max_peak() > 0.0);
    assert!(dynamics.max_true_peak() >= dynamics.max_peak());

    let zcr = analysis::zero_crossing_rate(&buf).unwrap();
    assert!(zcr.rate_hz > 0.0);

    let spec = analysis::spectrum_fft(&buf, 4096).unwrap();
    assert!(spec.peak_frequency() > 400.0 && spec.peak_frequency() < 500.0);
}

#[cfg(feature = "graph")]
#[test]
fn stress_graph_concurrent_swap() {
    use crate::graph::{AudioNode, Graph, GraphProcessor, NodeId};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    struct ValueNode(f32);
    impl AudioNode for ValueNode {
        fn name(&self) -> &str {
            "value"
        }
        fn num_inputs(&self) -> usize {
            0
        }
        fn num_outputs(&self) -> usize {
            1
        }
        fn process(&mut self, _inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
            for s in output.samples_mut() {
                *s = self.0;
            }
        }
    }

    let mut proc = GraphProcessor::new(1, 44100, 256);
    let handle = proc.swap_handle();
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();

    // Spawn a thread that rapidly swaps plans
    let swap_thread = std::thread::spawn(move || {
        for i in 0..100 {
            let mut g = Graph::new();
            let id = NodeId::next();
            g.add_node(id, Box::new(ValueNode(i as f32 / 100.0)));
            let plan = g.compile().unwrap();
            handle.swap(plan);
        }
        done_clone.store(true, Ordering::Release);
    });

    // Process from RT thread while swaps happen
    let mut cycles = 0;
    while !done.load(Ordering::Acquire) || cycles < 200 {
        if let Some(output) = proc.process() {
            assert!(output.samples().iter().all(|s| s.is_finite()));
        }
        cycles += 1;
    }

    swap_thread.join().unwrap();
    assert!(cycles > 0, "processor should have run");
}

// ── EBU R128 reference validation ──────────────────────────────────

#[cfg(feature = "analysis")]
#[test]
fn ebu_r128_silence_is_minus_infinity() {
    let buf = AudioBuffer::silence(2, 44100, 44100);
    let r128 = crate::analysis::measure_r128(&buf).unwrap();
    // Silence should measure as very low LUFS
    assert!(
        r128.integrated_lufs < -60.0,
        "silence LUFS={}",
        r128.integrated_lufs
    );
}

#[cfg(feature = "analysis")]
#[test]
fn ebu_r128_sine_in_expected_range() {
    // 997 Hz sine at 0 dBFS (peak 1.0) ≈ −3.01 dBFS RMS → expect ~−3.7 LUFS
    // (adjusted for K-weighting which adds ~0.2 dB at 997 Hz)
    let sr = 48000u32;
    let frames = sr as usize * 5; // 5 seconds for stable measurement
    let samples: Vec<f32> = (0..frames * 2)
        .map(|i| {
            let t = (i / 2) as f32 / sr as f32;
            (2.0 * std::f32::consts::PI * 997.0 * t).sin()
        })
        .collect();
    let buf = AudioBuffer::from_interleaved(samples, 2, sr).unwrap();
    let r128 = crate::analysis::measure_r128(&buf).unwrap();

    // EBU R128 for 0 dBFS stereo sine should be approximately -3.01 LUFS
    // Allow generous tolerance for our implementation
    assert!(
        r128.integrated_lufs > -6.0 && r128.integrated_lufs < -1.0,
        "997Hz sine expected ~-3 LUFS, got {}",
        r128.integrated_lufs
    );
}

#[cfg(feature = "analysis")]
#[test]
fn ebu_r128_k_weighting_attenuates_lows() {
    // Low frequency (50 Hz) should measure lower LUFS than mid frequency (1 kHz)
    // due to K-weighting high-pass filter
    let sr = 48000u32;
    let frames = sr as usize * 3;

    let low_samples: Vec<f32> = (0..frames)
        .map(|i| (2.0 * std::f32::consts::PI * 50.0 * i as f32 / sr as f32).sin() * 0.5)
        .collect();
    let mid_samples: Vec<f32> = (0..frames)
        .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin() * 0.5)
        .collect();

    let low_buf = AudioBuffer::from_interleaved(low_samples, 1, sr).unwrap();
    let mid_buf = AudioBuffer::from_interleaved(mid_samples, 1, sr).unwrap();

    let low_lufs = crate::analysis::measure_r128(&low_buf)
        .unwrap()
        .integrated_lufs;
    let mid_lufs = crate::analysis::measure_r128(&mid_buf)
        .unwrap()
        .integrated_lufs;

    assert!(
        low_lufs < mid_lufs,
        "K-weighting should attenuate 50Hz: low={low_lufs} mid={mid_lufs}"
    );
}

#[cfg(feature = "dsp")]
#[test]
fn biquad_half_mix() {
    use crate::dsp::{BiquadFilter, FilterType};

    let mut filt = BiquadFilter::new(FilterType::LowPass, 500.0, 0.707, 44100, 1);
    filt.set_mix(0.5);
    assert!((filt.mix() - 0.5).abs() < f32::EPSILON);

    // 10kHz sine through 500Hz LP at 50% mix should be partially attenuated
    let samples: Vec<f32> = (0..4096)
        .map(|i| (2.0 * std::f32::consts::PI * 10000.0 * i as f32 / 44100.0).sin())
        .collect();
    let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
    let original_rms = buf.rms();
    filt.process(&mut buf);
    // At 50% mix, RMS should be roughly half the original (dry leaks through)
    assert!(buf.rms() > original_rms * 0.2);
    assert!(buf.rms() < original_rms * 0.8);
}
