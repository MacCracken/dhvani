//! EBU R128 loudness measurement — K-weighting + gated LUFS.
//!
//! Implements the full EBU R128 algorithm:
//! 1. K-weighting filter (high shelf at 1681 Hz + high pass at 38 Hz)
//! 2. Mean square per 400ms block
//! 3. Absolute gate at -70 LUFS
//! 4. Relative gate at -10 LU below ungated loudness

use crate::buffer::AudioBuffer;
use crate::dsp::biquad::{BiquadCoeffs, FilterType};
use crate::error::NadaError;

/// Full EBU R128 loudness measurement result.
#[derive(Debug, Clone)]
pub struct R128Loudness {
    /// Integrated loudness (LUFS) — the main number.
    pub integrated_lufs: f32,
    /// Loudness range (LRA) in LU.
    pub range_lu: f32,
    /// Short-term loudness (last 3s window) in LUFS.
    pub short_term_lufs: f32,
    /// Momentary loudness (last 400ms window) in LUFS.
    pub momentary_lufs: f32,
}

/// Measure EBU R128 integrated loudness.
///
/// # Errors
///
/// Returns `NadaError::Dsp` if the buffer is empty or has zero channels.
pub fn measure_r128(buf: &AudioBuffer) -> crate::Result<R128Loudness> {
    if buf.samples.is_empty() {
        return Err(NadaError::Dsp("cannot measure R128 on empty buffer".into()));
    }
    if buf.channels == 0 {
        return Err(NadaError::Dsp(
            "cannot measure R128 with zero channels".into(),
        ));
    }

    let sr = buf.sample_rate;
    let ch = buf.channels as usize;

    // Step 1: Apply K-weighting filter
    let filtered = apply_k_weighting(buf);

    // Step 2: Compute mean square per 400ms block (with 75% overlap = 100ms hop)
    let block_samples = (sr as f64 * 0.4) as usize; // 400ms
    let hop_samples = (sr as f64 * 0.1) as usize; // 100ms hop
    let block_samples = block_samples.max(1);
    let hop_samples = hop_samples.max(1);

    let mut block_loudness: Vec<f32> = Vec::new();
    let mut pos = 0;

    while pos + block_samples <= filtered.frames {
        let mut mean_sq = 0.0f64;
        let mut count = 0usize;
        for frame in pos..pos + block_samples {
            for c in 0..ch {
                let s = filtered.samples[frame * ch + c] as f64;
                mean_sq += s * s;
                count += 1;
            }
        }
        if count > 0 {
            mean_sq /= count as f64;
        }
        let lufs = if mean_sq > 0.0 {
            (-0.691 + 10.0 * mean_sq.log10()) as f32
        } else {
            f32::NEG_INFINITY
        };
        block_loudness.push(lufs);
        pos += hop_samples;
    }

    if block_loudness.is_empty() {
        return Ok(R128Loudness {
            integrated_lufs: f32::NEG_INFINITY,
            range_lu: 0.0,
            short_term_lufs: f32::NEG_INFINITY,
            momentary_lufs: f32::NEG_INFINITY,
        });
    }

    // Step 3: Absolute gate at -70 LUFS
    let above_abs_gate: Vec<f32> = block_loudness
        .iter()
        .copied()
        .filter(|&l| l > -70.0)
        .collect();

    if above_abs_gate.is_empty() {
        return Ok(R128Loudness {
            integrated_lufs: f32::NEG_INFINITY,
            range_lu: 0.0,
            short_term_lufs: *block_loudness.last().unwrap_or(&f32::NEG_INFINITY),
            momentary_lufs: *block_loudness.last().unwrap_or(&f32::NEG_INFINITY),
        });
    }

    // Ungated loudness (above absolute gate)
    let ungated_mean: f32 = above_abs_gate.iter().sum::<f32>() / above_abs_gate.len() as f32;

    // Step 4: Relative gate at ungated - 10 LU
    let relative_threshold = ungated_mean - 10.0;
    let above_rel_gate: Vec<f32> = above_abs_gate
        .iter()
        .copied()
        .filter(|&l| l > relative_threshold)
        .collect();

    let integrated_lufs = if above_rel_gate.is_empty() {
        ungated_mean
    } else {
        above_rel_gate.iter().sum::<f32>() / above_rel_gate.len() as f32
    };

    // Momentary = last 400ms block
    let momentary_lufs = *block_loudness.last().unwrap_or(&f32::NEG_INFINITY);

    // Short-term = mean of last ~3s (30 blocks at 100ms hop)
    let short_term_blocks = 30.min(block_loudness.len());
    let short_term_slice = &block_loudness[block_loudness.len() - short_term_blocks..];
    let short_term_lufs = if short_term_slice.is_empty() {
        f32::NEG_INFINITY
    } else {
        short_term_slice.iter().sum::<f32>() / short_term_slice.len() as f32
    };

    // LRA: difference between 95th and 10th percentile of gated blocks
    let range_lu = compute_lra(&above_rel_gate);

    Ok(R128Loudness {
        integrated_lufs,
        range_lu,
        short_term_lufs,
        momentary_lufs,
    })
}

/// Apply K-weighting filter (high shelf + high pass).
fn apply_k_weighting(buf: &AudioBuffer) -> AudioBuffer {
    let mut filtered = buf.clone();
    let sr = buf.sample_rate;
    let ch = buf.channels;

    // Stage 1: High shelf at ~1681 Hz, +4 dB (pre-emphasis for head acoustics)
    let shelf_coeffs =
        BiquadCoeffs::design(FilterType::HighShelf { gain_db: 4.0 }, 1681.0, 0.707, sr);

    // Stage 2: High pass at ~38 Hz (remove DC and sub-bass)
    let hp_coeffs = BiquadCoeffs::design(FilterType::HighPass, 38.0, 0.5, sr);

    // Apply both filters
    let mut shelf_states: Vec<(f64, f64)> = vec![(0.0, 0.0); ch as usize];
    let mut hp_states: Vec<(f64, f64)> = vec![(0.0, 0.0); ch as usize];

    let ch = ch as usize;
    for frame in 0..filtered.frames {
        for c in 0..ch {
            let idx = frame * ch + c;
            let mut x = filtered.samples[idx] as f64;

            // High shelf
            let (z1, z2) = &mut shelf_states[c];
            let out = shelf_coeffs.b0 * x + *z1;
            *z1 = shelf_coeffs.b1 * x - shelf_coeffs.a1 * out + *z2;
            *z2 = shelf_coeffs.b2 * x - shelf_coeffs.a2 * out;
            x = out;

            // High pass
            let (z1, z2) = &mut hp_states[c];
            let out = hp_coeffs.b0 * x + *z1;
            *z1 = hp_coeffs.b1 * x - hp_coeffs.a1 * out + *z2;
            *z2 = hp_coeffs.b2 * x - hp_coeffs.a2 * out;

            filtered.samples[idx] = out as f32;
        }
    }

    filtered
}

/// Compute Loudness Range (LRA) from gated block loudness values.
fn compute_lra(blocks: &[f32]) -> f32 {
    if blocks.len() < 2 {
        return 0.0;
    }

    let mut sorted: Vec<f32> = blocks.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let p10 = sorted[(sorted.len() as f32 * 0.1) as usize];
    let p95 = sorted[(sorted.len() as f32 * 0.95).min(sorted.len() as f32 - 1.0) as usize];

    p95 - p10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_r128() {
        let buf = AudioBuffer::silence(2, 48000, 48000);
        let r = measure_r128(&buf).unwrap();
        assert!(r.integrated_lufs < -60.0 || r.integrated_lufs.is_infinite());
    }

    #[test]
    fn sine_r128_reasonable() {
        let sr = 48000u32;
        let frames = sr as usize * 2; // 2 seconds
        let samples: Vec<f32> = (0..frames * 2)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 1000.0 * (i / 2) as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 2, sr).unwrap();
        let r = measure_r128(&buf).unwrap();

        // A -6 dBFS sine should measure around -9 to -12 LUFS
        assert!(r.integrated_lufs > -20.0, "LUFS={}", r.integrated_lufs);
        assert!(r.integrated_lufs < 0.0, "LUFS={}", r.integrated_lufs);
    }

    #[test]
    fn k_weighting_applied() {
        // K-weighting should boost high frequencies and cut lows
        let sr = 48000u32;
        let frames = 48000;
        let samples: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 5000.0 * i as f32 / sr as f32).sin() * 0.5)
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let filtered = apply_k_weighting(&buf);

        // High shelf boosts 5kHz, so filtered RMS should be higher
        assert!(filtered.rms() > buf.rms() * 1.1);
    }

    #[test]
    fn range_non_negative() {
        let sr = 48000u32;
        let frames = sr as usize * 3;
        let samples: Vec<f32> = (0..frames)
            .map(|i| {
                let t = i as f32 / sr as f32;
                // Varying amplitude for non-zero LRA
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * (0.1 + 0.4 * (t * 0.5).sin())
            })
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let r = measure_r128(&buf).unwrap();
        assert!(r.range_lu >= 0.0);
    }
}
