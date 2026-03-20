//! Integration tests for nada.

use crate::buffer::{AudioBuffer, mix, resample_linear};
use crate::clock::AudioClock;
use crate::dsp;

#[test]
fn full_pipeline_mix_compress_normalize() {
    let a = AudioBuffer::from_interleaved(vec![0.8, 0.8, 0.7, 0.7], 2, 44100).unwrap();
    let b = AudioBuffer::from_interleaved(vec![0.6, 0.6, 0.5, 0.5], 2, 44100).unwrap();

    let mut mixed = mix(&[&a, &b]).unwrap();
    // Mixed peaks at 1.4 — compress then normalize
    dsp::compress(&mut mixed, 0.8, 4.0);
    dsp::normalize(&mut mixed, 0.95);

    assert!(mixed.peak() <= 0.96);
    assert!(mixed.peak() >= 0.94);
}

#[test]
fn resample_preserves_duration() {
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

#[test]
fn noise_gate_then_analyze() {
    use crate::analysis;

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

#[test]
fn db_roundtrip() {
    let amp = 0.707;
    let db = dsp::amplitude_to_db(amp);
    let back = dsp::db_to_amplitude(db);
    assert!((amp - back).abs() < 0.001);
}
