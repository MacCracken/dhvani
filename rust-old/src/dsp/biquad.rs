//! Biquad filter primitives — coefficients, per-channel state, processing.
//!
//! Implements the standard Robert Bristow-Johnson Audio EQ Cookbook formulas
//! for all common filter types. Coefficients computed in f64 for precision.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;
use abaco::dsp::{angular_frequency, db_gain_factor};

/// Filter type with associated parameters.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum FilterType {
    /// Second-order low-pass filter.
    LowPass,
    /// Second-order high-pass filter.
    HighPass,
    /// Band-pass filter (constant skirt gain).
    BandPass,
    /// Notch (band-reject) filter.
    Notch,
    /// All-pass filter (phase shift only).
    AllPass,
    /// Peaking EQ filter.
    Peaking {
        /// Boost/cut in decibels.
        gain_db: f32,
    },
    /// Low-shelf filter.
    LowShelf {
        /// Boost/cut in decibels.
        gain_db: f32,
    },
    /// High-shelf filter.
    HighShelf {
        /// Boost/cut in decibels.
        gain_db: f32,
    },
}

/// Biquad filter coefficients (Direct Form II Transposed).
#[must_use]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BiquadCoeffs {
    /// Feedforward coefficient b0.
    pub b0: f64,
    /// Feedforward coefficient b1.
    pub b1: f64,
    /// Feedforward coefficient b2.
    pub b2: f64,
    /// Feedback coefficient a1 (negated/normalized).
    pub a1: f64,
    /// Feedback coefficient a2 (negated/normalized).
    pub a2: f64,
}

impl BiquadCoeffs {
    /// Design filter coefficients using Bristow-Johnson Audio EQ Cookbook.
    pub fn design(filter_type: FilterType, freq_hz: f32, q: f32, sample_rate: u32) -> Self {
        let sr = sample_rate as f64;
        let f0 = (freq_hz as f64).clamp(1.0, sr * 0.499);
        let q = (q as f64).max(0.01);
        let w0 = angular_frequency(f0, sr);
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let alpha = sin_w0 / (2.0 * q);

        let (b0, b1, b2, a0, a1, a2) = match filter_type {
            FilterType::LowPass => {
                let b1 = 1.0 - cos_w0;
                let b0 = b1 / 2.0;
                let b2 = b0;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterType::HighPass => {
                let b1 = -(1.0 + cos_w0);
                let b0 = (1.0 + cos_w0) / 2.0;
                let b2 = b0;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterType::Notch => {
                let b0 = 1.0;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterType::AllPass => {
                let b0 = 1.0 - alpha;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 + alpha;
                (b0, b1, b2, 1.0 + alpha, -2.0 * cos_w0, 1.0 - alpha)
            }
            FilterType::Peaking { gain_db } => {
                let a = db_gain_factor(gain_db as f64);
                let b0 = 1.0 + alpha * a;
                let b1 = -2.0 * cos_w0;
                let b2 = 1.0 - alpha * a;
                (b0, b1, b2, 1.0 + alpha / a, -2.0 * cos_w0, 1.0 - alpha / a)
            }
            FilterType::LowShelf { gain_db } => {
                let a = db_gain_factor(gain_db as f64);
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = 2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighShelf { gain_db } => {
                let a = db_gain_factor(gain_db as f64);
                let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
                let b0 = a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha);
                let b1 = -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0);
                let b2 = a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha);
                let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha;
                let a1 = 2.0 * ((a - 1.0) - (a + 1.0) * cos_w0);
                let a2 = (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        // Normalize by a0
        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        }
    }

    /// Unity (pass-through) coefficients.
    pub fn unity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
        }
    }
}

/// Per-channel biquad state (Direct Form II Transposed).
#[derive(Debug, Clone, Default)]
struct BiquadState {
    z1: f64,
    z2: f64,
}

impl BiquadState {
    #[inline]
    fn process(&mut self, input: f64, c: &BiquadCoeffs) -> f64 {
        let out = c.b0 * input + self.z1;
        self.z1 = c.b1 * input - c.a1 * out + self.z2;
        self.z2 = c.b2 * input - c.a2 * out;
        out
    }
}

/// A biquad filter with per-channel state.
#[must_use]
#[derive(Debug, Clone)]
pub struct BiquadFilter {
    coeffs: BiquadCoeffs,
    states: Vec<BiquadState>,
    filter_type: FilterType,
    freq_hz: f32,
    q: f32,
    sample_rate: u32,
    bypassed: bool,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    mix: f32,
}

impl BiquadFilter {
    /// Create a new biquad filter.
    pub fn new(
        filter_type: FilterType,
        freq_hz: f32,
        q: f32,
        sample_rate: u32,
        channels: u32,
    ) -> Self {
        tracing::debug!(
            ?filter_type,
            freq_hz,
            q,
            sample_rate,
            channels,
            "BiquadFilter::new"
        );
        Self {
            coeffs: BiquadCoeffs::design(filter_type, freq_hz, q, sample_rate),
            states: vec![BiquadState::default(); channels as usize],
            filter_type,
            freq_hz,
            q,
            sample_rate,
            bypassed: false,
            mix: 1.0,
        }
    }

    /// Set whether this filter is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this filter is currently bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Process an entire audio buffer in-place.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.bypassed {
            return;
        }
        let ch = buf.channels as usize;
        let mix = self.mix;

        // Fast path: stereo, full wet, SIMD available — use cross-channel SIMD biquad
        #[cfg(feature = "simd")]
        if ch == 2 && (mix - 1.0).abs() < f32::EPSILON {
            let coeffs = [
                self.coeffs.b0,
                self.coeffs.b1,
                self.coeffs.b2,
                self.coeffs.a1,
                self.coeffs.a2,
            ];
            let mut state = [
                self.states[0].z1,
                self.states[0].z2,
                self.states[1].z1,
                self.states[1].z2,
            ];
            crate::simd::biquad_stereo(&mut buf.samples, &coeffs, &mut state);
            self.states[0].z1 = state[0];
            self.states[0].z2 = state[1];
            self.states[1].z1 = state[2];
            self.states[1].z2 = state[3];
            return;
        }

        let dry = 1.0 - mix;
        for frame in 0..buf.frames {
            for c in 0..ch {
                let idx = frame * ch + c;
                let input = buf.samples[idx] as f64;
                let wet = self.states[c].process(input, &self.coeffs) as f32;
                buf.samples[idx] = buf.samples[idx] * dry + wet * mix;
            }
        }
    }

    /// Process a single sample for a given channel.
    #[inline]
    #[must_use]
    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        if channel < self.states.len() {
            self.states[channel].process(sample as f64, &self.coeffs) as f32
        } else {
            sample
        }
    }

    /// Reset all filter state (e.g., on seek).
    pub fn reset(&mut self) {
        for s in &mut self.states {
            s.z1 = 0.0;
            s.z2 = 0.0;
        }
    }

    /// Update filter parameters without resetting state (for smooth automation).
    pub fn set_params(&mut self, filter_type: FilterType, freq_hz: f32, q: f32) {
        self.filter_type = filter_type;
        self.freq_hz = freq_hz;
        self.q = q;
        self.coeffs = BiquadCoeffs::design(filter_type, freq_hz, q, self.sample_rate);
    }

    /// Current filter type.
    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }

    /// Current center frequency in Hz.
    pub fn freq_hz(&self) -> f32 {
        self.freq_hz
    }

    /// Current Q factor.
    pub fn q(&self) -> f32 {
        self.q
    }

    /// Set the dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Current dry/wet mix.
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Update the sample rate and recompute coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        self.coeffs = BiquadCoeffs::design(self.filter_type, self.freq_hz, self.q, sample_rate);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(freq: f32, sample_rate: u32, frames: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin())
            .collect();
        AudioBuffer::from_interleaved(samples, 1, sample_rate).unwrap()
    }

    #[test]
    fn silence_passthrough() {
        let mut buf = AudioBuffer::silence(2, 1024, 44100);
        let mut filt = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100, 2);
        filt.process(&mut buf);
        assert!(buf.peak() < f32::EPSILON);
    }

    #[test]
    fn unity_coeffs_passthrough() {
        let coeffs = BiquadCoeffs::unity();
        let mut state = BiquadState::default();
        let out = state.process(0.5, &coeffs);
        assert!((out - 0.5).abs() < 1e-10);
    }

    #[test]
    fn lowpass_attenuates_high_frequencies() {
        // 10kHz sine through 500Hz low-pass should be heavily attenuated
        let mut buf = make_sine(10000.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt = BiquadFilter::new(FilterType::LowPass, 500.0, 0.707, 44100, 1);
        filt.process(&mut buf);
        assert!(buf.rms() < original_rms * 0.1, "LP should attenuate 10kHz");
    }

    #[test]
    fn highpass_attenuates_low_frequencies() {
        // 100Hz sine through 5kHz high-pass should be heavily attenuated
        let mut buf = make_sine(100.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt = BiquadFilter::new(FilterType::HighPass, 5000.0, 0.707, 44100, 1);
        filt.process(&mut buf);
        assert!(buf.rms() < original_rms * 0.1, "HP should attenuate 100Hz");
    }

    #[test]
    fn peaking_boosts_target_frequency() {
        let mut buf = make_sine(1000.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt =
            BiquadFilter::new(FilterType::Peaking { gain_db: 12.0 }, 1000.0, 1.0, 44100, 1);
        filt.process(&mut buf);
        // After transient settles, RMS should be higher
        assert!(buf.rms() > original_rms * 1.5, "Peaking should boost 1kHz");
    }

    #[test]
    fn notch_attenuates_target_frequency() {
        let mut buf = make_sine(1000.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt = BiquadFilter::new(FilterType::Notch, 1000.0, 10.0, 44100, 1);
        filt.process(&mut buf);
        assert!(
            buf.rms() < original_rms * 0.2,
            "Notch should attenuate 1kHz"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut filt = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100, 2);
        let mut buf = make_sine(440.0, 44100, 256);
        filt.process(&mut buf);
        filt.reset();
        // After reset, process_sample should behave as fresh
        let out = filt.process_sample(0.0, 0);
        assert!(out.abs() < f32::EPSILON);
    }

    #[test]
    fn set_params_updates_coefficients() {
        let mut filt = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100, 1);
        filt.set_params(FilterType::HighPass, 5000.0, 1.0);
        assert_eq!(filt.filter_type(), FilterType::HighPass);
        assert!((filt.freq_hz() - 5000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn stereo_channels_independent() {
        let samples = vec![1.0, 0.0, 0.5, 0.0, 0.25, 0.0, 0.0, 0.0];
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        let mut filt = BiquadFilter::new(FilterType::LowPass, 10000.0, 0.707, 44100, 2);
        filt.process(&mut buf);
        // Right channel was all zeros, should remain near zero
        for frame in 0..buf.frames {
            assert!(
                buf.samples[frame * 2 + 1].abs() < 0.01,
                "Right channel should stay near zero"
            );
        }
    }

    #[test]
    fn extreme_q_does_not_panic() {
        let mut buf = make_sine(440.0, 44100, 256);
        let mut filt = BiquadFilter::new(FilterType::BandPass, 20000.0, 100.0, 44100, 1);
        filt.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn allpass_preserves_magnitude() {
        let mut buf = make_sine(1000.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt = BiquadFilter::new(FilterType::AllPass, 1000.0, 0.707, 44100, 1);
        filt.process(&mut buf);
        // AllPass should preserve magnitude (only change phase)
        assert!(
            (buf.rms() - original_rms).abs() < original_rms * 0.05,
            "AllPass should preserve RMS: {} vs {}",
            buf.rms(),
            original_rms
        );
    }

    #[test]
    fn low_shelf_boosts_lows() {
        let mut buf = make_sine(200.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt = BiquadFilter::new(
            FilterType::LowShelf { gain_db: 12.0 },
            500.0,
            0.707,
            44100,
            1,
        );
        filt.process(&mut buf);
        assert!(
            buf.rms() > original_rms * 1.5,
            "Low shelf should boost 200Hz"
        );
    }

    #[test]
    fn high_shelf_boosts_highs() {
        let mut buf = make_sine(8000.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt = BiquadFilter::new(
            FilterType::HighShelf { gain_db: 12.0 },
            5000.0,
            0.707,
            44100,
            1,
        );
        filt.process(&mut buf);
        assert!(
            buf.rms() > original_rms * 1.5,
            "High shelf should boost 8kHz"
        );
    }

    #[test]
    fn low_shelf_cuts_lows() {
        let mut buf = make_sine(200.0, 44100, 4096);
        let original_rms = buf.rms();
        let mut filt = BiquadFilter::new(
            FilterType::LowShelf { gain_db: -12.0 },
            500.0,
            0.707,
            44100,
            1,
        );
        filt.process(&mut buf);
        assert!(buf.rms() < original_rms * 0.5, "Low shelf should cut 200Hz");
    }

    #[test]
    fn process_sample_out_of_range_channel() {
        let mut filt = BiquadFilter::new(FilterType::LowPass, 1000.0, 0.707, 44100, 1);
        // Channel 5 doesn't exist — should return input unchanged
        let out = filt.process_sample(0.75, 5);
        assert!((out - 0.75).abs() < f32::EPSILON);
    }
}
