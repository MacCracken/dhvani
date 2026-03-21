//! Ring-buffer recording — lock-free RT→accumulator pipeline.
//!
//! The RT thread pushes samples into a bounded channel. A background
//! accumulator thread drains samples into a Vec. When recording stops,
//! the accumulated samples are returned.

use std::sync::mpsc;
use std::thread;

use serde::{Deserialize, Serialize};

/// Maximum accumulator size (~500 MB of f32 samples).
const MAX_ACCUMULATOR_SAMPLES: usize = 125_000_000;

/// Recording mode for loop-aware recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum RecordingMode {
    /// Linear recording.
    Normal,
    /// Layer new audio on top of existing takes.
    Overdub,
    /// Overwrite existing audio per loop iteration.
    Replace,
}

/// Lock-free recording manager.
///
/// The RT thread pushes interleaved f32 samples via `push_samples()`.
/// A background thread accumulates them. Call `finish()` to get the recorded data.
pub struct RecordManager {
    sender: Option<mpsc::SyncSender<Vec<f32>>>,
    handle: Option<thread::JoinHandle<Vec<f32>>>,
    dropped_samples: u64,
}

impl RecordManager {
    /// Create a new record manager.
    ///
    /// `channel_bound` controls backpressure — how many chunks can queue before
    /// `push_samples` starts dropping.
    pub fn new(channel_bound: usize) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Vec<f32>>(channel_bound.max(1));

        let handle = thread::spawn(move || {
            let mut accumulated = Vec::new();
            while let Ok(chunk) = receiver.recv() {
                if accumulated.len() + chunk.len() <= MAX_ACCUMULATOR_SAMPLES {
                    accumulated.extend_from_slice(&chunk);
                }
            }
            accumulated
        });

        Self {
            sender: Some(sender),
            handle: Some(handle),
            dropped_samples: 0,
        }
    }

    /// Push interleaved f32 samples (call from RT thread).
    ///
    /// Non-blocking: if the accumulator falls behind, samples are dropped.
    pub fn push_samples(&mut self, data: &[f32]) {
        if let Some(sender) = &self.sender {
            match sender.try_send(data.to_vec()) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(_)) => {
                    self.dropped_samples += data.len() as u64;
                }
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    self.dropped_samples += data.len() as u64;
                }
            }
        }
    }

    /// Number of samples dropped due to backpressure.
    pub fn dropped_samples(&self) -> u64 {
        self.dropped_samples
    }

    /// Finish recording and return all accumulated samples.
    ///
    /// Blocks until the accumulator thread completes.
    pub fn finish(mut self) -> Vec<f32> {
        // Drop sender to signal accumulator thread
        self.sender.take();
        self.handle
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default()
    }
}

impl Drop for RecordManager {
    fn drop(&mut self) {
        // Drop sender to unblock accumulator
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Loop-aware recording manager — each loop iteration produces a separate take.
///
/// Call `push_loop_marker()` at each loop boundary. The accumulator splits
/// the recorded audio into separate takes at each marker.
pub struct LoopRecordManager {
    sender: Option<mpsc::SyncSender<RecordChunk>>,
    handle: Option<thread::JoinHandle<Vec<Vec<f32>>>>,
    mode: RecordingMode,
    dropped_samples: u64,
}

enum RecordChunk {
    Samples(Vec<f32>),
    LoopMarker,
}

impl LoopRecordManager {
    /// Create a loop record manager.
    pub fn new(channel_bound: usize, mode: RecordingMode) -> Self {
        let (sender, receiver) = mpsc::sync_channel::<RecordChunk>(channel_bound.max(1));

        let handle = thread::spawn(move || {
            let mut takes: Vec<Vec<f32>> = vec![Vec::new()];
            while let Ok(chunk) = receiver.recv() {
                match chunk {
                    RecordChunk::Samples(data) => {
                        if let Some(current) = takes.last_mut()
                            && current.len() + data.len() <= MAX_ACCUMULATOR_SAMPLES
                        {
                            current.extend_from_slice(&data);
                        }
                    }
                    RecordChunk::LoopMarker => {
                        takes.push(Vec::new());
                    }
                }
            }
            takes
        });

        Self {
            sender: Some(sender),
            handle: Some(handle),
            mode,
            dropped_samples: 0,
        }
    }

    /// Current recording mode.
    pub fn mode(&self) -> RecordingMode {
        self.mode
    }

    /// Push interleaved f32 samples (call from RT thread).
    pub fn push_samples(&mut self, data: &[f32]) {
        if let Some(sender) = &self.sender {
            match sender.try_send(RecordChunk::Samples(data.to_vec())) {
                Ok(()) => {}
                Err(mpsc::TrySendError::Full(_)) => {
                    self.dropped_samples += data.len() as u64;
                }
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    self.dropped_samples += data.len() as u64;
                }
            }
        }
    }

    /// Push a loop boundary marker.
    pub fn push_loop_marker(&mut self) {
        if let Some(sender) = &self.sender {
            let _ = sender.try_send(RecordChunk::LoopMarker);
        }
    }

    /// Number of samples dropped due to backpressure.
    pub fn dropped_samples(&self) -> u64 {
        self.dropped_samples
    }

    /// Finish recording and return all takes.
    ///
    /// Each inner Vec is one loop iteration's worth of samples.
    /// Empty takes are included (caller can filter them).
    pub fn finish(mut self) -> Vec<Vec<f32>> {
        self.sender.take();
        self.handle
            .take()
            .and_then(|h| h.join().ok())
            .unwrap_or_default()
    }
}

impl Drop for LoopRecordManager {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_finish() {
        let mut rec = RecordManager::new(64);
        rec.push_samples(&[0.1, 0.2, 0.3, 0.4]);
        rec.push_samples(&[0.5, 0.6]);
        let result = rec.finish();
        assert_eq!(result.len(), 6);
        assert!((result[0] - 0.1).abs() < f32::EPSILON);
        assert!((result[5] - 0.6).abs() < f32::EPSILON);
    }

    #[test]
    fn record_empty() {
        let rec = RecordManager::new(16);
        let result = rec.finish();
        assert!(result.is_empty());
    }

    #[test]
    fn loop_record_single_take() {
        let mut rec = LoopRecordManager::new(64, RecordingMode::Normal);
        rec.push_samples(&[1.0, 2.0, 3.0]);
        let takes = rec.finish();
        assert_eq!(takes.len(), 1);
        assert_eq!(takes[0].len(), 3);
    }

    #[test]
    fn loop_record_multiple_takes() {
        let mut rec = LoopRecordManager::new(64, RecordingMode::Normal);
        rec.push_samples(&[1.0, 2.0]);
        rec.push_loop_marker();
        rec.push_samples(&[3.0, 4.0, 5.0]);
        rec.push_loop_marker();
        rec.push_samples(&[6.0]);
        let takes = rec.finish();
        assert_eq!(takes.len(), 3);
        assert_eq!(takes[0].len(), 2);
        assert_eq!(takes[1].len(), 3);
        assert_eq!(takes[2].len(), 1);
    }

    #[test]
    fn loop_record_mode() {
        let rec = LoopRecordManager::new(16, RecordingMode::Overdub);
        assert_eq!(rec.mode(), RecordingMode::Overdub);
    }

    #[test]
    fn dropped_samples_initially_zero() {
        let rec = RecordManager::new(16);
        assert_eq!(rec.dropped_samples(), 0);
    }

    #[test]
    fn record_drop_without_finish() {
        // Should not hang or panic when dropped without calling finish()
        let mut rec = RecordManager::new(16);
        rec.push_samples(&[1.0, 2.0, 3.0]);
        drop(rec); // implicit Drop
    }

    #[test]
    fn loop_record_drop_without_finish() {
        let mut rec = LoopRecordManager::new(16, RecordingMode::Normal);
        rec.push_samples(&[1.0, 2.0]);
        rec.push_loop_marker();
        rec.push_samples(&[3.0]);
        drop(rec);
    }

    #[test]
    fn loop_record_empty_takes() {
        let mut rec = LoopRecordManager::new(64, RecordingMode::Normal);
        rec.push_loop_marker(); // empty first take
        rec.push_loop_marker(); // empty second take
        rec.push_samples(&[1.0]);
        let takes = rec.finish();
        // 3 takes: empty, empty, [1.0]
        assert_eq!(takes.len(), 3);
        assert!(takes[0].is_empty());
        assert!(takes[1].is_empty());
        assert_eq!(takes[2].len(), 1);
    }

    #[test]
    fn loop_record_replace_mode() {
        let rec = LoopRecordManager::new(16, RecordingMode::Replace);
        assert_eq!(rec.mode(), RecordingMode::Replace);
    }

    #[test]
    fn loop_record_dropped_samples() {
        let rec = LoopRecordManager::new(16, RecordingMode::Normal);
        assert_eq!(rec.dropped_samples(), 0);
    }

    #[test]
    fn large_recording() {
        let mut rec = RecordManager::new(128);
        for _ in 0..100 {
            rec.push_samples(&[0.5; 1024]);
        }
        let result = rec.finish();
        assert_eq!(result.len(), 100 * 1024);
    }
}
