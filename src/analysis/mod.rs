//! Audio analysis — FFT, spectrum, loudness (LUFS), peak, RMS.

use crate::buffer::AudioBuffer;

/// Spectrum analysis result.
#[derive(Debug, Clone)]
pub struct Spectrum {
    /// Magnitude bins (linear scale, 0.0–1.0 normalized).
    pub magnitudes: Vec<f32>,
    /// Frequency resolution (Hz per bin).
    pub freq_resolution: f32,
    /// Sample rate used for analysis.
    pub sample_rate: u32,
}

impl Spectrum {
    /// Number of frequency bins.
    pub fn bin_count(&self) -> usize {
        self.magnitudes.len()
    }

    /// Frequency of a given bin index.
    pub fn bin_frequency(&self, bin: usize) -> f32 {
        bin as f32 * self.freq_resolution
    }

    /// Find the bin with the highest magnitude.
    pub fn peak_bin(&self) -> Option<(usize, f32)> {
        self.magnitudes
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, &m)| (i, m))
    }

    /// Dominant frequency (frequency of peak bin).
    pub fn dominant_frequency(&self) -> Option<f32> {
        self.peak_bin().map(|(i, _)| self.bin_frequency(i))
    }
}

/// Compute a simple DFT magnitude spectrum (not FFT — for small windows).
///
/// For production use, replace with `rustfft` for O(n log n) performance.
/// This O(n^2) implementation is correct for testing and small buffers.
pub fn spectrum_dft(buf: &AudioBuffer, window_size: usize) -> Spectrum {
    let samples = if buf.samples.len() >= window_size {
        &buf.samples[..window_size]
    } else {
        &buf.samples
    };

    let n = samples.len();
    let num_bins = n / 2;
    let mut magnitudes = vec![0.0f32; num_bins];

    for (k, mag) in magnitudes.iter_mut().enumerate() {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (i, &s) in samples.iter().enumerate() {
            let angle = -2.0 * std::f64::consts::PI * k as f64 * i as f64 / n as f64;
            re += s as f64 * angle.cos();
            im += s as f64 * angle.sin();
        }
        *mag = ((re * re + im * im).sqrt() / n as f64) as f32;
    }

    // Normalize to 0-1
    let max = magnitudes.iter().cloned().fold(0.0f32, f32::max);
    if max > 0.0 {
        for m in &mut magnitudes {
            *m /= max;
        }
    }

    Spectrum {
        magnitudes,
        freq_resolution: buf.sample_rate as f32 / n as f32,
        sample_rate: buf.sample_rate,
    }
}

/// Compute integrated loudness in LUFS (EBU R128 simplified).
///
/// This is a simplified approximation — full R128 requires K-weighting filter
/// and gated measurement. Suitable for relative comparisons.
pub fn loudness_lufs(buf: &AudioBuffer) -> f32 {
    if buf.samples.is_empty() {
        return f32::NEG_INFINITY;
    }
    let mean_sq: f64 = buf
        .samples
        .iter()
        .map(|s| (*s as f64) * (*s as f64))
        .sum::<f64>()
        / buf.samples.len() as f64;

    if mean_sq <= 0.0 {
        return f32::NEG_INFINITY;
    }

    // LUFS ≈ -0.691 + 10 * log10(mean_square)
    -0.691 + 10.0 * (mean_sq as f32).log10()
}

/// Detect if a buffer is effectively silent (below threshold).
pub fn is_silent(buf: &AudioBuffer, threshold_db: f32) -> bool {
    let peak_db = crate::dsp::amplitude_to_db(buf.peak());
    peak_db < threshold_db
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_of_silence() {
        let buf = AudioBuffer::silence(1, 256, 44100);
        let spec = spectrum_dft(&buf, 256);
        assert_eq!(spec.bin_count(), 128);
        assert!(spec.magnitudes.iter().all(|&m| m == 0.0));
    }

    #[test]
    fn spectrum_bin_frequency() {
        let buf = AudioBuffer::silence(1, 1024, 44100);
        let spec = spectrum_dft(&buf, 1024);
        assert!((spec.freq_resolution - 44100.0 / 1024.0).abs() < 0.1);
        assert!((spec.bin_frequency(0)).abs() < 0.1);
        assert!((spec.bin_frequency(1) - spec.freq_resolution).abs() < 0.1);
    }

    #[test]
    fn loudness_of_silence() {
        let buf = AudioBuffer::silence(1, 1024, 44100);
        let lufs = loudness_lufs(&buf);
        assert!(lufs.is_infinite() && lufs < 0.0);
    }

    #[test]
    fn is_silent_detects_silence() {
        let buf = AudioBuffer::silence(1, 1024, 44100);
        assert!(is_silent(&buf, -60.0));
    }

    #[test]
    fn is_silent_detects_signal() {
        let buf = AudioBuffer::from_interleaved(vec![0.5; 1024], 1, 44100).unwrap();
        assert!(!is_silent(&buf, -60.0));
    }

    #[test]
    fn spectrum_dominant_frequency() {
        // Generate a sine wave at ~440Hz
        let sample_rate = 44100;
        let frames = 4096;
        let freq = 440.0f32;
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sample_rate).unwrap();
        let spec = spectrum_dft(&buf, frames);

        let dominant = spec.dominant_frequency().unwrap();
        // Should be near 440Hz (within one bin width)
        assert!(
            (dominant - 440.0).abs() < spec.freq_resolution * 2.0,
            "dominant={dominant}, expected ~440"
        );
    }
}
