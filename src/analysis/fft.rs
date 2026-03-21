//! Radix-2 Cooley-Tukey FFT — O(n log n) replacement for the O(n^2) DFT.

use crate::analysis::Spectrum;
use crate::buffer::AudioBuffer;

/// Compute a magnitude spectrum using radix-2 FFT with Hann window.
///
/// `window_size` must be a power of 2. If not, it is rounded down to the nearest power of 2.
/// This is the production replacement for [`spectrum_dft`](super::spectrum_dft).
pub fn spectrum_fft(buf: &AudioBuffer, window_size: usize) -> Spectrum {
    let window_size = window_size.next_power_of_two().min(window_size);
    let window_size = if window_size.is_power_of_two() {
        window_size
    } else {
        window_size.next_power_of_two() >> 1
    };
    let window_size = window_size.max(2);

    let n = window_size.min(buf.samples.len());

    // Apply Hann window and prepare complex buffer
    let mut real = vec![0.0f64; window_size];
    let mut imag = vec![0.0f64; window_size];

    for (i, r) in real.iter_mut().enumerate().take(n) {
        let window =
            0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (n - 1).max(1) as f64).cos());
        *r = buf.samples[i] as f64 * window;
    }

    // In-place FFT
    fft_in_place(&mut real, &mut imag);

    // Compute magnitude spectrum (first half only — symmetric for real input)
    let num_bins = window_size / 2;
    let mut magnitudes = vec![0.0f32; num_bins];

    for (k, mag) in magnitudes.iter_mut().enumerate() {
        *mag = ((real[k] * real[k] + imag[k] * imag[k]).sqrt() / window_size as f64) as f32;
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
        freq_resolution: buf.sample_rate as f32 / window_size as f32,
        sample_rate: buf.sample_rate,
    }
}

/// In-place radix-2 Cooley-Tukey FFT.
pub(crate) fn fft_in_place(real: &mut [f64], imag: &mut [f64]) {
    let n = real.len();
    assert!(n.is_power_of_two());
    assert_eq!(real.len(), imag.len());

    // Bit-reversal permutation
    let mut j = 0usize;
    for i in 0..n {
        if i < j {
            real.swap(i, j);
            imag.swap(i, j);
        }
        let mut m = n >> 1;
        while m >= 1 && j >= m {
            j -= m;
            m >>= 1;
        }
        j += m;
    }

    // Butterfly stages
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let angle_step = -2.0 * std::f64::consts::PI / len as f64;

        for start in (0..n).step_by(len) {
            let mut angle = 0.0f64;
            for k in 0..half {
                let cos_a = angle.cos();
                let sin_a = angle.sin();

                let i = start + k;
                let j = start + k + half;

                let tr = real[j] * cos_a - imag[j] * sin_a;
                let ti = real[j] * sin_a + imag[j] * cos_a;

                real[j] = real[i] - tr;
                imag[j] = imag[i] - ti;
                real[i] += tr;
                imag[i] += ti;

                angle += angle_step;
            }
        }
        len <<= 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_spectrum() {
        let buf = AudioBuffer::silence(1, 1024, 44100);
        let spec = spectrum_fft(&buf, 1024);
        assert_eq!(spec.bin_count(), 512);
        assert!(spec.magnitudes.iter().all(|&m| m == 0.0));
    }

    #[test]
    fn sine_dominant_frequency() {
        let sr = 44100u32;
        let frames = 4096;
        let freq = 440.0f32;
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let spec = spectrum_fft(&buf, 4096);

        let dominant = spec.dominant_frequency().unwrap();
        assert!(
            (dominant - 440.0).abs() < spec.freq_resolution * 2.0,
            "dominant={dominant}, expected ~440"
        );
    }

    #[test]
    fn fft_matches_dft_roughly() {
        // Both should find the same dominant frequency
        let sr = 44100u32;
        let frames = 1024;
        let freq = 1000.0f32;
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * freq * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();

        let spec_dft = crate::analysis::spectrum_dft(&buf, 1024);
        let spec_fft = spectrum_fft(&buf, 1024);

        let dom_dft = spec_dft.dominant_frequency().unwrap();
        let dom_fft = spec_fft.dominant_frequency().unwrap();
        assert!(
            (dom_dft - dom_fft).abs() < spec_fft.freq_resolution * 2.0,
            "DFT={dom_dft}, FFT={dom_fft}"
        );
    }

    #[test]
    fn non_power_of_two_handled() {
        let buf = AudioBuffer::from_interleaved(vec![0.5; 1000], 1, 44100).unwrap();
        let spec = spectrum_fft(&buf, 1000);
        // Should round down to 512 (nearest power of 2 <= 1000)
        assert!(spec.bin_count() <= 512);
    }

    #[test]
    fn frequency_resolution_correct() {
        let buf = AudioBuffer::silence(1, 2048, 48000);
        let spec = spectrum_fft(&buf, 2048);
        assert!((spec.freq_resolution - 48000.0 / 2048.0).abs() < 0.1);
    }
}
