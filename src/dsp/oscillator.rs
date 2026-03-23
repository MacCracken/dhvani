//! Oscillator — anti-aliased waveform generation with PolyBLEP.
//!
//! Provides sine, saw, square, triangle, and noise waveforms suitable
//! for synthesis. Consumers pair this with [`VoiceManager`](crate::midi::voice::VoiceManager)
//! and [`Envelope`](super::envelope::Envelope).

use serde::{Deserialize, Serialize};

/// Waveform type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Waveform {
    Sine,
    Saw,
    Square,
    Triangle,
    Noise,
}

/// Anti-aliased oscillator with PolyBLEP correction.
#[derive(Debug, Clone)]
pub struct Oscillator {
    waveform: Waveform,
    phase: f64,
    sample_rate: f64,
    rng_state: u32,
}

impl Oscillator {
    /// Validate that the oscillator is properly configured.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.sample_rate <= 0.0 {
            return Err("sample_rate must be > 0.0");
        }
        Ok(())
    }

    /// Create a new oscillator.
    pub fn new(waveform: Waveform, sample_rate: u32) -> Self {
        Self {
            waveform,
            phase: 0.0,
            sample_rate: sample_rate as f64,
            rng_state: 0x12345678,
        }
    }

    /// Set the waveform type.
    pub fn set_waveform(&mut self, waveform: Waveform) {
        self.waveform = waveform;
    }

    /// Current waveform.
    pub fn waveform(&self) -> Waveform {
        self.waveform
    }

    /// Generate the next sample at the given frequency.
    ///
    /// Call once per sample. Advances the internal phase.
    pub fn sample(&mut self, freq: f64) -> f32 {
        let dt = freq / self.sample_rate;
        let value = match self.waveform {
            Waveform::Sine => (self.phase * std::f64::consts::TAU).sin(),
            Waveform::Saw => {
                let naive = 2.0 * self.phase - 1.0;
                naive - poly_blep(self.phase, dt)
            }
            Waveform::Square => {
                let naive = if self.phase < 0.5 { 1.0 } else { -1.0 };
                naive + poly_blep(self.phase, dt) - poly_blep((self.phase + 0.5) % 1.0, dt)
            }
            Waveform::Triangle => {
                // Integrate square wave for triangle
                let sq = if self.phase < 0.5 { 1.0 } else { -1.0 };
                let sq_blep =
                    sq + poly_blep(self.phase, dt) - poly_blep((self.phase + 0.5) % 1.0, dt);
                // Leaky integrator approximation for triangle from square
                // Use direct formula instead: 2*|2*phase - 1| - 1
                let tri = 4.0 * (self.phase - 0.5).abs() - 1.0;
                // Blend: use direct formula (less aliased at high freq)
                let _ = sq_blep;
                tri
            }
            Waveform::Noise => {
                self.rng_state ^= self.rng_state << 13;
                self.rng_state ^= self.rng_state >> 17;
                self.rng_state ^= self.rng_state << 5;
                (self.rng_state as f64 / u32::MAX as f64) * 2.0 - 1.0
            }
        };

        // Advance phase
        self.phase += dt;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        value as f32
    }

    /// Reset phase to zero.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Current phase (0.0–1.0).
    pub fn phase(&self) -> f64 {
        self.phase
    }

    /// Set the sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate as f64;
    }
}

use abaco::dsp::poly_blep;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_oscillates() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        let mut has_positive = false;
        let mut has_negative = false;
        for _ in 0..44100 {
            let s = osc.sample(440.0);
            if s > 0.1 {
                has_positive = true;
            }
            if s < -0.1 {
                has_negative = true;
            }
        }
        assert!(has_positive && has_negative, "Sine should oscillate");
    }

    #[test]
    fn saw_range() {
        let mut osc = Oscillator::new(Waveform::Saw, 44100);
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for _ in 0..44100 {
            let s = osc.sample(440.0);
            min = min.min(s);
            max = max.max(s);
        }
        assert!(min < -0.9);
        assert!(max > 0.9);
    }

    #[test]
    fn square_bipolar() {
        let mut osc = Oscillator::new(Waveform::Square, 44100);
        let mut has_high = false;
        let mut has_low = false;
        for _ in 0..44100 {
            let s = osc.sample(440.0);
            if s > 0.5 {
                has_high = true;
            }
            if s < -0.5 {
                has_low = true;
            }
        }
        assert!(has_high && has_low);
    }

    #[test]
    fn triangle_range() {
        let mut osc = Oscillator::new(Waveform::Triangle, 44100);
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for _ in 0..44100 {
            let s = osc.sample(440.0);
            min = min.min(s);
            max = max.max(s);
        }
        assert!(min < -0.9);
        assert!(max > 0.9);
    }

    #[test]
    fn noise_nonzero() {
        let mut osc = Oscillator::new(Waveform::Noise, 44100);
        let mut sum = 0.0f64;
        for _ in 0..1000 {
            sum += osc.sample(440.0).abs() as f64;
        }
        assert!(sum > 0.0, "Noise should produce non-zero output");
    }

    #[test]
    fn reset_clears_phase() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        for _ in 0..100 {
            osc.sample(440.0);
        }
        assert!(osc.phase() > 0.0);
        osc.reset();
        assert!((osc.phase()).abs() < f64::EPSILON);
    }

    #[test]
    fn all_waveforms_finite() {
        for wf in [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
            Waveform::Noise,
        ] {
            let mut osc = Oscillator::new(wf, 44100);
            for _ in 0..4096 {
                let s = osc.sample(440.0);
                assert!(s.is_finite(), "{wf:?} produced non-finite sample");
            }
        }
    }

    #[test]
    fn set_waveform() {
        let mut osc = Oscillator::new(Waveform::Sine, 44100);
        assert_eq!(osc.waveform(), Waveform::Sine);
        osc.set_waveform(Waveform::Saw);
        assert_eq!(osc.waveform(), Waveform::Saw);
    }
}
