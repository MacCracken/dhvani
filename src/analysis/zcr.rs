//! Zero-crossing rate — counts sign changes per unit time.
//!
//! Useful for speech/music discrimination, percussive content detection,
//! and noise characterization.

use crate::buffer::AudioBuffer;

/// Zero-crossing rate analysis result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ZcrResult {
    /// Average zero-crossing rate across all channels (crossings per second).
    pub rate_hz: f32,
    /// Per-channel zero-crossing rates (crossings per second).
    pub per_channel: Vec<f32>,
    /// Total zero crossings summed across channels.
    pub total_crossings: usize,
}

/// Compute the zero-crossing rate of an audio buffer.
///
/// A zero crossing occurs when consecutive samples have different signs.
/// The rate is expressed as crossings per second.
///
/// # Errors
///
/// Returns an error if the buffer has fewer than 2 frames.
pub fn zero_crossing_rate(buf: &AudioBuffer) -> crate::Result<ZcrResult> {
    if buf.frames < 2 {
        return Err(crate::NadaError::Dsp(
            "zero_crossing_rate requires at least 2 frames".into(),
        ));
    }

    let ch = buf.channels as usize;
    let samples = buf.samples();
    let mut per_channel = Vec::with_capacity(ch);
    let mut total = 0usize;

    for c in 0..ch {
        let mut crossings = 0usize;
        let mut prev = samples[c]; // first frame, channel c
        for frame in 1..buf.frames {
            let curr = samples[frame * ch + c];
            // Sign change: positive↔negative (not counting zero→nonzero)
            if (prev > 0.0 && curr < 0.0) || (prev < 0.0 && curr > 0.0) {
                crossings += 1;
            }
            prev = curr;
        }
        let rate = crossings as f32 * buf.sample_rate as f32 / (buf.frames - 1) as f32;
        per_channel.push(rate);
        total += crossings;
    }

    let rate_hz = per_channel.iter().sum::<f32>() / ch as f32;

    Ok(ZcrResult {
        rate_hz,
        per_channel,
        total_crossings: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_zcr() {
        let buf = AudioBuffer::silence(1, 44100, 44100);
        let result = zero_crossing_rate(&buf).unwrap();
        assert_eq!(result.total_crossings, 0);
        assert_eq!(result.rate_hz, 0.0);
    }

    #[test]
    fn sine_zcr_matches_frequency() {
        // A 440 Hz sine should have ~880 zero crossings per second (2 per cycle)
        let samples: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let result = zero_crossing_rate(&buf).unwrap();
        // Allow ±5 Hz tolerance for edge effects
        assert!(
            (result.rate_hz - 880.0).abs() < 5.0,
            "expected ~880 Hz ZCR, got {}",
            result.rate_hz
        );
    }

    #[test]
    fn stereo_independent() {
        // L = 440 Hz, R = 220 Hz
        let mut samples = Vec::with_capacity(88200);
        for i in 0..44100 {
            let t = i as f32 / 44100.0;
            samples.push((2.0 * std::f32::consts::PI * 440.0 * t).sin());
            samples.push((2.0 * std::f32::consts::PI * 220.0 * t).sin());
        }
        let buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        let result = zero_crossing_rate(&buf).unwrap();
        assert!(
            (result.per_channel[0] - 880.0).abs() < 5.0,
            "L: expected ~880, got {}",
            result.per_channel[0]
        );
        assert!(
            (result.per_channel[1] - 440.0).abs() < 5.0,
            "R: expected ~440, got {}",
            result.per_channel[1]
        );
    }

    #[test]
    fn too_short_buffer() {
        let buf = AudioBuffer::from_interleaved(vec![0.5], 1, 44100).unwrap();
        assert!(zero_crossing_rate(&buf).is_err());
    }
}
