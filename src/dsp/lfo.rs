//! LFO — Low-Frequency Oscillator for modulation.
//!
//! Six waveform shapes with configurable rate and depth.
//! Output range: `[-depth, +depth]`.

use serde::{Deserialize, Serialize};

/// LFO waveform shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LfoShape {
    Sine,
    Triangle,
    Square,
    SawUp,
    SawDown,
    /// Random value held until next cycle.
    SampleAndHold,
}

/// Low-frequency oscillator for parameter modulation.
#[derive(Debug, Clone)]
pub struct Lfo {
    shape: LfoShape,
    /// Rate in Hz.
    rate: f32,
    /// Output amplitude (0.0–1.0).
    depth: f32,
    phase: f64,
    sample_rate: f32,
    /// Held value for sample-and-hold.
    sh_value: f32,
    rng_state: u32,
    prev_phase: f64,
}

impl Lfo {
    /// Create a new LFO.
    pub fn new(shape: LfoShape, rate: f32, depth: f32, sample_rate: u32) -> Self {
        Self {
            shape,
            rate,
            depth: depth.clamp(0.0, 1.0),
            phase: 0.0,
            sample_rate: sample_rate as f32,
            sh_value: 0.0,
            rng_state: 0xDEADBEEF,
            prev_phase: 0.0,
        }
    }

    /// Advance one sample and return the current value (-depth to +depth).
    pub fn tick(&mut self) -> f32 {
        let raw = match self.shape {
            LfoShape::Sine => (self.phase * std::f64::consts::TAU).sin() as f32,
            LfoShape::Triangle => {
                let t = self.phase as f32;
                4.0 * (t - 0.5).abs() - 1.0
            }
            LfoShape::Square => {
                if self.phase < 0.5 { 1.0 } else { -1.0 }
            }
            LfoShape::SawUp => (2.0 * self.phase - 1.0) as f32,
            LfoShape::SawDown => (1.0 - 2.0 * self.phase) as f32,
            LfoShape::SampleAndHold => {
                // Update on phase wrap
                if self.phase < self.prev_phase {
                    self.rng_state ^= self.rng_state << 13;
                    self.rng_state ^= self.rng_state >> 17;
                    self.rng_state ^= self.rng_state << 5;
                    self.sh_value = (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0;
                }
                self.sh_value
            }
        };

        self.prev_phase = self.phase;
        self.phase += self.rate as f64 / self.sample_rate as f64;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        raw * self.depth
    }

    /// Reset phase to zero.
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.prev_phase = 0.0;
        self.sh_value = 0.0;
    }

    /// Set the rate in Hz.
    pub fn set_rate(&mut self, rate: f32) {
        self.rate = rate;
    }

    /// Set the depth (0.0–1.0).
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth.clamp(0.0, 1.0);
    }

    /// Set the shape.
    pub fn set_shape(&mut self, shape: LfoShape) {
        self.shape = shape;
    }

    /// Current rate in Hz.
    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// Current depth.
    pub fn depth(&self) -> f32 {
        self.depth
    }

    /// Current shape.
    pub fn shape(&self) -> LfoShape {
        self.shape
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_oscillates() {
        let mut lfo = Lfo::new(LfoShape::Sine, 1.0, 1.0, 1000);
        let mut has_pos = false;
        let mut has_neg = false;
        for _ in 0..1000 {
            let v = lfo.tick();
            if v > 0.3 { has_pos = true; }
            if v < -0.3 { has_neg = true; }
        }
        assert!(has_pos && has_neg);
    }

    #[test]
    fn depth_scales_output() {
        let mut lfo = Lfo::new(LfoShape::Sine, 10.0, 0.5, 44100);
        let mut max_abs = 0.0f32;
        for _ in 0..44100 {
            max_abs = max_abs.max(lfo.tick().abs());
        }
        assert!(max_abs <= 0.51, "max={max_abs} should be <= depth 0.5");
        assert!(max_abs > 0.4, "max={max_abs} should be near 0.5");
    }

    #[test]
    fn square_bipolar() {
        let mut lfo = Lfo::new(LfoShape::Square, 10.0, 1.0, 44100);
        let mut values = std::collections::HashSet::new();
        for _ in 0..44100 {
            let v = lfo.tick();
            if v > 0.5 { values.insert(1); }
            if v < -0.5 { values.insert(-1); }
        }
        assert!(values.contains(&1) && values.contains(&-1));
    }

    #[test]
    fn triangle_range() {
        let mut lfo = Lfo::new(LfoShape::Triangle, 10.0, 1.0, 44100);
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for _ in 0..44100 {
            let v = lfo.tick();
            min = min.min(v);
            max = max.max(v);
        }
        assert!(min < -0.9);
        assert!(max > 0.9);
    }

    #[test]
    fn saw_up_range() {
        let mut lfo = Lfo::new(LfoShape::SawUp, 10.0, 1.0, 44100);
        let mut min = f32::MAX;
        let mut max = f32::MIN;
        for _ in 0..44100 {
            let v = lfo.tick();
            min = min.min(v);
            max = max.max(v);
        }
        assert!(min < -0.9);
        assert!(max > 0.9);
    }

    #[test]
    fn sample_and_hold_changes() {
        let mut lfo = Lfo::new(LfoShape::SampleAndHold, 10.0, 1.0, 44100);
        let mut values = Vec::new();
        let mut prev = lfo.tick();
        for _ in 0..44100 {
            let v = lfo.tick();
            if (v - prev).abs() > 0.01 {
                values.push(v);
            }
            prev = v;
        }
        assert!(values.len() > 5, "S&H should change multiple times per second");
    }

    #[test]
    fn reset_clears() {
        let mut lfo = Lfo::new(LfoShape::Sine, 10.0, 1.0, 44100);
        for _ in 0..1000 { lfo.tick(); }
        lfo.reset();
        assert!((lfo.phase).abs() < f64::EPSILON);
    }

    #[test]
    fn setters() {
        let mut lfo = Lfo::new(LfoShape::Sine, 1.0, 0.5, 44100);
        lfo.set_rate(5.0);
        lfo.set_depth(0.8);
        lfo.set_shape(LfoShape::Square);
        assert_eq!(lfo.rate(), 5.0);
        assert!((lfo.depth() - 0.8).abs() < f32::EPSILON);
        assert_eq!(lfo.shape(), LfoShape::Square);
    }
}
