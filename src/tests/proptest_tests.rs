//! Property-based tests using proptest.

use proptest::prelude::*;

use crate::buffer::AudioBuffer;
use crate::dsp;

// Strategy: generate valid audio buffers
fn arb_audio_buffer() -> impl Strategy<Value = AudioBuffer> {
    (1u32..=8, 1usize..4096, prop::collection::vec(-1.0f32..=1.0, 1..32768))
        .prop_filter_map("valid buffer", |(channels, _, samples)| {
            let len = samples.len();
            let frames = len / channels as usize;
            if frames == 0 {
                return None;
            }
            let truncated = samples[..frames * channels as usize].to_vec();
            AudioBuffer::from_interleaved(truncated, channels, 44100).ok()
        })
}

proptest! {
    #[test]
    fn mix_produces_finite_output(
        a in arb_audio_buffer(),
        b in arb_audio_buffer(),
    ) {
        // Mix only works if channels/rate match, so use same buffer twice
        let result = crate::buffer::mix(&[&a, &a]);
        if let Ok(mixed) = result {
            prop_assert!(mixed.samples.iter().all(|s| s.is_finite()));
        }
    }

    #[test]
    fn gain_preserves_finiteness(buf in arb_audio_buffer(), gain in -10.0f32..10.0) {
        let mut buf = buf;
        buf.apply_gain(gain);
        prop_assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn clamp_bounds_output(buf in arb_audio_buffer()) {
        let mut buf = buf;
        buf.clamp();
        prop_assert!(buf.samples.iter().all(|&s| s >= -1.0 && s <= 1.0));
    }

    #[test]
    fn peak_is_non_negative(buf in arb_audio_buffer()) {
        prop_assert!(buf.peak() >= 0.0);
    }

    #[test]
    fn rms_is_non_negative(buf in arb_audio_buffer()) {
        prop_assert!(buf.rms() >= 0.0);
    }

    #[test]
    fn normalize_reaches_target(buf in arb_audio_buffer()) {
        let mut buf = buf;
        if buf.peak() > 0.0 {
            dsp::normalize(&mut buf, 0.95);
            prop_assert!((buf.peak() - 0.95).abs() < 0.01);
        }
    }

    #[test]
    fn noise_gate_output_finite(buf in arb_audio_buffer(), threshold in 0.0f32..1.0) {
        let mut buf = buf;
        dsp::noise_gate(&mut buf, threshold);
        prop_assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn hard_limiter_bounds_output(buf in arb_audio_buffer(), ceiling in 0.01f32..2.0) {
        let mut buf = buf;
        dsp::hard_limiter(&mut buf, ceiling);
        prop_assert!(buf.samples.iter().all(|&s| s.abs() <= ceiling + f32::EPSILON));
    }

    #[test]
    fn db_roundtrip_accurate(amp in 0.001f32..10.0) {
        let db = dsp::amplitude_to_db(amp);
        let back = dsp::db_to_amplitude(db);
        prop_assert!((amp - back).abs() < amp * 0.001);
    }

    #[test]
    fn i16_f32_roundtrip(values in prop::collection::vec(-32768i16..=32767, 1..1000)) {
        let f32s = crate::buffer::convert::i16_to_f32(&values);
        let back = crate::buffer::convert::f32_to_i16(&f32s);
        for (a, b) in values.iter().zip(back.iter()) {
            prop_assert!((*a as i32 - *b as i32).abs() <= 1);
        }
    }

    #[test]
    fn velocity_roundtrip(v in 0u8..=127) {
        let v16 = crate::midi::translate::velocity_7_to_16(v);
        let back = crate::midi::translate::velocity_16_to_7(v16);
        prop_assert_eq!(back, v);
    }
}
