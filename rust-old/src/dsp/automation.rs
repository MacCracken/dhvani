//! Sample-accurate automation curves — linear, exponential, and smooth interpolation
//! between timestamped breakpoints.
//!
//! Provides per-sample parameter values for glitch-free, frame-accurate automation
//! of any DSP parameter (gain, filter cutoff, pan, etc.).

use serde::{Deserialize, Serialize};

/// Interpolation mode between breakpoints.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CurveType {
    /// Instant jump to new value (sample-and-hold).
    Step,
    /// Straight line between values.
    Linear,
    /// Exponential curve — fast attack or slow decay feel.
    /// Exponent > 1.0 = slow start, fast end. < 1.0 = fast start, slow end.
    Exponential(f32),
    /// Smooth (cosine) interpolation — no discontinuities in first derivative.
    Smooth,
}

/// A single automation breakpoint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Breakpoint {
    /// Sample position (frame index).
    pub frame: usize,
    /// Target value at this position.
    pub value: f32,
    /// Interpolation curve arriving at this breakpoint.
    pub curve: CurveType,
}

impl Breakpoint {
    /// Create a new breakpoint.
    #[must_use]
    pub fn new(frame: usize, value: f32, curve: CurveType) -> Self {
        Self {
            frame,
            value,
            curve,
        }
    }
}

/// Sample-accurate automation lane.
///
/// Stores an ordered sequence of breakpoints and interpolates between them
/// to produce per-sample parameter values.
#[derive(Debug, Clone)]
pub struct AutomationLane {
    breakpoints: Vec<Breakpoint>,
    /// Default value when no breakpoints exist or before the first breakpoint.
    default_value: f32,
}

impl AutomationLane {
    /// Create an empty automation lane with a default value.
    #[must_use]
    pub fn new(default_value: f32) -> Self {
        Self {
            breakpoints: Vec::new(),
            default_value,
        }
    }

    /// Add a breakpoint, maintaining sorted order by frame.
    pub fn add(&mut self, bp: Breakpoint) {
        let pos = self
            .breakpoints
            .binary_search_by_key(&bp.frame, |b| b.frame)
            .unwrap_or_else(|i| i);
        self.breakpoints.insert(pos, bp);
    }

    /// Remove all breakpoints at the given frame.
    pub fn remove_at(&mut self, frame: usize) {
        self.breakpoints.retain(|b| b.frame != frame);
    }

    /// Clear all breakpoints.
    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }

    /// Number of breakpoints.
    #[must_use]
    pub fn len(&self) -> usize {
        self.breakpoints.len()
    }

    /// Returns `true` if there are no breakpoints.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.breakpoints.is_empty()
    }

    /// Get the interpolated value at a specific frame.
    #[must_use]
    pub fn value_at(&self, frame: usize) -> f32 {
        if self.breakpoints.is_empty() {
            return self.default_value;
        }

        // Before first breakpoint
        if frame <= self.breakpoints[0].frame {
            return if self.breakpoints[0].curve == CurveType::Step {
                self.default_value
            } else {
                self.breakpoints[0].value
            };
        }

        // After last breakpoint
        if frame >= self.breakpoints[self.breakpoints.len() - 1].frame {
            return self.breakpoints[self.breakpoints.len() - 1].value;
        }

        // Find surrounding breakpoints via binary search
        let idx = self
            .breakpoints
            .binary_search_by_key(&frame, |b| b.frame)
            .unwrap_or_else(|i| i);

        if idx == 0 {
            return self.breakpoints[0].value;
        }

        let prev = &self.breakpoints[idx - 1];
        let next = &self.breakpoints[idx];

        interpolate(
            prev.value, next.value, prev.frame, next.frame, frame, next.curve,
        )
    }

    /// Render automation values for a range of frames into a buffer.
    ///
    /// Fills `output[0..frames]` with interpolated values.
    /// `start_frame` is the global frame offset of `output[0]`.
    pub fn render(&self, output: &mut [f32], start_frame: usize) {
        for (i, out) in output.iter_mut().enumerate() {
            *out = self.value_at(start_frame + i);
        }
    }

    /// Render with optimized segment walking (avoids binary search per sample).
    ///
    /// More efficient than `render()` for large buffers.
    pub fn render_fast(&self, output: &mut [f32], start_frame: usize) {
        if self.breakpoints.is_empty() {
            output.fill(self.default_value);
            return;
        }

        let end_frame = start_frame + output.len();

        // Find the first relevant breakpoint
        let mut seg_idx = self
            .breakpoints
            .binary_search_by_key(&start_frame, |b| b.frame)
            .unwrap_or_else(|i| i);

        for (i, out) in output.iter_mut().enumerate() {
            let frame = start_frame + i;

            // Advance segment if we've passed the current breakpoint
            while seg_idx < self.breakpoints.len() && self.breakpoints[seg_idx].frame <= frame {
                seg_idx += 1;
            }

            if seg_idx == 0 {
                // Before first breakpoint
                *out = if self.breakpoints[0].curve == CurveType::Step {
                    self.default_value
                } else {
                    self.breakpoints[0].value
                };
            } else if seg_idx >= self.breakpoints.len() {
                // After last breakpoint
                *out = self.breakpoints[self.breakpoints.len() - 1].value;
            } else {
                let prev = &self.breakpoints[seg_idx - 1];
                let next = &self.breakpoints[seg_idx];
                *out = interpolate(
                    prev.value, next.value, prev.frame, next.frame, frame, next.curve,
                );
            }
        }

        let _ = end_frame; // used for bounds reasoning
    }

    /// Access breakpoints slice.
    #[must_use]
    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }
}

/// Interpolate between two values.
#[inline]
fn interpolate(
    from: f32,
    to: f32,
    from_frame: usize,
    to_frame: usize,
    current_frame: usize,
    curve: CurveType,
) -> f32 {
    if from_frame == to_frame {
        return to;
    }

    let t = (current_frame - from_frame) as f32 / (to_frame - from_frame) as f32;
    let t = t.clamp(0.0, 1.0);

    match curve {
        CurveType::Step => from,
        CurveType::Linear => from + (to - from) * t,
        CurveType::Exponential(exp) => {
            let exp = exp.max(0.001);
            from + (to - from) * t.powf(exp)
        }
        CurveType::Smooth => {
            // Cosine interpolation: smooth start and end
            let t_smooth = (1.0 - (t * std::f32::consts::PI).cos()) * 0.5;
            from + (to - from) * t_smooth
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_lane_returns_default() {
        let lane = AutomationLane::new(0.5);
        assert_eq!(lane.value_at(0), 0.5);
        assert_eq!(lane.value_at(44100), 0.5);
    }

    #[test]
    fn single_breakpoint() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(1000, 1.0, CurveType::Linear));
        assert_eq!(lane.value_at(0), 1.0); // before first = first value (linear)
        assert_eq!(lane.value_at(1000), 1.0);
        assert_eq!(lane.value_at(2000), 1.0); // after last = last value
    }

    #[test]
    fn linear_interpolation() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(0, 0.0, CurveType::Linear));
        lane.add(Breakpoint::new(1000, 1.0, CurveType::Linear));

        assert!((lane.value_at(0) - 0.0).abs() < 1e-6);
        assert!((lane.value_at(500) - 0.5).abs() < 1e-6);
        assert!((lane.value_at(1000) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn step_interpolation() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(0, 0.0, CurveType::Step));
        lane.add(Breakpoint::new(1000, 1.0, CurveType::Step));

        assert_eq!(lane.value_at(0), 0.0);
        assert_eq!(lane.value_at(500), 0.0); // step holds previous value
        assert_eq!(lane.value_at(1000), 1.0);
    }

    #[test]
    fn exponential_curve() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(0, 0.0, CurveType::Linear));
        lane.add(Breakpoint::new(1000, 1.0, CurveType::Exponential(2.0)));

        let mid = lane.value_at(500);
        // With exponent 2.0, midpoint should be 0.25 (slow start)
        assert!((mid - 0.25).abs() < 0.01, "exp midpoint={mid}");
    }

    #[test]
    fn smooth_curve_midpoint() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(0, 0.0, CurveType::Linear));
        lane.add(Breakpoint::new(1000, 1.0, CurveType::Smooth));

        let mid = lane.value_at(500);
        // Cosine midpoint = 0.5
        assert!((mid - 0.5).abs() < 0.01, "smooth midpoint={mid}");
    }

    #[test]
    fn render_buffer() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(0, 0.0, CurveType::Linear));
        lane.add(Breakpoint::new(100, 1.0, CurveType::Linear));

        let mut output = vec![0.0f32; 101];
        lane.render(&mut output, 0);

        assert!((output[0] - 0.0).abs() < 1e-6);
        assert!((output[50] - 0.5).abs() < 1e-6);
        assert!((output[100] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn render_fast_matches_render() {
        let mut lane = AutomationLane::new(0.5);
        lane.add(Breakpoint::new(100, 0.0, CurveType::Linear));
        lane.add(Breakpoint::new(500, 1.0, CurveType::Smooth));
        lane.add(Breakpoint::new(900, 0.3, CurveType::Exponential(1.5)));

        let mut slow = vec![0.0f32; 1000];
        let mut fast = vec![0.0f32; 1000];
        lane.render(&mut slow, 0);
        lane.render_fast(&mut fast, 0);

        for (i, (s, f)) in slow.iter().zip(fast.iter()).enumerate() {
            assert!((s - f).abs() < 1e-6, "mismatch at {i}: slow={s} fast={f}");
        }
    }

    #[test]
    fn add_maintains_order() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(500, 0.5, CurveType::Linear));
        lane.add(Breakpoint::new(100, 0.1, CurveType::Linear));
        lane.add(Breakpoint::new(900, 0.9, CurveType::Linear));

        let frames: Vec<usize> = lane.breakpoints().iter().map(|b| b.frame).collect();
        assert_eq!(frames, vec![100, 500, 900]);
    }

    #[test]
    fn remove_at_frame() {
        let mut lane = AutomationLane::new(0.0);
        lane.add(Breakpoint::new(100, 1.0, CurveType::Linear));
        lane.add(Breakpoint::new(200, 2.0, CurveType::Linear));
        lane.remove_at(100);
        assert_eq!(lane.len(), 1);
        assert_eq!(lane.breakpoints()[0].frame, 200);
    }
}
