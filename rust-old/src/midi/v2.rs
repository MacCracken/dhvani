//! MIDI 2.0 / UMP types — 16-bit velocity, 32-bit CC, per-note expression.

use serde::{Deserialize, Serialize};

use super::FramePos;

/// MIDI 2.0 Note On with 16-bit velocity and per-note attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteOnV2 {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// MIDI note number (0–127).
    pub note: u8,
    /// 16-bit velocity (0–65535). 0 = note-off per MIDI 2.0 spec.
    pub velocity: u16,
    /// MIDI channel (0–15).
    pub channel: u8,
    /// Per-note attribute type (0=none, 1=manufacturer, 2=profile, 3=pitch 7.9).
    pub attribute_type: u8,
    /// Per-note attribute data.
    pub attribute_data: u16,
}

/// MIDI 2.0 Note Off with 16-bit velocity and per-note attributes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteOffV2 {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// MIDI note number (0–127).
    pub note: u8,
    /// 16-bit release velocity (0–65535).
    pub velocity: u16,
    /// MIDI channel (0–15).
    pub channel: u8,
    /// Per-note attribute type (0=none, 1=manufacturer, 2=profile, 3=pitch 7.9).
    pub attribute_type: u8,
    /// Per-note attribute data.
    pub attribute_data: u16,
}

/// MIDI 2.0 Control Change with 32-bit value (full resolution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlChangeV2 {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// CC index (0–255).
    pub controller: u8,
    /// 32-bit value (full resolution).
    pub value: u32,
    /// MIDI channel (0–15).
    pub channel: u8,
}

/// Per-note pitch bend (MIDI 2.0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerNotePitchBend {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// MIDI note number (0–127).
    pub note: u8,
    /// MIDI channel (0–15).
    pub channel: u8,
    /// 32-bit pitch bend value. 0x80000000 = center (no bend).
    pub value: u32,
}

/// Per-note controller (MIDI 2.0, enables MPE).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PerNoteController {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// MIDI note number (0–127).
    pub note: u8,
    /// MIDI channel (0–15).
    pub channel: u8,
    /// Controller index.
    pub controller: u8,
    /// 32-bit value.
    pub value: u32,
}

/// Channel pressure (MIDI 2.0, 32-bit resolution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelPressureV2 {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// MIDI channel (0–15).
    pub channel: u8,
    /// 32-bit pressure value.
    pub value: u32,
}

/// Polyphonic key pressure (MIDI 2.0, 32-bit resolution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolyPressureV2 {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// MIDI note number (0–127).
    pub note: u8,
    /// MIDI channel (0–15).
    pub channel: u8,
    /// 32-bit pressure value.
    pub value: u32,
}

/// Pitch bend (MIDI 2.0, 32-bit resolution).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PitchBendV2 {
    /// Frame position within the buffer.
    pub position: FramePos,
    /// MIDI channel (0–15).
    pub channel: u8,
    /// 32-bit pitch bend value. 0x80000000 = center (no bend).
    pub value: u32,
}

/// UMP (Universal MIDI Packet) message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum UmpMessageType {
    /// Utility messages (JR Timestamp, JR Clock).
    Utility,
    /// System Real-Time / System Common (MIDI 1.0 compatible).
    SystemCommon,
    /// MIDI 1.0 Channel Voice (7-bit note on/off, CC, etc.).
    Midi1ChannelVoice,
    /// Data messages (SysEx 7-bit).
    Data64,
    /// MIDI 2.0 Channel Voice (32-bit note on/off, CC, etc.).
    Midi2ChannelVoice,
    /// Data messages (SysEx 8-bit).
    Data128,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_on_v2_creation() {
        let note = NoteOnV2 {
            position: 0,
            note: 60,
            velocity: 32768,
            channel: 0,
            attribute_type: 0,
            attribute_data: 0,
        };
        assert_eq!(note.velocity, 32768);
    }

    #[test]
    fn control_change_v2_full_range() {
        let cc = ControlChangeV2 {
            position: 0,
            controller: 1,
            value: u32::MAX,
            channel: 0,
        };
        assert_eq!(cc.value, u32::MAX);
    }

    #[test]
    fn per_note_pitch_bend_center() {
        let pb = PerNotePitchBend {
            position: 0,
            note: 60,
            channel: 0,
            value: 0x80000000,
        };
        assert_eq!(pb.value, 0x80000000);
    }

    #[test]
    fn ump_message_types() {
        let mt = UmpMessageType::Midi2ChannelVoice;
        assert_eq!(mt, UmpMessageType::Midi2ChannelVoice);
    }

    #[test]
    fn serde_roundtrip() {
        let note = NoteOnV2 {
            position: 1000,
            note: 69,
            velocity: 65535,
            channel: 9,
            attribute_type: 3,
            attribute_data: 512,
        };
        let json = serde_json::to_string(&note).unwrap();
        let back: NoteOnV2 = serde_json::from_str(&json).unwrap();
        assert_eq!(note, back);
    }
}
