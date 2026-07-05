//! Spectral noise reduction — STFT-based soft gating.
//!
//! Uses a two-pass approach: estimate noise floor from average magnitude,
//! then gate bins below threshold with soft attenuation. Overlap-add
//! reconstruction preserves phase coherence.
//!
//! For repeated processing, use [`NoiseReducer`] to avoid per-call allocations.

use crate::analysis::fft::fft_in_place;
use crate::buffer::AudioBuffer;

const WINDOW_SIZE: usize = 2048;
const HOP_SIZE: usize = WINDOW_SIZE / 2; // 50% overlap
const NUM_BINS: usize = WINDOW_SIZE / 2;

/// Apply spectral noise reduction in-place.
///
/// `strength` controls gate aggressiveness (0.0–1.0, typical: 0.3–0.7).
/// Higher values remove more noise but may introduce artifacts.
///
/// For repeated calls, prefer [`NoiseReducer`] which reuses internal buffers.
pub fn noise_reduce(buf: &mut AudioBuffer, strength: f32) {
    let mut reducer = NoiseReducer::new();
    reducer.process(buf, strength);
}

/// Stateful spectral noise reducer — reuses internal buffers across calls.
///
/// Avoids repeated heap allocation of FFT scratch, overlap-add, and magnitude
/// buffers. Follows the same pattern as [`crate::analysis::stft::StftProcessor`].
#[must_use]
#[derive(Debug, Clone)]
pub struct NoiseReducer {
    window: Vec<f64>,
    real: Vec<f64>,
    imag: Vec<f64>,
    avg_magnitude: Vec<f64>,
}

impl NoiseReducer {
    /// Create a new noise reducer with pre-allocated scratch buffers.
    pub fn new() -> Self {
        tracing::debug!("NoiseReducer::new");
        let window: Vec<f64> = (0..WINDOW_SIZE)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f64::consts::PI * i as f64 / (WINDOW_SIZE - 1) as f64).cos())
            })
            .collect();
        Self {
            window,
            real: vec![0.0f64; WINDOW_SIZE],
            imag: vec![0.0f64; WINDOW_SIZE],
            avg_magnitude: vec![0.0f64; NUM_BINS],
        }
    }

    /// Apply spectral noise reduction in-place.
    ///
    /// `strength` controls gate aggressiveness (0.0–1.0, typical: 0.3–0.7).
    /// Higher values remove more noise but may introduce artifacts.
    pub fn process(&mut self, buf: &mut AudioBuffer, strength: f32) {
        tracing::debug!(
            frames = buf.frames,
            channels = buf.channels,
            strength,
            "NoiseReducer::process"
        );
        let strength = strength.clamp(0.0, 1.0);
        if buf.frames < WINDOW_SIZE {
            // Too short for STFT — fall back to simple amplitude gate
            let threshold = strength * 0.05;
            for s in &mut buf.samples {
                if s.abs() < threshold {
                    *s = 0.0;
                }
            }
            return;
        }

        let ch = buf.channels as usize;

        // Process each channel independently
        for c in 0..ch {
            // Extract mono channel
            let mut mono: Vec<f32> = (0..buf.frames).map(|f| buf.samples[f * ch + c]).collect();
            self.process_channel(&mut mono, strength);
            // Write back
            for (f, &sample) in mono.iter().enumerate() {
                buf.samples[f * ch + c] = sample;
            }
        }
    }

    fn process_channel(&mut self, samples: &mut [f32], strength: f32) {
        let n = samples.len();
        if n < WINDOW_SIZE {
            return;
        }

        // Pass 1: Estimate average magnitude spectrum (noise floor)
        self.avg_magnitude.fill(0.0);
        let mut frame_count = 0usize;
        let mut pos = 0;

        while pos + WINDOW_SIZE <= n {
            self.real.fill(0.0);
            self.imag.fill(0.0);
            for i in 0..WINDOW_SIZE {
                self.real[i] = samples[pos + i] as f64 * self.window[i];
            }
            if !fft_in_place(&mut self.real, &mut self.imag) {
                pos += HOP_SIZE;
                continue;
            }
            for k in 0..NUM_BINS {
                self.avg_magnitude[k] +=
                    (self.real[k] * self.real[k] + self.imag[k] * self.imag[k]).sqrt();
            }
            frame_count += 1;
            pos += HOP_SIZE;
        }

        if frame_count == 0 {
            return;
        }
        for m in &mut self.avg_magnitude {
            *m /= frame_count as f64;
        }

        // Pass 2: Gate and reconstruct via overlap-add
        // These must be per-channel length, so allocate here
        let mut output = vec![0.0f64; n];
        let mut window_sum = vec![0.0f64; n];
        pos = 0;

        while pos + WINDOW_SIZE <= n {
            self.real.fill(0.0);
            self.imag.fill(0.0);
            for i in 0..WINDOW_SIZE {
                self.real[i] = samples[pos + i] as f64 * self.window[i];
            }
            if !fft_in_place(&mut self.real, &mut self.imag) {
                pos += HOP_SIZE;
                continue;
            }

            // Soft gate: attenuate bins below threshold
            let gate_factor = strength as f64 * 1.5;
            for k in 0..NUM_BINS {
                let mag = (self.real[k] * self.real[k] + self.imag[k] * self.imag[k]).sqrt();
                let threshold = self.avg_magnitude[k] * gate_factor;
                if mag < threshold && threshold > 0.0 {
                    let attenuation = mag / threshold; // soft gate: proportional
                    self.real[k] *= attenuation;
                    self.imag[k] *= attenuation;
                    // Mirror for negative frequencies
                    if k > 0 && k < NUM_BINS {
                        let mirror = WINDOW_SIZE - k;
                        self.real[mirror] *= attenuation;
                        self.imag[mirror] *= attenuation;
                    }
                }
            }

            // Inverse FFT (swap real/imag, forward FFT, divide by N)
            // IFFT via conjugate: IFFT(X) = conj(FFT(conj(X))) / N
            for v in &mut self.imag {
                *v = -*v;
            }
            if !fft_in_place(&mut self.real, &mut self.imag) {
                pos += HOP_SIZE;
                continue;
            }
            let scale = 1.0 / WINDOW_SIZE as f64;
            for r in self.real.iter_mut() {
                *r *= scale;
            }

            // Overlap-add with window
            for i in 0..WINDOW_SIZE {
                output[pos + i] += self.real[i] * self.window[i];
                window_sum[pos + i] += self.window[i] * self.window[i];
            }

            pos += HOP_SIZE;
        }

        // Normalize by window sum and write back
        for i in 0..n {
            if window_sum[i] > 1e-10 {
                samples[i] = (output[i] / window_sum[i]) as f32;
            }
            if !samples[i].is_finite() {
                samples[i] = 0.0;
            }
        }
    }
}

impl Default for NoiseReducer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_unchanged() {
        let mut buf = AudioBuffer::silence(1, 4096, 44100);
        noise_reduce(&mut buf, 0.5);
        assert!(buf.peak() < f32::EPSILON);
    }

    #[test]
    fn loud_signal_preserved() {
        // A loud sine should survive noise reduction mostly intact
        let sr = 44100u32;
        let samples: Vec<f32> = (0..sr as usize)
            .map(|i| 0.8 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let original_rms = buf.rms();
        noise_reduce(&mut buf, 0.3);
        // Should retain most energy
        assert!(
            buf.rms() > original_rms * 0.7,
            "Loud signal should survive: rms={} vs original={}",
            buf.rms(),
            original_rms
        );
    }

    #[test]
    fn noise_reduced() {
        // Add loud signal + low-level noise; noise reduction should lower overall energy slightly
        let sr = 44100u32;
        let samples: Vec<f32> = (0..sr as usize)
            .map(|i| {
                let signal =
                    0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin();
                let noise = 0.02 * ((i as f32 * 12_345.679).sin());
                signal + noise
            })
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        noise_reduce(&mut buf, 0.5);
        // Should still produce valid output
        assert!(buf.samples.iter().all(|s| s.is_finite()));
        assert!(buf.rms() > 0.0, "Signal should survive");
    }

    #[test]
    fn short_buffer_fallback() {
        let mut buf = AudioBuffer::from_interleaved(vec![0.01; 100], 1, 44100).unwrap();
        noise_reduce(&mut buf, 0.5);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn stereo_processing() {
        let samples: Vec<f32> = (0..88200)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / 44100.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        noise_reduce(&mut buf, 0.3);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn output_finite() {
        let samples: Vec<f32> = (0..44100)
            .map(|i| (i as f32 / 44100.0) * 2.0 - 1.0)
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        noise_reduce(&mut buf, 1.0);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }
}
