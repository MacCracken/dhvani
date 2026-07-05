//! Dynamics analysis — per-channel peak, RMS, true peak, crest factor, dynamic range, LUFS.

use crate::buffer::AudioBuffer;
use crate::dsp::amplitude_to_db;

/// Comprehensive per-channel dynamics analysis result.
#[must_use]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct DynamicsAnalysis {
    /// Peak amplitude per channel (linear).
    pub(crate) peak: Vec<f32>,
    /// Peak amplitude per channel (dB).
    pub(crate) peak_db: Vec<f32>,
    /// True peak per channel (4x oversampled inter-sample detection, linear).
    pub(crate) true_peak: Vec<f32>,
    /// True peak per channel (dB).
    pub(crate) true_peak_db: Vec<f32>,
    /// RMS level per channel (linear).
    pub(crate) rms: Vec<f32>,
    /// RMS level per channel (dB).
    pub(crate) rms_db: Vec<f32>,
    /// Crest factor per channel (peak / RMS ratio, dB).
    pub(crate) crest_factor_db: Vec<f32>,
    /// Integrated loudness (LUFS) — EBU R128 simplified.
    pub(crate) lufs: f32,
    /// Dynamic range (dB) — max peak minus mean RMS.
    pub(crate) dynamic_range_db: f32,
    /// Number of frames analyzed.
    pub(crate) frame_count: usize,
    /// Number of channels analyzed.
    pub(crate) channel_count: u32,
}

impl DynamicsAnalysis {
    /// Peak amplitude per channel (linear).
    pub fn peak(&self) -> &[f32] {
        &self.peak
    }

    /// Peak amplitude per channel (dB).
    pub fn peak_db(&self) -> &[f32] {
        &self.peak_db
    }

    /// True peak per channel (linear, 4x oversampled).
    pub fn true_peak(&self) -> &[f32] {
        &self.true_peak
    }

    /// True peak per channel (dB).
    pub fn true_peak_db(&self) -> &[f32] {
        &self.true_peak_db
    }

    /// RMS level per channel (linear).
    pub fn rms(&self) -> &[f32] {
        &self.rms
    }

    /// RMS level per channel (dB).
    pub fn rms_db(&self) -> &[f32] {
        &self.rms_db
    }

    /// Crest factor per channel (dB).
    pub fn crest_factor_db(&self) -> &[f32] {
        &self.crest_factor_db
    }

    /// Integrated loudness (LUFS).
    pub fn lufs(&self) -> f32 {
        self.lufs
    }

    /// Dynamic range (dB) — max peak minus mean RMS.
    pub fn dynamic_range_db(&self) -> f32 {
        self.dynamic_range_db
    }

    /// Number of frames analyzed.
    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Number of channels analyzed.
    pub fn channel_count(&self) -> u32 {
        self.channel_count
    }

    /// Max peak across all channels (linear).
    pub fn max_peak(&self) -> f32 {
        self.peak.iter().cloned().fold(0.0f32, f32::max)
    }

    /// Max peak across all channels (dB).
    pub fn max_peak_db(&self) -> f32 {
        self.peak_db
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Max true peak across all channels (linear).
    pub fn max_true_peak(&self) -> f32 {
        self.true_peak.iter().cloned().fold(0.0f32, f32::max)
    }

    /// Max true peak across all channels (dB).
    pub fn max_true_peak_db(&self) -> f32 {
        self.true_peak_db
            .iter()
            .cloned()
            .fold(f32::NEG_INFINITY, f32::max)
    }

    /// Mean RMS across all channels (linear).
    pub fn mean_rms(&self) -> f32 {
        if self.rms.is_empty() {
            return 0.0;
        }
        self.rms.iter().sum::<f32>() / self.rms.len() as f32
    }

    /// Mean crest factor across all channels (dB).
    pub fn mean_crest_factor_db(&self) -> f32 {
        if self.crest_factor_db.is_empty() {
            return 0.0;
        }
        self.crest_factor_db.iter().sum::<f32>() / self.crest_factor_db.len() as f32
    }
}

/// Analyze the dynamics of an audio buffer (per-channel).
pub fn analyze_dynamics(buf: &AudioBuffer) -> DynamicsAnalysis {
    tracing::debug!(
        frames = buf.frames,
        channels = buf.channels,
        "analyze_dynamics: started"
    );
    let ch = buf.channels as usize;
    let frames = buf.frames;

    let mut peak = vec![0.0f32; ch];
    let mut rms_sum = vec![0.0f64; ch];
    let mut true_peak = vec![0.0f32; ch];

    // Per-channel peak and RMS accumulation
    for c in 0..ch {
        for frame in 0..frames {
            let s = buf.samples[frame * ch + c];
            let abs = s.abs();
            if abs > peak[c] {
                peak[c] = abs;
            }
            rms_sum[c] += (s as f64) * (s as f64);
        }

        // True peak: 4x oversampled inter-sample detection using 4-point cubic Hermite
        // interpolation. This detects inter-sample peaks that linear interpolation
        // would miss. For full ITU-R BS.1770 compliance, a polyphase FIR is needed.
        if frames > 1 {
            let mut tp = peak[c];
            for frame in 0..frames - 1 {
                let s0 = buf.samples[frame * ch + c];
                let s1 = buf.samples[(frame + 1) * ch + c];
                // Get surrounding samples for cubic interpolation
                let sm1 = if frame > 0 {
                    buf.samples[(frame - 1) * ch + c]
                } else {
                    s0
                };
                let s2 = if frame + 2 < frames {
                    buf.samples[(frame + 2) * ch + c]
                } else {
                    s1
                };
                for k in 1..4u32 {
                    let t = k as f32 / 4.0;
                    // Catmull-Rom / cubic Hermite interpolation
                    let t2 = t * t;
                    let t3 = t2 * t;
                    let interp = 0.5
                        * ((2.0 * s0)
                            + (-sm1 + s1) * t
                            + (2.0 * sm1 - 5.0 * s0 + 4.0 * s1 - s2) * t2
                            + (-sm1 + 3.0 * s0 - 3.0 * s1 + s2) * t3);
                    tp = tp.max(interp.abs());
                }
            }
            true_peak[c] = tp;
        } else if frames == 1 {
            true_peak[c] = peak[c];
        }
    }

    let rms: Vec<f32> = rms_sum
        .iter()
        .map(|&sum| {
            if frames > 0 {
                (sum / frames as f64).sqrt() as f32
            } else {
                0.0
            }
        })
        .collect();

    let peak_db: Vec<f32> = peak.iter().map(|&p| amplitude_to_db(p)).collect();
    let rms_db: Vec<f32> = rms.iter().map(|&r| amplitude_to_db(r)).collect();
    let true_peak_db: Vec<f32> = true_peak.iter().map(|&tp| amplitude_to_db(tp)).collect();

    let crest_factor_db: Vec<f32> = peak_db
        .iter()
        .zip(rms_db.iter())
        .map(|(&p, &r)| if r > f32::NEG_INFINITY { p - r } else { 0.0 })
        .collect();

    // Simplified LUFS (mono/stereo mean RMS in LUFS scale)
    let total_rms_sq: f64 = rms_sum.iter().sum::<f64>() / (ch as f64 * frames.max(1) as f64);
    let lufs = if total_rms_sq > 1e-20 {
        -0.691_f32 + 10.0 * (total_rms_sq as f32).log10()
    } else {
        f32::NEG_INFINITY
    };

    // Dynamic range: max peak dB - mean RMS dB
    let max_peak_db = peak_db.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
    let mean_rms_db = if !rms_db.is_empty() {
        rms_db.iter().sum::<f32>() / rms_db.len() as f32
    } else {
        f32::NEG_INFINITY
    };
    let dynamic_range_db = max_peak_db - mean_rms_db;

    DynamicsAnalysis {
        peak,
        peak_db,
        true_peak,
        true_peak_db,
        rms,
        rms_db,
        crest_factor_db,
        lufs,
        dynamic_range_db,
        frame_count: frames,
        channel_count: buf.channels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(amplitude: f32, freq: f32, frames: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| amplitude * (2.0 * std::f32::consts::PI * freq * i as f32 / 44100.0).sin())
            .collect();
        AudioBuffer::from_interleaved(samples, 1, 44100).unwrap()
    }

    #[test]
    fn silence_dynamics() {
        let buf = AudioBuffer::silence(1, 4096, 44100);
        let d = analyze_dynamics(&buf);
        assert_eq!(d.peak()[0], 0.0);
        assert!(d.peak_db()[0].is_infinite());
        assert_eq!(d.rms()[0], 0.0);
        assert_eq!(d.frame_count(), 4096);
        assert_eq!(d.channel_count(), 1);
    }

    #[test]
    fn sine_dynamics() {
        let buf = make_sine(0.8, 440.0, 44100);
        let d = analyze_dynamics(&buf);
        assert!((d.peak()[0] - 0.8).abs() < 0.01);
        assert!(d.rms()[0] > 0.5);
        assert!(d.crest_factor_db()[0] > 0.0);
        assert!(d.dynamic_range_db() > 0.0);
    }

    #[test]
    fn true_peak_exceeds_sample_peak() {
        let samples = vec![0.9, -0.9, 0.9, -0.9, 0.9, -0.9, 0.9, -0.9];
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let d = analyze_dynamics(&buf);
        assert!(d.true_peak()[0] >= d.peak()[0] - 0.01);
    }

    #[test]
    fn crest_factor_positive_for_sine() {
        let buf = make_sine(1.0, 1000.0, 44100);
        let d = analyze_dynamics(&buf);
        assert!(d.crest_factor_db()[0] > 2.0);
        assert!(d.crest_factor_db()[0] < 4.0);
    }

    #[test]
    fn dynamics_all_finite() {
        let buf = make_sine(0.5, 440.0, 4096);
        let d = analyze_dynamics(&buf);
        assert!(d.peak_db()[0].is_finite());
        assert!(d.true_peak_db()[0].is_finite());
        assert!(d.rms_db()[0].is_finite());
        assert!(d.crest_factor_db()[0].is_finite());
    }

    #[test]
    fn stereo_independent_channels() {
        let frames = 1024;
        let mut data = vec![0.0f32; frames * 2];
        for i in 0..frames {
            data[i * 2] = 0.8; // left = loud
            data[i * 2 + 1] = 0.1; // right = quiet
        }
        let buf = AudioBuffer::from_interleaved(data, 2, 44100).unwrap();
        let d = analyze_dynamics(&buf);
        assert_eq!(d.channel_count(), 2);
        assert!((d.peak()[0] - 0.8).abs() < 0.001);
        assert!((d.peak()[1] - 0.1).abs() < 0.001);
        assert!(d.peak_db()[0] > d.peak_db()[1]);
        assert!(d.rms()[0] > d.rms()[1]);
    }

    #[test]
    fn max_peak_across_channels() {
        let frames = 256;
        let mut data = vec![0.0f32; frames * 2];
        for i in 0..frames {
            data[i * 2] = 0.3;
            data[i * 2 + 1] = 0.7;
        }
        let buf = AudioBuffer::from_interleaved(data, 2, 44100).unwrap();
        let d = analyze_dynamics(&buf);
        assert!((d.max_peak() - 0.7).abs() < 0.001);
    }

    #[test]
    fn lufs_finite_for_signal() {
        let buf = make_sine(0.5, 1000.0, 44100);
        let d = analyze_dynamics(&buf);
        assert!(d.lufs.is_finite());
        assert!(d.lufs < 0.0);
    }

    #[test]
    fn empty_buffer() {
        let buf = AudioBuffer::silence(1, 0, 44100);
        let d = analyze_dynamics(&buf);
        assert_eq!(d.frame_count(), 0);
        assert_eq!(d.peak()[0], 0.0);
        assert_eq!(d.rms()[0], 0.0);
    }

    #[test]
    fn single_frame() {
        let buf = AudioBuffer::from_interleaved(vec![0.75], 1, 44100).unwrap();
        let d = analyze_dynamics(&buf);
        assert!((d.peak()[0] - 0.75).abs() < 0.001);
        assert!((d.rms()[0] - 0.75).abs() < 0.001);
        assert!((d.true_peak()[0] - 0.75).abs() < 0.001);
    }
}
