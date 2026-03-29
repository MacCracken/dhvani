//! Beat and tempo detection — autocorrelation of onset strength function.
//!
//! Estimates tempo (BPM) from spectral flux onset detection.
//! Uses autocorrelation to find the dominant periodicity in the onset
//! strength envelope.

use crate::buffer::AudioBuffer;

/// Beat detection result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct BeatResult {
    /// Estimated tempo in beats per minute.
    pub bpm: f32,
    /// Confidence of the BPM estimate (0.0–1.0).
    pub confidence: f32,
    /// Beat positions as sample indices.
    pub beat_positions: Vec<usize>,
}

/// Detect tempo and beat positions from an audio buffer.
///
/// Uses spectral flux onset detection followed by autocorrelation
/// to find the dominant tempo.
///
/// # Arguments
///
/// * `buf` — Audio buffer to analyze
/// * `min_bpm` — Minimum expected tempo (e.g., 60.0)
/// * `max_bpm` — Maximum expected tempo (e.g., 200.0)
///
/// # Errors
///
/// Returns an error if the buffer is too short for analysis (< 1 second)
/// or if onset detection fails.
pub fn detect_tempo(buf: &AudioBuffer, min_bpm: f32, max_bpm: f32) -> crate::Result<BeatResult> {
    let duration_s = buf.frames as f32 / buf.sample_rate as f32;
    if duration_s < 1.0 {
        return Err(crate::NadaError::Dsp(
            "detect_tempo requires at least 1 second of audio".into(),
        ));
    }

    // Step 1: Compute onset strength envelope via spectral flux
    let window_size = 2048;
    let hop_size = 512;
    let onset_env = onset_strength_envelope(buf, window_size, hop_size)?;

    if onset_env.len() < 4 {
        return Ok(BeatResult {
            bpm: 0.0,
            confidence: 0.0,
            beat_positions: Vec::new(),
        });
    }

    // Step 2: Autocorrelation of onset envelope
    let onset_sr = buf.sample_rate as f32 / hop_size as f32; // frames per second
    let min_lag = (onset_sr * 60.0 / max_bpm) as usize; // shortest period (fastest tempo)
    let max_lag = (onset_sr * 60.0 / min_bpm) as usize; // longest period (slowest tempo)
    let max_lag = max_lag.min(onset_env.len() / 2);

    if min_lag >= max_lag || max_lag == 0 {
        return Ok(BeatResult {
            bpm: 0.0,
            confidence: 0.0,
            beat_positions: Vec::new(),
        });
    }

    let autocorr = autocorrelation(&onset_env, min_lag, max_lag);

    // Step 3: Find peak in autocorrelation → dominant tempo
    let (best_lag, best_val, max_val) = find_peak(&autocorr, min_lag);

    if max_val <= 0.0 {
        return Ok(BeatResult {
            bpm: 0.0,
            confidence: 0.0,
            beat_positions: Vec::new(),
        });
    }

    let bpm = onset_sr * 60.0 / best_lag as f32;
    let confidence = (best_val / max_val).clamp(0.0, 1.0);

    // Step 4: Extract beat positions using detected period
    let beat_positions = extract_beats(buf, &onset_env, hop_size, best_lag);

    Ok(BeatResult {
        bpm,
        confidence,
        beat_positions,
    })
}

/// Compute onset strength envelope (spectral flux per STFT frame).
fn onset_strength_envelope(
    buf: &AudioBuffer,
    window_size: usize,
    hop_size: usize,
) -> crate::Result<Vec<f32>> {
    let spectrogram = super::stft::stft(buf, window_size, hop_size)?;
    let frames = &spectrogram.frames;

    if frames.len() < 2 {
        return Ok(Vec::new());
    }

    let mut envelope = Vec::with_capacity(frames.len());
    envelope.push(0.0); // first frame has no previous

    for i in 1..frames.len() {
        let flux: f32 = frames[i]
            .iter()
            .zip(frames[i - 1].iter())
            .map(|(&curr, &prev)| (curr - prev).max(0.0))
            .sum();
        envelope.push(flux);
    }

    // Normalize
    let max_flux = envelope.iter().cloned().fold(0.0f32, f32::max);
    if max_flux > 0.0 {
        for v in &mut envelope {
            *v /= max_flux;
        }
    }

    Ok(envelope)
}

/// Autocorrelation of a signal over a lag range.
#[inline]
fn autocorrelation(signal: &[f32], min_lag: usize, max_lag: usize) -> Vec<f32> {
    let n = signal.len();
    let mut result = vec![0.0f32; max_lag + 1];

    for lag in min_lag..=max_lag {
        let mut sum = 0.0f32;
        let count = n - lag;
        for i in 0..count {
            sum += signal[i] * signal[i + lag];
        }
        result[lag] = sum / count as f32;
    }

    result
}

/// Find the peak in the autocorrelation result.
/// Returns (best_lag, best_value, max_possible_value).
fn find_peak(autocorr: &[f32], min_lag: usize) -> (usize, f32, f32) {
    let mut best_lag = min_lag;
    let mut best_val = 0.0f32;
    let mut max_val = 0.0f32;

    for (lag, &val) in autocorr.iter().enumerate().skip(min_lag) {
        if val > max_val {
            max_val = val;
        }
        if val > best_val {
            best_val = val;
            best_lag = lag;
        }
    }

    (best_lag, best_val, max_val)
}

/// Extract beat positions from onset envelope using detected period.
fn extract_beats(
    buf: &AudioBuffer,
    onset_env: &[f32],
    hop_size: usize,
    period_frames: usize,
) -> Vec<usize> {
    if period_frames == 0 || onset_env.is_empty() {
        return Vec::new();
    }

    let mut beats = Vec::new();
    let half_window = period_frames / 4;

    // Walk through onset envelope at beat-period intervals,
    // snapping to the strongest onset within a window around each expected position.
    let mut pos = 0usize;
    while pos < onset_env.len() {
        let start = pos.saturating_sub(half_window);
        let end = (pos + half_window).min(onset_env.len());

        // Find strongest onset in window
        let mut best_idx = pos.min(onset_env.len() - 1);
        let mut best_strength = 0.0f32;
        for (i, &strength) in onset_env.iter().enumerate().take(end).skip(start) {
            if strength > best_strength {
                best_strength = strength;
                best_idx = i;
            }
        }

        let sample_pos = best_idx * hop_size;
        if sample_pos < buf.frames * buf.channels as usize {
            beats.push(sample_pos);
        }

        pos += period_frames;
    }

    beats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_no_beats() {
        let buf = AudioBuffer::silence(1, 44100 * 2, 44100);
        let result = detect_tempo(&buf, 60.0, 200.0).unwrap();
        assert!(result.beat_positions.is_empty() || result.confidence < 0.1);
    }

    #[test]
    fn click_track_120bpm() {
        // Generate a click track at 120 BPM (one click every 0.5s)
        let sr = 44100;
        let duration = 4; // 4 seconds
        let total_samples = sr * duration;
        let mut samples = vec![0.0f32; total_samples];
        let click_interval = sr / 2; // 0.5s = 120 BPM

        for beat in 0..8 {
            let start = beat * click_interval;
            // Short click: 10ms burst
            let click_len = sr / 100;
            for i in 0..click_len.min(total_samples - start) {
                samples[start + i] =
                    0.9 * (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / sr as f32).sin();
            }
        }

        let buf = AudioBuffer::from_interleaved(samples, 1, sr as u32).unwrap();
        let result = detect_tempo(&buf, 60.0, 200.0).unwrap();

        // Should detect ~120 BPM (allow ±15 BPM tolerance for short signal)
        assert!(
            (result.bpm - 120.0).abs() < 15.0,
            "expected ~120 BPM, got {} (confidence: {})",
            result.bpm,
            result.confidence
        );
    }

    #[test]
    fn too_short_buffer() {
        let buf = AudioBuffer::from_interleaved(vec![0.5; 22050], 1, 44100).unwrap();
        assert!(detect_tempo(&buf, 60.0, 200.0).is_err());
    }

    #[test]
    fn beat_positions_ordered() {
        let sr = 44100;
        let total_samples = sr * 3;
        let mut samples = vec![0.0f32; total_samples];
        let click_interval = sr / 2;

        for beat in 0..6 {
            let start = beat * click_interval;
            let click_len = sr / 100;
            for i in 0..click_len.min(total_samples - start) {
                samples[start + i] = 0.9;
            }
        }

        let buf = AudioBuffer::from_interleaved(samples, 1, sr as u32).unwrap();
        let result = detect_tempo(&buf, 60.0, 200.0).unwrap();

        // Beat positions should be monotonically increasing
        for w in result.beat_positions.windows(2) {
            assert!(w[0] < w[1], "beats not ordered: {} >= {}", w[0], w[1]);
        }
    }
}
