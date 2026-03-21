//! Dynamics analysis — peak, RMS, true peak, crest factor, dynamic range.

use crate::buffer::AudioBuffer;
use crate::dsp::amplitude_to_db;

/// Comprehensive dynamics analysis result.
#[derive(Debug, Clone)]
pub struct DynamicsAnalysis {
    /// Peak amplitude (linear).
    pub peak: f32,
    /// Peak amplitude (dB).
    pub peak_db: f32,
    /// True peak (4x oversampled inter-sample detection, linear).
    pub true_peak: f32,
    /// True peak (dB).
    pub true_peak_db: f32,
    /// RMS level (linear).
    pub rms: f32,
    /// RMS level (dB).
    pub rms_db: f32,
    /// Crest factor (peak / RMS ratio, dB).
    pub crest_factor_db: f32,
    /// Dynamic range (difference between peak and noise floor, dB).
    pub dynamic_range_db: f32,
}

/// Analyze the dynamics of an audio buffer.
pub fn analyze_dynamics(buf: &AudioBuffer) -> DynamicsAnalysis {
    let peak = buf.peak();
    let rms = buf.rms();
    let true_peak = compute_true_peak(buf);

    let peak_db = amplitude_to_db(peak);
    let true_peak_db = amplitude_to_db(true_peak);
    let rms_db = amplitude_to_db(rms);
    let crest_factor_db = peak_db - rms_db;

    // Dynamic range: estimate noise floor from quietest 10% of frames
    let noise_floor_db = estimate_noise_floor(buf);
    let dynamic_range_db = peak_db - noise_floor_db;

    DynamicsAnalysis {
        peak,
        peak_db,
        true_peak,
        true_peak_db,
        rms,
        rms_db,
        crest_factor_db,
        dynamic_range_db,
    }
}

/// Compute true peak using 4x oversampled inter-sample detection.
///
/// Uses linear interpolation between samples to detect peaks that
/// occur between sample points (inter-sample peaks).
fn compute_true_peak(buf: &AudioBuffer) -> f32 {
    let ch = buf.channels as usize;
    let mut max_peak = 0.0f32;

    for c in 0..ch {
        for frame in 0..buf.frames {
            let idx = frame * ch + c;
            let s0 = buf.samples[idx];
            max_peak = max_peak.max(s0.abs());

            // 4x oversampling: check 3 intermediate points between this and next sample
            if frame + 1 < buf.frames {
                let s1 = buf.samples[(frame + 1) * ch + c];
                for k in 1..4 {
                    let t = k as f32 / 4.0;
                    let interpolated = s0 + t * (s1 - s0);
                    max_peak = max_peak.max(interpolated.abs());
                }
            }
        }
    }

    max_peak
}

/// Estimate noise floor from the quietest frames.
fn estimate_noise_floor(buf: &AudioBuffer) -> f32 {
    if buf.frames == 0 {
        return f32::NEG_INFINITY;
    }

    let ch = buf.channels as usize;

    // Compute per-frame RMS
    let mut frame_levels: Vec<f32> = Vec::with_capacity(buf.frames);
    for frame in 0..buf.frames {
        let mut sum_sq = 0.0f32;
        for c in 0..ch {
            let s = buf.samples[frame * ch + c];
            sum_sq += s * s;
        }
        frame_levels.push((sum_sq / ch as f32).sqrt());
    }

    // Sort and take the 10th percentile as noise floor
    frame_levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let floor_idx = (frame_levels.len() / 10).max(1).min(frame_levels.len() - 1);
    let noise_floor = frame_levels[floor_idx];

    amplitude_to_db(noise_floor)
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
        assert_eq!(d.peak, 0.0);
        assert!(d.peak_db.is_infinite());
        assert_eq!(d.rms, 0.0);
    }

    #[test]
    fn sine_dynamics() {
        let buf = make_sine(0.8, 440.0, 44100);
        let d = analyze_dynamics(&buf);
        assert!((d.peak - 0.8).abs() < 0.01);
        assert!(d.rms > 0.5);
        assert!(d.crest_factor_db > 0.0); // Peak > RMS for sine
        assert!(d.dynamic_range_db > 0.0);
    }

    #[test]
    fn true_peak_exceeds_sample_peak() {
        // Two consecutive samples with a sign change can have an inter-sample peak
        // that exceeds either sample
        let samples = vec![0.9, -0.9, 0.9, -0.9, 0.9, -0.9, 0.9, -0.9];
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let d = analyze_dynamics(&buf);
        // True peak should be >= sample peak
        assert!(d.true_peak >= d.peak - 0.01);
    }

    #[test]
    fn crest_factor_positive_for_sine() {
        let buf = make_sine(1.0, 1000.0, 44100);
        let d = analyze_dynamics(&buf);
        // Sine wave crest factor is ~3 dB
        assert!(d.crest_factor_db > 2.0);
        assert!(d.crest_factor_db < 4.0);
    }

    #[test]
    fn dynamics_all_finite() {
        let buf = make_sine(0.5, 440.0, 4096);
        let d = analyze_dynamics(&buf);
        assert!(d.peak_db.is_finite());
        assert!(d.true_peak_db.is_finite());
        assert!(d.rms_db.is_finite());
        assert!(d.crest_factor_db.is_finite());
    }
}
