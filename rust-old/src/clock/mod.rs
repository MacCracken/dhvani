//! Sample-accurate audio clock — transport, tempo, A/V sync.

use serde::{Deserialize, Serialize};

/// Audio transport clock — tracks position in samples, with tempo awareness.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioClock {
    position_samples: u64,
    sample_rate: u32,
    tempo_bpm: f64,
    running: bool,
}

impl AudioClock {
    /// Create a new clock at position 0.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            position_samples: 0,
            sample_rate,
            tempo_bpm: 0.0,
            running: false,
        }
    }

    /// Create a clock with tempo.
    pub fn with_tempo(sample_rate: u32, bpm: f64) -> Self {
        Self {
            position_samples: 0,
            sample_rate,
            tempo_bpm: bpm,
            running: false,
        }
    }

    /// Current position in seconds.
    #[inline]
    pub fn position_secs(&self) -> f64 {
        self.position_samples as f64 / self.sample_rate as f64
    }

    /// Current position in milliseconds.
    #[inline]
    pub fn position_ms(&self) -> f64 {
        self.position_secs() * 1000.0
    }

    /// Current beat position (requires tempo > 0).
    #[inline]
    pub fn position_beats(&self) -> Option<f64> {
        if self.tempo_bpm <= 0.0 {
            return None;
        }
        Some(self.position_secs() * self.tempo_bpm / 60.0)
    }

    /// Advance the clock by the given number of frames.
    pub fn advance(&mut self, frames: u64) {
        if self.running {
            self.position_samples += frames;
        }
    }

    /// Start the clock.
    pub fn start(&mut self) {
        self.running = true;
    }

    /// Stop the clock.
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Reset to position 0.
    pub fn reset(&mut self) {
        self.position_samples = 0;
    }

    /// Seek to a position in seconds.
    pub fn seek_secs(&mut self, secs: f64) {
        self.position_samples = (secs * self.sample_rate as f64).max(0.0) as u64;
    }

    /// Samples per beat (requires tempo > 0).
    #[inline]
    pub fn samples_per_beat(&self) -> Option<f64> {
        if self.tempo_bpm <= 0.0 {
            return None;
        }
        Some(self.sample_rate as f64 * 60.0 / self.tempo_bpm)
    }

    /// Generate a PTS (presentation timestamp) in microseconds for A/V sync.
    #[inline]
    pub fn pts_us(&self) -> u64 {
        (self.position_secs() * 1_000_000.0) as u64
    }

    /// Current position in samples.
    #[inline]
    pub fn position_samples(&self) -> u64 {
        self.position_samples
    }

    /// Sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Tempo in BPM (0 = no tempo).
    #[inline]
    pub fn tempo_bpm(&self) -> f64 {
        self.tempo_bpm
    }

    /// Set the tempo in BPM. Negative and NaN values are clamped to 0.
    pub fn set_tempo(&mut self, bpm: f64) {
        self.tempo_bpm = if bpm.is_finite() && bpm > 0.0 {
            bpm
        } else {
            0.0
        };
    }

    /// Whether the clock is running.
    #[inline]
    pub fn is_running(&self) -> bool {
        self.running
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_basic() {
        let mut clock = AudioClock::new(44100);
        assert_eq!(clock.position_samples(), 0);
        assert!(!clock.is_running());
        clock.start();
        clock.advance(44100);
        assert!((clock.position_secs() - 1.0).abs() < 0.001);
    }

    #[test]
    fn clock_no_advance_when_stopped() {
        let mut clock = AudioClock::new(44100);
        clock.advance(44100);
        assert_eq!(clock.position_samples(), 0);
    }

    #[test]
    fn clock_seek() {
        let mut clock = AudioClock::new(48000);
        clock.seek_secs(2.5);
        assert_eq!(clock.position_samples(), 120000);
        assert!((clock.position_secs() - 2.5).abs() < 0.001);
    }

    #[test]
    fn clock_tempo() {
        let clock = AudioClock::with_tempo(44100, 120.0);
        assert!(clock.samples_per_beat().is_some());
        assert!((clock.samples_per_beat().unwrap() - 22050.0).abs() < 1.0);
    }

    #[test]
    fn clock_beats_position() {
        let mut clock = AudioClock::with_tempo(44100, 120.0);
        clock.start();
        clock.advance(44100); // 1 second = 2 beats at 120bpm
        let beats = clock.position_beats().unwrap();
        assert!((beats - 2.0).abs() < 0.01);
    }

    #[test]
    fn clock_no_beats_without_tempo() {
        let clock = AudioClock::new(44100);
        assert!(clock.position_beats().is_none());
        assert!(clock.samples_per_beat().is_none());
    }

    #[test]
    fn clock_pts() {
        let mut clock = AudioClock::new(48000);
        clock.start();
        clock.advance(48000); // 1 second
        assert_eq!(clock.pts_us(), 1_000_000);
    }

    #[test]
    fn clock_reset() {
        let mut clock = AudioClock::new(44100);
        clock.start();
        clock.advance(44100);
        clock.reset();
        assert_eq!(clock.position_samples(), 0);
    }

    #[test]
    fn clock_ms() {
        let mut clock = AudioClock::new(44100);
        clock.start();
        clock.advance(441); // 10ms
        assert!((clock.position_ms() - 10.0).abs() < 0.1);
    }
}
