//! Voice management — polyphonic voice pool with allocation and stealing.

use serde::{Deserialize, Serialize};

/// A4 reference frequency in Hz.
pub const A4_FREQUENCY: f64 = 440.0;
/// A4 MIDI note number.
pub const A4_MIDI_NOTE: f64 = 69.0;
/// Semitones per octave.
pub const SEMITONES_PER_OCTAVE: f64 = 12.0;

/// Voice state in the synthesis lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceState {
    /// Voice is not producing sound.
    Idle,
    /// Voice is actively playing.
    Active,
    /// Voice is in release phase (envelope releasing).
    Releasing,
}

/// Voice stealing policy when all voices are in use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VoiceStealMode {
    /// Steal the oldest voice (longest active).
    Oldest,
    /// Steal the quietest voice (lowest envelope level).
    Quietest,
    /// Steal the lowest-pitched voice.
    Lowest,
    /// Don't steal — ignore new notes when full.
    None,
}

/// A single synthesizer voice.
///
/// Nada provides voice management (allocation, stealing, state tracking).
/// Consumers own synthesis state (oscillator phases, etc.) indexed by voice slot.
#[derive(Debug, Clone)]
pub struct Voice {
    pub state: VoiceState,
    pub note: u8,
    pub velocity: u8,
    pub channel: u8,
    /// Amplitude envelope level (0.0–1.0), managed by consumer.
    pub envelope_level: f32,
    /// Age in process blocks (incremented by `VoiceManager::tick_age`).
    pub age: u64,
    /// Per-note pitch bend (-1.0 to +1.0, 0.0 = no bend).
    pub pitch_bend: f32,
    /// Per-note pressure / aftertouch (0.0 to 1.0).
    pub pressure: f32,
    /// Per-note brightness (CC#74, 0.0 to 1.0).
    pub brightness: f32,
}

impl Voice {
    /// Create a new idle voice.
    pub fn new() -> Self {
        Self {
            state: VoiceState::Idle,
            note: 0,
            velocity: 0,
            channel: 0,
            envelope_level: 0.0,
            age: 0,
            pitch_bend: 0.0,
            pressure: 0.0,
            brightness: 0.0,
        }
    }

    /// Whether this voice is idle.
    pub fn is_idle(&self) -> bool {
        self.state == VoiceState::Idle
    }

    /// Whether this voice is producing sound (Active or Releasing).
    pub fn is_active(&self) -> bool {
        matches!(self.state, VoiceState::Active | VoiceState::Releasing)
    }

    /// Frequency in Hz for this voice's note (12-TET, A4=440).
    pub fn frequency(&self) -> f64 {
        A4_FREQUENCY * 2.0f64.powf((self.note as f64 - A4_MIDI_NOTE) / SEMITONES_PER_OCTAVE)
    }

    /// Apply a per-note CC. Currently handles CC#74 (brightness).
    pub fn apply_per_note_cc(&mut self, controller: u8, value_normalized: f32) {
        if controller == 74 {
            self.brightness = value_normalized.clamp(0.0, 1.0);
        }
    }
}

impl Default for Voice {
    fn default() -> Self {
        Self::new()
    }
}

/// Polyphonic voice manager with configurable voice stealing.
#[derive(Debug)]
pub struct VoiceManager {
    voices: Vec<Voice>,
    max_voices: usize,
    steal_mode: VoiceStealMode,
}

impl VoiceManager {
    /// Create a voice pool with the given capacity and steal mode.
    pub fn new(max_voices: usize, steal_mode: VoiceStealMode) -> Self {
        Self {
            voices: (0..max_voices).map(|_| Voice::new()).collect(),
            max_voices,
            steal_mode,
        }
    }

    /// Allocate a voice for a new note. Returns the voice index, or None if
    /// no voice is available and stealing is disabled.
    pub fn note_on(&mut self, note: u8, velocity: u8, channel: u8) -> Option<usize> {
        // First: find an idle voice
        if let Some(idx) = self.voices.iter().position(|v| v.is_idle()) {
            self.activate_voice(idx, note, velocity, channel);
            return Some(idx);
        }

        // No idle voice — try stealing
        let steal_idx = match self.steal_mode {
            VoiceStealMode::Oldest => self
                .voices
                .iter()
                .enumerate()
                .max_by_key(|(_, v)| v.age)
                .map(|(i, _)| i),
            VoiceStealMode::Quietest => self
                .voices
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    a.envelope_level
                        .partial_cmp(&b.envelope_level)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i),
            VoiceStealMode::Lowest => self
                .voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.note)
                .map(|(i, _)| i),
            VoiceStealMode::None => None,
        };

        if let Some(idx) = steal_idx {
            self.activate_voice(idx, note, velocity, channel);
            Some(idx)
        } else {
            None
        }
    }

    /// Release a voice matching the given note and channel.
    pub fn note_off(&mut self, note: u8, channel: u8) {
        for voice in &mut self.voices {
            if voice.state == VoiceState::Active && voice.note == note && voice.channel == channel {
                voice.state = VoiceState::Releasing;
                return;
            }
        }
    }

    /// Mark a voice as idle (e.g., after envelope finishes).
    pub fn free_voice(&mut self, index: usize) {
        if let Some(voice) = self.voices.get_mut(index) {
            voice.state = VoiceState::Idle;
            voice.age = 0;
        }
    }

    /// Number of non-idle voices.
    pub fn active_count(&self) -> usize {
        self.voices.iter().filter(|v| !v.is_idle()).count()
    }

    /// Increment the age of all active voices. Call once per process block.
    pub fn tick_age(&mut self) {
        for voice in &mut self.voices {
            if voice.is_active() {
                voice.age = voice.age.saturating_add(1);
            }
        }
    }

    /// Reset all voices to idle.
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            *voice = Voice::new();
        }
    }

    /// Get a reference to a voice by index.
    pub fn voice(&self, index: usize) -> Option<&Voice> {
        self.voices.get(index)
    }

    /// Get a mutable reference to a voice by index.
    pub fn voice_mut(&mut self, index: usize) -> Option<&mut Voice> {
        self.voices.get_mut(index)
    }

    /// Number of voice slots.
    pub fn capacity(&self) -> usize {
        self.max_voices
    }

    /// Current steal mode.
    pub fn steal_mode(&self) -> VoiceStealMode {
        self.steal_mode
    }

    fn activate_voice(&mut self, idx: usize, note: u8, velocity: u8, channel: u8) {
        let voice = &mut self.voices[idx];
        voice.state = VoiceState::Active;
        voice.note = note;
        voice.velocity = velocity;
        voice.channel = channel;
        voice.envelope_level = 0.0;
        voice.age = 0;
        voice.pitch_bend = 0.0;
        voice.pressure = 0.0;
        voice.brightness = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_frequency() {
        let mut voice = Voice::new();
        voice.note = 69; // A4
        assert!((voice.frequency() - 440.0).abs() < 0.01);

        voice.note = 60; // C4
        assert!((voice.frequency() - 261.63).abs() < 0.1);

        voice.note = 0;
        assert!(voice.frequency() > 0.0);
        assert!(voice.frequency() < 10.0); // ~8.18 Hz
    }

    #[test]
    fn voice_state() {
        let voice = Voice::new();
        assert!(voice.is_idle());
        assert!(!voice.is_active());
    }

    #[test]
    fn voice_per_note_cc() {
        let mut voice = Voice::new();
        voice.apply_per_note_cc(74, 0.8);
        assert!((voice.brightness - 0.8).abs() < f32::EPSILON);

        // Non-74 CC should not affect brightness
        voice.apply_per_note_cc(1, 0.5);
        assert!((voice.brightness - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn voice_manager_basic() {
        let mut mgr = VoiceManager::new(4, VoiceStealMode::Oldest);
        assert_eq!(mgr.active_count(), 0);

        let idx = mgr.note_on(60, 100, 0);
        assert_eq!(idx, Some(0));
        assert_eq!(mgr.active_count(), 1);

        mgr.note_off(60, 0);
        assert_eq!(mgr.voice(0).unwrap().state, VoiceState::Releasing);

        mgr.free_voice(0);
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn voice_stealing_oldest() {
        let mut mgr = VoiceManager::new(2, VoiceStealMode::Oldest);
        mgr.note_on(60, 100, 0); // slot 0, age 0
        mgr.tick_age(); // slot 0 age = 1
        mgr.note_on(62, 100, 0); // slot 1, age 0
        mgr.tick_age(); // slot 0 age = 2, slot 1 age = 1

        // All voices full, should steal oldest (slot 0)
        let idx = mgr.note_on(64, 100, 0);
        assert_eq!(idx, Some(0));
        assert_eq!(mgr.voice(0).unwrap().note, 64);
    }

    #[test]
    fn voice_stealing_quietest() {
        let mut mgr = VoiceManager::new(2, VoiceStealMode::Quietest);
        mgr.note_on(60, 100, 0);
        mgr.voice_mut(0).unwrap().envelope_level = 0.8;
        mgr.note_on(62, 100, 0);
        mgr.voice_mut(1).unwrap().envelope_level = 0.2;

        // Should steal quietest (slot 1)
        let idx = mgr.note_on(64, 100, 0);
        assert_eq!(idx, Some(1));
    }

    #[test]
    fn voice_stealing_lowest() {
        let mut mgr = VoiceManager::new(2, VoiceStealMode::Lowest);
        mgr.note_on(72, 100, 0); // high note
        mgr.note_on(48, 100, 0); // low note

        // Should steal lowest (48)
        let idx = mgr.note_on(60, 100, 0);
        assert_eq!(idx, Some(1));
        assert_eq!(mgr.voice(1).unwrap().note, 60);
    }

    #[test]
    fn voice_stealing_none() {
        let mut mgr = VoiceManager::new(2, VoiceStealMode::None);
        mgr.note_on(60, 100, 0);
        mgr.note_on(62, 100, 0);

        let idx = mgr.note_on(64, 100, 0);
        assert_eq!(idx, None);
    }

    #[test]
    fn voice_manager_reset() {
        let mut mgr = VoiceManager::new(4, VoiceStealMode::Oldest);
        mgr.note_on(60, 100, 0);
        mgr.note_on(62, 90, 0);
        assert_eq!(mgr.active_count(), 2);

        mgr.reset();
        assert_eq!(mgr.active_count(), 0);
    }

    #[test]
    fn voice_reuse_after_free() {
        let mut mgr = VoiceManager::new(2, VoiceStealMode::None);
        let _idx0 = mgr.note_on(60, 100, 0).unwrap();
        let idx1 = mgr.note_on(62, 100, 0).unwrap();
        assert_eq!(mgr.note_on(64, 100, 0), None); // full

        mgr.free_voice(idx1);
        let idx2 = mgr.note_on(64, 100, 0);
        assert_eq!(idx2, Some(idx1)); // reuses freed slot
    }

    #[test]
    fn tick_age_only_active() {
        let mut mgr = VoiceManager::new(2, VoiceStealMode::Oldest);
        mgr.note_on(60, 100, 0);
        mgr.tick_age();
        mgr.tick_age();
        assert_eq!(mgr.voice(0).unwrap().age, 2);
        assert_eq!(mgr.voice(1).unwrap().age, 0); // idle, not ticked
    }
}
