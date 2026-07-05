//! MIDI routing — channel/note filtering, velocity curves, CC mapping.

use serde::{Deserialize, Serialize};

use super::NoteEvent;

/// Velocity transformation curve.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VelocityCurve {
    /// Linear passthrough (identity).
    Linear,
    /// Gentler response — compresses high velocities (sqrt curve).
    Soft,
    /// More aggressive response — expands high velocities (square curve).
    Hard,
    /// Always emit a fixed velocity regardless of input.
    Fixed(u8),
}

impl VelocityCurve {
    /// Apply the velocity curve to a raw MIDI velocity value.
    #[must_use]
    #[inline]
    pub fn apply(&self, velocity: u8) -> u8 {
        match self {
            Self::Linear => velocity,
            Self::Soft => {
                let normalized = velocity as f32 / 127.0;
                (normalized.sqrt() * 127.0) as u8
            }
            Self::Hard => {
                let normalized = velocity as f32 / 127.0;
                (normalized * normalized * 127.0) as u8
            }
            Self::Fixed(v) => *v,
        }
    }
}

/// A MIDI routing rule — filters and transforms events.
///
/// Generic (not tied to track UUIDs) so it works outside DAW context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MidiRoute {
    /// If set, only events on this MIDI channel (0–15) pass through.
    channel_filter: Option<u8>,
    /// Velocity transformation.
    velocity_curve: VelocityCurve,
    /// Inclusive note range (min, max). Notes outside are filtered out.
    note_range: (u8, u8),
}

impl MidiRoute {
    /// Create a new route with the given settings.
    ///
    /// `note_range` is clamped so that min <= max and both are in 0–127.
    pub fn new(
        channel_filter: Option<u8>,
        velocity_curve: VelocityCurve,
        note_range: (u8, u8),
    ) -> Self {
        let min = note_range.0.min(127);
        let max = note_range.1.min(127).max(min);
        Self {
            channel_filter: channel_filter.map(|ch| ch.min(15)),
            velocity_curve,
            note_range: (min, max),
        }
    }

    /// Create a route that passes all events unmodified.
    pub fn passthrough() -> Self {
        Self {
            channel_filter: None,
            velocity_curve: VelocityCurve::Linear,
            note_range: (0, 127),
        }
    }

    /// Channel filter (None = all channels).
    pub fn channel_filter(&self) -> Option<u8> {
        self.channel_filter
    }
    /// Velocity transformation curve.
    pub fn velocity_curve(&self) -> &VelocityCurve {
        &self.velocity_curve
    }
    /// Inclusive note range (min, max).
    pub fn note_range(&self) -> (u8, u8) {
        self.note_range
    }

    /// Set the channel filter. Channel is clamped to 0–15.
    pub fn set_channel_filter(&mut self, channel: Option<u8>) {
        self.channel_filter = channel.map(|ch| ch.min(15));
    }

    /// Set the velocity curve.
    pub fn set_velocity_curve(&mut self, curve: VelocityCurve) {
        self.velocity_curve = curve;
    }

    /// Set the note range. Values are clamped to 0–127 and min <= max.
    pub fn set_note_range(&mut self, min: u8, max: u8) {
        let min = min.min(127);
        let max = max.min(127).max(min);
        self.note_range = (min, max);
    }

    /// Filter and transform a NoteEvent. Returns None if the event is rejected.
    pub fn filter_event(&self, event: &NoteEvent) -> Option<NoteEvent> {
        // Channel filter
        if let Some(ch) = self.channel_filter
            && event.channel != ch
        {
            return None;
        }

        // Note range filter
        if event.note < self.note_range.0 || event.note > self.note_range.1 {
            return None;
        }

        // Apply velocity curve
        let velocity = self.velocity_curve.apply(event.velocity);

        Some(NoteEvent {
            velocity,
            ..event.clone()
        })
    }
}

impl Default for MidiRoute {
    fn default() -> Self {
        Self::passthrough()
    }
}

/// Maps a MIDI CC number to a parameter value range.
#[non_exhaustive]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CcMapping {
    /// MIDI CC number (0–127).
    pub cc: u8,
    /// Target parameter index.
    pub param_index: usize,
    /// Minimum parameter value (maps from CC=0).
    pub min_value: f32,
    /// Maximum parameter value (maps from CC=127).
    pub max_value: f32,
}

impl CcMapping {
    /// Create a new CC mapping.
    pub fn new(cc: u8, param_index: usize, min_value: f32, max_value: f32) -> Self {
        Self {
            cc,
            param_index,
            min_value,
            max_value,
        }
    }

    /// Map a 7-bit CC value (0–127) to the parameter range.
    #[must_use]
    pub fn map_value(&self, cc_value: u8) -> f32 {
        let normalized = cc_value as f32 / 127.0;
        self.min_value + normalized * (self.max_value - self.min_value)
    }

    /// Map a 32-bit CC value (MIDI 2.0) to the parameter range.
    #[must_use]
    pub fn map_value_32(&self, cc_value: u32) -> f32 {
        let normalized = cc_value as f64 / u32::MAX as f64;
        self.min_value + normalized as f32 * (self.max_value - self.min_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_curve_linear() {
        assert_eq!(VelocityCurve::Linear.apply(0), 0);
        assert_eq!(VelocityCurve::Linear.apply(64), 64);
        assert_eq!(VelocityCurve::Linear.apply(127), 127);
    }

    #[test]
    fn velocity_curve_soft() {
        // Soft (sqrt) should map mid-range higher than linear
        let soft_64 = VelocityCurve::Soft.apply(64);
        assert!(
            soft_64 > 64,
            "Soft curve at 64 should be > 64, got {soft_64}"
        );
        assert_eq!(VelocityCurve::Soft.apply(0), 0);
        assert_eq!(VelocityCurve::Soft.apply(127), 127);
    }

    #[test]
    fn velocity_curve_hard() {
        // Hard (square) should map mid-range lower than linear
        let hard_64 = VelocityCurve::Hard.apply(64);
        assert!(
            hard_64 < 64,
            "Hard curve at 64 should be < 64, got {hard_64}"
        );
        assert_eq!(VelocityCurve::Hard.apply(0), 0);
        assert_eq!(VelocityCurve::Hard.apply(127), 127);
    }

    #[test]
    fn velocity_curve_fixed() {
        assert_eq!(VelocityCurve::Fixed(100).apply(0), 100);
        assert_eq!(VelocityCurve::Fixed(100).apply(127), 100);
    }

    #[test]
    fn route_passthrough() {
        let route = MidiRoute::passthrough();
        let event = NoteEvent {
            position: 0,
            duration: 100,
            note: 60,
            velocity: 100,
            channel: 0,
        };
        let result = route.filter_event(&event).unwrap();
        assert_eq!(result.velocity, 100);
        assert_eq!(result.note, 60);
    }

    #[test]
    fn route_channel_filter() {
        let route = MidiRoute::new(Some(5), VelocityCurve::Linear, (0, 127));
        let event_ch5 = NoteEvent {
            position: 0,
            duration: 100,
            note: 60,
            velocity: 100,
            channel: 5,
        };
        let event_ch0 = NoteEvent {
            channel: 0,
            ..event_ch5.clone()
        };

        assert!(route.filter_event(&event_ch5).is_some());
        assert!(route.filter_event(&event_ch0).is_none());
    }

    #[test]
    fn route_note_range() {
        let route = MidiRoute::new(None, VelocityCurve::Linear, (36, 96));
        let event_in = NoteEvent {
            position: 0,
            duration: 100,
            note: 60,
            velocity: 100,
            channel: 0,
        };
        let event_below = NoteEvent {
            note: 20,
            ..event_in.clone()
        };
        let event_above = NoteEvent {
            note: 120,
            ..event_in.clone()
        };

        assert!(route.filter_event(&event_in).is_some());
        assert!(route.filter_event(&event_below).is_none());
        assert!(route.filter_event(&event_above).is_none());
    }

    #[test]
    fn route_velocity_curve_applied() {
        let route = MidiRoute::new(None, VelocityCurve::Fixed(80), (0, 127));
        let event = NoteEvent {
            position: 0,
            duration: 100,
            note: 60,
            velocity: 127,
            channel: 0,
        };
        let result = route.filter_event(&event).unwrap();
        assert_eq!(result.velocity, 80);
    }

    #[test]
    fn cc_mapping_7bit() {
        let mapping = CcMapping::new(1, 0, 0.0, 1.0);
        assert!((mapping.map_value(0) - 0.0).abs() < f32::EPSILON);
        assert!((mapping.map_value(127) - 1.0).abs() < 0.01);
        assert!((mapping.map_value(64) - 0.504).abs() < 0.01);
    }

    #[test]
    fn cc_mapping_32bit() {
        let mapping = CcMapping::new(1, 0, -1.0, 1.0);
        assert!((mapping.map_value_32(0) - (-1.0)).abs() < 0.01);
        assert!((mapping.map_value_32(u32::MAX) - 1.0).abs() < 0.01);
    }

    #[test]
    fn cc_mapping_custom_range() {
        let mapping = CcMapping::new(7, 0, 20.0, 20000.0);
        assert!((mapping.map_value(0) - 20.0).abs() < 0.01);
        assert!((mapping.map_value(127) - 20000.0).abs() < 1.0);
    }
}
