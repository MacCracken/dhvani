//! Property-based tests using proptest.

use proptest::prelude::*;

use crate::buffer::AudioBuffer;

// Strategy: generate valid audio buffers
fn arb_audio_buffer() -> impl Strategy<Value = AudioBuffer> {
    (
        1u32..=8,
        1usize..4096,
        prop::collection::vec(-1.0f32..=1.0, 1..32768),
    )
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
        _b in arb_audio_buffer(),
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
        prop_assert!(buf.samples.iter().all(|&s| (-1.0..=1.0).contains(&s)));
    }

    #[test]
    fn peak_is_non_negative(buf in arb_audio_buffer()) {
        prop_assert!(buf.peak() >= 0.0);
    }

    #[test]
    fn rms_is_non_negative(buf in arb_audio_buffer()) {
        prop_assert!(buf.rms() >= 0.0);
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn normalize_reaches_target(buf in arb_audio_buffer()) {
        let mut buf = buf;
        if buf.peak() > 0.0 {
            crate::dsp::normalize(&mut buf, 0.95);
            prop_assert!((buf.peak() - 0.95).abs() < 0.01);
        }
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn noise_gate_output_finite(buf in arb_audio_buffer(), threshold in 0.0f32..1.0) {
        let mut buf = buf;
        crate::dsp::noise_gate(&mut buf, threshold);
        prop_assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn hard_limiter_bounds_output(buf in arb_audio_buffer(), ceiling in 0.01f32..2.0) {
        let mut buf = buf;
        crate::dsp::hard_limiter(&mut buf, ceiling);
        prop_assert!(buf.samples.iter().all(|&s| s.abs() <= ceiling + f32::EPSILON));
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn db_roundtrip_accurate(amp in 0.001f32..10.0) {
        let db = crate::dsp::amplitude_to_db(amp);
        let back = crate::dsp::db_to_amplitude(db);
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

    #[cfg(feature = "midi")]
    #[test]
    fn velocity_roundtrip(v in 0u8..=127) {
        let v16 = crate::midi::translate::velocity_7_to_16(v);
        let back = crate::midi::translate::velocity_16_to_7(v16);
        prop_assert_eq!(back, v);
    }

    // ── Expanded coverage ──────────────────────────────────────────

    #[test]
    fn add_buffers_commutative(
        a in arb_audio_buffer(),
    ) {
        // a + a should equal 2 * a
        let sum = crate::buffer::mix(&[&a, &a]);
        if let Ok(mixed) = sum {
            for (i, s) in mixed.samples.iter().enumerate() {
                let expected = a.samples[i] * 2.0;
                prop_assert!((s - expected).abs() < 1e-4,
                    "mix sum[{i}] = {s}, expected {expected}");
            }
        }
    }

    #[test]
    fn i24_f32_roundtrip(values in prop::collection::vec(-8388608i32..=8388607, 1..1000)) {
        let f32s = crate::buffer::convert::i24_to_f32(&values);
        let back = crate::buffer::convert::f32_to_i24(&f32s);
        for (a, b) in values.iter().zip(back.iter()) {
            prop_assert!((*a - *b).abs() <= 1, "i24 roundtrip: {a} != {b}");
        }
    }

    #[test]
    fn u8_f32_roundtrip(values in prop::collection::vec(0u8..=255, 1..1000)) {
        let f32s = crate::buffer::convert::u8_to_f32(&values);
        let back = crate::buffer::convert::f32_to_u8(&f32s);
        for (a, b) in values.iter().zip(back.iter()) {
            prop_assert!((*a as i16 - *b as i16).abs() <= 1, "u8 roundtrip: {a} != {b}");
        }
    }

    #[cfg(feature = "simd")]
    #[test]
    fn simd_sum_of_squares_non_negative(buf in arb_audio_buffer()) {
        let result = crate::simd::sum_of_squares(&buf.samples);
        prop_assert!(result >= 0.0);
        prop_assert!(result.is_finite());
    }

    #[cfg(feature = "simd")]
    #[test]
    fn simd_weighted_sum_finite(
        samples in prop::collection::vec(-1.0f32..=1.0, 1..1000),
        weights in prop::collection::vec(0.0f32..=1.0, 1..1000),
    ) {
        let len = samples.len().min(weights.len());
        let (sum, wt) = crate::simd::weighted_sum(&samples[..len], &weights[..len]);
        prop_assert!(sum.is_finite(), "weighted_sum not finite: {sum}");
        prop_assert!(wt.is_finite(), "weight_sum not finite: {wt}");
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn svf_output_finite(
        buf in arb_audio_buffer(),
        freq in 20.0f32..20000.0,
        q in 0.1f32..10.0,
    ) {
        let mut buf = buf;
        let mut svf = crate::dsp::SvfFilter::new(
            crate::dsp::SvfMode::LowPass, freq, q, 0.0, 44100, buf.channels,
        );
        svf.process(&mut buf);
        prop_assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn automation_monotonic_linear(
        start_val in -10.0f32..10.0,
        end_val in -10.0f32..10.0,
        frames in 10usize..10000,
    ) {
        use crate::dsp::automation::{AutomationLane, Breakpoint, CurveType};
        let mut lane = AutomationLane::new(start_val);
        lane.add(Breakpoint::new(0, start_val, CurveType::Linear));
        lane.add(Breakpoint::new(frames, end_val, CurveType::Linear));

        let mut output = vec![0.0f32; frames + 1];
        lane.render(&mut output, 0);

        // All values should be between start and end (inclusive)
        let lo = start_val.min(end_val);
        let hi = start_val.max(end_val);
        for (i, &v) in output.iter().enumerate() {
            prop_assert!(v >= lo - 1e-5 && v <= hi + 1e-5,
                "automation[{i}] = {v}, expected [{lo}, {hi}]");
        }
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn routing_matrix_preserves_energy(
        buf in arb_audio_buffer(),
    ) {
        // Identity routing should preserve energy exactly
        let m = crate::dsp::RoutingMatrix::identity(buf.channels as usize);
        if let Ok(out) = m.apply(&buf) {
            for (a, b) in buf.samples.iter().zip(out.samples.iter()) {
                prop_assert!((a - b).abs() < 1e-6, "identity routing changed value");
            }
        }
    }

    #[cfg(feature = "analysis")]
    #[test]
    fn zcr_non_negative(buf in arb_audio_buffer()) {
        if buf.frames >= 2 {
            let result = crate::analysis::zero_crossing_rate(&buf).unwrap();
            prop_assert!(result.rate_hz >= 0.0);
            for &r in &result.per_channel {
                prop_assert!(r >= 0.0);
            }
        }
    }
}
