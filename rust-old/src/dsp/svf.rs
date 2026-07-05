//! State Variable Filter — Cytomic (Andrew Simper) topology.
//!
//! Superior to biquad for modulated cutoff: stable under fast parameter changes,
//! no coefficient recalculation needed. Computes lowpass, highpass, bandpass,
//! and notch outputs simultaneously from a single processing step.
//!
//! Reference: Andrew Simper, "Linear Trapezoidal Integrated SVF", Cytomic, 2013.

use crate::buffer::AudioBuffer;
use serde::{Deserialize, Serialize};

/// SVF filter mode — selects which output to use.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SvfMode {
    /// Low-pass: passes frequencies below cutoff.
    LowPass,
    /// High-pass: passes frequencies above cutoff.
    HighPass,
    /// Band-pass: passes frequencies around cutoff.
    BandPass,
    /// Notch (band-reject): attenuates frequencies around cutoff.
    Notch,
    /// All-pass: passes all frequencies with phase shift.
    AllPass,
    /// Peaking: boosts or cuts around cutoff.
    Peak,
    /// Low shelf: boosts or cuts below cutoff.
    LowShelf,
    /// High shelf: boosts or cuts above cutoff.
    HighShelf,
}

/// Per-channel SVF state.
#[derive(Debug, Clone, Default)]
struct SvfState {
    ic1eq: f64,
    ic2eq: f64,
}

/// State Variable Filter (Cytomic / Simper topology).
///
/// Produces all filter outputs simultaneously from a single processing step.
/// Safe for real-time cutoff modulation — no coefficient discontinuities.
#[must_use]
#[derive(Debug, Clone)]
pub struct SvfFilter {
    mode: SvfMode,
    freq_hz: f32,
    q: f32,
    gain_db: f32,
    sample_rate: u32,
    states: Vec<SvfState>,
    bypassed: bool,
    mix: f32,
    // Pre-computed coefficients
    g: f64,
    k: f64,
    a1: f64,
    a2: f64,
    a3: f64,
    // Shelf/peak gain
    a: f64,
}

impl SvfFilter {
    /// Create a new SVF filter.
    ///
    /// `gain_db` is only used for `Peak`, `LowShelf`, and `HighShelf` modes.
    pub fn new(
        mode: SvfMode,
        freq_hz: f32,
        q: f32,
        gain_db: f32,
        sample_rate: u32,
        channels: u32,
    ) -> Self {
        tracing::debug!(
            ?mode,
            freq_hz,
            q,
            gain_db,
            sample_rate,
            channels,
            "SvfFilter::new"
        );
        let mut f = Self {
            mode,
            freq_hz,
            q: q.max(0.01),
            gain_db,
            sample_rate,
            states: vec![SvfState::default(); channels as usize],
            bypassed: false,
            mix: 1.0,
            g: 0.0,
            k: 0.0,
            a1: 0.0,
            a2: 0.0,
            a3: 0.0,
            a: 1.0,
        };
        f.update_coefficients();
        f
    }

    /// Update filter parameters without resetting state.
    ///
    /// This is the key advantage over biquad: smooth, click-free modulation.
    pub fn set_params(&mut self, mode: SvfMode, freq_hz: f32, q: f32, gain_db: f32) {
        self.mode = mode;
        self.freq_hz = freq_hz;
        self.q = q.max(0.01);
        self.gain_db = gain_db;
        self.update_coefficients();
    }

    /// Set cutoff frequency. Safe for real-time modulation.
    pub fn set_frequency(&mut self, freq_hz: f32) {
        self.freq_hz = freq_hz;
        self.update_coefficients();
    }

    /// Set resonance (Q factor). Safe for real-time modulation.
    pub fn set_q(&mut self, q: f32) {
        self.q = q.max(0.01);
        self.update_coefficients();
    }

    /// Set gain for shelf/peak modes (dB).
    pub fn set_gain_db(&mut self, gain_db: f32) {
        self.gain_db = gain_db;
        self.update_coefficients();
    }

    /// Set sample rate and recompute coefficients.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        self.update_coefficients();
    }

    /// Set bypass state.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Set dry/wet mix (0.0 = dry, 1.0 = wet).
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Reset filter state (zeros delay elements).
    pub fn reset(&mut self) {
        for s in &mut self.states {
            s.ic1eq = 0.0;
            s.ic2eq = 0.0;
        }
    }

    /// Process an entire audio buffer in-place.
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.bypassed {
            return;
        }
        let ch = buf.channels as usize;
        let mix = self.mix;
        let dry = 1.0 - mix;
        for frame in 0..buf.frames {
            for c in 0..ch {
                let idx = frame * ch + c;
                let input = buf.samples[idx] as f64;
                let output = self.process_sample_internal(input, c);
                buf.samples[idx] =
                    (buf.samples[idx] as f64 * dry as f64 + output * mix as f64) as f32;
            }
        }
    }

    /// Process a single sample for a given channel.
    #[inline]
    #[must_use]
    pub fn process_sample(&mut self, sample: f32, channel: usize) -> f32 {
        if channel < self.states.len() {
            self.process_sample_internal(sample as f64, channel) as f32
        } else {
            sample
        }
    }

    /// Current filter mode.
    pub fn mode(&self) -> SvfMode {
        self.mode
    }

    /// Current cutoff frequency.
    pub fn freq_hz(&self) -> f32 {
        self.freq_hz
    }

    /// Current Q factor.
    pub fn q(&self) -> f32 {
        self.q
    }

    /// Current gain (dB).
    pub fn gain_db(&self) -> f32 {
        self.gain_db
    }

    /// Current dry/wet mix.
    pub fn mix(&self) -> f32 {
        self.mix
    }

    // ── Internal ───────────────────────────────────────────────────────

    fn update_coefficients(&mut self) {
        let sr = self.sample_rate as f64;
        let freq = (self.freq_hz as f64).clamp(1.0, sr * 0.499);

        // Cytomic SVF: g = tan(pi * fc / sr)
        self.g = (std::f64::consts::PI * freq / sr).tan();
        self.k = 1.0 / self.q.max(0.01) as f64;

        // Linear gain for shelf/peak modes
        self.a = 10.0f64.powf(self.gain_db as f64 / 40.0); // sqrt of dB→linear

        match self.mode {
            SvfMode::Peak => {
                // Cytomic peak EQ: k = 1/(Q*A) so bandpass gain scales with boost/cut
                self.k = 1.0 / (self.q.max(0.01) as f64 * self.a);
                let denom = 1.0 + self.g * (self.g + self.k);
                self.a1 = 1.0 / denom;
                self.a2 = self.g * self.a1;
                self.a3 = self.g * self.a2;
            }
            SvfMode::LowShelf => {
                self.g *= self.a.sqrt();
                let denom = 1.0 + self.g * (self.g + self.k);
                self.a1 = 1.0 / denom;
                self.a2 = self.g * self.a1;
                self.a3 = self.g * self.a2;
            }
            SvfMode::HighShelf => {
                self.g /= self.a.sqrt();
                let denom = 1.0 + self.g * (self.g + self.k);
                self.a1 = 1.0 / denom;
                self.a2 = self.g * self.a1;
                self.a3 = self.g * self.a2;
            }
            _ => {
                let denom = 1.0 + self.g * (self.g + self.k);
                self.a1 = 1.0 / denom;
                self.a2 = self.g * self.a1;
                self.a3 = self.g * self.a2;
            }
        }
    }

    #[inline]
    fn process_sample_internal(&mut self, input: f64, channel: usize) -> f64 {
        let s = &mut self.states[channel];

        let v3 = input - s.ic2eq;
        let v1 = self.a1 * s.ic1eq + self.a2 * v3;
        let v2 = s.ic2eq + self.a2 * s.ic1eq + self.a3 * v3;

        s.ic1eq = 2.0 * v1 - s.ic1eq;
        s.ic2eq = 2.0 * v2 - s.ic2eq;

        match self.mode {
            SvfMode::LowPass => v2,
            SvfMode::HighPass => input - self.k * v1 - v2,
            SvfMode::BandPass => v1,
            SvfMode::Notch => input - self.k * v1,
            SvfMode::AllPass => input - 2.0 * self.k * v1,
            SvfMode::Peak => {
                // Cytomic peak EQ: boost/cut the bandpass component
                input + (self.a * self.a - 1.0) * v1
            }
            SvfMode::LowShelf => {
                // Cytomic low shelf: boost/cut LP region, BP transition band
                input + (self.a * self.a - 1.0) * v2 + (self.a - 1.0) * self.k * v1
            }
            SvfMode::HighShelf => {
                let hp = input - self.k * v1 - v2;
                // Cytomic high shelf: boost/cut HP region, BP transition band
                input + (self.a * self.a - 1.0) * hp + (self.a - 1.0) * self.k * v1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(freq: f32, sr: u32, frames: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin() * 0.8)
            .collect();
        AudioBuffer::from_interleaved(samples, 1, sr).unwrap()
    }

    #[test]
    fn lowpass_attenuates_highs() {
        let mut buf = make_sine(10000.0, 44100, 4096);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::LowPass, 500.0, 0.707, 0.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        assert!(
            rms_after < rms_before * 0.2,
            "LP didn't attenuate: before={rms_before} after={rms_after}"
        );
    }

    #[test]
    fn highpass_attenuates_lows() {
        let mut buf = make_sine(100.0, 44100, 4096);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::HighPass, 5000.0, 0.707, 0.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        assert!(
            rms_after < rms_before * 0.2,
            "HP didn't attenuate: before={rms_before} after={rms_after}"
        );
    }

    #[test]
    fn bandpass_passes_center() {
        let mut buf = make_sine(1000.0, 44100, 4096);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::BandPass, 1000.0, 1.0, 0.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        // Signal at center frequency should pass through with some attenuation
        assert!(
            rms_after > rms_before * 0.3,
            "BP attenuated center too much: before={rms_before} after={rms_after}"
        );
    }

    #[test]
    fn bypass_no_change() {
        let mut buf = make_sine(440.0, 44100, 1024);
        let original: Vec<f32> = buf.samples().to_vec();
        let mut svf = SvfFilter::new(SvfMode::LowPass, 200.0, 0.707, 0.0, 44100, 1);
        svf.set_bypass(true);
        svf.process(&mut buf);
        assert_eq!(buf.samples(), &original);
    }

    #[test]
    fn reset_clears_state() {
        let mut svf = SvfFilter::new(SvfMode::LowPass, 1000.0, 0.707, 0.0, 44100, 2);
        let _ = svf.process_sample(0.5, 0);
        let _ = svf.process_sample(0.5, 1);
        svf.reset();
        // After reset, processing silence should give silence
        let out = svf.process_sample(0.0, 0);
        assert_eq!(out, 0.0);
    }

    #[test]
    fn modulation_safe() {
        // Rapidly sweep cutoff — should not produce NaN or Inf
        let mut svf = SvfFilter::new(SvfMode::LowPass, 1000.0, 0.707, 0.0, 44100, 1);
        for i in 0..10000 {
            let freq = 100.0 + (i as f32 / 10000.0) * 15000.0;
            svf.set_frequency(freq);
            let out = svf.process_sample(0.5, 0);
            assert!(out.is_finite(), "NaN/Inf at freq={freq}");
        }
    }

    #[test]
    fn notch_rejects_center() {
        let mut buf = make_sine(1000.0, 44100, 4096);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::Notch, 1000.0, 5.0, 0.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        assert!(
            rms_after < rms_before * 0.3,
            "Notch didn't reject: before={rms_before} after={rms_after}"
        );
    }

    #[test]
    fn stereo_independent() {
        let mut svf = SvfFilter::new(SvfMode::LowPass, 1000.0, 0.707, 0.0, 44100, 2);
        let out_l = svf.process_sample(1.0, 0);
        let out_r = svf.process_sample(0.0, 1);
        // Channels should produce different outputs
        assert!((out_l - out_r).abs() > 0.001);
    }

    #[test]
    fn output_finite() {
        let mut buf = make_sine(440.0, 44100, 44100);
        let mut svf = SvfFilter::new(SvfMode::LowPass, 2000.0, 0.707, 0.0, 44100, 1);
        svf.process(&mut buf);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn peak_unity_passthrough() {
        // At 0 dB gain, peak filter should pass signal unchanged
        let mut buf = make_sine(1000.0, 44100, 4096);
        let original: Vec<f32> = buf.samples().to_vec();
        let mut svf = SvfFilter::new(SvfMode::Peak, 1000.0, 1.0, 0.0, 44100, 1);
        svf.process(&mut buf);
        // After settling, output should closely match input
        for (out, orig) in buf.samples()[2048..4096].iter().zip(&original[2048..4096]) {
            assert!(
                (out - orig).abs() < 0.01,
                "Peak 0dB diverged: {out} vs {orig}",
            );
        }
    }

    #[test]
    fn peak_boosts_center() {
        // +12 dB peak at 1kHz should boost a 1kHz sine
        let mut buf = make_sine(1000.0, 44100, 8192);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::Peak, 1000.0, 1.0, 12.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        assert!(
            rms_after > rms_before * 1.5,
            "Peak +12dB didn't boost: before={rms_before} after={rms_after}"
        );
    }

    #[test]
    fn peak_leaves_far_frequencies() {
        // +12 dB peak at 1kHz should barely affect 100 Hz
        let mut buf = make_sine(100.0, 44100, 8192);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::Peak, 1000.0, 2.0, 12.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        let ratio = rms_after / rms_before;
        assert!(
            (0.7..1.5).contains(&ratio),
            "Peak affected distant freq too much: ratio={ratio}"
        );
    }

    #[test]
    fn low_shelf_boosts_lows() {
        // +12 dB low shelf at 1kHz should boost 100 Hz
        let mut buf = make_sine(100.0, 44100, 8192);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::LowShelf, 1000.0, 0.707, 12.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        assert!(
            rms_after > rms_before * 1.5,
            "LowShelf didn't boost lows: before={rms_before} after={rms_after}"
        );
    }

    #[test]
    fn low_shelf_spares_highs() {
        // +12 dB low shelf at 1kHz should barely affect 10kHz
        let mut buf = make_sine(10000.0, 44100, 8192);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::LowShelf, 1000.0, 0.707, 12.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        let ratio = rms_after / rms_before;
        assert!(
            (0.7..1.5).contains(&ratio),
            "LowShelf affected highs too much: ratio={ratio}"
        );
    }

    #[test]
    fn high_shelf_boosts_highs() {
        // +12 dB high shelf at 1kHz should boost 10kHz
        let mut buf = make_sine(10000.0, 44100, 8192);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::HighShelf, 1000.0, 0.707, 12.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        assert!(
            rms_after > rms_before * 1.5,
            "HighShelf didn't boost highs: before={rms_before} after={rms_after}"
        );
    }

    #[test]
    fn high_shelf_spares_lows() {
        // +12 dB high shelf at 1kHz should barely affect 100 Hz
        let mut buf = make_sine(100.0, 44100, 8192);
        let rms_before = buf.rms();
        let mut svf = SvfFilter::new(SvfMode::HighShelf, 1000.0, 0.707, 12.0, 44100, 1);
        svf.process(&mut buf);
        let rms_after = buf.rms();
        let ratio = rms_after / rms_before;
        assert!(
            (0.7..1.5).contains(&ratio),
            "HighShelf affected lows too much: ratio={ratio}"
        );
    }
}
