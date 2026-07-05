//! Waveform extraction — downsampled peak data for visualization.

use crate::buffer::AudioBuffer;

/// Waveform visualization data for a single channel.
#[must_use]
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct WaveformData {
    /// Per-channel peak data: `channels[ch_index]` is a Vec of (min, max) pairs.
    pub channels: Vec<Vec<(f32, f32)>>,
    /// Number of peaks per second used for extraction.
    pub peaks_per_second: u32,
    /// Sample rate of the source audio.
    pub sample_rate: u32,
}

impl WaveformData {
    /// Total number of peak points per channel.
    pub fn num_peaks(&self) -> usize {
        self.channels.first().map(|c| c.len()).unwrap_or(0)
    }

    /// Number of channels.
    pub fn num_channels(&self) -> usize {
        self.channels.len()
    }

    /// Duration in seconds.
    pub fn duration_secs(&self) -> f32 {
        if self.peaks_per_second == 0 {
            return 0.0;
        }
        self.num_peaks() as f32 / self.peaks_per_second as f32
    }
}

/// Compute downsampled waveform peak data for visualization.
///
/// Returns (min, max) pairs for each time window, per channel.
///
/// - `peaks_per_second`: resolution of the output (e.g., 100 = 100 peaks per second)
pub fn compute_waveform(buf: &AudioBuffer, peaks_per_second: u32) -> WaveformData {
    let peaks_per_second = peaks_per_second.max(1);
    let ch = buf.channels as usize;
    let samples_per_peak = (buf.sample_rate as f64 / peaks_per_second as f64).ceil() as usize;
    let samples_per_peak = samples_per_peak.max(1);
    let num_peaks = buf.frames.div_ceil(samples_per_peak);

    let mut channels: Vec<Vec<(f32, f32)>> =
        (0..ch).map(|_| Vec::with_capacity(num_peaks)).collect();

    for peak_idx in 0..num_peaks {
        let start_frame = peak_idx * samples_per_peak;
        let end_frame = ((peak_idx + 1) * samples_per_peak).min(buf.frames);

        for (c, channel) in channels.iter_mut().enumerate() {
            let mut min_val = f32::MAX;
            let mut max_val = f32::MIN;

            for frame in start_frame..end_frame {
                let sample = buf.samples[frame * ch + c];
                min_val = min_val.min(sample);
                max_val = max_val.max(sample);
            }

            if min_val == f32::MAX {
                min_val = 0.0;
                max_val = 0.0;
            }

            channel.push((min_val, max_val));
        }
    }

    WaveformData {
        channels,
        peaks_per_second,
        sample_rate: buf.sample_rate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_waveform() {
        let buf = AudioBuffer::silence(2, 44100, 44100);
        let wf = compute_waveform(&buf, 100);
        assert_eq!(wf.num_channels(), 2);
        assert!(wf.num_peaks() > 0);
        for ch in &wf.channels {
            for &(min, max) in ch {
                assert_eq!(min, 0.0);
                assert_eq!(max, 0.0);
            }
        }
    }

    #[test]
    fn sine_waveform_has_range() {
        let sr = 44100u32;
        // Use 440Hz so many cycles fit in each peak window
        let samples: Vec<f32> = (0..sr as usize)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();
        let wf = compute_waveform(&buf, 100);

        assert_eq!(wf.num_channels(), 1);
        // With 440Hz, each peak window (~441 samples) spans many cycles
        let mut has_range = false;
        for &(min, max) in &wf.channels[0] {
            if min < -0.5 && max > 0.5 {
                has_range = true;
            }
        }
        assert!(has_range, "Sine waveform should show amplitude range");
    }

    #[test]
    fn resolution_affects_peak_count() {
        let buf = AudioBuffer::silence(1, 44100, 44100); // 1 second
        let wf_100 = compute_waveform(&buf, 100);
        let wf_50 = compute_waveform(&buf, 50);
        assert!(wf_100.num_peaks() > wf_50.num_peaks());
    }

    #[test]
    fn duration_correct() {
        let buf = AudioBuffer::silence(1, 44100, 44100);
        let wf = compute_waveform(&buf, 100);
        assert!((wf.duration_secs() - 1.0).abs() < 0.1);
    }

    #[test]
    fn stereo_waveform() {
        let samples: Vec<f32> = (0..88200)
            .map(|i| if i % 2 == 0 { 0.5 } else { -0.5 })
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        let wf = compute_waveform(&buf, 100);
        assert_eq!(wf.num_channels(), 2);
        // L channel all 0.5, R channel all -0.5
        for &(min, max) in &wf.channels[0] {
            assert!((min - 0.5).abs() < f32::EPSILON);
            assert!((max - 0.5).abs() < f32::EPSILON);
        }
        for &(min, max) in &wf.channels[1] {
            assert!((min + 0.5).abs() < f32::EPSILON);
            assert!((max + 0.5).abs() < f32::EPSILON);
        }
    }
}
