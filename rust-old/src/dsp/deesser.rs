//! De-esser — sibilance reduction using a biquad sidechain detector.
//!
//! Detects energy in the sibilant frequency range (typically 4–9 kHz) and
//! applies dynamic gain reduction when it exceeds the threshold.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;
use crate::dsp::biquad::{BiquadFilter, FilterType};
use crate::dsp::db_to_amplitude;

/// De-esser parameters.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct DeEsserParams {
    /// Center frequency of the sibilance band in Hz (typically 5000–8000).
    pub freq_hz: f32,
    /// Threshold in dB — sibilance above this is reduced.
    pub threshold_db: f32,
    /// Maximum gain reduction in dB (positive value, e.g., 6.0).
    pub reduction_db: f32,
    /// Q factor / bandwidth of the detection and reduction band.
    pub q: f32,
}

impl Default for DeEsserParams {
    fn default() -> Self {
        Self {
            freq_hz: 6000.0,
            threshold_db: -20.0,
            reduction_db: 6.0,
            q: 2.0,
        }
    }
}

impl DeEsserParams {
    /// Validate parameters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.freq_hz <= 0.0 {
            return Err("freq_hz must be > 0.0");
        }
        if self.reduction_db < 0.0 {
            return Err("reduction_db must be >= 0.0");
        }
        if self.q <= 0.0 {
            return Err("q must be > 0.0");
        }
        Ok(())
    }
}

/// Sibilance reduction processor.
#[must_use]
#[derive(Debug, Clone)]
pub struct DeEsser {
    params: DeEsserParams,
    /// Band-pass filter for detecting sibilant energy.
    detector: BiquadFilter,
    /// Pre-allocated sidechain buffer (avoids clone per process call).
    sidechain: Vec<f32>,
    sample_rate: u32,
    channels: u32,
    bypassed: bool,
}

impl DeEsser {
    /// Create a new de-esser. Returns an error if parameters are invalid.
    pub fn new(params: DeEsserParams, sample_rate: u32, channels: u32) -> crate::Result<Self> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "DeEsserParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        let detector = BiquadFilter::new(
            FilterType::BandPass,
            params.freq_hz,
            params.q,
            sample_rate,
            channels,
        );
        tracing::debug!(
            sample_rate,
            channels,
            freq_hz = params.freq_hz,
            "DeEsser: created"
        );
        Ok(Self {
            params,
            detector,
            sidechain: Vec::new(),
            sample_rate,
            channels,
            bypassed: false,
        })
    }

    /// Set whether this de-esser is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this de-esser is currently bypassed.
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
        // Reuse pre-allocated sidechain buffer (no heap allocation in hot path)
        self.sidechain.resize(buf.samples.len(), 0.0);
        self.sidechain.copy_from_slice(&buf.samples);
        let mut sidechain = AudioBuffer {
            samples: std::mem::take(&mut self.sidechain),
            channels: buf.channels,
            sample_rate: buf.sample_rate,
            frames: buf.frames,
        };
        self.detector.process(&mut sidechain);

        let threshold_lin = db_to_amplitude(self.params.threshold_db);
        let max_reduction_lin = db_to_amplitude(-self.params.reduction_db);

        for frame in 0..buf.frames {
            // Detect sidechain peak across channels
            let mut sidechain_peak = 0.0f32;
            for c in 0..ch {
                let idx = frame * ch + c;
                sidechain_peak = sidechain_peak.max(sidechain.samples[idx].abs());
            }

            // If sidechain exceeds threshold, compute gain reduction
            if sidechain_peak > threshold_lin {
                let excess_ratio = sidechain_peak / threshold_lin;
                // Proportional reduction: more excess = more reduction
                let reduction = (1.0 / excess_ratio).max(max_reduction_lin);

                for c in 0..ch {
                    let idx = frame * ch + c;
                    buf.samples[idx] *= reduction;
                    if !buf.samples[idx].is_finite() {
                        buf.samples[idx] = 0.0;
                    }
                }
            }
        }

        // Reclaim sidechain buffer for reuse
        self.sidechain = sidechain.samples;
    }

    /// Update the sample rate and rebuild the detector filter.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        tracing::debug!(sample_rate, "DeEsser: sample rate updated");
        self.sample_rate = sample_rate;
        self.detector = BiquadFilter::new(
            FilterType::BandPass,
            self.params.freq_hz,
            self.params.q,
            sample_rate,
            self.channels,
        );
    }

    /// Reset filter state.
    pub fn reset(&mut self) {
        self.detector.reset();
    }

    /// Update parameters. Returns an error if parameters are invalid.
    pub fn set_params(&mut self, params: DeEsserParams) -> crate::Result<()> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "DeEsserParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        self.detector = BiquadFilter::new(
            FilterType::BandPass,
            params.freq_hz,
            params.q,
            self.sample_rate,
            self.channels,
        );
        self.params = params;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(freq: f32, amplitude: f32, frames: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| amplitude * (2.0 * std::f32::consts::PI * freq * i as f32 / 44100.0).sin())
            .collect();
        AudioBuffer::from_interleaved(samples, 1, 44100).unwrap()
    }

    #[test]
    fn low_frequency_unaffected() {
        let params = DeEsserParams {
            freq_hz: 6000.0,
            threshold_db: -30.0,
            reduction_db: 12.0,
            q: 2.0,
        };
        let mut deesser = DeEsser::new(params, 44100, 1).unwrap();
        let mut buf = make_sine(200.0, 0.8, 4096);
        let original_rms = buf.rms();
        deesser.process(&mut buf);
        // Low frequency should pass through mostly unchanged
        assert!(
            (buf.rms() - original_rms).abs() < original_rms * 0.1,
            "200Hz should not trigger de-esser at 6kHz"
        );
    }

    #[test]
    fn sibilance_reduced() {
        let params = DeEsserParams {
            freq_hz: 6000.0,
            threshold_db: -30.0,
            reduction_db: 12.0,
            q: 1.0,
        };
        let mut deesser = DeEsser::new(params, 44100, 1).unwrap();
        let mut buf = make_sine(6000.0, 0.8, 4096);
        let original_rms = buf.rms();
        deesser.process(&mut buf);
        assert!(
            buf.rms() < original_rms * 0.8,
            "6kHz signal should be reduced by de-esser"
        );
    }

    #[test]
    fn below_threshold_passthrough() {
        let params = DeEsserParams {
            freq_hz: 6000.0,
            threshold_db: 0.0, // Very high threshold
            reduction_db: 12.0,
            q: 2.0,
        };
        let mut deesser = DeEsser::new(params, 44100, 1).unwrap();
        let mut buf = make_sine(6000.0, 0.1, 4096);
        let original_rms = buf.rms();
        deesser.process(&mut buf);
        // Signal too quiet to trigger de-esser
        assert!(
            (buf.rms() - original_rms).abs() < original_rms * 0.15,
            "Below-threshold signal should be near-unchanged"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut deesser = DeEsser::new(DeEsserParams::default(), 44100, 1).unwrap();
        let mut buf = make_sine(6000.0, 0.8, 256);
        deesser.process(&mut buf);
        deesser.reset();
        let out = deesser.detector.process_sample(0.0, 0);
        assert!(out.abs() < f32::EPSILON);
    }

    #[test]
    fn set_params_updates_detector() {
        let mut deesser = DeEsser::new(DeEsserParams::default(), 44100, 1).unwrap();
        deesser
            .set_params(DeEsserParams {
                freq_hz: 8000.0,
                threshold_db: -20.0,
                reduction_db: 8.0,
                q: 3.0,
            })
            .unwrap();
        let mut buf = make_sine(8000.0, 0.8, 4096);
        deesser.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn stereo_deessing() {
        let params = DeEsserParams {
            freq_hz: 6000.0,
            threshold_db: -30.0,
            reduction_db: 12.0,
            q: 1.0,
        };
        let mut deesser = DeEsser::new(params, 44100, 2).unwrap();
        let samples: Vec<f32> = (0..8192)
            .map(|i| 0.8 * (2.0 * std::f32::consts::PI * 6000.0 * (i / 2) as f32 / 44100.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        deesser.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn sidechain_buffer_reused() {
        let mut deesser = DeEsser::new(DeEsserParams::default(), 44100, 1).unwrap();
        let mut buf = make_sine(6000.0, 0.8, 1024);
        deesser.process(&mut buf);
        // After process, sidechain buffer should be populated (reusable)
        assert!(!deesser.sidechain.is_empty());
        let cap_after_first = deesser.sidechain.capacity();
        // Process again — should reuse same allocation
        let mut buf2 = make_sine(6000.0, 0.8, 1024);
        deesser.process(&mut buf2);
        assert_eq!(deesser.sidechain.capacity(), cap_after_first);
    }
}
