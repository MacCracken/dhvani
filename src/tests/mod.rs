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
        },
        44100,
    );
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
    use crate::dsp::{self,
        BandType, Compressor, CompressorParams, DelayLine, EqBandConfig, ParametricEq, Reverb,
        ReverbParams,
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
        },
        sr,
    );
    comp.process(&mut buf);

    // Reverb
    let mut reverb = Reverb::new(
        ReverbParams {
            room_size: 0.3,
            damping: 0.5,
            mix: 0.2,
        },
        sr,
    );
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
    let spec = spectrum_dft(&resampled, 4096);
    let dominant = spec.dominant_frequency().unwrap();
    assert!(
        (dominant - 440.0).abs() < spec.freq_resolution * 2.0,
        "Dominant freq {dominant} should be near 440Hz"
    );
}
