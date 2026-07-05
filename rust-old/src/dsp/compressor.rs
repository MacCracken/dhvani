//! Dynamic range compressor with envelope follower.
//!
//! Implements dB-domain gain calculation with configurable attack/release
//! envelope, optional soft knee, and makeup gain.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;
use crate::dsp::{amplitude_to_db, db_to_amplitude};

/// Compressor parameters.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct CompressorParams {
    /// Threshold in dB (signals above this are compressed).
    pub threshold_db: f32,
    /// Compression ratio (e.g., 4.0 means 4:1).
    pub ratio: f32,
    /// Attack time in milliseconds.
    pub attack_ms: f32,
    /// Release time in milliseconds.
    pub release_ms: f32,
    /// Makeup gain in dB (applied after compression).
    pub makeup_gain_db: f32,
    /// Soft knee width in dB (0.0 = hard knee).
    pub knee_db: f32,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet). Enables parallel compression.
    pub mix: f32,
}

impl Default for CompressorParams {
    fn default() -> Self {
        Self {
            threshold_db: -20.0,
            ratio: 4.0,
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 0.0,
            knee_db: 0.0,
            mix: 1.0,
        }
    }
}

impl CompressorParams {
    /// Create compressor parameters with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set threshold in dB.
    #[inline]
    pub fn with_threshold(mut self, db: f32) -> Self {
        self.threshold_db = db;
        self
    }

    /// Set compression ratio.
    #[inline]
    pub fn with_ratio(mut self, ratio: f32) -> Self {
        self.ratio = ratio;
        self
    }

    /// Set attack time in milliseconds.
    #[inline]
    pub fn with_attack(mut self, ms: f32) -> Self {
        self.attack_ms = ms;
        self
    }

    /// Set release time in milliseconds.
    #[inline]
    pub fn with_release(mut self, ms: f32) -> Self {
        self.release_ms = ms;
        self
    }

    /// Set makeup gain in dB.
    #[inline]
    pub fn with_makeup_gain(mut self, db: f32) -> Self {
        self.makeup_gain_db = db;
        self
    }

    /// Set soft knee width in dB.
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

    /// Validate parameters. Returns an error description if invalid.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.ratio < 1.0 {
            return Err("ratio must be >= 1.0");
        }
        if self.attack_ms < 0.0 {
            return Err("attack_ms must be >= 0.0");
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

/// Envelope-following dynamic range compressor.
#[must_use]
#[derive(Debug, Clone)]
pub struct Compressor {
    params: CompressorParams,
    envelope_db: f32,
    gain_reduction_db: f32,
    sample_rate: u32,
    bypassed: bool,
}

impl Compressor {
    /// Create a new compressor. Returns an error if parameters are invalid.
    pub fn new(params: CompressorParams, sample_rate: u32) -> crate::Result<Self> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "CompressorParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        tracing::debug!(
            sample_rate,
            threshold_db = params.threshold_db,
            ratio = params.ratio,
            "Compressor: created"
        );
        Ok(Self {
            params,
            envelope_db: -120.0,
            gain_reduction_db: 0.0,
            sample_rate,
            bypassed: false,
        })
    }

    /// Process an audio buffer in-place.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.params.ratio <= 1.0 {
            return;
        }
        if self.bypassed {
            return;
        }

        let ch = buf.channels as usize;
        let attack_coeff = Self::time_constant(self.params.attack_ms, self.sample_rate);
        let release_coeff = Self::time_constant(self.params.release_ms, self.sample_rate);
        let makeup_lin = db_to_amplitude(self.params.makeup_gain_db);
        let mix = self.params.mix.clamp(0.0, 1.0);
        let dry = 1.0 - mix;

        for frame in 0..buf.frames {
            // Detect peak across channels for this frame
            let mut peak = 0.0f32;
            for c in 0..ch {
                peak = peak.max(buf.samples[frame * ch + c].abs());
            }

            let input_db = amplitude_to_db(peak).max(-120.0);

            // Envelope follower (attack/release)
            let coeff = if input_db > self.envelope_db {
                attack_coeff
            } else {
                release_coeff
            };
            self.envelope_db = coeff * self.envelope_db + (1.0 - coeff) * input_db;

            // Compute gain reduction
            let gain_db = self.compute_gain(self.envelope_db);
            self.gain_reduction_db = gain_db;

            // Apply gain + makeup, blend dry/wet
            if gain_db.is_finite() {
                let gain_lin = db_to_amplitude(gain_db) * makeup_lin;
                for c in 0..ch {
                    let idx = frame * ch + c;
                    let dry_sample = buf.samples[idx];
                    let wet_sample = dry_sample * gain_lin;
                    buf.samples[idx] = dry_sample * dry + wet_sample * mix;
                    if !buf.samples[idx].is_finite() {
                        buf.samples[idx] = 0.0;
                    }
                }
            }
        }
    }

    /// Compute gain curve for a given envelope level in dB.
    fn compute_gain(&self, env_db: f32) -> f32 {
        let slope = 1.0 / self.params.ratio - 1.0;
        super::soft_knee_gain(env_db, self.params.threshold_db, self.params.knee_db, slope)
    }

    /// Time constant from milliseconds.
    fn time_constant(time_ms: f32, sample_rate: u32) -> f32 {
        abaco::dsp::time_constant(time_ms, sample_rate)
    }

    /// Current gain reduction in dB (for metering).
    pub fn gain_reduction_db(&self) -> f32 {
        self.gain_reduction_db
    }

    /// Set whether this compressor is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this compressor is currently bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Update parameters. Returns an error if parameters are invalid.
    pub fn set_params(&mut self, params: CompressorParams) -> crate::Result<()> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "CompressorParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        self.params = params;
        Ok(())
    }

    /// Update the sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        tracing::debug!(sample_rate, "Compressor: sample rate updated");
        self.sample_rate = sample_rate;
    }

    /// Reset envelope state.
    pub fn reset(&mut self) {
        self.envelope_db = -120.0;
        self.gain_reduction_db = 0.0;
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
    fn ratio_one_no_compression() {
        let params = CompressorParams {
            ratio: 1.0,
            ..Default::default()
        };
        let mut comp = Compressor::new(params, 44100).unwrap();
        let mut buf = make_sine(1.0, 4096);
        let original = buf.samples.clone();
        comp.process(&mut buf);
        assert_eq!(buf.samples, original);
    }

    #[test]
    fn below_threshold_unchanged() {
        let params = CompressorParams {
            threshold_db: 0.0, // 0 dBFS threshold
            ratio: 10.0,
            attack_ms: 0.01,
            release_ms: 0.01,
            makeup_gain_db: 0.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut comp = Compressor::new(params, 44100).unwrap();
        // -20 dBFS signal (amplitude ~0.1)
        let mut buf = make_sine(0.1, 4096);
        let original_rms = buf.rms();
        comp.process(&mut buf);
        // Should be mostly unchanged since below threshold
        assert!(
            (buf.rms() - original_rms).abs() < original_rms * 0.1,
            "Below-threshold signal should be mostly unchanged"
        );
    }

    #[test]
    fn above_threshold_compressed() {
        let params = CompressorParams {
            threshold_db: -20.0,
            ratio: 10.0,
            attack_ms: 0.01,
            release_ms: 0.01,
            makeup_gain_db: 0.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut comp = Compressor::new(params, 44100).unwrap();
        // 0 dBFS signal (amplitude 1.0) — well above -20 dB threshold
        let mut buf = make_sine(1.0, 4096);
        let original_rms = buf.rms();
        comp.process(&mut buf);
        // Envelope follower needs time to converge; check compression happened
        assert!(
            buf.rms() < original_rms * 0.95,
            "Above-threshold signal should be compressed: rms={} vs original={}",
            buf.rms(),
            original_rms
        );
    }

    #[test]
    fn makeup_gain_boosts() {
        let params = CompressorParams {
            threshold_db: 0.0,
            ratio: 4.0,
            attack_ms: 0.01,
            release_ms: 0.01,
            makeup_gain_db: 12.0,
            knee_db: 0.0,
            ..Default::default()
        };
        let mut comp = Compressor::new(params, 44100).unwrap();
        let mut buf = make_sine(0.1, 4096);
        let original_rms = buf.rms();
        comp.process(&mut buf);
        // Makeup gain of 12 dB ≈ 4x amplitude, signal below threshold so no compression
        assert!(buf.rms() > original_rms * 2.0);
    }

    #[test]
    fn soft_knee_smoother_than_hard() {
        // Just verify soft knee doesn't crash and produces valid output
        let params = CompressorParams {
            threshold_db: -12.0,
            ratio: 4.0,
            attack_ms: 5.0,
            release_ms: 50.0,
            makeup_gain_db: 0.0,
            knee_db: 12.0,
            ..Default::default()
        };
        let mut comp = Compressor::new(params, 44100).unwrap();
        let mut buf = make_sine(1.0, 4096);
        comp.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut comp = Compressor::new(CompressorParams::default(), 44100).unwrap();
        let mut buf = make_sine(1.0, 1024);
        comp.process(&mut buf);
        comp.reset();
        assert!((comp.envelope_db - (-120.0)).abs() < f32::EPSILON);
        assert!(comp.gain_reduction_db().abs() < f32::EPSILON);
    }
}
