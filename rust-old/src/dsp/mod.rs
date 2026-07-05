//! DSP effects — EQ, compressor, limiter, gate, noise suppression, SVF, automation, routing.

pub mod automation;
pub mod biquad;
pub mod compressor;
#[cfg(feature = "analysis")]
pub mod convolution;
pub mod deesser;
pub mod delay;
pub mod envelope;
pub mod eq;
pub mod gain_smoother;
pub mod graphic_eq;
pub mod lfo;
pub mod limiter;
#[cfg(feature = "analysis")]
pub mod noise_reduction;
pub mod oscillator;
pub mod pan;
pub mod reverb;
pub mod routing;
pub mod svf;

use crate::buffer::AudioBuffer;

pub use automation::{AutomationLane, Breakpoint, CurveType};
pub use biquad::{BiquadCoeffs, BiquadFilter, FilterType};
pub use compressor::{Compressor, CompressorParams};
#[cfg(feature = "analysis")]
pub use convolution::ConvolutionReverb;
pub use deesser::{DeEsser, DeEsserParams};
pub use delay::{DelayLine, ModulatedDelay, ModulatedDelayParams};
pub use envelope::{AdsrParams, Envelope, EnvelopeState};
pub use eq::{BandType, EqBandConfig, ParametricEq};
pub use gain_smoother::{GainSmoother, GainSmootherParams};
pub use graphic_eq::{GraphicEq, GraphicEqSettings, ISO_BANDS};
pub use lfo::{Lfo, LfoShape};
pub use limiter::{EnvelopeLimiter, LimiterParams};
#[cfg(feature = "analysis")]
pub use noise_reduction::{NoiseReducer, noise_reduce};
pub use oscillator::{Oscillator, Waveform};
pub use pan::StereoPanner;
pub use reverb::{Reverb, ReverbParams};
pub use routing::RoutingMatrix;
pub use svf::{SvfFilter, SvfMode};

/// Noise gate: silence samples below threshold.
pub fn noise_gate(buf: &mut AudioBuffer, threshold: f32) {
    #[cfg(feature = "simd")]
    {
        crate::simd::noise_gate(&mut buf.samples, threshold);
    }
    #[cfg(not(feature = "simd"))]
    {
        for s in &mut buf.samples {
            if s.abs() < threshold {
                *s = 0.0;
            }
        }
    }
}

/// Hard limiter: clamp samples at the given ceiling.
pub fn hard_limiter(buf: &mut AudioBuffer, ceiling: f32) {
    let ceiling = ceiling.abs();
    #[cfg(feature = "simd")]
    {
        crate::simd::clamp(&mut buf.samples, -ceiling, ceiling);
    }
    #[cfg(not(feature = "simd"))]
    {
        for s in &mut buf.samples {
            *s = s.clamp(-ceiling, ceiling);
        }
    }
}

/// Normalize buffer to peak at target level.
pub fn normalize(buf: &mut AudioBuffer, target_peak: f32) {
    let peak = buf.peak();
    if peak > 0.0 {
        let gain = target_peak / peak;
        buf.apply_gain(gain);
    }
}

// Re-export core DSP math from abaco.
pub use abaco::dsp::{amplitude_to_db, db_to_amplitude, sanitize_sample};

/// Compute soft-knee gain reduction in dB.
///
/// Shared by [`Compressor`] and [`EnvelopeLimiter`] for the quadratic
/// interpolation region around the threshold.
///
/// - `env_db` — current envelope level in dB
/// - `threshold_db` — knee center (threshold or ceiling)
/// - `knee_db` — total knee width in dB (0 = hard knee)
/// - `slope` — gain slope in the fully-compressed region
///   (compressor: `1/ratio - 1`, limiter: `-1.0`)
///
/// Returns gain adjustment in dB (negative = reduction, 0 = no change).
#[inline]
pub fn soft_knee_gain(env_db: f32, threshold_db: f32, knee_db: f32, slope: f32) -> f32 {
    let half_knee = knee_db / 2.0;
    let lower = threshold_db - half_knee;
    let upper = threshold_db + half_knee;

    if env_db <= lower {
        0.0
    } else if env_db >= upper {
        slope * (env_db - threshold_db)
    } else {
        // Quadratic interpolation in knee region
        let x = env_db - lower;
        (slope * x * x) / (2.0 * knee_db)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_gate_silences_below_threshold() {
        let mut buf =
            AudioBuffer::from_interleaved(vec![0.01, -0.01, 0.5, -0.5], 1, 44100).unwrap();
        noise_gate(&mut buf, 0.1);
        assert_eq!(buf.samples[0], 0.0);
        assert_eq!(buf.samples[1], 0.0);
        assert!((buf.samples[2] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn hard_limiter_clamps() {
        let mut buf = AudioBuffer::from_interleaved(vec![2.0, -2.0, 0.5], 1, 44100).unwrap();
        hard_limiter(&mut buf, 1.0);
        assert_eq!(buf.samples[0], 1.0);
        assert_eq!(buf.samples[1], -1.0);
        assert!((buf.samples[2] - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn normalize_to_peak() {
        let mut buf = AudioBuffer::from_interleaved(vec![0.5, -0.25], 1, 44100).unwrap();
        normalize(&mut buf, 1.0);
        assert!((buf.peak() - 1.0).abs() < 0.01);
    }

    #[test]
    fn db_conversions() {
        assert!((amplitude_to_db(1.0)).abs() < 0.01);
        assert!((amplitude_to_db(0.5) - (-6.02)).abs() < 0.1);
        assert!((db_to_amplitude(0.0) - 1.0).abs() < 0.01);
        assert!((db_to_amplitude(-6.02) - 0.5).abs() < 0.01);
    }

    // ── soft_knee_gain tests ─────────────────────────────────────────────

    #[test]
    fn soft_knee_hard_below_threshold() {
        // Hard knee (knee=0), below threshold → no reduction
        assert_eq!(soft_knee_gain(-30.0, -20.0, 0.0, -0.75), 0.0);
    }

    #[test]
    fn soft_knee_hard_above_threshold() {
        // Hard knee, 10 dB above threshold, slope = 1/4 - 1 = -0.75
        let gain = soft_knee_gain(-10.0, -20.0, 0.0, -0.75);
        assert!((gain - (-7.5)).abs() < 0.01); // -0.75 * 10 = -7.5
    }

    #[test]
    fn soft_knee_quadratic_mid() {
        // Soft knee, at threshold center → half the full reduction
        let gain = soft_knee_gain(-20.0, -20.0, 12.0, -0.75);
        // At threshold with 12dB knee: x = half_knee = 6, slope*x²/(2*knee) = -0.75*36/24 = -1.125
        assert!((gain - (-1.125)).abs() < 0.01);
    }

    #[test]
    fn soft_knee_zero_knee_is_hard() {
        // knee=0 should behave identically to hard knee
        assert_eq!(soft_knee_gain(-25.0, -20.0, 0.0, -1.0), 0.0);
        let gain = soft_knee_gain(-15.0, -20.0, 0.0, -1.0);
        assert!((gain - (-5.0)).abs() < 0.01); // limiter: ceiling - env
    }

    // ── bypass tests ─────────────────────────────────────────────────────

    #[test]
    fn biquad_bypass() {
        let mut filt = BiquadFilter::new(FilterType::LowPass, 500.0, 0.707, 44100, 1);
        filt.set_bypass(true);
        assert!(filt.is_bypassed());
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 64], 1, 44100).unwrap();
        let original = buf.samples.clone();
        filt.process(&mut buf);
        assert_eq!(
            buf.samples, original,
            "bypassed filter should not modify signal"
        );
    }

    #[test]
    fn compressor_bypass() {
        let mut comp = Compressor::new(
            CompressorParams {
                threshold_db: -40.0,
                ratio: 10.0,
                ..Default::default()
            },
            44100,
        )
        .unwrap();
        comp.set_bypass(true);
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 64], 1, 44100).unwrap();
        let original = buf.samples.clone();
        comp.process(&mut buf);
        assert_eq!(buf.samples, original);
    }

    #[test]
    fn limiter_bypass() {
        let mut lim = EnvelopeLimiter::new(
            LimiterParams {
                ceiling_db: -20.0,
                ..Default::default()
            },
            44100,
        )
        .unwrap();
        lim.set_bypass(true);
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 64], 1, 44100).unwrap();
        let original = buf.samples.clone();
        lim.process(&mut buf);
        assert_eq!(buf.samples, original);
    }

    // ── set_sample_rate tests ────────────────────────────────────────────

    #[test]
    fn biquad_set_sample_rate() {
        let mut filt = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100, 1);
        filt.set_sample_rate(48000);
        // Should not panic, coefficients rebuilt
        let mut buf = AudioBuffer::from_interleaved(vec![0.5; 64], 1, 48000).unwrap();
        filt.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn compressor_set_sample_rate() {
        let mut comp = Compressor::new(CompressorParams::default(), 44100).unwrap();
        comp.set_sample_rate(48000);
        let mut buf = AudioBuffer::from_interleaved(vec![0.5; 64], 1, 48000).unwrap();
        comp.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    // ── AudioClock getter tests ──────────────────────────────────────────

    #[test]
    fn clock_getters() {
        let clock = crate::clock::AudioClock::with_tempo(48000, 120.0);
        assert_eq!(clock.sample_rate(), 48000);
        assert_eq!(clock.position_samples(), 0);
        assert!((clock.tempo_bpm() - 120.0).abs() < f64::EPSILON);
        assert!(!clock.is_running());
    }

    #[test]
    fn clock_set_tempo() {
        let mut clock = crate::clock::AudioClock::with_tempo(44100, 120.0);
        clock.set_tempo(140.0);
        assert!((clock.tempo_bpm() - 140.0).abs() < f64::EPSILON);
    }

    // ── latency_frames tests ─────────────────────────────────────────────

    #[test]
    fn delay_latency_frames() {
        let delay = DelayLine::new(10.0, 10.0, 0.0, 1.0, 44100, 1);
        let expected = ((10.0 / 1000.0) * 44100.0) as usize;
        assert_eq!(delay.latency_frames(), expected);
    }

    #[test]
    fn modulated_delay_latency_frames() {
        let md = ModulatedDelay::new(ModulatedDelayParams::default(), 44100, 1);
        assert!(md.latency_frames() > 0);
    }

    // ── ParametricEq set_params test ─────────────────────────────────────

    #[test]
    fn parametric_eq_set_params() {
        let mut eq = ParametricEq::new(vec![], 44100, 1);
        assert_eq!(eq.band_count(), 0);
        eq.set_params(vec![
            EqBandConfig {
                band_type: BandType::Peaking,
                freq_hz: 1000.0,
                gain_db: 6.0,
                q: 1.0,
                enabled: true,
            },
            EqBandConfig {
                band_type: BandType::HighShelf,
                freq_hz: 8000.0,
                gain_db: -3.0,
                q: 0.707,
                enabled: true,
            },
        ]);
        assert_eq!(eq.band_count(), 2);
        assert_eq!(eq.band(0).unwrap().band_type, BandType::Peaking);
    }

    // ── ModulatedDelay set_params test ────────────────────────────────────

    #[test]
    fn modulated_delay_set_params() {
        let mut md = ModulatedDelay::new(ModulatedDelayParams::default(), 44100, 1);
        md.set_params(ModulatedDelayParams {
            base_delay_ms: 1.0,
            depth_ms: 0.5,
            rate_hz: 2.0,
            feedback: 0.5,
            mix: 0.8,
        });
        let mut buf = AudioBuffer::from_interleaved(vec![0.5; 256], 1, 44100).unwrap();
        md.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }
}
