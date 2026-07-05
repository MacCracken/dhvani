//! MIDI 1.0 ↔ 2.0 translation — velocity, CC, pitch bend conversion.

use super::v2::{ControlChangeV2, NoteOnV2};
use super::{ControlChange, FramePos, NoteEvent};

/// Pitch bend center value in MIDI 2.0 (32-bit).
pub const PITCH_BEND_CENTER: u32 = 0x80000000;

/// Scale 7-bit MIDI 1.0 velocity to 16-bit MIDI 2.0.
///
/// Uses the MIDI 2.0 spec scaling: `v << 9 | v << 2 | v >> 5`.
#[must_use]
#[inline]
pub fn velocity_7_to_16(v: u8) -> u16 {
    if v == 0 {
        return 0;
    }
    let v = v as u16;
    (v << 9) | (v << 2) | (v >> 5)
}

/// Scale 16-bit MIDI 2.0 velocity to 7-bit MIDI 1.0 (lossy).
#[must_use]
#[inline]
pub fn velocity_16_to_7(v: u16) -> u8 {
    (v >> 9) as u8
}

/// Scale 7-bit CC value to 32-bit MIDI 2.0.
#[must_use]
#[inline]
pub fn cc_7_to_32(v: u8) -> u32 {
    let v = v as u32;
    (v << 25) | (v << 18) | (v << 11) | (v << 4) | (v >> 3)
}

/// Scale 32-bit CC value to 7-bit MIDI 1.0 (lossy).
#[must_use]
#[inline]
pub fn cc_32_to_7(v: u32) -> u8 {
    (v >> 25) as u8
}

/// Convert 14-bit MIDI 1.0 pitch bend to 32-bit MIDI 2.0.
#[must_use]
#[inline]
pub fn pitch_bend_14_to_32(v: u16) -> u32 {
    let v = v as u32;
    (v << 18) | (v << 4) | (v >> 10)
}

/// Convert 32-bit MIDI 2.0 pitch bend to 14-bit MIDI 1.0 (lossy).
#[must_use]
#[inline]
pub fn pitch_bend_32_to_14(v: u32) -> u16 {
    (v >> 18) as u16
}

/// Convert a MIDI 1.0 NoteEvent to a MIDI 2.0 NoteOnV2.
#[must_use]
#[inline]
pub fn note_event_to_v2(event: &NoteEvent) -> NoteOnV2 {
    NoteOnV2 {
        position: event.position,
        note: event.note,
        velocity: velocity_7_to_16(event.velocity),
        channel: event.channel,
        attribute_type: 0,
        attribute_data: 0,
    }
}

/// Convert a MIDI 2.0 NoteOnV2 back to a MIDI 1.0 NoteEvent (lossy).
///
/// Requires a duration since NoteOnV2 doesn't carry duration.
#[must_use]
#[inline]
pub fn note_on_v2_to_event(event: &NoteOnV2, duration: FramePos) -> NoteEvent {
    NoteEvent {
        position: event.position,
        duration,
        note: event.note,
        velocity: velocity_16_to_7(event.velocity),
        channel: event.channel,
    }
}

/// Convert a MIDI 1.0 ControlChange to MIDI 2.0 ControlChangeV2.
#[must_use]
#[inline]
pub fn cc_to_v2(cc: &ControlChange) -> ControlChangeV2 {
    ControlChangeV2 {
        position: cc.position,
        controller: cc.controller,
        value: cc_7_to_32(cc.value),
        channel: cc.channel,
    }
}

/// Convert a MIDI 2.0 ControlChangeV2 to MIDI 1.0 ControlChange (lossy).
#[must_use]
#[inline]
pub fn cc_v2_to_cc(cc: &ControlChangeV2) -> ControlChange {
    ControlChange {
        position: cc.position,
        controller: cc.controller,
        value: cc_32_to_7(cc.value),
        channel: cc.channel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_7_to_16_zero() {
        assert_eq!(velocity_7_to_16(0), 0);
    }

    #[test]
    fn velocity_7_to_16_max() {
        let v16 = velocity_7_to_16(127);
        assert!(v16 > 65000, "max velocity should be near 65535, got {v16}");
    }

    #[test]
    fn velocity_roundtrip() {
        for v in 0..=127u8 {
            let v16 = velocity_7_to_16(v);
            let back = velocity_16_to_7(v16);
            assert_eq!(back, v, "roundtrip failed for velocity {v}");
        }
    }

    #[test]
    fn cc_roundtrip() {
        for v in 0..=127u8 {
            let v32 = cc_7_to_32(v);
            let back = cc_32_to_7(v32);
            assert_eq!(back, v, "roundtrip failed for CC value {v}");
        }
    }

    #[test]
    fn pitch_bend_roundtrip() {
        for v in [0u16, 1, 8192, 16383] {
            let v32 = pitch_bend_14_to_32(v);
            let back = pitch_bend_32_to_14(v32);
            assert_eq!(back, v, "roundtrip failed for pitch bend {v}");
        }
    }

    #[test]
    fn pitch_bend_center() {
        let center_14 = 8192u16;
        let v32 = pitch_bend_14_to_32(center_14);
        let back = pitch_bend_32_to_14(v32);
        assert_eq!(back, center_14);
    }

    #[test]
    fn note_event_to_v2_and_back() {
        let event = NoteEvent {
            position: 1000,
            duration: 500,
            note: 60,
            velocity: 100,
            channel: 0,
        };
        let v2 = note_event_to_v2(&event);
        assert_eq!(v2.note, 60);
        assert_eq!(v2.channel, 0);

        let back = note_on_v2_to_event(&v2, 500);
        assert_eq!(back.note, 60);
        assert_eq!(back.velocity, 100);
        assert_eq!(back.duration, 500);
    }

    #[test]
    fn cc_to_v2_and_back() {
        let cc = ControlChange {
            position: 0,
            controller: 7,
            value: 100,
            channel: 0,
        };
        let v2 = cc_to_v2(&cc);
        assert_eq!(v2.controller, 7);

        let back = cc_v2_to_cc(&v2);
        assert_eq!(back.controller, 7);
        assert_eq!(back.value, 100);
    }

    #[test]
    fn cc_7_to_32_max() {
        let v32 = cc_7_to_32(127);
        assert!(
            v32 > 4_000_000_000,
            "max CC should be near u32::MAX, got {v32}"
        );
    }

    #[test]
    fn pitch_bend_center_constant() {
        assert_eq!(PITCH_BEND_CENTER, 0x80000000);
    }
}
