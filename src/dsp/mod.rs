//! DSP effects — EQ, compressor, limiter, gate, noise suppression.

pub mod biquad;
pub mod compressor;
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

use crate::buffer::AudioBuffer;

pub use biquad::{BiquadCoeffs, BiquadFilter, FilterType};
pub use compressor::{Compressor, CompressorParams};
pub use deesser::{DeEsser, DeEsserParams};
pub use delay::{DelayLine, ModulatedDelay, ModulatedDelayParams};
pub use envelope::{AdsrParams, Envelope, EnvelopeState};
pub use eq::{BandType, EqBandConfig, ParametricEq};
pub use gain_smoother::{GainSmoother, GainSmootherParams};
pub use graphic_eq::{GraphicEq, GraphicEqSettings, ISO_BANDS};
pub use lfo::{Lfo, LfoShape};
pub use limiter::{EnvelopeLimiter, LimiterParams};
#[cfg(feature = "analysis")]
pub use noise_reduction::noise_reduce;
pub use oscillator::{Oscillator, Waveform};
pub use pan::StereoPanner;
pub use reverb::{Reverb, ReverbParams};

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

/// Convert amplitude to decibels.
pub fn amplitude_to_db(amplitude: f32) -> f32 {
    if amplitude <= 0.0 {
        return f32::NEG_INFINITY;
    }
    20.0 * amplitude.log10()
}

/// Convert decibels to amplitude.
pub fn db_to_amplitude(db: f32) -> f32 {
    10.0f32.powf(db / 20.0)
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
}
