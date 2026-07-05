//! Buffer utilities — crossfade, fade in/out, target loudness normalization.

use crate::buffer::AudioBuffer;
use crate::error::NadaError;

/// Crossfade type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CrossfadeType {
    /// Linear crossfade (constant amplitude sum).
    Linear,
    /// Equal-power crossfade (constant energy sum).
    EqualPower,
}

/// Fade curve type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FadeCurve {
    /// Linear ramp.
    Linear,
    /// Exponential curve (more natural sounding).
    Exponential,
}

/// Crossfade between two buffers over their full length.
///
/// Both buffers must have the same channels, sample rate, and frame count.
/// Returns a new buffer where `a` fades out and `b` fades in.
pub fn crossfade(
    a: &AudioBuffer,
    b: &AudioBuffer,
    kind: CrossfadeType,
) -> Result<AudioBuffer, NadaError> {
    if a.channels != b.channels {
        return Err(NadaError::FormatMismatch {
            expected: format!("{} ch", a.channels),
            actual: format!("{} ch", b.channels),
        });
    }
    if a.sample_rate != b.sample_rate {
        return Err(NadaError::InvalidSampleRate(b.sample_rate));
    }
    if a.frames != b.frames {
        return Err(NadaError::LengthMismatch {
            expected: a.frames,
            actual: b.frames,
        });
    }

    let len = a.samples.len();
    let mut samples = vec![0.0f32; len];
    let frames = a.frames;

    for (i, (sa, sb)) in a.samples.iter().zip(b.samples.iter()).enumerate() {
        let frame = i / a.channels as usize;
        let t = if frames > 1 {
            frame as f32 / (frames - 1) as f32
        } else {
            1.0
        };

        let (gain_a, gain_b) = match kind {
            CrossfadeType::Linear => (1.0 - t, t),
            CrossfadeType::EqualPower => abaco::dsp::equal_power_crossfade(t),
        };

        samples[i] = sa * gain_a + sb * gain_b;
    }

    Ok(AudioBuffer {
        samples,
        channels: a.channels,
        sample_rate: a.sample_rate,
        frames,
    })
}

/// Apply a fade-in to the beginning of a buffer.
///
/// `fade_frames` is the number of frames over which to ramp from silence to full volume.
/// If `fade_frames` exceeds the buffer length, the entire buffer is faded.
pub fn fade_in(buf: &mut AudioBuffer, fade_frames: usize, curve: FadeCurve) {
    let fade_frames = fade_frames.min(buf.frames);
    let ch = buf.channels as usize;

    for frame in 0..fade_frames {
        let t = if fade_frames > 1 {
            frame as f32 / (fade_frames - 1) as f32
        } else {
            1.0
        };

        let gain = match curve {
            FadeCurve::Linear => t,
            FadeCurve::Exponential => t * t,
        };

        for c in 0..ch {
            let idx = frame * ch + c;
            buf.samples[idx] *= gain;
        }
    }
}

/// Apply a fade-out to the end of a buffer.
///
/// `fade_frames` is the number of frames over which to ramp from full volume to silence.
/// If `fade_frames` exceeds the buffer length, the entire buffer is faded.
pub fn fade_out(buf: &mut AudioBuffer, fade_frames: usize, curve: FadeCurve) {
    let fade_frames = fade_frames.min(buf.frames);
    let ch = buf.channels as usize;
    let start = buf.frames - fade_frames;

    for frame in 0..fade_frames {
        let t = if fade_frames > 1 {
            frame as f32 / (fade_frames - 1) as f32
        } else {
            0.0
        };

        let gain = match curve {
            FadeCurve::Linear => 1.0 - t,
            FadeCurve::Exponential => {
                let inv = 1.0 - t;
                inv * inv
            }
        };

        for c in 0..ch {
            let idx = (start + frame) * ch + c;
            buf.samples[idx] *= gain;
        }
    }
}

/// Normalize a buffer to a target loudness in LUFS.
///
/// Measures the current integrated loudness using EBU R128, computes the
/// required gain, and applies it. Returns the applied gain in dB.
///
/// # Errors
///
/// Returns an error if loudness measurement fails (e.g., empty buffer).
#[cfg(feature = "analysis")]
pub fn normalize_to_lufs(buf: &mut AudioBuffer, target_lufs: f32) -> crate::Result<f32> {
    let r128 = crate::analysis::loudness::measure_r128(buf)?;
    let current_lufs = r128.integrated_lufs;

    if !current_lufs.is_finite() || current_lufs < -120.0 {
        // Signal is silent or too quiet to normalize
        return Ok(0.0);
    }

    let gain_db = target_lufs - current_lufs;
    let gain_lin = abaco::dsp::db_to_amplitude(gain_db);
    buf.apply_gain(gain_lin);

    Ok(gain_db)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_sine(frames: usize) -> AudioBuffer {
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        AudioBuffer::from_interleaved(samples, 1, 44100).unwrap()
    }

    #[test]
    fn linear_crossfade_midpoint() {
        let a = AudioBuffer::from_interleaved(vec![1.0; 4], 1, 44100).unwrap();
        let b = AudioBuffer::from_interleaved(vec![0.0; 4], 1, 44100).unwrap();
        let result = crossfade(&a, &b, CrossfadeType::Linear).unwrap();
        // First sample: 100% a, last sample: 100% b
        assert!((result.samples[0] - 1.0).abs() < 0.01);
        assert!(result.samples[3].abs() < 0.01);
    }

    #[test]
    fn equal_power_crossfade() {
        let a = AudioBuffer::from_interleaved(vec![1.0; 100], 1, 44100).unwrap();
        let b = AudioBuffer::from_interleaved(vec![1.0; 100], 1, 44100).unwrap();
        let result = crossfade(&a, &b, CrossfadeType::EqualPower).unwrap();
        // Equal-power should keep roughly constant energy
        for s in &result.samples {
            assert!(s.is_finite());
            assert!(*s > 0.5);
        }
    }

    #[test]
    fn crossfade_channel_mismatch() {
        let a = AudioBuffer::silence(1, 100, 44100);
        let b = AudioBuffer::silence(2, 100, 44100);
        assert!(crossfade(&a, &b, CrossfadeType::Linear).is_err());
    }

    #[test]
    fn fade_in_starts_silent() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 100], 1, 44100).unwrap();
        fade_in(&mut buf, 50, FadeCurve::Linear);
        assert!(buf.samples[0].abs() < 0.02);
        // Past fade region should be unchanged
        assert!((buf.samples[99] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fade_out_ends_silent() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 100], 1, 44100).unwrap();
        fade_out(&mut buf, 50, FadeCurve::Linear);
        assert!(buf.samples[99].abs() < 0.02);
        // Before fade region should be unchanged
        assert!((buf.samples[0] - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn fade_exponential() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 100], 1, 44100).unwrap();
        fade_in(&mut buf, 50, FadeCurve::Exponential);
        assert!(buf.samples[0].abs() < 0.01);
        // Exponential should be quieter than linear at midpoint
        assert!(buf.samples[25] < 0.5);
    }

    #[cfg(feature = "analysis")]
    #[test]
    fn normalize_to_target_lufs() {
        let mut buf = make_sine(44100);
        let gain = normalize_to_lufs(&mut buf, -14.0).unwrap();
        assert!(gain.is_finite());
    }
}
