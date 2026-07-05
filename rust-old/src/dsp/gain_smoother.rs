//! Smooth gain transitions via exponential moving average.
//!
//! Prevents "pumping" artifacts when applying per-buffer normalization or
//! dynamic gain changes. The smoother tracks a target gain and converges
//! toward it at different rates depending on direction:
//!
//! - **Attack** (gain decreasing): fast response to prevent clipping
//! - **Release** (gain increasing): slow response to avoid audible jumps
//!
//! # Example
//!
//! ```rust
//! use dhvani::dsp::GainSmoother;
//!
//! let mut smoother = GainSmoother::new(0.3, 0.05); // attack=0.3, release=0.05
//! let smoothed = smoother.smooth(0.5); // first call jumps toward 0.5
//! let smoothed = smoother.smooth(0.5); // converges further
//! ```

use serde::{Deserialize, Serialize};

/// Parameters for gain smoothing.
#[must_use]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct GainSmootherParams {
    /// EMA coefficient when gain is decreasing (0.0–1.0). Higher = faster.
    pub attack: f32,
    /// EMA coefficient when gain is increasing (0.0–1.0). Higher = faster.
    pub release: f32,
}

impl Default for GainSmootherParams {
    fn default() -> Self {
        Self {
            attack: 0.3,
            release: 0.05,
        }
    }
}

impl GainSmootherParams {
    /// Validate parameters. Returns an error description if invalid.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(0.0..=1.0).contains(&self.attack) {
            return Err("attack must be in 0.0..=1.0");
        }
        if !(0.0..=1.0).contains(&self.release) {
            return Err("release must be in 0.0..=1.0");
        }
        Ok(())
    }
}

/// Exponential moving average gain smoother.
///
/// Tracks a target gain value and smoothly converges toward it, using
/// separate attack (fast) and release (slow) coefficients to prevent
/// audible pumping in normalization and volume automation.
#[must_use]
#[derive(Debug, Clone)]
pub struct GainSmoother {
    params: GainSmootherParams,
    current: f32,
}

impl GainSmoother {
    /// Create a new gain smoother with the given attack and release coefficients.
    ///
    /// Both values should be in `0.0..=1.0`. Higher values mean faster convergence.
    /// Typical values: attack `0.3`, release `0.05`.
    pub fn new(attack: f32, release: f32) -> Self {
        tracing::debug!(attack, release, "GainSmoother::new");
        Self {
            params: GainSmootherParams {
                attack: attack.clamp(0.0, 1.0),
                release: release.clamp(0.0, 1.0),
            },
            current: 1.0,
        }
    }

    /// Create from parameters.
    pub fn from_params(params: GainSmootherParams) -> Self {
        Self {
            params,
            current: 1.0,
        }
    }

    /// Smooth a target gain value, returning the smoothed result.
    ///
    /// Call once per buffer with the desired gain. The smoother will
    /// converge toward the target at the configured rate.
    #[inline]
    pub fn smooth(&mut self, target: f32) -> f32 {
        let alpha = if target < self.current {
            self.params.attack
        } else {
            self.params.release
        };
        self.current += alpha * (target - self.current);
        self.current
    }

    /// Get the current smoothed gain value.
    pub fn current(&self) -> f32 {
        self.current
    }

    /// Reset to a specific gain value (e.g., 1.0 on track change).
    pub fn reset(&mut self, value: f32) {
        self.current = value;
    }

    /// Update the smoothing parameters.
    pub fn set_params(&mut self, params: GainSmootherParams) {
        self.params = params;
    }

    /// Get the current parameters.
    pub fn params(&self) -> &GainSmootherParams {
        &self.params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_toward_target() {
        let mut s = GainSmoother::new(0.3, 0.05);
        // Start at 1.0, smooth toward 0.5 (decreasing = attack)
        for _ in 0..50 {
            s.smooth(0.5);
        }
        assert!((s.current() - 0.5).abs() < 0.01);
    }

    #[test]
    fn attack_faster_than_release() {
        let mut s1 = GainSmoother::new(0.3, 0.05);
        let mut s2 = GainSmoother::new(0.3, 0.05);

        // s1: decrease from 1.0 to 0.5 (attack)
        let after_attack = s1.smooth(0.5);
        // s2: increase from 0.5 to 1.0 (release)
        s2.reset(0.5);
        let after_release = s2.smooth(1.0);

        // Attack should move further from start than release
        let attack_delta = (1.0 - after_attack).abs();
        let release_delta = (after_release - 0.5).abs();
        assert!(attack_delta > release_delta);
    }

    #[test]
    fn reset_sets_value() {
        let mut s = GainSmoother::new(0.3, 0.05);
        s.smooth(0.5);
        s.reset(1.0);
        assert_eq!(s.current(), 1.0);
    }

    #[test]
    fn identity_when_target_equals_current() {
        let mut s = GainSmoother::new(0.3, 0.05);
        let result = s.smooth(1.0);
        assert_eq!(result, 1.0);
    }

    #[test]
    fn params_default() {
        let p = GainSmootherParams::default();
        assert_eq!(p.attack, 0.3);
        assert_eq!(p.release, 0.05);
        assert!(p.validate().is_ok());
    }

    #[test]
    fn params_validate_rejects_out_of_range() {
        let p = GainSmootherParams {
            attack: 1.5,
            release: 0.05,
        };
        assert!(p.validate().is_err());
        let p = GainSmootherParams {
            attack: 0.3,
            release: -0.1,
        };
        assert!(p.validate().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let p = GainSmootherParams {
            attack: 0.2,
            release: 0.1,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: GainSmootherParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back.attack, p.attack);
        assert_eq!(back.release, p.release);
    }
}
