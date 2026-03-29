//! Grapheme-to-phoneme (G2P) conversion — text to phoneme sequences for vocal synthesis.
//!
//! Re-exports from [`shabda`](https://crates.io/crates/shabda), the AGNOS G2P crate,
//! and [`shabdakosh`](https://crates.io/crates/shabdakosh) pronunciation dictionaries.
//! Converts text strings into [`PhonemeEvent`] sequences ready for voice synthesis.
//!
//! # Feature: `g2p`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "0.22", features = ["g2p"] }
//! ```
//!
//! # Data Flow
//!
//! ```text
//! text → G2PEngine → Vec<PhonemeEvent> → PhonemeSequence → voice_synth::render_sequence → AudioBuffer
//! ```

use crate::buffer::AudioBuffer;

// ── Engine ─────────────────────────────────────────────────────────

/// G2P conversion engine with language-specific rules and dictionary lookup.
pub use shabda::engine::{G2PEngine, Language};

// ── Rules ──────────────────────────────────────────────────────────

/// Grapheme-to-phoneme rule set.
pub use shabda::rules;

// ── Text normalization ─────────────────────────────────────────────

/// Text normalization (numbers, abbreviations, punctuation).
pub use shabda::normalize;

// ── Syllabification ────────────────────────────────────────────────

/// Syllable boundary detection.
pub use shabda::syllable;

// ── Prosody ────────────────────────────────────────────────────────

/// Prosody assignment from text structure (stress, phrasing).
pub use shabda::prosody;

// ── Dictionary ─────────────────────────────────────────────────────

/// ARPABET phoneme mappings.
pub use shabda::arpabet;

/// Pronunciation dictionary (CMUdict-based).
pub use shabda::dictionary;

// ── Phoneme events (re-exported from svara via shabda) ─────────────

/// Timed phoneme event (phoneme + duration + stress).
pub use svara::sequence::PhonemeEvent;

/// Ordered phoneme sequence with coarticulation.
pub use svara::sequence::PhonemeSequence;

// ── Error ──────────────────────────────────────────────────────────

/// Shabda error type.
pub use shabda::error::ShabdaError;

// ── Bridge: text → dhvani AudioBuffer ──────────────────────────────

/// Convert text to phonemes using the G2P engine.
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if G2P conversion fails.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::g2p::*;
///
/// let engine = G2PEngine::new(Language::English);
/// let phonemes = text_to_phonemes(&engine, "hello world").unwrap();
/// ```
pub fn text_to_phonemes(engine: &G2PEngine, text: &str) -> crate::Result<Vec<PhonemeEvent>> {
    engine
        .convert(text)
        .map_err(|e| crate::NadaError::Dsp(format!("G2P conversion failed: {e}")))
}

/// Speak text directly to a dhvani [`AudioBuffer`].
///
/// Combines G2P conversion and voice synthesis in one call.
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if G2P or synthesis fails.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::g2p::*;
/// use dhvani::voice_synth::VoiceProfile;
///
/// let engine = G2PEngine::new(Language::English);
/// let voice = VoiceProfile::new_female();
/// let buf = speak(&engine, "hello", &voice, 44100).unwrap();
/// ```
pub fn speak(
    engine: &G2PEngine,
    text: &str,
    voice: &svara::voice::VoiceProfile,
    sample_rate: u32,
) -> crate::Result<AudioBuffer> {
    let samples = engine
        .speak(text, voice, sample_rate as f32)
        .map_err(|e| crate::NadaError::Dsp(format!("speak failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .map_err(|e| crate::NadaError::Dsp(format!("buffer from speech output: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_g2p() {
        let engine = G2PEngine::new(Language::English);
        let phonemes = text_to_phonemes(&engine, "hello").unwrap();
        assert!(!phonemes.is_empty());
    }

    #[test]
    fn speak_hello() {
        let engine = G2PEngine::new(Language::English);
        let voice = svara::voice::VoiceProfile::new_female();
        let buf = speak(&engine, "hello", &voice, 44100).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.rms() > 0.0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn phonemes_to_sequence() {
        let engine = G2PEngine::new(Language::English);
        let phonemes = text_to_phonemes(&engine, "test").unwrap();
        let mut seq = PhonemeSequence::new();
        for p in phonemes {
            seq.push(p);
        }
        let voice = svara::voice::VoiceProfile::new_male();
        let buf = crate::voice_synth::render_sequence(&seq, &voice, 44100).unwrap();
        assert!(buf.frames() > 0);
    }
}
