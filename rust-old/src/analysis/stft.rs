//! STFT — Short-Time Fourier Transform for spectrogram generation.

use crate::analysis::fft::fft_in_place;
use crate::buffer::AudioBuffer;
use crate::error::NadaError;

/// A spectrogram: time-frequency energy matrix.
#[must_use]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Spectrogram {
    /// Magnitude frames: `frames[time_index][freq_bin]`.
    /// Each inner Vec has `window_size / 2` bins.
    pub frames: Vec<Vec<f32>>,
    /// Number of frequency bins per frame.
    pub num_bins: usize,
    /// Frequency resolution (Hz per bin).
    pub freq_resolution: f32,
    /// Time resolution (seconds per frame).
    pub time_resolution: f32,
    /// Sample rate used.
    pub sample_rate: u32,
}

impl Spectrogram {
    /// Number of time frames.
    pub fn num_frames(&self) -> usize {
        self.frames.len()
    }

    /// Get the frequency of a bin index.
    pub fn bin_frequency(&self, bin: usize) -> f32 {
        bin as f32 * self.freq_resolution
    }

    /// Get the time in seconds of a frame index.
    pub fn frame_time(&self, frame: usize) -> f32 {
        frame as f32 * self.time_resolution
    }
}

/// Compute a spectrogram using STFT.
///
/// - `window_size`: FFT window size (must be power of 2)
/// - `hop_size`: number of samples between consecutive windows
///
/// Uses a Hann window.
///
/// # Errors
///
/// Returns `NadaError::Dsp` if the buffer is empty or `window_size` is not a power of two.
pub fn stft(buf: &AudioBuffer, window_size: usize, hop_size: usize) -> crate::Result<Spectrogram> {
    tracing::debug!(frames = buf.frames, window_size, hop_size, "stft: started");
    if buf.samples.is_empty() {
        return Err(NadaError::Dsp("cannot compute STFT on empty buffer".into()));
    }
    if !window_size.is_power_of_two() || window_size < 4 {
        return Err(NadaError::Dsp(format!(
            "window_size must be a power of two >= 4, got {window_size}"
        )));
    }
    if hop_size == 0 {
        return Err(NadaError::Dsp("hop_size must be > 0".into()));
    }
    let num_bins = window_size / 2;

    // Pre-compute Hann window
    let window: Vec<f64> = (0..window_size)
        .map(|i| {
            0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (window_size - 1) as f64).cos())
        })
        .collect();

    // Use first channel only for analysis
    let samples = &buf.samples;
    let ch = buf.channels as usize;
    let total_mono_frames = buf.frames;

    let mut frames = Vec::new();
    let mut pos = 0usize;
    // Pre-allocate scratch buffers to avoid per-frame heap allocation
    let mut real = vec![0.0f64; window_size];
    let mut imag = vec![0.0f64; window_size];

    while pos + window_size <= total_mono_frames {
        real.fill(0.0);
        imag.fill(0.0);

        for i in 0..window_size {
            real[i] = samples[(pos + i) * ch] as f64 * window[i];
        }

        if !fft_in_place(&mut real, &mut imag) {
            frames.push(vec![0.0; num_bins]);
            pos += hop_size;
            continue;
        }

        let mut magnitudes = Vec::with_capacity(num_bins);
        for k in 0..num_bins {
            let mag = ((real[k] * real[k] + imag[k] * imag[k]).sqrt() / window_size as f64) as f32;
            magnitudes.push(mag);
        }

        frames.push(magnitudes);
        pos += hop_size;
    }

    Ok(Spectrogram {
        frames,
        num_bins,
        freq_resolution: buf.sample_rate as f32 / window_size as f32,
        time_resolution: hop_size as f32 / buf.sample_rate as f32,
        sample_rate: buf.sample_rate,
    })
}

/// STFT processor with cached Hann window.
///
/// Reuse this struct when computing multiple STFTs with the same window size
/// to avoid recomputing the window each time.
#[must_use]
#[derive(Debug, Clone)]
pub struct StftProcessor {
    window: Vec<f64>,
    window_size: usize,
}

impl StftProcessor {
    /// Create a new STFT processor with the given window size.
    ///
    /// The Hann window is pre-computed once and reused.
    pub fn new(window_size: usize) -> crate::Result<Self> {
        if !window_size.is_power_of_two() || window_size < 4 {
            return Err(NadaError::Dsp(format!(
                "window_size must be a power of two >= 4, got {window_size}"
            )));
        }
        let window: Vec<f64> = (0..window_size)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f64::consts::PI * i as f64 / (window_size - 1) as f64).cos())
            })
            .collect();
        Ok(Self {
            window,
            window_size,
        })
    }

    /// Compute a spectrogram using the cached window.
    pub fn compute(&self, buf: &AudioBuffer, hop_size: usize) -> crate::Result<Spectrogram> {
        tracing::debug!(
            frames = buf.frames,
            window_size = self.window_size,
            hop_size,
            "StftProcessor::compute: started"
        );
        if buf.samples.is_empty() {
            return Err(NadaError::Dsp("cannot compute STFT on empty buffer".into()));
        }
        if hop_size == 0 {
            return Err(NadaError::Dsp("hop_size must be > 0".into()));
        }
        let num_bins = self.window_size / 2;
        let samples = &buf.samples;
        let ch = buf.channels as usize;
        let total_mono_frames = buf.frames;

        let mut frames = Vec::new();
        let mut pos = 0usize;
        // Pre-allocate scratch buffers to avoid per-frame heap allocation
        let mut real = vec![0.0f64; self.window_size];
        let mut imag = vec![0.0f64; self.window_size];

        while pos + self.window_size <= total_mono_frames {
            real.fill(0.0);
            imag.fill(0.0);

            for i in 0..self.window_size {
                real[i] = samples[(pos + i) * ch] as f64 * self.window[i];
            }

            if !fft_in_place(&mut real, &mut imag) {
                frames.push(vec![0.0; num_bins]);
                pos += hop_size;
                continue;
            }

            let mut magnitudes = Vec::with_capacity(num_bins);
            for k in 0..num_bins {
                let mag = ((real[k] * real[k] + imag[k] * imag[k]).sqrt() / self.window_size as f64)
                    as f32;
                magnitudes.push(mag);
            }

            frames.push(magnitudes);
            pos += hop_size;
        }

        Ok(Spectrogram {
            frames,
            num_bins,
            freq_resolution: buf.sample_rate as f32 / self.window_size as f32,
            time_resolution: hop_size as f32 / buf.sample_rate as f32,
            sample_rate: buf.sample_rate,
        })
    }

    /// Window size.
    pub fn window_size(&self) -> usize {
        self.window_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_spectrogram() {
        let buf = AudioBuffer::silence(1, 4096, 44100);
        let sg = stft(&buf, 1024, 512).unwrap();
        assert!(sg.num_frames() > 0);
        assert_eq!(sg.num_bins, 512);
        // All magnitudes should be zero
        for frame in &sg.frames {
            assert!(frame.iter().all(|&m| m == 0.0));
        }
    }

    #[test]
    fn sine_spectrogram_has_energy() {
        let sr = 44100u32;
        let samples: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let sg = stft(&buf, 2048, 1024).unwrap();

        assert!(sg.num_frames() > 10);
        // Each frame should have non-zero energy
        for frame in &sg.frames {
            let total: f32 = frame.iter().sum();
            assert!(total > 0.0);
        }
    }

    #[test]
    fn time_resolution() {
        let buf = AudioBuffer::silence(1, 44100, 44100);
        let sg = stft(&buf, 2048, 1024).unwrap();
        assert!((sg.time_resolution - 1024.0 / 44100.0).abs() < 0.001);
    }

    #[test]
    fn freq_resolution() {
        let buf = AudioBuffer::silence(1, 4096, 48000);
        let sg = stft(&buf, 2048, 1024).unwrap();
        assert!((sg.freq_resolution - 48000.0 / 2048.0).abs() < 0.1);
    }
}
