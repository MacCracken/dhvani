//! MIDI types — events, clips, MIDI 2.0, voice management, routing.
//!
//! Canonical MIDI crate for the AGNOS ecosystem. Consumers (shruti, jalwa,
//! hoosh, tarang) all use `dhvani::midi` rather than rolling their own.

pub mod routing;
pub mod translate;
pub mod v2;
pub mod voice;

use serde::{Deserialize, Serialize};

/// Frame position in samples (from start of timeline).
pub type FramePos = u64;

// ── MIDI 1.0 core types ────────────────────────────────────────────

/// A note event (note-on with duration).
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteEvent {
    /// Start position in frames.
    pub position: FramePos,
    /// Duration in frames.
    pub duration: FramePos,
    /// MIDI note number (0–127).
    pub note: u8,
    /// Velocity (0–127).
    pub velocity: u8,
    /// MIDI channel (0–15).
    pub channel: u8,
}

impl NoteEvent {
    /// Create a new note event.
    pub fn new(
        position: FramePos,
        duration: FramePos,
        note: u8,
        velocity: u8,
        channel: u8,
    ) -> Self {
        Self {
            position,
            duration,
            note,
            velocity,
            channel,
        }
    }
}

/// A control change event.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlChange {
    /// Position in frames.
    pub position: FramePos,
    /// CC number (0–127).
    pub controller: u8,
    /// CC value (0–127).
    pub value: u8,
    /// MIDI channel (0–15).
    pub channel: u8,
}

impl ControlChange {
    /// Create a new control change event.
    pub fn new(position: FramePos, controller: u8, value: u8, channel: u8) -> Self {
        Self {
            position,
            controller,
            value,
            channel,
        }
    }
}

/// Unified MIDI event type for pattern matching.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MidiEvent {
    /// A note-on event.
    NoteOn {
        /// Frame position within the buffer.
        position: FramePos,
        /// MIDI note number (0–127).
        note: u8,
        /// Note-on velocity (0–127).
        velocity: u8,
        /// MIDI channel (0–15).
        channel: u8,
    },
    /// A note-off event.
    NoteOff {
        /// Frame position within the buffer.
        position: FramePos,
        /// MIDI note number (0–127).
        note: u8,
        /// Release velocity (0–127).
        velocity: u8,
        /// MIDI channel (0–15).
        channel: u8,
    },
    /// A control-change (CC) event.
    ControlChange {
        /// Frame position within the buffer.
        position: FramePos,
        /// Controller number (0–127).
        controller: u8,
        /// Controller value (0–127).
        value: u8,
        /// MIDI channel (0–15).
        channel: u8,
    },
    /// A pitch-bend event.
    PitchBend {
        /// Frame position within the buffer.
        position: FramePos,
        /// 14-bit value (0–16383, center = 8192).
        value: u16,
        /// MIDI channel (0–15).
        channel: u8,
    },
    /// A channel aftertouch event.
    Aftertouch {
        /// Frame position within the buffer.
        position: FramePos,
        /// Channel pressure (0–127).
        pressure: u8,
        /// MIDI channel (0–15).
        channel: u8,
    },
    /// A polyphonic aftertouch event.
    PolyAftertouch {
        /// Frame position within the buffer.
        position: FramePos,
        /// MIDI note number (0–127).
        note: u8,
        /// Per-note pressure (0–127).
        pressure: u8,
        /// MIDI channel (0–15).
        channel: u8,
    },
    /// A program-change event.
    ProgramChange {
        /// Frame position within the buffer.
        position: FramePos,
        /// Program number (0–127).
        program: u8,
        /// MIDI channel (0–15).
        channel: u8,
    },
}

impl MidiEvent {
    /// Position in frames.
    pub fn position(&self) -> FramePos {
        match self {
            Self::NoteOn { position, .. }
            | Self::NoteOff { position, .. }
            | Self::ControlChange { position, .. }
            | Self::PitchBend { position, .. }
            | Self::Aftertouch { position, .. }
            | Self::PolyAftertouch { position, .. }
            | Self::ProgramChange { position, .. } => *position,
        }
    }

    /// MIDI channel (0–15).
    pub fn channel(&self) -> u8 {
        match self {
            Self::NoteOn { channel, .. }
            | Self::NoteOff { channel, .. }
            | Self::ControlChange { channel, .. }
            | Self::PitchBend { channel, .. }
            | Self::Aftertouch { channel, .. }
            | Self::PolyAftertouch { channel, .. }
            | Self::ProgramChange { channel, .. } => *channel,
        }
    }
}

// ── MIDI Clip ───────────────────────────────────────────────────────

/// A MIDI clip containing sorted note and CC events.
#[must_use]
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiClip {
    /// Clip name.
    pub name: String,
    /// Note events (sorted by position).
    pub notes: Vec<NoteEvent>,
    /// Control change events (sorted by position).
    pub control_changes: Vec<ControlChange>,
    /// Position on the timeline in frames.
    pub timeline_pos: FramePos,
    /// Clip duration in frames.
    pub duration: FramePos,
}

impl MidiClip {
    /// Create a new empty MIDI clip.
    pub fn new(name: impl Into<String>, timeline_pos: FramePos, duration: FramePos) -> Self {
        Self {
            name: name.into(),
            notes: Vec::new(),
            control_changes: Vec::new(),
            timeline_pos,
            duration,
        }
    }

    /// Add a note, maintaining sorted order by position.
    pub fn add_note(
        &mut self,
        position: FramePos,
        duration: FramePos,
        note: u8,
        velocity: u8,
        channel: u8,
    ) {
        let event = NoteEvent {
            position,
            duration,
            note,
            velocity,
            channel,
        };
        let idx = self.notes.partition_point(|n| n.position <= position);
        self.notes.insert(idx, event);
    }

    /// Add a CC event, maintaining sorted order by position.
    pub fn add_cc(&mut self, position: FramePos, controller: u8, value: u8, channel: u8) {
        let event = ControlChange {
            position,
            controller,
            value,
            channel,
        };
        let idx = self
            .control_changes
            .partition_point(|c| c.position <= position);
        self.control_changes.insert(idx, event);
    }

    /// End position (timeline_pos + duration).
    pub fn end_pos(&self) -> FramePos {
        self.timeline_pos + self.duration
    }

    /// Notes active at the given frame (note-on <= frame < note-on + duration).
    pub fn notes_at(&self, frame: FramePos) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| n.position <= frame && frame < n.position + n.duration)
            .collect()
    }

    /// Note-on events starting at exactly the given frame.
    pub fn note_ons_at(&self, frame: FramePos) -> Vec<&NoteEvent> {
        self.notes.iter().filter(|n| n.position == frame).collect()
    }

    /// Note-off events ending at exactly the given frame.
    pub fn note_offs_at(&self, frame: FramePos) -> Vec<&NoteEvent> {
        self.notes
            .iter()
            .filter(|n| n.position + n.duration == frame)
            .collect()
    }

    /// Get all note events in a frame range [start, end) using binary search.
    ///
    /// More efficient than scanning the entire list for large clips.
    pub fn events_in_range(&self, start: FramePos, end: FramePos) -> Vec<&NoteEvent> {
        // Binary search for first note that could be active in range
        // A note is relevant if position < end AND position + duration > start
        let first = self
            .notes
            .partition_point(|n| n.position + n.duration <= start);
        self.notes[first..]
            .iter()
            .take_while(|n| n.position < end)
            .filter(|n| n.position + n.duration > start)
            .collect()
    }

    /// Merge another clip's events into this one, maintaining sort order.
    pub fn merge(&mut self, other: &MidiClip) {
        for note in &other.notes {
            let idx = self.notes.partition_point(|n| n.position <= note.position);
            self.notes.insert(idx, note.clone());
        }
        for cc in &other.control_changes {
            let idx = self
                .control_changes
                .partition_point(|c| c.position <= cc.position);
            self.control_changes.insert(idx, cc.clone());
        }
        // Extend duration if needed
        if let Some(last) = other.notes.last() {
            let other_end = last.position + last.duration;
            if other_end > self.duration {
                self.duration = other_end;
            }
        }
    }

    /// Transpose all notes by the given number of semitones.
    ///
    /// Notes that would go out of 0–127 range are clamped.
    pub fn transpose(&mut self, semitones: i8) {
        for note in &mut self.notes {
            let new_note = note.note as i16 + semitones as i16;
            note.note = new_note.clamp(0, 127) as u8;
        }
    }

    /// Quantize all note positions to the nearest grid boundary.
    ///
    /// `grid_frames` is the grid size in frames (e.g., samples_per_beat / 4 for 16th notes).
    pub fn quantize(&mut self, grid_frames: FramePos) {
        if grid_frames == 0 {
            return;
        }
        for note in &mut self.notes {
            let half = grid_frames / 2;
            note.position = ((note.position + half) / grid_frames) * grid_frames;
        }
        for cc in &mut self.control_changes {
            let half = grid_frames / 2;
            cc.position = ((cc.position + half) / grid_frames) * grid_frames;
        }
    }

    /// Total number of events (notes + CCs).
    pub fn event_count(&self) -> usize {
        self.notes.len() + self.control_changes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_creation() {
        let clip = MidiClip::new("test", 0, 44100);
        assert_eq!(clip.name, "test");
        assert_eq!(clip.timeline_pos, 0);
        assert_eq!(clip.duration, 44100);
        assert!(clip.notes.is_empty());
    }

    #[test]
    fn add_note_sorted() {
        let mut clip = MidiClip::new("test", 0, 44100);
        clip.add_note(1000, 500, 60, 100, 0);
        clip.add_note(500, 500, 62, 90, 0);
        clip.add_note(2000, 500, 64, 80, 0);
        // Should be sorted by position
        assert_eq!(clip.notes[0].position, 500);
        assert_eq!(clip.notes[1].position, 1000);
        assert_eq!(clip.notes[2].position, 2000);
    }

    #[test]
    fn notes_at_frame() {
        let mut clip = MidiClip::new("test", 0, 44100);
        clip.add_note(100, 200, 60, 100, 0); // 100..300
        clip.add_note(250, 100, 62, 90, 0); // 250..350

        let at_150 = clip.notes_at(150);
        assert_eq!(at_150.len(), 1);
        assert_eq!(at_150[0].note, 60);

        let at_275 = clip.notes_at(275);
        assert_eq!(at_275.len(), 2);

        let at_400 = clip.notes_at(400);
        assert!(at_400.is_empty());
    }

    #[test]
    fn note_ons_and_offs() {
        let mut clip = MidiClip::new("test", 0, 44100);
        clip.add_note(100, 200, 60, 100, 0); // on@100, off@300
        clip.add_note(100, 300, 62, 90, 0); // on@100, off@400

        assert_eq!(clip.note_ons_at(100).len(), 2);
        assert_eq!(clip.note_ons_at(101).len(), 0);
        assert_eq!(clip.note_offs_at(300).len(), 1);
        assert_eq!(clip.note_offs_at(400).len(), 1);
    }

    #[test]
    fn events_in_range_binary_search() {
        let mut clip = MidiClip::new("test", 0, 100000);
        for i in 0..100 {
            clip.add_note(i * 1000, 500, 60, 100, 0);
        }
        let range = clip.events_in_range(10000, 15000);
        // Notes at 10000..10500, 11000..11500, ..., 14000..14500 = 5 notes
        assert_eq!(range.len(), 5);
        assert_eq!(range[0].position, 10000);
    }

    #[test]
    fn merge_clips() {
        let mut a = MidiClip::new("a", 0, 2000);
        a.add_note(0, 100, 60, 100, 0);
        a.add_note(500, 100, 62, 90, 0);

        let mut b = MidiClip::new("b", 0, 2000);
        b.add_note(250, 100, 64, 80, 0);

        a.merge(&b);
        assert_eq!(a.notes.len(), 3);
        assert_eq!(a.notes[0].position, 0);
        assert_eq!(a.notes[1].position, 250);
        assert_eq!(a.notes[2].position, 500);
    }

    #[test]
    fn transpose() {
        let mut clip = MidiClip::new("test", 0, 1000);
        clip.add_note(0, 100, 60, 100, 0);
        clip.add_note(100, 100, 3, 100, 0);
        clip.add_note(200, 100, 125, 100, 0);

        clip.transpose(5);
        assert_eq!(clip.notes[0].note, 65); // 60+5
        assert_eq!(clip.notes[1].note, 8); // 3+5
        assert_eq!(clip.notes[2].note, 127); // 125+5 clamped to 127

        clip.transpose(-20);
        assert_eq!(clip.notes[0].note, 45); // 65-20
        assert_eq!(clip.notes[1].note, 0); // 8-20 clamped to 0
        assert_eq!(clip.notes[2].note, 107); // 127-20
    }

    #[test]
    fn quantize_basic() {
        let mut clip = MidiClip::new("test", 0, 44100);
        clip.add_note(90, 100, 60, 100, 0); // 90+100=190, 190/200=0, 0*200=0
        clip.add_note(110, 100, 62, 90, 0); // 110+100=210, 210/200=1, 1*200=200
        clip.add_note(400, 100, 64, 80, 0); // 400+100=500, 500/200=2, 2*200=400

        clip.quantize(200);
        assert_eq!(clip.notes[0].position, 0);
        assert_eq!(clip.notes[1].position, 200);
        assert_eq!(clip.notes[2].position, 400);
    }

    #[test]
    fn quantize_zero_grid_noop() {
        let mut clip = MidiClip::new("test", 0, 44100);
        clip.add_note(105, 100, 60, 100, 0);
        clip.quantize(0);
        assert_eq!(clip.notes[0].position, 105);
    }

    #[test]
    fn add_cc_sorted() {
        let mut clip = MidiClip::new("test", 0, 44100);
        clip.add_cc(1000, 1, 64, 0);
        clip.add_cc(500, 7, 100, 0);
        assert_eq!(clip.control_changes[0].position, 500);
        assert_eq!(clip.control_changes[1].position, 1000);
    }

    #[test]
    fn midi_event_position_and_channel() {
        let event = MidiEvent::NoteOn {
            position: 1000,
            note: 60,
            velocity: 100,
            channel: 5,
        };
        assert_eq!(event.position(), 1000);
        assert_eq!(event.channel(), 5);
    }

    #[test]
    fn end_pos() {
        let clip = MidiClip::new("test", 1000, 5000);
        assert_eq!(clip.end_pos(), 6000);
    }
}
