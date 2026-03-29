//! Key detection — Krumhansl-Schmuckler algorithm on chromagram data.
//!
//! Classifies audio into one of 24 keys (12 major + 12 minor) by
//! correlating the chromagram with key profiles.

use super::chroma::{Chromagram, chromagram};
use crate::buffer::AudioBuffer;

/// Pitch class names for display.
const PITCH_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Key detection result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct KeyResult {
    /// Detected key name (e.g., "C major", "A minor").
    pub key: String,
    /// Root pitch class index (0 = C, 1 = C#, ..., 11 = B).
    pub root: usize,
    /// Whether the detected key is major (true) or minor (false).
    pub is_major: bool,
    /// Correlation strength (0.0–1.0). Higher = more confident.
    pub confidence: f32,
    /// All 24 key correlations: [C major, C# major, ..., B major, C minor, ..., B minor].
    pub correlations: [f32; 24],
}

/// Krumhansl-Kessler major key profile.
/// Normalized correlation weights for each pitch class relative to the tonic.
const MAJOR_PROFILE: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];

/// Krumhansl-Kessler minor key profile.
const MINOR_PROFILE: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

/// Detect the musical key of an audio buffer.
///
/// Uses the Krumhansl-Schmuckler algorithm: computes a chromagram,
/// then correlates it with each of the 24 major/minor key profiles.
///
/// # Arguments
///
/// * `buf` — Audio buffer to analyze
/// * `window_size` — FFT window size for chromagram (e.g., 8192 for good frequency resolution)
///
/// # Errors
///
/// Returns an error if the chromagram computation fails.
pub fn detect_key(buf: &AudioBuffer, window_size: usize) -> crate::Result<KeyResult> {
    let chroma = chromagram(buf, window_size)?;
    Ok(detect_key_from_chroma(&chroma))
}

/// Detect key from a pre-computed chromagram.
///
/// Useful when you already have a chromagram and want to avoid recomputing it.
#[must_use]
pub fn detect_key_from_chroma(chroma: &Chromagram) -> KeyResult {
    let chroma_vals = chroma.chroma();
    let mut correlations = [0.0f32; 24];
    let mut best_idx = 0;
    let mut best_corr = f32::NEG_INFINITY;

    // Test all 24 keys (12 major + 12 minor)
    for root in 0..12 {
        // Major key rooted at `root`
        let major_corr = pearson_correlation(chroma_vals, &MAJOR_PROFILE, root);
        correlations[root] = major_corr;
        if major_corr > best_corr {
            best_corr = major_corr;
            best_idx = root;
        }

        // Minor key rooted at `root`
        let minor_corr = pearson_correlation(chroma_vals, &MINOR_PROFILE, root);
        correlations[12 + root] = minor_corr;
        if minor_corr > best_corr {
            best_corr = minor_corr;
            best_idx = 12 + root;
        }
    }

    let is_major = best_idx < 12;
    let root = best_idx % 12;
    let mode = if is_major { "major" } else { "minor" };
    let key = format!("{} {mode}", PITCH_NAMES[root]);

    // Confidence: how much better is the best match vs second-best?
    let mut sorted: Vec<f32> = correlations.to_vec();
    sorted.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    let confidence = if sorted.len() >= 2 && sorted[0] > 0.0 {
        ((sorted[0] - sorted[1]) / sorted[0].abs().max(0.001)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    KeyResult {
        key,
        root,
        is_major,
        confidence,
        correlations,
    }
}

/// Pearson correlation between chroma vector and a key profile rotated to `root`.
fn pearson_correlation(chroma: &[f32; 12], profile: &[f32; 12], root: usize) -> f32 {
    let n = 12.0f32;

    // Compute means
    let chroma_mean: f32 = chroma.iter().sum::<f32>() / n;
    let profile_mean: f32 = profile.iter().sum::<f32>() / n;

    let mut cov = 0.0f32;
    let mut var_chroma = 0.0f32;
    let mut var_profile = 0.0f32;

    for i in 0..12 {
        let c = chroma[(i + root) % 12] - chroma_mean;
        let p = profile[i] - profile_mean;
        cov += c * p;
        var_chroma += c * c;
        var_profile += p * p;
    }

    let denom = (var_chroma * var_profile).sqrt();
    if denom > 1e-10 { cov / denom } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_major_scale() {
        // Simulate a C major chromagram: strong C, E, G
        let mut chroma_vals = [0.1f32; 12];
        chroma_vals[0] = 1.0; // C
        chroma_vals[4] = 0.8; // E
        chroma_vals[7] = 0.7; // G
        chroma_vals[5] = 0.4; // F
        chroma_vals[2] = 0.3; // D

        let chroma = Chromagram::from_array(chroma_vals);
        let result = detect_key_from_chroma(&chroma);

        assert_eq!(
            result.root, 0,
            "expected root C, got {}",
            PITCH_NAMES[result.root]
        );
        assert!(result.is_major, "expected major, got minor");
        assert!(result.confidence > 0.0);
    }

    #[test]
    fn a_minor_scale() {
        // Simulate an A minor chromagram: strong A, C, E
        let mut chroma_vals = [0.1f32; 12];
        chroma_vals[9] = 1.0; // A
        chroma_vals[0] = 0.7; // C
        chroma_vals[4] = 0.6; // E
        chroma_vals[7] = 0.3; // G
        chroma_vals[2] = 0.25; // D

        let chroma = Chromagram::from_array(chroma_vals);
        let result = detect_key_from_chroma(&chroma);

        assert_eq!(
            result.root, 9,
            "expected root A, got {}",
            PITCH_NAMES[result.root]
        );
        assert!(!result.is_major, "expected minor, got major");
    }

    #[test]
    fn all_correlations_present() {
        let chroma = Chromagram::from_array([1.0; 12]);
        let result = detect_key_from_chroma(&chroma);
        assert_eq!(result.correlations.len(), 24);
    }

    #[test]
    fn detect_key_from_audio() {
        // Generate A440 sine — should detect A as dominant
        let samples: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.8)
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let result = detect_key(&buf, 8192).unwrap();

        // A pure sine at A440 should have A as the root (index 9)
        assert_eq!(
            result.root, 9,
            "expected A (9), got {} ({})",
            result.root, PITCH_NAMES[result.root]
        );
    }

    #[test]
    fn key_result_display() {
        let chroma =
            Chromagram::from_array([1.0, 0.0, 0.0, 0.0, 0.8, 0.0, 0.0, 0.7, 0.0, 0.0, 0.0, 0.0]);
        let result = detect_key_from_chroma(&chroma);
        assert!(result.key.contains("major") || result.key.contains("minor"));
    }
}
