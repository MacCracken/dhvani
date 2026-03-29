//! Onset detection — find transient boundaries using spectral flux.

use crate::analysis::stft::stft;
use crate::buffer::AudioBuffer;

/// Onset detection result.
#[must_use]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct OnsetResult {
    /// Frame positions (in samples) of detected onsets.
    pub(crate) positions: Vec<usize>,
    /// Spectral flux values at each onset.
    pub(crate) strengths: Vec<f32>,
}

impl OnsetResult {
    /// Frame positions (in samples) of detected onsets.
    pub fn positions(&self) -> &[usize] {
        &self.positions
    }

    /// Spectral flux values at each onset.
    pub fn strengths(&self) -> &[f32] {
        &self.strengths
    }

    /// Number of detected onsets.
    pub fn count(&self) -> usize {
        self.positions.len()
    }
}

/// Detect note/transient onsets in an audio buffer using spectral flux.
///
/// - `window_size`: FFT window size (power of 2, e.g., 2048)
/// - `hop_size`: hop between windows (e.g., 512)
/// - `threshold`: onset sensitivity (0.0–1.0, lower = more sensitive, default ~0.3)
///
/// # Errors
///
/// Returns `NadaError::Dsp` if the underlying STFT computation fails.
pub fn detect_onsets(
    buf: &AudioBuffer,
    window_size: usize,
    hop_size: usize,
    threshold: f32,
) -> crate::Result<OnsetResult> {
    tracing::debug!(
        frames = buf.frames,
        window_size,
        hop_size,
        threshold,
        "detect_onsets: started"
    );
    let sg = stft(buf, window_size, hop_size)?;

    if sg.num_frames() < 2 {
        return Ok(OnsetResult {
            positions: Vec::new(),
            strengths: Vec::new(),
        });
    }

    // Compute spectral flux: sum of positive differences between consecutive frames
    let mut flux: Vec<f32> = Vec::with_capacity(sg.num_frames());
    flux.push(0.0); // First frame has no predecessor

    for i in 1..sg.num_frames() {
        let mut diff_sum = 0.0f32;
        for bin in 0..sg.num_bins {
            let diff = sg.frames[i][bin] - sg.frames[i - 1][bin];
            if diff > 0.0 {
                diff_sum += diff;
            }
        }
        flux.push(diff_sum);
    }

    // Normalize flux
    let max_flux = flux.iter().cloned().fold(0.0f32, f32::max);
    if max_flux > 0.0 {
        for f in &mut flux {
            *f /= max_flux;
        }
    }

    // Peak-pick: find flux values above threshold that are local maxima
    let mut positions = Vec::new();
    let mut strengths = Vec::new();

    for i in 1..flux.len().saturating_sub(1) {
        if flux[i] > threshold && flux[i] > flux[i - 1] && flux[i] >= flux[i + 1] {
            // Convert STFT frame index to sample position
            let sample_pos = i * hop_size;
            positions.push(sample_pos);
            strengths.push(flux[i]);
        }
    }

    Ok(OnsetResult {
        positions,
        strengths,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_no_onsets() {
        let buf = AudioBuffer::silence(1, 44100, 44100);
        let result = detect_onsets(&buf, 2048, 512, 0.3).unwrap();
        assert!(result.positions().is_empty());
        assert_eq!(result.count(), 0);
    }

    #[test]
    fn impulse_detected() {
        // Create a buffer with a loud impulse in the middle
        let mut samples = vec![0.0f32; 44100];
        // Impulse at ~0.5 seconds
        samples[22050..22150].fill(0.9);
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let result = detect_onsets(&buf, 2048, 512, 0.2).unwrap();

        assert!(!result.positions.is_empty(), "Should detect the impulse");
        // Onset should be near sample 22050
        let nearest = result
            .positions
            .iter()
            .min_by_key(|&&p| (p as i64 - 22050).unsigned_abs())
            .unwrap();
        assert!(
            (*nearest as i64 - 22050).unsigned_abs() < 2048,
            "Onset at {nearest} should be near 22050"
        );
    }

    #[test]
    fn multiple_onsets() {
        // Two impulses separated by silence
        let mut samples = vec![0.0f32; 44100];
        samples[10000..10100].fill(0.8);
        samples[30000..30100].fill(0.8);
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let result = detect_onsets(&buf, 2048, 512, 0.2).unwrap();

        assert!(
            result.positions.len() >= 2,
            "Should detect at least 2 onsets, got {}",
            result.positions.len()
        );
    }

    #[test]
    fn strengths_match_positions() {
        let mut samples = vec![0.0f32; 44100];
        samples[22050..22150].fill(0.9);
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let result = detect_onsets(&buf, 2048, 512, 0.2).unwrap();

        assert_eq!(result.positions.len(), result.strengths.len());
        for &s in &result.strengths {
            assert!(s > 0.0);
            assert!(s <= 1.0);
        }
    }

    #[test]
    fn high_threshold_fewer_onsets() {
        let mut samples = vec![0.0f32; 44100];
        samples[10000..10050].fill(0.3); // Soft
        samples[30000..30050].fill(0.9); // Loud
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();

        let sensitive = detect_onsets(&buf, 2048, 512, 0.1).unwrap();
        let strict = detect_onsets(&buf, 2048, 512, 0.5).unwrap();

        assert!(sensitive.positions.len() >= strict.positions.len());
    }
}
