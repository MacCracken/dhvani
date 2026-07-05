//! 10-band ISO graphic equalizer with named presets.
//!
//! Wraps [`ParametricEq`] with fixed ISO center frequencies and a
//! convenience API for graphic EQ workflows (per-band gain sliders,
//! preset loading, flat detection).
//!
//! # Example
//!
//! ```rust
//! use dhvani::buffer::AudioBuffer;
//! use dhvani::dsp::{GraphicEq, GraphicEqSettings};
//!
//! let mut eq = GraphicEq::new(44100, 2);
//! eq.load_preset("rock");
//! eq.set_enabled(true);
//!
//! let mut buf = AudioBuffer::from_interleaved(vec![0.5; 4096], 2, 44100).unwrap();
//! eq.process(&mut buf);
//! ```

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;
use crate::dsp::eq::{BandType, EqBandConfig, ParametricEq};

/// Standard 10-band ISO center frequencies (Hz).
pub const ISO_BANDS: [f32; 10] = [
    31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0,
];

/// Display names for each band.
const BAND_NAMES: [&str; 10] = [
    "31 Hz", "62 Hz", "125 Hz", "250 Hz", "500 Hz", "1 kHz", "2 kHz", "4 kHz", "8 kHz", "16 kHz",
];

/// Standard Q factor for graphic EQ bands.
const GRAPHIC_EQ_Q: f32 = 1.4;

/// Gain range limit in dB.
const MAX_GAIN_DB: f32 = 12.0;

/// Graphic EQ settings: per-band gain and enabled state.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct GraphicEqSettings {
    /// Gain per band in dB. Length is always 10.
    pub bands: [f32; 10],
    /// Whether the EQ is active.
    pub enabled: bool,
}

impl Default for GraphicEqSettings {
    fn default() -> Self {
        Self {
            bands: [0.0; 10],
            enabled: false,
        }
    }
}

impl GraphicEqSettings {
    /// Flat EQ (all bands at 0 dB, disabled).
    pub fn flat() -> Self {
        Self::default()
    }

    /// Check if all bands are effectively at 0 dB.
    pub fn is_flat(&self) -> bool {
        !self.enabled || self.bands.iter().all(|b| b.abs() < 0.01)
    }

    /// Set a specific band's gain in dB, clamped to ±12.
    pub fn set_band(&mut self, band: usize, gain_db: f32) {
        if band < 10 {
            self.bands[band] = gain_db.clamp(-MAX_GAIN_DB, MAX_GAIN_DB);
        }
    }

    /// Load a named preset. Unknown names return flat.
    pub fn preset(name: &str) -> Self {
        //                           31   62  125  250  500   1k   2k   4k   8k  16k
        let bands = match name {
            "rock" => [4.0, 3.0, 1.0, -1.0, -2.0, 0.0, 2.0, 3.0, 4.0, 4.0],
            "pop" => [-1.0, 1.0, 3.0, 4.0, 3.0, 0.0, -1.0, 0.0, 1.0, 2.0],
            "jazz" => [2.0, 1.0, 0.0, 1.0, -1.0, -1.0, 0.0, 1.0, 2.0, 3.0],
            "classical" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -2.0, -3.0, -2.0, 0.0],
            "bass" => [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            "treble" => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 2.0, 4.0, 5.0, 6.0],
            "vocal" => [-2.0, -1.0, 0.0, 2.0, 4.0, 4.0, 3.0, 1.0, 0.0, -1.0],
            "electronic" => [5.0, 4.0, 1.0, 0.0, -2.0, 0.0, 1.0, 3.0, 4.0, 5.0],
            "acoustic" => [2.0, 1.0, 0.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 1.0],
            _ => [0.0; 10],
        };
        Self {
            bands,
            enabled: true,
        }
    }

    /// List all available preset names.
    pub fn preset_names() -> &'static [&'static str] {
        &[
            "flat",
            "rock",
            "pop",
            "jazz",
            "classical",
            "bass",
            "treble",
            "vocal",
            "electronic",
            "acoustic",
        ]
    }

    /// Get the display name for a band index.
    pub fn band_name(band: usize) -> &'static str {
        BAND_NAMES.get(band).copied().unwrap_or("?")
    }
}

/// 10-band graphic equalizer processor.
///
/// Wraps [`ParametricEq`] with fixed ISO center frequencies at Q=1.4.
/// Provides preset loading, per-band gain control, and efficient
/// flat-detection bypass.
#[must_use]
#[derive(Debug, Clone)]
pub struct GraphicEq {
    inner: ParametricEq,
    settings: GraphicEqSettings,
    sample_rate: u32,
    channels: u32,
}

impl GraphicEq {
    /// Create a new graphic EQ (all bands flat, disabled).
    pub fn new(sample_rate: u32, channels: u32) -> Self {
        tracing::debug!(sample_rate, channels, "GraphicEq::new");
        let settings = GraphicEqSettings::default();
        let inner = Self::build_eq(&settings, sample_rate, channels);
        Self {
            inner,
            settings,
            sample_rate,
            channels,
        }
    }

    /// Process a buffer through the 10-band EQ.
    ///
    /// Bypasses processing entirely if disabled or flat.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.settings.is_flat() {
            return;
        }
        // Adapt if sample rate changed
        if buf.sample_rate != self.sample_rate {
            self.sample_rate = buf.sample_rate;
            self.rebuild();
        }
        self.inner.process(buf);
    }

    /// Load a named preset.
    pub fn load_preset(&mut self, name: &str) {
        self.settings = GraphicEqSettings::preset(name);
        self.rebuild();
    }

    /// Set a specific band's gain in dB.
    pub fn set_band(&mut self, band: usize, gain_db: f32) {
        self.settings.set_band(band, gain_db);
        self.rebuild();
    }

    /// Apply full settings and rebuild coefficients.
    pub fn set_settings(&mut self, settings: GraphicEqSettings) {
        self.settings = settings;
        self.rebuild();
    }

    /// Get the current settings.
    pub fn settings(&self) -> &GraphicEqSettings {
        &self.settings
    }

    /// Enable or disable the EQ.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.settings.enabled = enabled;
    }

    /// Reset filter state (call on seek or track change to prevent clicks).
    pub fn reset(&mut self) {
        self.inner.reset();
    }

    fn rebuild(&mut self) {
        self.inner = Self::build_eq(&self.settings, self.sample_rate, self.channels);
    }

    fn build_eq(settings: &GraphicEqSettings, sample_rate: u32, channels: u32) -> ParametricEq {
        let bands: Vec<EqBandConfig> = ISO_BANDS
            .iter()
            .zip(settings.bands.iter())
            .map(|(&freq, &gain_db)| EqBandConfig {
                band_type: BandType::Peaking,
                freq_hz: freq,
                gain_db,
                q: GRAPHIC_EQ_Q,
                enabled: gain_db.abs() >= 0.01,
            })
            .collect();
        ParametricEq::new(bands, sample_rate, channels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_is_passthrough() {
        let mut eq = GraphicEq::new(44100, 2);
        eq.set_enabled(true);
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 44100.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples.clone(), 1, 44100).unwrap();
        eq.process(&mut buf);
        // Flat EQ should not modify buffer
        assert_eq!(buf.samples, samples);
    }

    #[test]
    fn boost_increases_energy() {
        let mut eq = GraphicEq::new(44100, 1);
        eq.set_enabled(true);
        eq.set_band(5, 12.0); // boost 1kHz

        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 44100.0).sin() * 0.5)
            .collect();
        let original_rms = {
            let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
            (sum / samples.len() as f64).sqrt()
        };
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        eq.process(&mut buf);
        let boosted_rms = buf.rms() as f64;
        assert!(boosted_rms > original_rms, "boost should increase energy");
    }

    #[test]
    fn cut_decreases_energy() {
        let mut eq = GraphicEq::new(44100, 1);
        eq.set_enabled(true);
        eq.set_band(5, -12.0); // cut 1kHz

        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 44100.0).sin() * 0.5)
            .collect();
        let original_rms = {
            let sum: f64 = samples.iter().map(|s| (*s as f64).powi(2)).sum();
            (sum / samples.len() as f64).sqrt()
        };
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        eq.process(&mut buf);
        let cut_rms = buf.rms() as f64;
        assert!(cut_rms < original_rms, "cut should decrease energy");
    }

    #[test]
    fn preset_loading() {
        let mut eq = GraphicEq::new(48000, 2);
        eq.load_preset("rock");
        assert!(eq.settings().enabled);
        assert!(!eq.settings().is_flat());
        assert!(eq.settings().bands[0] > 0.0); // bass boost
    }

    #[test]
    fn all_presets_valid_range() {
        for name in GraphicEqSettings::preset_names() {
            let settings = GraphicEqSettings::preset(name);
            for &b in &settings.bands {
                assert!(
                    (-MAX_GAIN_DB..=MAX_GAIN_DB).contains(&b),
                    "preset '{name}' band out of range: {b}"
                );
            }
        }
    }

    #[test]
    fn unknown_preset_is_flat() {
        let settings = GraphicEqSettings::preset("nonexistent");
        assert!(settings.bands.iter().all(|b| *b == 0.0));
    }

    #[test]
    fn band_names() {
        assert_eq!(GraphicEqSettings::band_name(0), "31 Hz");
        assert_eq!(GraphicEqSettings::band_name(5), "1 kHz");
        assert_eq!(GraphicEqSettings::band_name(9), "16 kHz");
        assert_eq!(GraphicEqSettings::band_name(10), "?");
    }

    #[test]
    fn set_band_clamps() {
        let mut s = GraphicEqSettings::default();
        s.set_band(0, 20.0);
        assert_eq!(s.bands[0], 12.0);
        s.set_band(0, -20.0);
        assert_eq!(s.bands[0], -12.0);
    }

    #[test]
    fn set_band_out_of_range_ignored() {
        let mut s = GraphicEqSettings::default();
        s.set_band(99, 6.0); // should not panic
        assert!(s.is_flat());
    }

    #[test]
    fn disabled_is_passthrough() {
        let mut eq = GraphicEq::new(44100, 1);
        eq.load_preset("rock"); // sets enabled=true
        eq.set_enabled(false);

        let samples: Vec<f32> = (0..1024)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples.clone(), 1, 44100).unwrap();
        eq.process(&mut buf);
        assert_eq!(buf.samples, samples);
    }

    #[test]
    fn stereo_processing() {
        let mut eq = GraphicEq::new(48000, 2);
        eq.set_enabled(true);
        eq.set_band(5, 6.0);

        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * (i / 2) as f32 / 48000.0).sin() * 0.5)
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 48000).unwrap();
        eq.process(&mut buf);
        assert_eq!(buf.channels, 2);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn serde_roundtrip() {
        let settings = GraphicEqSettings::preset("jazz");
        let json = serde_json::to_string(&settings).unwrap();
        let back: GraphicEqSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(back.bands, settings.bands);
        assert_eq!(back.enabled, settings.enabled);
    }

    #[test]
    fn reset_clears_state() {
        let mut eq = GraphicEq::new(44100, 1);
        eq.set_enabled(true);
        eq.set_band(5, 6.0);
        // Process some audio to build up state
        let mut buf = AudioBuffer::from_interleaved(vec![0.5; 1024], 1, 44100).unwrap();
        eq.process(&mut buf);
        // Reset should not panic
        eq.reset();
    }
}
