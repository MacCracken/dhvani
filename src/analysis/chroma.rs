//! Chromagram — pitch class energy distribution (C, C#, D, ..., B).

use crate::analysis::fft::spectrum_fft;
use crate::buffer::AudioBuffer;

/// Pitch class names.
pub const PITCH_CLASSES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// A chromagram: energy distribution across 12 pitch classes.
#[derive(Debug, Clone)]
pub struct Chromagram {
    /// Energy per pitch class (0=C, 1=C#, ..., 11=B). Normalized 0–1.
    chroma: [f32; 12],
}

impl Chromagram {
    /// Energy per pitch class (0=C, 1=C#, ..., 11=B). Normalized 0–1.
    pub fn chroma(&self) -> &[f32; 12] {
        &self.chroma
    }

    /// Index of the dominant pitch class.
    pub fn dominant_class(&self) -> usize {
        self.chroma
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0)
    }

    /// Name of the dominant pitch class.
    pub fn dominant_name(&self) -> &'static str {
        PITCH_CLASSES[self.dominant_class()]
    }
}

/// Compute a chromagram from an audio buffer.
///
/// Maps FFT bin frequencies to the 12 pitch classes using the relationship:
/// `pitch_class = round(12 * log2(freq / C0)) mod 12`
/// where C0 ≈ 16.35 Hz.
///
/// # Errors
///
/// Returns `NadaError::Dsp` if the underlying FFT computation fails.
pub fn chromagram(buf: &AudioBuffer, window_size: usize) -> crate::Result<Chromagram> {
    let spec = spectrum_fft(buf, window_size)?;
    let mut chroma = [0.0f32; 12];

    let c0 = 16.3516f32; // C0 frequency

    for (bin, &mag) in spec.magnitudes().iter().enumerate() {
        let freq = spec.bin_frequency(bin);
        if freq < c0 || freq < 1.0 {
            continue;
        }

        // Map frequency to pitch class
        let semitones = 12.0 * (freq / c0).log2();
        let class = (semitones.round() as i32).rem_euclid(12) as usize;
        chroma[class] += mag * mag; // Energy (squared magnitude)
    }

    // Normalize to 0–1
    let max = chroma.iter().cloned().fold(0.0f32, f32::max);
    if max > 0.0 {
        for c in &mut chroma {
            *c /= max;
        }
    }

    Ok(Chromagram { chroma })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a440_dominant_class() {
        let sr = 44100u32;
        let frames = 8192;
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let c = chromagram(&buf, 4096).unwrap();

        // A440 should map to pitch class A (index 9)
        assert_eq!(
            c.dominant_name(),
            "A",
            "Expected A, got {} with chroma {:?}",
            c.dominant_name(),
            c.chroma()
        );
    }

    #[test]
    fn c_note_dominant() {
        let sr = 44100u32;
        let frames = 8192;
        // C4 = 261.63 Hz
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 261.63 * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let c = chromagram(&buf, 4096).unwrap();

        assert_eq!(c.dominant_name(), "C");
    }

    #[test]
    fn silence_chromagram() {
        let buf = AudioBuffer::silence(1, 4096, 44100);
        let c = chromagram(&buf, 4096).unwrap();
        // All zeros
        assert!(c.chroma().iter().all(|&v| v == 0.0));
    }

    #[test]
    fn pitch_class_names() {
        assert_eq!(PITCH_CLASSES[0], "C");
        assert_eq!(PITCH_CLASSES[9], "A");
        assert_eq!(PITCH_CLASSES[11], "B");
    }
}
