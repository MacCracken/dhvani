//! Channel routing matrix — N×M routing with per-crosspoint gain.
//!
//! Routes audio from N input channels to M output channels with independent
//! gain at each crosspoint. Supports mono-to-surround, surround-to-stereo,
//! mid-side encoding, and arbitrary channel mapping.

use crate::buffer::AudioBuffer;
use serde::{Deserialize, Serialize};

/// N×M channel routing matrix.
///
/// Each element `gains[out][inp]` is the gain applied when routing
/// input channel `inp` to output channel `out`.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMatrix {
    /// Gain matrix: `gains[output_ch][input_ch]`.
    gains: Vec<Vec<f32>>,
    /// Number of input channels.
    inputs: usize,
    /// Number of output channels.
    outputs: usize,
}

impl RoutingMatrix {
    /// Create a routing matrix with all crosspoints at zero.
    pub fn new(inputs: usize, outputs: usize) -> Self {
        Self {
            gains: vec![vec![0.0; inputs]; outputs],
            inputs,
            outputs,
        }
    }

    /// Create a pass-through (identity) routing matrix.
    ///
    /// Maps each input channel to the corresponding output channel at unity gain.
    /// Extra inputs or outputs are silent.
    pub fn identity(channels: usize) -> Self {
        let mut m = Self::new(channels, channels);
        for i in 0..channels {
            m.gains[i][i] = 1.0;
        }
        m
    }

    /// Create a mono-to-stereo matrix (duplicate to both channels).
    pub fn mono_to_stereo() -> Self {
        let mut m = Self::new(1, 2);
        m.gains[0][0] = 1.0; // L ← mono
        m.gains[1][0] = 1.0; // R ← mono
        m
    }

    /// Create a stereo-to-mono matrix (average L+R).
    pub fn stereo_to_mono() -> Self {
        let mut m = Self::new(2, 1);
        m.gains[0][0] = 0.5; // mono ← L
        m.gains[0][1] = 0.5; // mono ← R
        m
    }

    /// Create a mid-side encoder (stereo → M/S).
    pub fn mid_side_encode() -> Self {
        let mut m = Self::new(2, 2);
        m.gains[0][0] = 0.5; // Mid ← 0.5*L
        m.gains[0][1] = 0.5; // Mid ← 0.5*R
        m.gains[1][0] = 0.5; // Side ← 0.5*L
        m.gains[1][1] = -0.5; // Side ← -0.5*R
        m
    }

    /// Create a mid-side decoder (M/S → stereo).
    pub fn mid_side_decode() -> Self {
        let mut m = Self::new(2, 2);
        m.gains[0][0] = 1.0; // L ← Mid
        m.gains[0][1] = 1.0; // L ← Side
        m.gains[1][0] = 1.0; // R ← Mid
        m.gains[1][1] = -1.0; // R ← -Side
        m
    }

    /// Set the gain at a crosspoint.
    ///
    /// Returns `false` if indices are out of bounds.
    pub fn set_gain(&mut self, output_ch: usize, input_ch: usize, gain: f32) -> bool {
        if output_ch < self.outputs && input_ch < self.inputs {
            self.gains[output_ch][input_ch] = gain;
            true
        } else {
            false
        }
    }

    /// Get the gain at a crosspoint.
    #[must_use]
    pub fn gain(&self, output_ch: usize, input_ch: usize) -> f32 {
        if output_ch < self.outputs && input_ch < self.inputs {
            self.gains[output_ch][input_ch]
        } else {
            0.0
        }
    }

    /// Number of input channels.
    #[must_use]
    pub fn inputs(&self) -> usize {
        self.inputs
    }

    /// Number of output channels.
    #[must_use]
    pub fn outputs(&self) -> usize {
        self.outputs
    }

    /// Apply the routing matrix to an audio buffer.
    ///
    /// Input buffer must have `self.inputs()` channels.
    /// Returns a new buffer with `self.outputs()` channels.
    ///
    /// # Errors
    ///
    /// Returns an error if the input channel count doesn't match.
    pub fn apply(&self, buf: &AudioBuffer) -> crate::Result<AudioBuffer> {
        let in_ch = buf.channels as usize;
        if in_ch != self.inputs {
            return Err(crate::NadaError::Conversion(format!(
                "routing matrix expects {} input channels, got {}",
                self.inputs, in_ch
            )));
        }

        let frames = buf.frames;
        let out_ch = self.outputs;
        let mut output = vec![0.0f32; frames * out_ch];

        for frame in 0..frames {
            for o in 0..out_ch {
                let mut sum = 0.0f32;
                for i in 0..self.inputs {
                    let gain = self.gains[o][i];
                    if gain != 0.0 {
                        sum += buf.samples[frame * in_ch + i] * gain;
                    }
                }
                output[frame * out_ch + o] = sum;
            }
        }

        AudioBuffer::from_interleaved(output, out_ch as u32, buf.sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_passthrough() {
        let m = RoutingMatrix::identity(2);
        let buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 2, 44100).unwrap();
        let out = m.apply(&buf).unwrap();
        assert_eq!(out.channels(), 2);
        assert_eq!(out.samples(), buf.samples());
    }

    #[test]
    fn mono_to_stereo() {
        let m = RoutingMatrix::mono_to_stereo();
        let buf = AudioBuffer::from_interleaved(vec![0.5, 0.3], 1, 44100).unwrap();
        let out = m.apply(&buf).unwrap();
        assert_eq!(out.channels(), 2);
        // L and R should both equal the mono input
        assert_eq!(out.samples()[0], 0.5); // frame 0 L
        assert_eq!(out.samples()[1], 0.5); // frame 0 R
        assert_eq!(out.samples()[2], 0.3); // frame 1 L
        assert_eq!(out.samples()[3], 0.3); // frame 1 R
    }

    #[test]
    fn stereo_to_mono() {
        let m = RoutingMatrix::stereo_to_mono();
        let buf = AudioBuffer::from_interleaved(vec![1.0, 0.5], 2, 44100).unwrap();
        let out = m.apply(&buf).unwrap();
        assert_eq!(out.channels(), 1);
        assert!((out.samples()[0] - 0.75).abs() < 1e-6); // (1.0 + 0.5) / 2
    }

    #[test]
    fn mid_side_roundtrip() {
        let buf = AudioBuffer::from_interleaved(vec![0.8, 0.2, 0.6, 0.4], 2, 44100).unwrap();
        let encoded = RoutingMatrix::mid_side_encode().apply(&buf).unwrap();
        let decoded = RoutingMatrix::mid_side_decode().apply(&encoded).unwrap();

        for (orig, dec) in buf.samples().iter().zip(decoded.samples().iter()) {
            assert!((orig - dec).abs() < 1e-5, "M/S roundtrip: {orig} != {dec}");
        }
    }

    #[test]
    fn custom_crosspoint_gain() {
        let mut m = RoutingMatrix::new(2, 2);
        assert!(m.set_gain(0, 0, 0.7));
        assert!(m.set_gain(1, 1, 0.3));
        assert_eq!(m.gain(0, 0), 0.7);
        assert_eq!(m.gain(1, 1), 0.3);
        assert_eq!(m.gain(0, 1), 0.0); // unset crosspoint
    }

    #[test]
    fn channel_mismatch_error() {
        let m = RoutingMatrix::new(2, 2);
        let buf = AudioBuffer::from_interleaved(vec![0.5], 1, 44100).unwrap();
        assert!(m.apply(&buf).is_err());
    }

    #[test]
    fn zero_gain_passthrough() {
        let m = RoutingMatrix::new(2, 2); // all zeros
        let buf = AudioBuffer::from_interleaved(vec![1.0, 1.0], 2, 44100).unwrap();
        let out = m.apply(&buf).unwrap();
        assert!(out.samples().iter().all(|&s| s == 0.0));
    }

    #[test]
    fn out_of_bounds_set_gain() {
        let mut m = RoutingMatrix::new(2, 2);
        assert!(!m.set_gain(5, 0, 1.0));
        assert_eq!(m.gain(5, 0), 0.0);
    }
}
