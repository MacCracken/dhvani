//! Audio analysis — FFT, spectrum, loudness (LUFS), peak, RMS, dynamics, chromagram, onset detection.

pub mod chroma;
pub mod dynamics;
pub mod fft;
pub mod loudness;
pub mod onset;
pub mod stft;
pub mod waveform;

pub use chroma::{Chromagram, chromagram};
pub use dynamics::{DynamicsAnalysis, analyze_dynamics};
pub use fft::spectrum_fft;
pub use loudness::{R128Loudness, measure_r128};
pub use onset::{OnsetResult, detect_onsets};
pub use stft::{Spectrogram, StftProcessor, stft as compute_stft};
pub use waveform::{WaveformData, compute_waveform};

use crate::buffer::AudioBuffer;
use crate::error::NadaError;

/// Spectrum analysis result.
#[derive(Debug, Clone)]
pub struct Spectrum {
    /// Magnitude bins (linear scale, 0.0–1.0 normalized).
    magnitudes: Vec<f32>,
    /// Magnitude bins in dB (relative to peak).
    magnitude_db: Vec<f32>,
    /// Frequency resolution (Hz per bin).
    freq_resolution: f32,
    /// Sample rate used for analysis.
    sample_rate: u32,
    /// FFT window size used.
    fft_size: usize,
    /// Frequency of the peak bin (Hz).
    peak_frequency: f32,
    /// Magnitude of the peak bin (dB).
    peak_magnitude_db: f32,
}

impl Spectrum {
    /// Construct a Spectrum from linear magnitudes, computing dB and peak fields.
    pub(crate) fn from_magnitudes(
        magnitudes: Vec<f32>,
        freq_resolution: f32,
        sample_rate: u32,
        fft_size: usize,
    ) -> Self {
        let magnitude_db: Vec<f32> = magnitudes
            .iter()
            .map(|&m| if m > 1e-10 { 20.0 * m.log10() } else { -200.0 })
            .collect();

        let (peak_bin_idx, peak_mag) = magnitudes
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, &m)| (i, m))
            .unwrap_or((0, 0.0));

        let peak_frequency = peak_bin_idx as f32 * freq_resolution;
        let peak_magnitude_db = if peak_mag > 1e-10 {
            20.0 * peak_mag.log10()
        } else {
            -200.0
        };

        Self {
            magnitudes,
            magnitude_db,
            freq_resolution,
            sample_rate,
            fft_size,
            peak_frequency,
            peak_magnitude_db,
        }
    }

    /// Magnitude bins (linear scale, 0.0–1.0 normalized).
    pub fn magnitudes(&self) -> &[f32] {
        &self.magnitudes
    }
    /// Magnitude bins in dB (relative to peak).
    pub fn magnitude_db(&self) -> &[f32] {
        &self.magnitude_db
    }
    /// Frequency resolution (Hz per bin).
    pub fn freq_resolution(&self) -> f32 {
        self.freq_resolution
    }
    /// Sample rate used for analysis.
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
    /// FFT window size used.
    pub fn fft_size(&self) -> usize {
        self.fft_size
    }
    /// Frequency of the peak bin (Hz).
    pub fn peak_frequency(&self) -> f32 {
        self.peak_frequency
    }
    /// Magnitude of the peak bin (dB).
    pub fn peak_magnitude_db(&self) -> f32 {
        self.peak_magnitude_db
    }

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

    /// Spectral centroid — weighted mean of frequencies by magnitude.
    ///
    /// A brightness indicator: higher centroid = brighter sound.
    /// Returns 0.0 for silence.
    pub fn spectral_centroid(&self) -> f32 {
        let total_mag: f32 = self.magnitudes.iter().sum();
        if total_mag <= 0.0 {
            return 0.0;
        }
        self.magnitudes
            .iter()
            .enumerate()
            .map(|(i, &m)| i as f32 * self.freq_resolution * m)
            .sum::<f32>()
            / total_mag
    }

    /// Spectral rolloff — frequency below which a given fraction of spectral energy sits.
    ///
    /// Default threshold is 0.95 (95% of energy). A timbral shape descriptor.
    /// Returns 0.0 for silence.
    pub fn spectral_rolloff(&self, threshold: f32) -> f32 {
        let total_energy: f32 = self.magnitudes.iter().map(|m| m * m).sum();
        if total_energy <= 0.0 {
            return 0.0;
        }
        let target = total_energy * threshold.clamp(0.0, 1.0);
        let mut cumulative = 0.0f32;
        for (i, &m) in self.magnitudes.iter().enumerate() {
            cumulative += m * m;
            if cumulative >= target {
                return i as f32 * self.freq_resolution;
            }
        }
        self.magnitudes.len() as f32 * self.freq_resolution
    }
}

/// Compute a simple DFT magnitude spectrum (not FFT — for small windows).
///
/// For production use, replace with `rustfft` for O(n log n) performance.
/// This O(n^2) implementation is correct for testing and small buffers.
///
/// # Errors
///
/// Returns `NadaError::Dsp` if the buffer has no samples or zero channels.
pub fn spectrum_dft(buf: &AudioBuffer, window_size: usize) -> crate::Result<Spectrum> {
    if buf.samples.is_empty() {
        return Err(NadaError::Dsp("cannot compute DFT on empty buffer".into()));
    }
    if buf.channels == 0 {
        return Err(NadaError::Dsp(
            "cannot compute DFT with zero channels".into(),
        ));
    }

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

    let freq_resolution = buf.sample_rate as f32 / n as f32;
    Ok(Spectrum::from_magnitudes(
        magnitudes,
        freq_resolution,
        buf.sample_rate,
        n,
    ))
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

/// Suggest a normalization gain to reach a target RMS level.
///
/// Returns a linear gain factor clamped to `0.1..=10.0` to prevent
/// extreme amplification or attenuation. Returns `1.0` for silence.
///
/// This is the per-buffer gain computation that media players need for
/// volume normalization. Pair with [`GainSmoother`](crate::dsp::GainSmoother)
/// to prevent pumping.
///
/// # Arguments
///
/// * `buf` — audio buffer to analyze
/// * `target_rms` — desired RMS level in linear scale (e.g., 0.125 for ~-18 dBFS)
///
/// # Example
///
/// ```rust
/// use dhvani::buffer::AudioBuffer;
/// use dhvani::analysis::suggest_gain;
///
/// let buf = AudioBuffer::from_interleaved(vec![0.5; 1024], 1, 44100).unwrap();
/// let gain = suggest_gain(&buf, 0.125);
/// assert!(gain < 1.0); // loud signal → attenuate
/// ```
pub fn suggest_gain(buf: &AudioBuffer, target_rms: f32) -> f32 {
    let rms = buf.rms();
    if rms < 1e-6 {
        return 1.0;
    }
    (target_rms / rms).clamp(0.1, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_of_silence() {
        let buf = AudioBuffer::silence(1, 256, 44100);
        let spec = spectrum_dft(&buf, 256).unwrap();
        assert_eq!(spec.bin_count(), 128);
        assert!(spec.magnitudes().iter().all(|&m| m == 0.0));
    }

    #[test]
    fn spectrum_bin_frequency() {
        let buf = AudioBuffer::silence(1, 1024, 44100);
        let spec = spectrum_dft(&buf, 1024).unwrap();
        assert!((spec.freq_resolution() - 44100.0 / 1024.0).abs() < 0.1);
        assert!((spec.bin_frequency(0)).abs() < 0.1);
        assert!((spec.bin_frequency(1) - spec.freq_resolution()).abs() < 0.1);
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
        let spec = spectrum_dft(&buf, frames).unwrap();

        let dominant = spec.dominant_frequency().unwrap();
        // Should be near 440Hz (within one bin width)
        assert!(
            (dominant - 440.0).abs() < spec.freq_resolution() * 2.0,
            "dominant={dominant}, expected ~440"
        );
    }

    #[test]
    fn suggest_gain_loud_signal() {
        // Full-scale signal → should attenuate toward target
        let buf = AudioBuffer::from_interleaved(vec![0.8; 1024], 1, 44100).unwrap();
        let gain = suggest_gain(&buf, 0.125);
        assert!(gain < 1.0, "loud signal should get gain < 1.0, got {gain}");
    }

    #[test]
    fn suggest_gain_quiet_signal() {
        // Very quiet signal → should amplify
        let buf = AudioBuffer::from_interleaved(vec![0.01; 1024], 1, 44100).unwrap();
        let gain = suggest_gain(&buf, 0.125);
        assert!(gain > 1.0, "quiet signal should get gain > 1.0, got {gain}");
        assert!(gain <= 10.0, "gain should be clamped to 10.0");
    }

    #[test]
    fn suggest_gain_silence() {
        let buf = AudioBuffer::silence(1, 1024, 44100);
        let gain = suggest_gain(&buf, 0.125);
        assert_eq!(gain, 1.0, "silence should return 1.0");
    }

    #[test]
    fn suggest_gain_clamps_extreme() {
        // Extremely quiet → gain would be huge, should clamp to 10.0
        let buf = AudioBuffer::from_interleaved(vec![0.0001; 1024], 1, 44100).unwrap();
        let gain = suggest_gain(&buf, 0.125);
        assert_eq!(gain, 10.0);
    }

    #[test]
    fn spectral_centroid_of_silence() {
        let buf = AudioBuffer::silence(1, 256, 44100);
        let spec = spectrum_dft(&buf, 256).unwrap();
        assert_eq!(spec.spectral_centroid(), 0.0);
    }

    #[test]
    fn spectral_centroid_of_sine() {
        let sr = 44100u32;
        let freq = 2000.0f32;
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let spec = spectrum_fft(&buf, 4096).unwrap();
        let centroid = spec.spectral_centroid();
        // Centroid should be near the sine frequency
        assert!(
            (centroid - freq).abs() < 200.0,
            "centroid at {centroid} Hz, expected near {freq} Hz"
        );
    }

    #[test]
    fn spectral_rolloff_of_silence() {
        let buf = AudioBuffer::silence(1, 256, 44100);
        let spec = spectrum_dft(&buf, 256).unwrap();
        assert_eq!(spec.spectral_rolloff(0.95), 0.0);
    }

    #[test]
    fn spectral_rolloff_below_nyquist() {
        let sr = 44100u32;
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let spec = spectrum_fft(&buf, 4096).unwrap();
        let rolloff = spec.spectral_rolloff(0.95);
        // For a pure sine, 95% rolloff should be near the fundamental
        assert!(rolloff > 0.0);
        assert!(rolloff < sr as f32 / 2.0, "rolloff should be below Nyquist");
    }
}
