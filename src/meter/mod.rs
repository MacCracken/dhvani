//! Lock-free audio metering — stereo peak levels via atomic operations.
//!
//! Designed for real-time audio: the RT thread writes peak levels via
//! `AtomicU32` (f32 bit patterns), and the UI thread reads them without
//! mutexes or blocking.

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

/// A single stereo peak level stored as two atomic u32 (f32 bit patterns).
///
/// Lock-free: safe to write from the RT thread and read from the UI thread.
#[must_use]
#[derive(Debug)]
pub struct PeakMeter {
    left: AtomicU32,
    right: AtomicU32,
}

impl PeakMeter {
    /// Create a meter reading zero.
    pub fn new() -> Self {
        Self {
            left: AtomicU32::new(0.0f32.to_bits()),
            right: AtomicU32::new(0.0f32.to_bits()),
        }
    }

    /// Store a stereo peak level (call from RT thread).
    #[inline]
    pub fn store(&self, left: f32, right: f32) {
        self.left.store(left.to_bits(), Ordering::Relaxed);
        self.right.store(right.to_bits(), Ordering::Relaxed);
    }

    /// Load the current stereo peak level (call from UI thread).
    #[inline]
    pub fn load(&self) -> [f32; 2] {
        let l = f32::from_bits(self.left.load(Ordering::Relaxed));
        let r = f32::from_bits(self.right.load(Ordering::Relaxed));
        [l, r]
    }
}

impl Default for PeakMeter {
    fn default() -> Self {
        Self::new()
    }
}

/// A bank of stereo peak meters with dynamic slot activation.
///
/// Pre-allocates capacity at creation time. Slots can be activated
/// up to capacity without reallocation.
#[must_use]
pub struct MeterBank {
    slots: Vec<PeakMeter>,
    active: AtomicUsize,
}

impl MeterBank {
    /// Create a meter bank with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            slots: (0..capacity).map(|_| PeakMeter::new()).collect(),
            active: AtomicUsize::new(capacity),
        }
    }

    /// Number of active slots.
    pub fn len(&self) -> usize {
        self.active.load(Ordering::Relaxed)
    }

    /// Whether the bank has no active slots.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Total allocated capacity.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Store a stereo peak level at the given slot index.
    ///
    /// No-op if index is out of bounds.
    pub fn store(&self, index: usize, left: f32, right: f32) {
        if let Some(slot) = self.slots.get(index) {
            slot.store(left, right);
        }
    }

    /// Load the stereo peak level at the given slot index.
    ///
    /// Returns `[0.0, 0.0]` if index is out of bounds.
    pub fn load(&self, index: usize) -> [f32; 2] {
        self.slots
            .get(index)
            .map(|s| s.load())
            .unwrap_or([0.0, 0.0])
    }

    /// Read all active slots into a Vec.
    pub fn read_all(&self) -> Vec<[f32; 2]> {
        let active = self.len();
        (0..active).map(|i| self.load(i)).collect()
    }

    /// Set the number of active slots (clamped to capacity).
    pub fn set_active(&self, count: usize) {
        self.active
            .store(count.min(self.slots.len()), Ordering::Relaxed);
    }
}

// SAFETY: PeakMeter uses only atomics, no interior mutability beyond that.
unsafe impl Sync for MeterBank {}
unsafe impl Send for MeterBank {}

impl std::fmt::Debug for MeterBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeterBank")
            .field("capacity", &self.capacity())
            .field("active", &self.len())
            .finish()
    }
}

/// Shared meter bank — `Arc`-wrapped for multi-thread sharing.
pub type SharedMeterBank = Arc<MeterBank>;

/// Create a shared meter bank with the given capacity.
pub fn shared_meter_bank(capacity: usize) -> SharedMeterBank {
    Arc::new(MeterBank::new(capacity))
}

// ── Block-accumulating level meter ────────────────────────────────

/// Floor value for dB conversion — represents silence.
const DB_FLOOR: f32 = -200.0;
/// Minimum linear amplitude treated as non-zero.
const LINEAR_FLOOR: f32 = 1e-10;
/// EBU R128 LUFS offset constant.
const LUFS_OFFSET: f64 = -0.691;
/// LUFS block duration in seconds (EBU R128: 400ms).
const LUFS_BLOCK_DURATION: f32 = 0.4;
/// Peak hold decay coefficient per sample.
const PEAK_DECAY_COEFFICIENT: f32 = 0.9995;
/// Maximum number of LUFS blocks retained (~10 minutes of 400ms blocks).
const MAX_LUFS_BLOCKS: usize = 1500;

/// Convert linear amplitude to dB.
fn linear_to_db(linear: f32) -> f32 {
    if linear < LINEAR_FLOOR {
        DB_FLOOR
    } else {
        20.0 * linear.log10()
    }
}

/// Block-accumulating audio level meter with peak, RMS, LUFS, and peak-hold.
///
/// Unlike [`PeakMeter`] (lock-free atomic for RT threads), this meter
/// accumulates statistics across multiple `process()` calls and computes
/// integrated LUFS using simplified EBU R128 gating.
///
/// Use for offline analysis, UI metering displays, and loudness monitoring.
#[must_use]
#[derive(Debug, Clone)]
pub struct LevelMeter {
    /// Current peak level per channel (linear).
    pub(crate) peak: Vec<f32>,
    /// Current RMS level per channel (linear).
    pub(crate) rms: Vec<f32>,
    /// Integrated LUFS value (mono/stereo).
    pub(crate) lufs: f32,
    channels: usize,
    rms_sum: Vec<f64>,
    rms_count: u64,
    lufs_blocks: Vec<f64>,
    lufs_buffer: Vec<f64>,
    lufs_buffer_pos: usize,
    /// Peak hold with decay.
    peak_hold: Vec<f32>,
    peak_decay: f32,
}

impl LevelMeter {
    /// Create a new level meter for the given channel count and sample rate.
    pub fn new(channels: usize, sample_rate: f32) -> Self {
        Self {
            peak: vec![0.0; channels],
            rms: vec![0.0; channels],
            lufs: DB_FLOOR,
            channels,
            rms_sum: vec![0.0; channels],
            rms_count: 0,
            lufs_blocks: Vec::new(),
            lufs_buffer: vec![0.0; (sample_rate * LUFS_BLOCK_DURATION) as usize],
            lufs_buffer_pos: 0,
            peak_hold: vec![0.0; channels],
            peak_decay: PEAK_DECAY_COEFFICIENT,
        }
    }

    /// Analyze an audio buffer and update all meter values.
    ///
    /// The buffer's channel count is clamped to the meter's channel count.
    /// Call repeatedly with successive buffers for continuous metering.
    pub fn process(&mut self, buf: &crate::buffer::AudioBuffer) {
        let frames = buf.frames;
        let buf_channels = buf.channels as usize;
        let active_channels = buf_channels.min(self.channels);

        // Reset per-block peak
        for ch_peak in &mut self.peak {
            *ch_peak = 0.0;
        }

        for frame in 0..frames {
            let mut channel_sq_sum: f64 = 0.0;

            for ch in 0..active_channels {
                let sample = buf.samples[frame * buf_channels + ch];
                let abs = sample.abs();

                if abs > self.peak[ch] {
                    self.peak[ch] = abs;
                }

                let sq = (sample as f64).powi(2);
                self.rms_sum[ch] += sq;
                channel_sq_sum += sq;
            }

            self.rms_count += 1;

            let mean_sq = if active_channels > 0 {
                channel_sq_sum / active_channels as f64
            } else {
                0.0
            };
            if self.lufs_buffer_pos < self.lufs_buffer.len() {
                self.lufs_buffer[self.lufs_buffer_pos] = mean_sq;
                self.lufs_buffer_pos += 1;
            }

            if self.lufs_buffer_pos >= self.lufs_buffer.len() {
                let block_power: f64 =
                    self.lufs_buffer.iter().sum::<f64>() / self.lufs_buffer.len() as f64;
                self.lufs_blocks.push(block_power);
                self.lufs_buffer_pos = 0;
                self.compute_lufs();
            }
        }

        // Update RMS
        if self.rms_count > 0 {
            for ch in 0..self.channels {
                self.rms[ch] = (self.rms_sum[ch] / self.rms_count as f64).sqrt() as f32;
            }
        }

        // Update peak hold with decay (scale by frame count for buffer-size-independent rate)
        let decay = self.peak_decay.powi(frames as i32);
        for ch in 0..self.channels {
            if self.peak[ch] > self.peak_hold[ch] {
                self.peak_hold[ch] = self.peak[ch];
            } else {
                self.peak_hold[ch] *= decay;
            }
        }
    }

    /// Compute integrated LUFS using simplified EBU R128 gating.
    fn compute_lufs(&mut self) {
        if self.lufs_blocks.is_empty() {
            self.lufs = DB_FLOOR;
            return;
        }
        if self.lufs_blocks.len() > MAX_LUFS_BLOCKS {
            let drain_count = self.lufs_blocks.len() - MAX_LUFS_BLOCKS;
            self.lufs_blocks.drain(..drain_count);
        }

        let abs_gate = 10.0_f64.powf(-70.0 / 10.0);

        let mut gated_count: usize = 0;
        let mut gated_sum: f64 = 0.0;
        for &p in &self.lufs_blocks {
            if p > abs_gate {
                gated_count += 1;
                gated_sum += p;
            }
        }

        if gated_count == 0 {
            self.lufs = DB_FLOOR;
            return;
        }

        let mean_power = gated_sum / gated_count as f64;
        let rel_gate = mean_power * 10.0_f64.powf(-10.0 / 10.0);

        let mut final_count: usize = 0;
        let mut final_sum: f64 = 0.0;
        for &p in &self.lufs_blocks {
            if p > abs_gate && p > rel_gate {
                final_count += 1;
                final_sum += p;
            }
        }

        if final_count == 0 {
            self.lufs = DB_FLOOR;
            return;
        }

        let integrated = final_sum / final_count as f64;
        self.lufs = (LUFS_OFFSET + 10.0 * integrated.log10()) as f32;
    }

    /// Peak levels per channel (linear).
    pub fn peak(&self) -> &[f32] {
        &self.peak
    }

    /// RMS levels per channel (linear).
    pub fn rms(&self) -> &[f32] {
        &self.rms
    }

    /// Integrated LUFS value.
    pub fn lufs(&self) -> f32 {
        self.lufs
    }

    /// Get peak level in dB for a channel.
    pub fn peak_db(&self, channel: usize) -> f32 {
        linear_to_db(self.peak.get(channel).copied().unwrap_or(0.0))
    }

    /// Get RMS level in dB for a channel.
    pub fn rms_db(&self, channel: usize) -> f32 {
        linear_to_db(self.rms.get(channel).copied().unwrap_or(0.0))
    }

    /// Get peak hold level in dB for a channel.
    pub fn peak_hold_db(&self, channel: usize) -> f32 {
        linear_to_db(self.peak_hold.get(channel).copied().unwrap_or(0.0))
    }

    /// Peak hold values per channel (linear).
    pub fn peak_hold(&self) -> &[f32] {
        &self.peak_hold
    }

    /// Number of channels this meter tracks.
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Reset all meter state.
    pub fn reset(&mut self) {
        self.peak.fill(0.0);
        self.rms.fill(0.0);
        self.lufs = DB_FLOOR;
        self.rms_sum.fill(0.0);
        self.rms_count = 0;
        self.lufs_blocks.clear();
        self.lufs_buffer.fill(0.0);
        self.lufs_buffer_pos = 0;
        self.peak_hold.fill(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peak_meter_store_load() {
        let meter = PeakMeter::new();
        assert_eq!(meter.load(), [0.0, 0.0]);
        meter.store(0.8, 0.6);
        assert_eq!(meter.load(), [0.8, 0.6]);
    }

    #[test]
    fn meter_bank_basic() {
        let bank = MeterBank::new(4);
        assert_eq!(bank.len(), 4);
        assert_eq!(bank.capacity(), 4);

        bank.store(0, 0.9, 0.7);
        bank.store(1, 0.5, 0.3);
        assert_eq!(bank.load(0), [0.9, 0.7]);
        assert_eq!(bank.load(1), [0.5, 0.3]);
        assert_eq!(bank.load(99), [0.0, 0.0]); // out of bounds
    }

    #[test]
    fn meter_bank_read_all() {
        let bank = MeterBank::new(3);
        bank.store(0, 0.1, 0.2);
        bank.store(1, 0.3, 0.4);
        bank.store(2, 0.5, 0.6);
        let all = bank.read_all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], [0.1, 0.2]);
        assert_eq!(all[2], [0.5, 0.6]);
    }

    #[test]
    fn meter_bank_set_active() {
        let bank = MeterBank::new(8);
        assert_eq!(bank.len(), 8);
        bank.set_active(3);
        assert_eq!(bank.len(), 3);
        let all = bank.read_all();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn shared_meter_bank_threaded() {
        let bank = shared_meter_bank(4);
        let bank2 = bank.clone();

        let handle = std::thread::spawn(move || {
            bank2.store(0, 0.99, 0.88);
        });
        handle.join().unwrap();

        let levels = bank.load(0);
        assert_eq!(levels, [0.99, 0.88]);
    }

    #[test]
    fn meter_bank_out_of_bounds_store_noop() {
        let bank = MeterBank::new(2);
        bank.store(999, 1.0, 1.0); // should not panic
        assert_eq!(bank.load(999), [0.0, 0.0]);
    }

    // ── LevelMeter tests ─────────────────────────────────────────

    #[test]
    fn level_meter_silence() {
        let buf = crate::buffer::AudioBuffer::silence(2, 256, 48000);
        let mut meter = LevelMeter::new(2, 48000.0);
        meter.process(&buf);
        assert_eq!(meter.peak[0], 0.0);
        assert_eq!(meter.peak[1], 0.0);
        assert_eq!(meter.rms[0], 0.0);
    }

    #[test]
    fn level_meter_peak_detection() {
        let mut data = vec![0.0f32; 256];
        data[100] = 0.75;
        data[200] = -0.9;
        let buf = crate::buffer::AudioBuffer::from_interleaved(data, 1, 48000).unwrap();
        let mut meter = LevelMeter::new(1, 48000.0);
        meter.process(&buf);
        assert!((meter.peak[0] - 0.9).abs() < 0.001);
    }

    #[test]
    fn level_meter_rms_constant() {
        let buf =
            crate::buffer::AudioBuffer::from_interleaved(vec![0.5f32; 1024], 1, 48000).unwrap();
        let mut meter = LevelMeter::new(1, 48000.0);
        meter.process(&buf);
        assert!((meter.rms[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn level_meter_rms_sine() {
        let frames = 48000;
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin())
            .collect();
        let buf = crate::buffer::AudioBuffer::from_interleaved(data, 1, 48000).unwrap();
        let mut meter = LevelMeter::new(1, 48000.0);
        meter.process(&buf);
        assert!(
            (meter.rms[0] - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01,
            "RMS of unit sine should be ~0.707, got {}",
            meter.rms[0]
        );
    }

    #[test]
    fn level_meter_db_conversion() {
        assert!(linear_to_db(1.0).abs() < 0.001);
        assert!((linear_to_db(0.5) - (-6.02)).abs() < 0.1);
        assert!(linear_to_db(0.0) < -100.0);
    }

    #[test]
    fn level_meter_reset() {
        let buf =
            crate::buffer::AudioBuffer::from_interleaved(vec![0.5f32; 512], 2, 48000).unwrap();
        let mut meter = LevelMeter::new(2, 48000.0);
        meter.process(&buf);
        meter.reset();
        assert_eq!(meter.peak[0], 0.0);
        assert_eq!(meter.rms[0], 0.0);
        assert_eq!(meter.lufs, DB_FLOOR);
    }

    #[test]
    fn level_meter_lufs_with_signal() {
        let frames = 96000; // 2 seconds
        let data: Vec<f32> = (0..frames)
            .map(|i| (2.0 * std::f32::consts::PI * 1000.0 * i as f32 / 48000.0).sin() * 0.5)
            .collect();
        let buf = crate::buffer::AudioBuffer::from_interleaved(data, 1, 48000).unwrap();
        let mut meter = LevelMeter::new(1, 48000.0);
        meter.process(&buf);
        assert!(
            meter.lufs > -200.0,
            "LUFS should be computed, got {}",
            meter.lufs
        );
        assert!(
            meter.lufs < 0.0,
            "LUFS should be negative, got {}",
            meter.lufs
        );
    }

    #[test]
    fn level_meter_peak_hold_decay() {
        let mut meter = LevelMeter::new(1, 48000.0);
        let mut data = vec![0.0f32; 256];
        data[50] = 0.8;
        let buf = crate::buffer::AudioBuffer::from_interleaved(data, 1, 48000).unwrap();
        meter.process(&buf);
        assert!((meter.peak_hold[0] - 0.8).abs() < 0.001);

        // Second quiet block — peak hold should decay
        let buf2 =
            crate::buffer::AudioBuffer::from_interleaved(vec![0.01f32; 256], 1, 48000).unwrap();
        meter.process(&buf2);
        assert!(meter.peak_hold[0] > 0.0);
        assert!(meter.peak_hold[0] < 0.8);
    }

    #[test]
    fn level_meter_multi_channel() {
        let frames = 256;
        let mut data = vec![0.0f32; frames * 2];
        for i in 0..frames {
            data[i * 2] = 0.6;
            data[i * 2 + 1] = 0.3;
        }
        let buf = crate::buffer::AudioBuffer::from_interleaved(data, 2, 48000).unwrap();
        let mut meter = LevelMeter::new(2, 48000.0);
        meter.process(&buf);
        assert!((meter.peak[0] - 0.6).abs() < 0.001);
        assert!((meter.peak[1] - 0.3).abs() < 0.001);
        assert!(meter.peak_db(0) > meter.peak_db(1));
    }

    #[test]
    fn level_meter_invalid_channel_db() {
        let meter = LevelMeter::new(1, 48000.0);
        assert!(meter.peak_db(5) < -100.0);
        assert!(meter.rms_db(5) < -100.0);
        assert!(meter.peak_hold_db(5) < -100.0);
    }

    #[test]
    fn level_meter_public_accessors() {
        let buf =
            crate::buffer::AudioBuffer::from_interleaved(vec![0.5f32; 512], 2, 48000).unwrap();
        let mut meter = LevelMeter::new(2, 48000.0);
        meter.process(&buf);
        assert_eq!(meter.peak().len(), 2);
        assert_eq!(meter.rms().len(), 2);
        assert!(meter.peak()[0] > 0.0);
        assert!(meter.rms()[0] > 0.0);
        assert_eq!(meter.channels(), 2);
        let _ = meter.lufs();
        assert_eq!(meter.peak_hold().len(), 2);
    }
}
