//! Sinc resampling — windowed sinc interpolation with configurable quality.

use serde::{Deserialize, Serialize};

use crate::NadaError;
use crate::buffer::AudioBuffer;

/// Resampling quality level, controlling the sinc kernel width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ResampleQuality {
    /// Fast, lower quality (4-point sinc kernel).
    Draft,
    /// Balanced quality (16-point sinc kernel).
    Good,
    /// Highest quality (64-point sinc kernel).
    Best,
}

impl ResampleQuality {
    /// Number of sinc lobes (half-width of the kernel in samples).
    fn kernel_half_width(self) -> usize {
        match self {
            Self::Draft => 2,
            Self::Good => 8,
            Self::Best => 32,
        }
    }
}

/// Resample an audio buffer using windowed sinc interpolation.
///
/// Higher quality levels use wider sinc kernels for better frequency
/// preservation at the cost of more computation.
pub fn resample_sinc(
    buf: &AudioBuffer,
    target_rate: u32,
    quality: ResampleQuality,
) -> Result<AudioBuffer, NadaError> {
    tracing::debug!(
        source_rate = buf.sample_rate,
        target_rate,
        frames = buf.frames,
        channels = buf.channels,
        quality = ?quality,
        "resample_sinc: started"
    );
    if target_rate == 0 {
        return Err(NadaError::InvalidSampleRate(0));
    }
    if target_rate == buf.sample_rate {
        return Ok(buf.clone());
    }
    if buf.frames == 0 {
        return Ok(AudioBuffer::silence(buf.channels, 0, target_rate));
    }

    let ratio = target_rate as f64 / buf.sample_rate as f64;
    let new_frames = (buf.frames as f64 * ratio).ceil() as usize;
    let ch = buf.channels as usize;
    let half_width = quality.kernel_half_width();

    // For downsampling, scale the kernel to avoid aliasing
    let (filter_scale, kernel_scale) = if ratio < 1.0 {
        (ratio, ratio)
    } else {
        (1.0, 1.0)
    };

    let mut out = vec![0.0f32; new_frames * ch];

    // Pre-allocate kernel weight buffer for SIMD path
    #[cfg(feature = "simd")]
    let max_kernel_len = ((half_width as f64 / filter_scale).ceil() as usize) * 2 + 1;
    #[cfg(feature = "simd")]
    let mut kernel_weights = vec![0.0f32; max_kernel_len];
    #[cfg(feature = "simd")]
    let mut kernel_samples = vec![0.0f32; max_kernel_len];

    for frame in 0..new_frames {
        let src_pos = frame as f64 / ratio;
        let src_center = src_pos.floor() as i64;
        let frac = src_pos - src_center as f64;

        let scaled_half = (half_width as f64 / filter_scale).ceil() as i64;

        #[cfg(feature = "simd")]
        {
            // Pre-compute kernel weights for this frame
            let mut kernel_len = 0usize;
            let mut first_valid_idx = 0i64;
            for i in -scaled_half..=scaled_half {
                let src_idx = src_center + i;
                if src_idx < 0 || src_idx >= buf.frames as i64 {
                    continue;
                }
                if kernel_len == 0 {
                    first_valid_idx = src_idx;
                }
                let x = (i as f64 - frac) * kernel_scale;
                kernel_weights[kernel_len] = windowed_sinc(x, scaled_half as f64) as f32;
                kernel_len += 1;
            }

            // For each channel, gather samples and use SIMD dot product
            for c in 0..ch {
                for (k, ks) in kernel_samples.iter_mut().enumerate().take(kernel_len) {
                    let src_idx = (first_valid_idx as usize + k) * ch + c;
                    *ks = buf.samples[src_idx];
                }

                let (sum, weight_sum) = crate::simd::weighted_sum(
                    &kernel_samples[..kernel_len],
                    &kernel_weights[..kernel_len],
                );

                let idx = frame * ch + c;
                if weight_sum.abs() > 1e-6 {
                    out[idx] = sum / weight_sum;
                }
            }
        }

        #[cfg(not(feature = "simd"))]
        {
            for c in 0..ch {
                let mut sum = 0.0f64;
                let mut weight_sum = 0.0f64;

                for i in -scaled_half..=scaled_half {
                    let src_idx = src_center + i;
                    if src_idx < 0 || src_idx >= buf.frames as i64 {
                        continue;
                    }

                    let x = (i as f64 - frac) * kernel_scale;
                    let w = windowed_sinc(x, scaled_half as f64);
                    let sample = buf.samples[src_idx as usize * ch + c] as f64;
                    sum += sample * w;
                    weight_sum += w;
                }

                let idx = frame * ch + c;
                if weight_sum.abs() > 1e-10 {
                    out[idx] = (sum / weight_sum) as f32;
                }
            }
        }
    }

    Ok(AudioBuffer {
        samples: out,
        channels: buf.channels,
        sample_rate: target_rate,
        frames: new_frames,
    })
}

/// Windowed sinc function using a Blackman-Harris window.
fn windowed_sinc(x: f64, half_width: f64) -> f64 {
    if x.abs() < 1e-10 {
        return 1.0;
    }

    let sinc = (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x);

    // Blackman-Harris window
    if x.abs() > half_width {
        return 0.0;
    }
    let t = (x / half_width + 1.0) * 0.5; // Normalize to [0, 1]
    let tau = std::f64::consts::TAU;
    let window = 0.35875 - 0.48829 * (tau * t).cos() + 0.14128 * (2.0 * tau * t).cos()
        - 0.01168 * (3.0 * tau * t).cos();

    sinc * window
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn same_rate_identity() {
        let buf = AudioBuffer::from_interleaved(vec![0.5, -0.5, 0.3, -0.3], 1, 44100).unwrap();
        let out = resample_sinc(&buf, 44100, ResampleQuality::Good).unwrap();
        assert_eq!(out.frames, buf.frames);
        assert_eq!(out.samples, buf.samples);
    }

    #[test]
    fn zero_rate_rejected() {
        let buf = AudioBuffer::silence(1, 100, 44100);
        assert!(resample_sinc(&buf, 0, ResampleQuality::Draft).is_err());
    }

    #[test]
    fn empty_buffer() {
        let buf = AudioBuffer::silence(2, 0, 44100);
        let out = resample_sinc(&buf, 48000, ResampleQuality::Good).unwrap();
        assert_eq!(out.frames, 0);
        assert_eq!(out.sample_rate, 48000);
    }

    #[test]
    fn upsample_increases_frames() {
        let buf = AudioBuffer::silence(1, 1000, 44100);
        let out = resample_sinc(&buf, 96000, ResampleQuality::Draft).unwrap();
        assert!(out.frames > 1000);
        assert_eq!(out.sample_rate, 96000);
    }

    #[test]
    fn downsample_decreases_frames() {
        let buf = AudioBuffer::silence(1, 1000, 96000);
        let out = resample_sinc(&buf, 44100, ResampleQuality::Draft).unwrap();
        assert!(out.frames < 1000);
        assert_eq!(out.sample_rate, 44100);
    }

    #[test]
    fn roundtrip_preserves_signal() {
        // Create a 440Hz sine at 44100
        let sr = 44100u32;
        let frames = 4096;
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let original_rms = buf.rms();

        // Up to 48000 then back to 44100
        let up = resample_sinc(&buf, 48000, ResampleQuality::Good).unwrap();
        let back = resample_sinc(&up, 44100, ResampleQuality::Good).unwrap();

        // RMS should be preserved within 10%
        assert!(
            (back.rms() - original_rms).abs() < original_rms * 0.1,
            "Round-trip RMS: {} vs original: {}",
            back.rms(),
            original_rms
        );
    }

    #[cfg(feature = "analysis")]
    #[test]
    fn sine_frequency_preserved() {
        use crate::analysis::spectrum_dft;
        // 440Hz sine at 44100, resample to 48000, check dominant frequency
        let sr = 44100u32;
        let frames = 4096;
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();

        let resampled = resample_sinc(&buf, 48000, ResampleQuality::Good).unwrap();
        let spec = spectrum_dft(&resampled, 4096).unwrap();
        let dominant = spec.dominant_frequency().unwrap();

        assert!(
            (dominant - 440.0).abs() < spec.freq_resolution() * 2.0,
            "Dominant freq {dominant} should be near 440Hz"
        );
    }

    #[test]
    fn quality_levels_all_work() {
        let buf = AudioBuffer::from_interleaved(
            (0..1024)
                .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
                .collect(),
            1,
            44100,
        )
        .unwrap();

        for quality in [
            ResampleQuality::Draft,
            ResampleQuality::Good,
            ResampleQuality::Best,
        ] {
            let out = resample_sinc(&buf, 48000, quality).unwrap();
            assert!(out.frames > 0);
            assert!(out.samples.iter().all(|s| s.is_finite()));
        }
    }
}
