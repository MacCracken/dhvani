//! Stereo panner — constant-power panning law.

use crate::buffer::AudioBuffer;

/// Stereo panner using constant-power (sin/cos) law.
///
/// Pan position: -1.0 = full left, 0.0 = center, +1.0 = full right.
#[must_use]
#[derive(Debug, Clone)]
pub struct StereoPanner {
    pan: f32,
    gain_l: f32,
    gain_r: f32,
}

impl StereoPanner {
    /// Create a panner at the given position (-1.0 to +1.0).
    pub fn new(pan: f32) -> Self {
        tracing::debug!(pan, "StereoPanner::new");
        let mut p = Self {
            pan: 0.0,
            gain_l: 0.0,
            gain_r: 0.0,
        };
        p.set_pan(pan);
        p
    }

    /// Set pan position (-1.0 = left, 0.0 = center, +1.0 = right).
    pub fn set_pan(&mut self, pan: f32) {
        self.pan = pan.clamp(-1.0, 1.0);
        let (l, r) = abaco::dsp::constant_power_pan(self.pan);
        self.gain_l = l;
        self.gain_r = r;
    }

    /// Current pan position.
    pub fn pan(&self) -> f32 {
        self.pan
    }

    /// Process a stereo audio buffer in-place.
    ///
    /// For mono buffers, this is a no-op. For stereo+, applies pan gains to L/R channels.
    #[inline]
    pub fn process(&self, buf: &mut AudioBuffer) {
        if buf.channels < 2 {
            return;
        }
        let ch = buf.channels as usize;
        for frame in 0..buf.frames {
            let l_idx = frame * ch;
            let r_idx = frame * ch + 1;
            buf.samples[l_idx] *= self.gain_l;
            buf.samples[r_idx] *= self.gain_r;
        }
    }
}

impl Default for StereoPanner {
    fn default() -> Self {
        Self::new(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_pan_equal_gains() {
        let panner = StereoPanner::new(0.0);
        // At center, both gains should be equal (~0.707)
        assert!((panner.gain_l - panner.gain_r).abs() < 0.01);
        assert!((panner.gain_l - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01);
    }

    #[test]
    fn full_left() {
        let panner = StereoPanner::new(-1.0);
        assert!((panner.gain_l - 1.0).abs() < 0.01);
        assert!(panner.gain_r.abs() < 0.01);
    }

    #[test]
    fn full_right() {
        let panner = StereoPanner::new(1.0);
        assert!(panner.gain_l.abs() < 0.01);
        assert!((panner.gain_r - 1.0).abs() < 0.01);
    }

    #[test]
    fn constant_power_preserved() {
        // At any position, L^2 + R^2 should equal 1.0
        for p in [-1.0, -0.5, 0.0, 0.25, 0.5, 1.0] {
            let panner = StereoPanner::new(p);
            let power = panner.gain_l * panner.gain_l + panner.gain_r * panner.gain_r;
            assert!((power - 1.0).abs() < 0.01, "power={power} at pan={p}");
        }
    }

    #[test]
    fn process_stereo() {
        let panner = StereoPanner::new(-1.0); // full left
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, 1.0, 1.0, 1.0], 2, 44100).unwrap();
        panner.process(&mut buf);
        assert!((buf.samples[0] - 1.0).abs() < 0.01); // L preserved
        assert!(buf.samples[1].abs() < 0.01); // R zeroed
    }

    #[test]
    fn mono_no_op() {
        let panner = StereoPanner::new(1.0);
        let mut buf = AudioBuffer::from_interleaved(vec![0.5, 0.5], 1, 44100).unwrap();
        let original = buf.samples.clone();
        panner.process(&mut buf);
        assert_eq!(buf.samples, original);
    }
}
