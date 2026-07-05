//! Environmental and nature sound synthesis — thunder, rain, wind, fire, impacts, ambience.
//!
//! Re-exports from [`garjan`](https://crates.io/crates/garjan), the AGNOS environmental sound crate.
//! Produces procedural, real-time environmental and nature sounds from physical models.
//!
//! # Feature: `environment`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["environment"] }
//! ```
//!
//! # Architecture
//!
//! ```text
//! Parameters (intensity, material, distance) → Synthesizer → AudioBuffer
//! ```

use crate::buffer::AudioBuffer;

// ── Weather ────────────────────────────────────────────────────────

/// Rain synthesis with configurable intensity.
pub use garjan::weather::{Rain, RainIntensity};

/// Thunder synthesis with distance-based filtering.
pub use garjan::weather::Thunder;

/// Wind synthesis with speed and gustiness.
pub use garjan::weather::Wind;

// ── Fire ───────────────────────────────────────────────────────────

/// Fire/combustion synthesis.
pub use garjan::fire::Fire;

// ── Water ──────────────────────────────────────────────────────────

/// Water synthesis (streams, rivers, drips).
pub use garjan::water::{Water, WaterType};

/// Surf/ocean wave synthesis.
pub use garjan::surf::{Surf, SurfIntensity};

/// Underwater ambient synthesis.
pub use garjan::underwater::{Underwater, UnderwaterDepth};

/// Bubble synthesis.
pub use garjan::bubble::Bubble;

// ── Impact & contact ───────────────────────────────────────────────

/// Impact synthesis (collisions, hits).
pub use garjan::impact::{Impact, ImpactType};

/// Friction synthesis (scraping, sliding).
pub use garjan::friction::Friction;

/// Rolling contact synthesis.
pub use garjan::rolling::Rolling;

/// Footstep synthesis.
pub use garjan::footstep::Footstep;

/// Creaking synthesis (wood, metal, hinges).
pub use garjan::creak::Creak;

// ── Ambient textures ───────────────────────────────────────────────

/// Ambient environmental textures.
pub use garjan::texture::{AmbientTexture, TextureType};

/// Foliage rustling synthesis.
pub use garjan::foliage::Foliage;

/// Insect sounds (crickets, cicadas, bees).
pub use garjan::insect::Insect;

// ── Aero ───────────────────────────────────────────────────────────

/// Cloth flapping synthesis.
pub use garjan::cloth::Cloth;

/// Whistle synthesis.
pub use garjan::whistle::Whistle;

/// Whoosh (air displacement) synthesis.
pub use garjan::whoosh::Whoosh;

/// Wing flap synthesis.
pub use garjan::wingflap::{BirdSize, WingFlap};

// ── Physical modeling ──────────────────────────────────────────────

/// Material properties for modal synthesis.
pub use garjan::material::Material;

/// Modal resonance bank for physically-modeled impacts.
pub use garjan::modal::{ExcitationType, Exciter, ModalBank, ModePattern, ModeSpec};

/// Level-of-detail quality control.
pub use garjan::lod::Quality;

// ── Contact types ──────────────────────────────────────────────────

pub use garjan::contact::{
    CreakSource, FoliageType, FrictionType, MovementType, RollingBody, Terrain,
};

// ── Error ──────────────────────────────────────────────────────────

/// Garjan error type.
pub use garjan::error::GarjanError;

// ── Bridge: garjan synthesis → dhvani AudioBuffer ──────────────────

/// Render an environmental sound to a dhvani [`AudioBuffer`].
///
/// Calls `synthesize(duration)` on any garjan type that supports it
/// and wraps the result in an `AudioBuffer`.
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if synthesis fails.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::environment::*;
///
/// let mut rain = Rain::new(RainIntensity::Heavy, 44100.0).unwrap();
/// let buf = render_environment(&mut rain, 44100, 2.0).unwrap();
/// ```
pub fn render_environment<S: EnvironmentSynth>(
    synth: &mut S,
    sample_rate: u32,
    duration: f32,
) -> crate::Result<AudioBuffer> {
    let samples = synth
        .synthesize(duration)
        .map_err(|e| crate::NadaError::Dsp(format!("environment synthesis failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .map_err(|e| crate::NadaError::Dsp(format!("buffer from environment output: {e}")))
}

/// Trait for garjan types that produce audio via `synthesize(duration)`.
///
/// Implemented for all garjan sound sources that have a `synthesize` method.
pub trait EnvironmentSynth {
    /// Synthesize audio for the given duration in seconds.
    ///
    /// # Errors
    ///
    /// Returns an error if synthesis fails.
    fn synthesize(&mut self, duration: f32) -> std::result::Result<Vec<f32>, GarjanError>;
}

macro_rules! impl_env_synth {
    ($($ty:ty),* $(,)?) => {
        $(
            impl EnvironmentSynth for $ty {
                #[inline]
                fn synthesize(&mut self, duration: f32) -> std::result::Result<Vec<f32>, GarjanError> {
                    self.synthesize(duration)
                }
            }
        )*
    };
}

impl_env_synth!(
    Rain,
    Thunder,
    Wind,
    Fire,
    Water,
    Surf,
    Underwater,
    Bubble,
    Friction,
    Rolling,
    Footstep,
    Creak,
    AmbientTexture,
    Foliage,
    Insect,
    Cloth,
    Whistle,
    Whoosh,
    WingFlap,
);

/// Render an impact sound to a dhvani [`AudioBuffer`].
///
/// Impact synthesis requires specifying the impact type per call.
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if synthesis fails.
pub fn render_impact(
    impact: &mut Impact,
    impact_type: ImpactType,
    sample_rate: u32,
) -> crate::Result<AudioBuffer> {
    let samples = impact
        .synthesize(impact_type)
        .map_err(|e| crate::NadaError::Dsp(format!("impact synthesis failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .map_err(|e| crate::NadaError::Dsp(format!("buffer from impact output: {e}")))
}

/// Render an environmental sound via `process_block` for streaming/real-time use.
///
/// Fills a buffer of `frames` length by calling `process_block` on the source.
pub fn render_block(
    synth: &mut impl EnvironmentBlock,
    frames: usize,
    sample_rate: u32,
) -> AudioBuffer {
    let mut samples = vec![0.0_f32; frames];
    synth.process_block(&mut samples);
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .unwrap_or_else(|_| AudioBuffer::silence(1, frames, sample_rate.max(1)))
}

/// Trait for garjan types that support block-based processing.
pub trait EnvironmentBlock {
    /// Fill `output` with synthesized audio.
    fn process_block(&mut self, output: &mut [f32]);
}

macro_rules! impl_env_block {
    ($($ty:ty),* $(,)?) => {
        $(
            impl EnvironmentBlock for $ty {
                #[inline]
                fn process_block(&mut self, output: &mut [f32]) {
                    self.process_block(output);
                }
            }
        )*
    };
}

impl_env_block!(
    Rain,
    Thunder,
    Wind,
    Fire,
    Water,
    Surf,
    Underwater,
    Bubble,
    Friction,
    Rolling,
    Footstep,
    Creak,
    AmbientTexture,
    Foliage,
    Insect,
    Cloth,
    Whistle,
    Whoosh,
    WingFlap,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rain_synthesis() {
        let mut rain = Rain::new(RainIntensity::Heavy, 44100.0).unwrap();
        let buf = render_environment(&mut rain, 44100, 0.5).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn wind_synthesis() {
        let mut wind = Wind::new(15.0, 0.5, 44100.0).unwrap();
        let buf = render_environment(&mut wind, 44100, 0.5).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn thunder_synthesis() {
        let mut thunder = Thunder::new(500.0, 44100.0).unwrap();
        let buf = render_environment(&mut thunder, 44100, 0.5).unwrap();
        assert!(buf.frames() > 0);
    }

    #[test]
    fn fire_synthesis() {
        let mut fire = Fire::new(0.7, 44100.0).unwrap();
        let buf = render_environment(&mut fire, 44100, 0.5).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn impact_synthesis() {
        let mut impact = Impact::new(Material::Wood, 44100.0).unwrap();
        let buf = render_impact(&mut impact, ImpactType::Strike, 44100).unwrap();
        assert!(buf.frames() > 0);
    }

    #[test]
    fn render_block_streaming() {
        let mut rain = Rain::new(RainIntensity::Light, 44100.0).unwrap();
        let buf = render_block(&mut rain, 4410, 44100);
        assert_eq!(buf.frames(), 4410);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }
}
