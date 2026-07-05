//! Creature and animal vocal synthesis — species-specific voice models and call patterns.
//!
//! Re-exports from [`prani`](https://crates.io/crates/prani), the AGNOS creature vocal synthesis crate.
//! Produces deterministic, real-time animal and creature vocalizations from species models.
//!
//! # Feature: `creature`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["creature"] }
//! ```
//!
//! # Architecture
//!
//! ```text
//! Species (vocal params) → CreatureVoice (individual) → Vocalization + Intent → AudioBuffer
//! ```

use crate::buffer::AudioBuffer;

// ── Species ─────────────────────────────────────────────────────────

/// Species definitions with vocal tract geometry and f0 range.
pub use prani::species::{Species, SpeciesParams, VocalApparatus};

// ── Voice ───────────────────────────────────────────────────────────

/// Creature voice instance with individual variation.
pub use prani::voice::CreatureVoice;

// ── Vocal tract ─────────────────────────────────────────────────────

/// Species-specific vocal tract model.
pub use prani::tract::CreatureTract;

// ── Vocalizations ───────────────────────────────────────────────────

/// Vocalization types (howl, bark, growl, roar, chirp, purr, etc.).
pub use prani::vocalization::Vocalization;

/// Behavioral intent that modifies vocalization prosody.
pub use prani::vocalization::{CallIntent, IntentModifiers};

// ── Error ───────────────────────────────────────────────────────────

/// Prani error type.
pub use prani::error::PraniError;

// ── Bridge: prani synthesis → dhvani AudioBuffer ────────────────────

/// Render a creature vocalization to a dhvani [`AudioBuffer`].
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if synthesis fails.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::creature::*;
///
/// let voice = CreatureVoice::new(Species::Wolf);
/// let buf = render_vocalization(&voice, &Vocalization::Howl, 44100, 2.0).unwrap();
/// ```
pub fn render_vocalization(
    voice: &CreatureVoice,
    vocalization: &Vocalization,
    sample_rate: u32,
    duration: f32,
) -> crate::Result<AudioBuffer> {
    let samples = voice
        .vocalize(vocalization, sample_rate as f32, duration)
        .map_err(|e| crate::NadaError::Dsp(format!("creature vocalization failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .map_err(|e| crate::NadaError::Dsp(format!("buffer from creature output: {e}")))
}

/// Render a creature vocalization with behavioral intent to a dhvani [`AudioBuffer`].
///
/// Intent modifies the vocalization's pitch, amplitude, duration, and urgency.
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if synthesis fails.
pub fn render_vocalization_with_intent(
    voice: &CreatureVoice,
    vocalization: &Vocalization,
    intent: CallIntent,
    sample_rate: u32,
    duration: f32,
) -> crate::Result<AudioBuffer> {
    let samples = voice
        .vocalize_with_intent(vocalization, intent, sample_rate as f32, duration)
        .map_err(|e| crate::NadaError::Dsp(format!("creature vocalization failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .map_err(|e| crate::NadaError::Dsp(format!("buffer from creature output: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wolf_howl() {
        let voice = CreatureVoice::new(Species::Wolf);
        let buf = render_vocalization(&voice, &Vocalization::Howl, 44100, 0.5).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.rms() > 0.0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn cat_purr() {
        let voice = CreatureVoice::new(Species::Cat);
        let buf = render_vocalization(&voice, &Vocalization::Purr, 44100, 0.5).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn dragon_roar_with_intent() {
        let voice = CreatureVoice::new(Species::Dragon);
        let buf = render_vocalization_with_intent(
            &voice,
            &Vocalization::Roar,
            CallIntent::Threat,
            44100,
            0.5,
        )
        .unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn songbird_chirp() {
        let voice = CreatureVoice::new(Species::Songbird);
        let buf = render_vocalization(&voice, &Vocalization::Chirp, 44100, 0.3).unwrap();
        assert!(buf.frames() > 0);
    }

    #[test]
    fn species_supports_vocalization() {
        assert!(Species::Wolf.supports_vocalization(&Vocalization::Howl));
        assert!(Species::Snake.supports_vocalization(&Vocalization::Hiss));
        assert!(Species::Cricket.supports_vocalization(&Vocalization::Stridulate));
    }

    #[test]
    fn voice_builder() {
        let voice = CreatureVoice::new(Species::Lion)
            .with_size(1.3)
            .with_breathiness(0.2);
        let buf = render_vocalization(&voice, &Vocalization::Roar, 44100, 0.5).unwrap();
        assert!(buf.rms() > 0.0);
    }
}
