//! Envelope limiter — soft-knee brick-wall limiter with envelope follower.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;
use crate::dsp::{amplitude_to_db, db_to_amplitude};

/// Limiter parameters.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct LimiterParams {
    /// Ceiling in dB (maximum output level, typically -0.1 to -1.0 dBFS).
    pub ceiling_db: f32,
    /// Release time in milliseconds.
    pub release_ms: f32,
    /// Knee width in dB (0.0 = hard knee).
    pub knee_db: f32,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub mix: f32,
}

impl Default for LimiterParams {
    fn default() -> Self {
        Self {
            ceiling_db: -0.3,
            release_ms: 50.0,
            knee_db: 0.0,
            mix: 1.0,
        }
    }
}

impl LimiterParams {
    /// Create limiter parameters with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set ceiling in dB.
    #[inline]
    pub fn with_ceiling(mut self, db: f32) -> Self {
        self.ceiling_db = db;
        self
    }

    /// Set release time in milliseconds.
    #[inline]
    pub fn with_release(mut self, ms: f32) -> Self {
        self.release_ms = ms;
        self
    }

    /// Set knee width in dB.
    #[inline]
    pub fn with_knee(mut self, db: f32) -> Self {
        self.knee_db = db;
        self
    }

    /// Set dry/wet mix.
    #[inline]
    pub fn with_mix(mut self, mix: f32) -> Self {
        self.mix = mix;
        self
    }

    /// Validate parameters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.ceiling_db > 0.0 {
            return Err("ceiling_db should be <= 0.0");
        }
        if self.release_ms < 0.0 {
            return Err("release_ms must be >= 0.0");
        }
        if self.knee_db < 0.0 {
            return Err("knee_db must be >= 0.0");
        }
        Ok(())
    }
}

/// Brick-wall limiter with envelope follower.
///
/// Ensures output never exceeds the ceiling. Uses instant attack
/// (true peak limiting) and configurable release.
#[must_use]
#[derive(Debug, Clone)]
pub struct EnvelopeLimiter {
    params: LimiterParams,
    envelope_db: f32,
    sample_rate: u32,
    bypassed: bool,
}

impl EnvelopeLimiter {
    /// Create a new limiter. Returns an error if parameters are invalid.
    pub fn new(params: LimiterParams, sample_rate: u32) -> crate::Result<Self> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "LimiterParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        tracing::debug!(
            sample_rate,
            ceiling_db = params.ceiling_db,
            "EnvelopeLimiter: created"
        );
        Ok(Self {
            params,
            envelope_db: -120.0,
            sample_rate,
            bypassed: false,
        })
    }

    /// Set whether this limiter is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this limiter is currently bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Process an audio buffer in-place.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.bypassed {
            return;
        }
        let ch = buf.channels as usize;
        let release_coeff = Self::time_constant(self.params.release_ms, self.sample_rate);
        let ceiling_lin = db_to_amplitude(self.params.ceiling_db);
        let mix = self.params.mix.clamp(0.0, 1.0);
        let dry = 1.0 - mix;

        for frame in 0..buf.frames {
            // Detect peak across channels
            let mut peak = 0.0f32;
            for c in 0..ch {
                peak = peak.max(buf.samples[frame * ch + c].abs());
            }

            let input_db = amplitude_to_db(peak).max(-120.0);

            // Instant attack, smooth release
            if input_db > self.envelope_db {
                self.envelope_db = input_db; // Instant attack
            } else {
                self.envelope_db =
                    release_coeff * self.envelope_db + (1.0 - release_coeff) * input_db;
            }

            // Compute gain reduction
            let gain_db = self.compute_gain(self.envelope_db);

            if gain_db < 0.0 && gain_db.is_finite() {
                let gain_lin = db_to_amplitude(gain_db);
                for c in 0..ch {
                    let idx = frame * ch + c;
                    let dry_sample = buf.samples[idx];
                    buf.samples[idx] = dry_sample * dry + (dry_sample * gain_lin) * mix;
                }
            }

            // Hard clamp as safety net
            for c in 0..ch {
                let idx = frame * ch + c;
                buf.samples[idx] = buf.samples[idx].clamp(-ceiling_lin, ceiling_lin);
            }
        }
    }

    fn compute_gain(&self, env_db: f32) -> f32 {
        // Limiter is effectively ∞:1 compression → slope = -1.0
        super::soft_knee_gain(env_db, self.params.ceiling_db, self.params.knee_db, -1.0)
    }

    fn time_constant(time_ms: f32, sample_rate: u32) -> f32 {
        abaco::dsp::time_constant(time_ms, sample_rate)
    }

    /// Current gain reduction in dB.
    pub fn gain_reduction_db(&self) -> f32 {
        self.compute_gain(self.envelope_db)
    }

    /// Update parameters. Returns an error if parameters are invalid.
    pub fn set_params(&mut self, params: LimiterParams) -> crate::Result<()> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "LimiterParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        self.params = params;
        Ok(())
    }

    /// Update the sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        tracing::debug!(sample_rate, "EnvelopeLimiter: sample rate updated");
        self.sample_rate = sample_rate;
    }

    /// Reset envelope state.
    pub fn reset(&mut self) {
        self.envelope_db = -120.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(amplitude: f32, frames: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| amplitude * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        AudioBuffer::from_interleaved(samples, 1, 44100).unwrap()
    }

    #[test]
    fn below_ceiling_unchanged() {
        let params = LimiterParams {
            ceiling_db: 0.0,
            release_ms: 50.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut limiter = EnvelopeLimiter::new(params, 44100).unwrap();
        let mut buf = make_sine(0.5, 4096);
        let original_rms = buf.rms();
        limiter.process(&mut buf);
        assert!(
            (buf.rms() - original_rms).abs() < original_rms * 0.05,
            "Below ceiling should be mostly unchanged"
        );
    }

    #[test]
    fn above_ceiling_limited() {
        let params = LimiterParams {
            ceiling_db: -6.0, // ~0.5 linear
            release_ms: 10.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut limiter = EnvelopeLimiter::new(params, 44100).unwrap();
        let mut buf = make_sine(1.0, 4096);
        limiter.process(&mut buf);
        let ceiling_lin = db_to_amplitude(-6.0);
        // All samples should be at or below ceiling
        assert!(
            buf.peak() <= ceiling_lin + 0.01,
            "Peak {} should be <= ceiling {}",
            buf.peak(),
            ceiling_lin
        );
    }

    #[test]
    fn output_finite() {
        let mut limiter = EnvelopeLimiter::new(LimiterParams::default(), 44100).unwrap();
        let mut buf = make_sine(2.0, 4096);
        limiter.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn reset_clears_state() {
        let mut limiter = EnvelopeLimiter::new(LimiterParams::default(), 44100).unwrap();
        let mut buf = make_sine(1.0, 1024);
        limiter.process(&mut buf);
        limiter.reset();
        assert!((limiter.envelope_db + 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn soft_knee_limits() {
        let params = LimiterParams {
            ceiling_db: -6.0,
            release_ms: 10.0,
            knee_db: 6.0, // soft knee
            ..Default::default()
        };
        let mut limiter = EnvelopeLimiter::new(params, 44100).unwrap();
        let mut buf = make_sine(1.0, 4096);
        limiter.process(&mut buf);
        let ceiling_lin = db_to_amplitude(-6.0);
        assert!(buf.peak() <= ceiling_lin + 0.02);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn gain_reduction_reports() {
        let params = LimiterParams {
            ceiling_db: -12.0,
            release_ms: 10.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut limiter = EnvelopeLimiter::new(params, 44100).unwrap();
        let mut buf = make_sine(1.0, 4096);
        limiter.process(&mut buf);
        // Should have applied gain reduction
        assert!(limiter.gain_reduction_db() < 0.0);
    }

    #[test]
    fn set_params_updates() {
        let mut limiter = EnvelopeLimiter::new(LimiterParams::default(), 44100).unwrap();
        limiter
            .set_params(LimiterParams {
                ceiling_db: -3.0,
                release_ms: 100.0,
                knee_db: 3.0,
                ..Default::default()
            })
            .unwrap();
        let mut buf = make_sine(1.0, 2048);
        limiter.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn stereo_limiting() {
        let params = LimiterParams {
            ceiling_db: -6.0,
            release_ms: 10.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut limiter = EnvelopeLimiter::new(params, 44100).unwrap();
        let samples: Vec<f32> = (0..8192)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / 44100.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        limiter.process(&mut buf);
        let ceiling_lin = db_to_amplitude(-6.0);
        assert!(buf.peak() <= ceiling_lin + 0.02);
    }
}
