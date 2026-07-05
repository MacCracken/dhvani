//! Parametric EQ — N-band biquad cascade with configurable band types.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;
use crate::dsp::biquad::{BiquadFilter, FilterType};

/// Band type for parametric EQ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum BandType {
    /// Peaking (bell) EQ.
    Peaking,
    /// Low-shelf EQ.
    LowShelf,
    /// High-shelf EQ.
    HighShelf,
    /// Low-pass filter.
    LowPass,
    /// High-pass filter.
    HighPass,
    /// Band-pass filter.
    BandPass,
    /// Notch (band-reject) filter.
    Notch,
}

/// Configuration for a single EQ band.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EqBandConfig {
    /// Filter type for this band.
    pub band_type: BandType,
    /// Center/corner frequency in Hz.
    pub freq_hz: f32,
    /// Gain in decibels (used by peaking/shelf types).
    pub gain_db: f32,
    /// Quality factor (bandwidth).
    pub q: f32,
    /// Whether this band is active.
    pub enabled: bool,
}

impl EqBandConfig {
    /// Create a new EQ band configuration.
    pub fn new(band_type: BandType, freq_hz: f32, gain_db: f32, q: f32, enabled: bool) -> Self {
        Self {
            band_type,
            freq_hz,
            gain_db,
            q,
            enabled,
        }
    }

    fn to_filter_type(&self) -> FilterType {
        match self.band_type {
            BandType::Peaking => FilterType::Peaking {
                gain_db: self.gain_db,
            },
            BandType::LowShelf => FilterType::LowShelf {
                gain_db: self.gain_db,
            },
            BandType::HighShelf => FilterType::HighShelf {
                gain_db: self.gain_db,
            },
            BandType::LowPass => FilterType::LowPass,
            BandType::HighPass => FilterType::HighPass,
            BandType::BandPass => FilterType::BandPass,
            BandType::Notch => FilterType::Notch,
        }
    }
}

/// N-band parametric equalizer — cascade of biquad filters.
#[must_use]
#[derive(Debug, Clone)]
pub struct ParametricEq {
    bands: Vec<(EqBandConfig, BiquadFilter)>,
    sample_rate: u32,
    channels: u32,
    bypassed: bool,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    mix: f32,
}

impl ParametricEq {
    /// Create a parametric EQ with the given bands.
    pub fn new(bands: Vec<EqBandConfig>, sample_rate: u32, channels: u32) -> Self {
        tracing::debug!(
            sample_rate,
            channels,
            band_count = bands.len(),
            "ParametricEq: created"
        );
        let bands = bands
            .into_iter()
            .map(|cfg| {
                let filt = BiquadFilter::new(
                    cfg.to_filter_type(),
                    cfg.freq_hz,
                    cfg.q,
                    sample_rate,
                    channels,
                );
                (cfg, filt)
            })
            .collect();

        Self {
            bands,
            sample_rate,
            channels,
            bypassed: false,
            mix: 1.0,
        }
    }

    /// Set whether this EQ is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this EQ is currently bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Process an audio buffer through all enabled bands in series.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.bypassed {
            return;
        }
        let mix = self.mix;
        if mix < 1.0 {
            let dry_samples = buf.samples.clone();
            for (cfg, filt) in &mut self.bands {
                if cfg.enabled {
                    filt.process(buf);
                }
            }
            let dry = 1.0 - mix;
            for (s, d) in buf.samples.iter_mut().zip(dry_samples.iter()) {
                *s = d * dry + *s * mix;
            }
        } else {
            for (cfg, filt) in &mut self.bands {
                if cfg.enabled {
                    filt.process(buf);
                }
            }
        }
    }

    /// Update a band's configuration. Rebuilds that band's filter coefficients.
    pub fn set_band(&mut self, index: usize, config: EqBandConfig) {
        if let Some((cfg, filt)) = self.bands.get_mut(index) {
            filt.set_params(config.to_filter_type(), config.freq_hz, config.q);
            *cfg = config;
        }
    }

    /// Add a new band.
    pub fn add_band(&mut self, config: EqBandConfig) {
        let filt = BiquadFilter::new(
            config.to_filter_type(),
            config.freq_hz,
            config.q,
            self.sample_rate,
            self.channels,
        );
        self.bands.push((config, filt));
    }

    /// Remove a band by index.
    pub fn remove_band(&mut self, index: usize) {
        if index < self.bands.len() {
            drop(self.bands.remove(index));
        }
    }

    /// Number of bands.
    pub fn band_count(&self) -> usize {
        self.bands.len()
    }

    /// Get a band's current configuration.
    pub fn band(&self, index: usize) -> Option<&EqBandConfig> {
        self.bands.get(index).map(|(cfg, _)| cfg)
    }

    /// Replace all bands at once. Rebuilds all filter coefficients.
    pub fn set_params(&mut self, bands: Vec<EqBandConfig>) {
        self.bands = bands
            .into_iter()
            .map(|cfg| {
                let filt = BiquadFilter::new(
                    cfg.to_filter_type(),
                    cfg.freq_hz,
                    cfg.q,
                    self.sample_rate,
                    self.channels,
                );
                (cfg, filt)
            })
            .collect();
    }

    /// Set the dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Current dry/wet mix.
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Update the sample rate and rebuild all filter coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        tracing::debug!(sample_rate, "ParametricEq: sample rate updated");
        self.sample_rate = sample_rate;
        for (cfg, filt) in &mut self.bands {
            filt.set_sample_rate(sample_rate);
            filt.set_params(cfg.to_filter_type(), cfg.freq_hz, cfg.q);
        }
    }

    /// Reset all filter states.
    pub fn reset(&mut self) {
        for (_, filt) in &mut self.bands {
            filt.reset();
        }
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
    fn flat_eq_passthrough() {
        let bands = vec![EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        let mut buf = make_sine(440.0, 44100, 4096);
        let original_rms = buf.rms();
        eq.process(&mut buf);
        // 0 dB gain should not change signal significantly
        assert!(
            (buf.rms() - original_rms).abs() < original_rms * 0.01,
            "0 dB peaking should be near-passthrough"
        );
    }

    #[test]
    fn peaking_boosts_frequency() {
        let bands = vec![EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 440.0,
            gain_db: 12.0,
            q: 1.0,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        let mut buf = make_sine(440.0, 44100, 4096);
        let original_rms = buf.rms();
        eq.process(&mut buf);
        assert!(buf.rms() > original_rms * 1.5);
    }

    #[test]
    fn disabled_band_no_effect() {
        let bands = vec![EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 440.0,
            gain_db: 24.0,
            q: 1.0,
            enabled: false,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        let mut buf = make_sine(440.0, 44100, 4096);
        let original = buf.samples.clone();
        eq.process(&mut buf);
        assert_eq!(buf.samples, original);
    }

    #[test]
    fn add_remove_band() {
        let mut eq = ParametricEq::new(vec![], 44100, 1);
        assert_eq!(eq.band_count(), 0);
        eq.add_band(EqBandConfig {
            band_type: BandType::LowPass,
            freq_hz: 5000.0,
            gain_db: 0.0,
            q: 0.707,
            enabled: true,
        });
        assert_eq!(eq.band_count(), 1);
        eq.remove_band(0);
        assert_eq!(eq.band_count(), 0);
    }

    #[test]
    fn set_band_updates_config() {
        let bands = vec![EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 1000.0,
            gain_db: 0.0,
            q: 1.0,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        eq.set_band(
            0,
            EqBandConfig {
                band_type: BandType::HighShelf,
                freq_hz: 5000.0,
                gain_db: 6.0,
                q: 0.707,
                enabled: true,
            },
        );
        let cfg = eq.band(0).unwrap();
        assert_eq!(cfg.band_type, BandType::HighShelf);
        assert!((cfg.freq_hz - 5000.0).abs() < f32::EPSILON);
    }

    #[test]
    fn lowpass_band_attenuates_highs() {
        let bands = vec![EqBandConfig {
            band_type: BandType::LowPass,
            freq_hz: 500.0,
            gain_db: 0.0,
            q: 0.707,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        let mut buf = make_sine(10000.0, 44100, 4096);
        let original_rms = buf.rms();
        eq.process(&mut buf);
        assert!(buf.rms() < original_rms * 0.1);
    }

    #[test]
    fn reset_clears_all_bands() {
        let bands = vec![EqBandConfig {
            band_type: BandType::Peaking,
            freq_hz: 440.0,
            gain_db: 12.0,
            q: 1.0,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        let mut buf = make_sine(440.0, 44100, 256);
        eq.process(&mut buf);
        eq.reset();
        // After reset, filters should be clean — no transient from prior state
        let mut silence = AudioBuffer::silence(1, 64, 44100);
        eq.process(&mut silence);
        assert!(silence.peak() < 0.001);
    }

    #[test]
    fn empty_eq_passthrough() {
        let mut eq = ParametricEq::new(vec![], 44100, 1);
        let mut buf = make_sine(440.0, 44100, 1024);
        let original = buf.samples.clone();
        eq.process(&mut buf);
        assert_eq!(buf.samples, original);
    }

    #[test]
    fn remove_band_out_of_bounds() {
        let mut eq = ParametricEq::new(vec![], 44100, 1);
        eq.remove_band(99); // should not panic
        assert_eq!(eq.band_count(), 0);
    }

    #[test]
    fn set_band_out_of_bounds() {
        let mut eq = ParametricEq::new(vec![], 44100, 1);
        eq.set_band(
            99,
            EqBandConfig {
                band_type: BandType::Peaking,
                freq_hz: 1000.0,
                gain_db: 0.0,
                q: 1.0,
                enabled: true,
            },
        ); // should not panic
        assert_eq!(eq.band_count(), 0);
    }

    #[test]
    fn band_returns_none_for_invalid_index() {
        let eq = ParametricEq::new(vec![], 44100, 1);
        assert!(eq.band(0).is_none());
        assert!(eq.band(99).is_none());
    }

    #[test]
    fn notch_band() {
        let bands = vec![EqBandConfig {
            band_type: BandType::Notch,
            freq_hz: 1000.0,
            gain_db: 0.0,
            q: 10.0,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        let mut buf = make_sine(1000.0, 44100, 4096);
        let original_rms = buf.rms();
        eq.process(&mut buf);
        assert!(buf.rms() < original_rms * 0.2);
    }

    #[test]
    fn highpass_band() {
        let bands = vec![EqBandConfig {
            band_type: BandType::HighPass,
            freq_hz: 5000.0,
            gain_db: 0.0,
            q: 0.707,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        let mut buf = make_sine(100.0, 44100, 4096);
        let original_rms = buf.rms();
        eq.process(&mut buf);
        assert!(buf.rms() < original_rms * 0.1);
    }

    #[test]
    fn bandpass_band() {
        let bands = vec![EqBandConfig {
            band_type: BandType::BandPass,
            freq_hz: 1000.0,
            gain_db: 0.0,
            q: 5.0,
            enabled: true,
        }];
        let mut eq = ParametricEq::new(bands, 44100, 1);
        // 100Hz should be attenuated by bandpass at 1kHz
        let mut buf = make_sine(100.0, 44100, 4096);
        let original_rms = buf.rms();
        eq.process(&mut buf);
        assert!(buf.rms() < original_rms * 0.3);
    }
}
