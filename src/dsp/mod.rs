//! DSP effects — EQ, compressor, limiter, gate, noise suppression.

use serde::{Deserialize, Serialize};

use crate::NadaError;
use crate::buffer::AudioBuffer;

/// Parametric EQ band.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqBand {
    /// Center frequency in Hz.
    pub freq_hz: f32,
    /// Gain in dB (-24 to +24).
    pub gain_db: f32,
    /// Q factor (bandwidth).
    pub q: f32,
}

/// Apply a simple gain-based EQ band (approximation — proper biquad in v0.21).
pub fn apply_eq_band(_buf: &mut AudioBuffer, _band: &EqBand) -> Result<(), NadaError> {
    // Placeholder — full biquad filter implementation in next version
    Ok(())
}

/// Noise gate: silence samples below threshold.
pub fn noise_gate(buf: &mut AudioBuffer, threshold: f32) {
    for s in &mut buf.samples {
        if s.abs() < threshold {
            *s = 0.0;
        }
    }
}

/// Hard limiter: clamp samples at the given ceiling.
pub fn hard_limiter(buf: &mut AudioBuffer, ceiling: f32) {
    let ceiling = ceiling.abs();
    for s in &mut buf.samples {
        *s = s.clamp(-ceiling, ceiling);
    }
}

/// Simple compressor: reduce dynamic range above threshold.
pub fn compress(buf: &mut AudioBuffer, threshold: f32, ratio: f32) {
    if ratio <= 1.0 {
        return;
    }
    for s in &mut buf.samples {
        let abs = s.abs();
        if abs > threshold {
            let excess = abs - threshold;
            let compressed = threshold + excess / ratio;
            *s = compressed.copysign(*s);
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
    fn compressor_reduces_peaks() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, -1.0, 0.3], 1, 44100).unwrap();
        compress(&mut buf, 0.5, 4.0);
        // 1.0 above 0.5 threshold: excess=0.5, compressed=0.5+0.5/4=0.625
        assert!((buf.samples[0] - 0.625).abs() < 0.01);
        // 0.3 below threshold: unchanged
        assert!((buf.samples[2] - 0.3).abs() < f32::EPSILON);
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
