//! Voice synthesis — glottal source, formant filtering, phoneme sequencing, prosody.
//!
//! Re-exports from [`svara`](https://crates.io/crates/svara), the AGNOS vocal synthesis crate.
//! Produces deterministic, real-time voice from phoneme sequences — no neural TTS.
//!
//! # Feature: `voice`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["voice"] }
//! ```
//!
//! # Data Flow
//!
//! ```text
//! text → phoneme sequence → prosody contour → glottal source → vocal tract → AudioBuffer
//! ```

use crate::buffer::AudioBuffer;

// ── Bhava personality/mood bridge ───────────────────────────────────

#[cfg(feature = "bhava-voice")]
mod bhava_bridge;
#[cfg(feature = "bhava-voice")]
pub use bhava_bridge::*;

// ── Voice profiles ──────────────────────────────────────────────────

/// Speaker voice characteristics (f0, breathiness, vibrato, jitter, shimmer).
pub use svara::voice::VoiceProfile;

/// Vocal effort level (breathy, soft, modal, pressed, harsh).
pub use svara::voice::{EffortParams, VocalEffort};

// ── Glottal source ──────────────────────────────────────────────────

/// Glottal pulse model selection (Rosenberg, Liljencrants-Fant).
pub use svara::glottal::{GlottalModel, GlottalSource};

// ── Formant synthesis ───────────────────────────────────────────────

/// Single formant resonance (frequency, bandwidth, amplitude).
pub use svara::formant::Formant;

/// Vowel category for formant lookup.
pub use svara::formant::Vowel;

/// Formant frequency/bandwidth targets (F1–F5).
pub use svara::formant::VowelTarget;

/// Parallel bank of biquad filters tuned to formant frequencies.
pub use svara::formant::FormantFilter;

// ── Vocal tract ─────────────────────────────────────────────────────

/// Place of articulation for nasal consonants.
pub use svara::tract::{NasalPlace, VocalTract};

// ── Level of detail ────────────────────────────────────────────────

/// Synthesis quality level for multi-voice LOD (full, reduced, minimal).
pub use svara::lod::Quality;

// ── Phonemes ────────────────────────────────────────────────────────

/// IPA-subset phoneme inventory (~50 phonemes).
pub use svara::phoneme::{Phoneme, PhonemeClass};

/// Formant targets for a phoneme.
pub use svara::phoneme::phoneme_formants;

/// Default duration for a phoneme.
pub use svara::phoneme::phoneme_duration;

/// Spectral tilt for a phoneme.
pub use svara::phoneme::phoneme_spectral_tilt;

/// Height-adjusted formant amplitudes for a phoneme.
pub use svara::phoneme::height_adjusted_amplitudes;

/// F2 locus equation for stop consonants.
pub use svara::phoneme::f2_locus_equation;

/// Synthesize a single phoneme to samples.
pub use svara::phoneme::synthesize_phoneme;

/// Synthesize a single phoneme with anticipatory nasalization.
pub use svara::phoneme::synthesize_phoneme_nasalized;

/// Anticipatory nasalization parameters.
pub use svara::phoneme::Nasalization;

/// Voice onset time for stop consonants.
pub use svara::phoneme::VoiceOnsetTime;

/// Reusable synthesis context for zero-allocation rendering.
pub use svara::phoneme::SynthesisContext;

/// Detect anticipatory nasalization across a phoneme sequence.
pub use svara::phoneme::detect_nasalization;

// ── Prosody ─────────────────────────────────────────────────────────

/// Stress level (primary, secondary, unstressed).
pub use svara::prosody::Stress;

/// Intonation patterns (declarative, interrogative, continuation, exclamatory).
pub use svara::prosody::IntonationPattern;

/// Prosodic contour — f0 trajectory, duration, and amplitude scaling.
pub use svara::prosody::ProsodyContour;

/// Lexical tone (for tonal languages).
pub use svara::prosody::Tone;

// ── Sequencing ──────────────────────────────────────────────────────

/// Timed phoneme event (phoneme + duration + stress).
pub use svara::sequence::PhonemeEvent;

/// Ordered phoneme sequence with coarticulation and crossfading.
pub use svara::sequence::PhonemeSequence;

// ── Spectral analysis ───────────────────────────────────────────────

/// Spectral analysis for vocal synthesis diagnostics.
pub use svara::spectral;

// ── Object pool ────────────────────────────────────────────────────

/// Pre-allocated synthesis pool for zero-allocation phoneme rendering.
pub use svara::pool::SynthesisPool;

// ── Batch rendering ────────────────────────────────────────────────

/// Non-real-time batch renderer for phoneme sequences.
pub use svara::render::BatchRenderer;

/// Progress information emitted during batch rendering.
pub use svara::render::RenderProgress;

/// Result of a batch render operation.
pub use svara::render::RenderOutput;

// ── Trajectory planning ────────────────────────────────────────────

/// Formant trajectory planner across multi-phoneme windows (Catmull-Rom).
pub use svara::trajectory::TrajectoryPlanner;

/// A formant keypoint: target formants at a specific time.
pub use svara::trajectory::FormantKeypoint;

// ── Error ──────────────────────────────────────────────────────────

/// Svara error type.
pub use svara::prelude::SvaraError;

// ── Bridge: svara synthesis → dhvani AudioBuffer ────────────────────

/// Render a [`PhonemeSequence`] to a dhvani [`AudioBuffer`].
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if the sequence fails to render.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::voice_synth::*;
///
/// let voice = VoiceProfile::new_female();
/// let mut seq = PhonemeSequence::new();
/// seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.3, Stress::Primary));
/// seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.1, Stress::Unstressed));
///
/// let buf = render_sequence(&seq, &voice, 44100).unwrap();
/// ```
pub fn render_sequence(
    sequence: &PhonemeSequence,
    voice: &VoiceProfile,
    sample_rate: u32,
) -> crate::Result<AudioBuffer> {
    let samples = sequence
        .render(voice, sample_rate as f32)
        .map_err(|e| crate::NadaError::Dsp(format!("voice synthesis failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate).map_err(|e| {
        crate::NadaError::Dsp(format!("failed to create buffer from voice output: {e}"))
    })
}

/// Render a single [`Phoneme`] to a dhvani [`AudioBuffer`].
///
/// # Errors
///
/// Returns `crate::NadaError::Dsp` if synthesis fails.
pub fn render_phoneme(
    phoneme: &Phoneme,
    voice: &VoiceProfile,
    sample_rate: u32,
    duration: f32,
) -> crate::Result<AudioBuffer> {
    let samples = synthesize_phoneme(phoneme, voice, sample_rate as f32, duration)
        .map_err(|e| crate::NadaError::Dsp(format!("phoneme synthesis failed: {e}")))?;
    AudioBuffer::from_interleaved(samples, 1, sample_rate).map_err(|e| {
        crate::NadaError::Dsp(format!("failed to create buffer from phoneme output: {e}"))
    })
}

/// Render a [`GlottalSource`] through a [`VocalTract`] for the given number of frames.
///
/// This is the lowest-level voice rendering function — for when you need
/// direct control over the glottal source and tract parameters.
pub fn render_vocal_tract(
    source: &mut GlottalSource,
    tract: &mut VocalTract,
    frames: usize,
    sample_rate: u32,
) -> AudioBuffer {
    let mut samples = Vec::with_capacity(frames);
    for _ in 0..frames {
        let glottal = source.next_sample();
        samples.push(tract.process_sample(glottal));
    }
    AudioBuffer::from_interleaved(samples, 1, sample_rate)
        .unwrap_or_else(|_| AudioBuffer::silence(1, frames, sample_rate.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_vowel_a() {
        let voice = VoiceProfile::new_male();
        let buf = render_phoneme(&Phoneme::VowelA, &voice, 44100, 0.2).unwrap();
        assert_eq!(buf.channels(), 1);
        assert!(buf.frames() > 0);
        assert!(buf.rms() > 0.0);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn render_phoneme_sequence() {
        let voice = VoiceProfile::new_female();
        let mut seq = PhonemeSequence::new();
        seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.2, Stress::Primary));
        seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.1, Stress::Unstressed));
        seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.2, Stress::Unstressed));

        let buf = render_sequence(&seq, &voice, 44100).unwrap();
        assert!(buf.frames() > 0);
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn voice_profiles() {
        let male = VoiceProfile::new_male();
        let female = VoiceProfile::new_female();
        let child = VoiceProfile::new_child();

        // Female f0 > male f0 > 0
        assert!(female.base_f0 > male.base_f0);
        assert!(child.base_f0 > female.base_f0);
        assert!(male.base_f0 > 0.0);
    }

    #[test]
    fn glottal_source_renders() {
        let voice = VoiceProfile::new_male();
        let mut source = voice.create_glottal_source(44100.0).unwrap();
        let mut tract = VocalTract::new(44100.0);

        let buf = render_vocal_tract(&mut source, &mut tract, 4410, 44100);
        assert_eq!(buf.frames(), 4410);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    // ── v2 bridge tests ────────────────────────────────────────────

    #[test]
    fn vocal_effort_variants() {
        // Ensure VocalEffort variants are accessible through the bridge
        let _whisper = VocalEffort::Whisper;
        let _soft = VocalEffort::Soft;
        let _normal = VocalEffort::Normal;
        let _loud = VocalEffort::Loud;
        let _shout = VocalEffort::Shout;
        let params = VocalEffort::Normal.params();
        assert!(params.breathiness >= 0.0);
    }

    #[test]
    fn quality_lod_levels() {
        assert_eq!(Quality::Full.max_formants(), 5);
        assert_eq!(Quality::Reduced.max_formants(), 3);
        assert_eq!(Quality::Minimal.max_formants(), 2);
        assert!(Quality::Full.use_nasal_coupling());
        assert!(!Quality::Minimal.use_nasal_coupling());
    }

    #[test]
    fn tone_accessible() {
        let _high = Tone::High;
        let _rising = Tone::Rising;
        let _falling = Tone::Falling;
        let contour = Tone::Rising.to_contour();
        assert!(!contour.f0_points.is_empty());
    }

    #[test]
    fn nasalization_detection() {
        let phonemes = [Phoneme::VowelA, Phoneme::NasalN, Phoneme::VowelI];
        let nasalizations = detect_nasalization(&phonemes);
        assert_eq!(nasalizations.len(), 3);
        // Vowel before nasal should have anticipatory nasalization
        assert!(nasalizations[0].is_some());
    }

    #[test]
    fn phoneme_spectral_tilt_values() {
        let tilt_a = phoneme_spectral_tilt(&Phoneme::VowelA);
        let tilt_s = phoneme_spectral_tilt(&Phoneme::FricativeS);
        assert!(tilt_a.is_finite());
        assert!(tilt_s.is_finite());
    }

    #[test]
    fn height_adjusted_amplitudes_values() {
        let amps = height_adjusted_amplitudes(&Phoneme::VowelA);
        assert_eq!(amps.len(), 5);
        assert!(amps.iter().all(|a| a.is_finite()));
    }

    #[test]
    fn voice_onset_time_accessible() {
        let vot = VoiceOnsetTime::for_plosive(&Phoneme::PlosiveT);
        assert!(vot.closure_fraction > 0.0);
        assert!(vot.burst_fraction > 0.0);
    }

    #[test]
    fn synthesis_context_creation() {
        let voice = VoiceProfile::new_male();
        let ctx = SynthesisContext::new(&voice, 44100.0).unwrap();
        assert!(ctx.sample_rate() > 0.0);
    }

    #[test]
    fn synthesize_phoneme_nasalized_works() {
        let voice = VoiceProfile::new_male();
        let nasal = Nasalization::for_nasal(&Phoneme::NasalN).unwrap();
        let samples =
            synthesize_phoneme_nasalized(&Phoneme::VowelA, &voice, 44100.0, 0.1, Some(&nasal))
                .unwrap();
        assert!(!samples.is_empty());
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn synthesis_pool_render() {
        let voice = VoiceProfile::new_male();
        let mut pool = SynthesisPool::new(&voice, 44100.0).unwrap();
        let samples = pool.render(&Phoneme::VowelA, &voice, 0.1).unwrap();
        assert!(!samples.is_empty());
        assert!(samples.iter().all(|s| s.is_finite()));
        assert_eq!(pool.render_count(), 1);
    }

    #[test]
    fn synthesis_pool_with_capacity() {
        let voice = VoiceProfile::new_male();
        let pool = SynthesisPool::with_capacity(&voice, 44100.0, 0.5).unwrap();
        assert!(pool.peak_samples() >= (0.5 * 44100.0) as usize);
    }

    #[test]
    fn batch_renderer_render() {
        let voice = VoiceProfile::new_male();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelA, 0.1, Stress::Primary);
        renderer.push(Phoneme::NasalN, 0.06, Stress::Unstressed);
        renderer.push(Phoneme::VowelI, 0.1, Stress::Primary);

        let output = renderer.render_all().unwrap();
        assert!(!output.samples.is_empty());
        assert!(output.samples.iter().all(|s| s.is_finite()));
        assert_eq!(output.progress.total_phonemes, 3);
        assert_eq!(output.progress.phoneme_index, 3);
    }

    #[test]
    fn batch_renderer_with_progress() {
        let voice = VoiceProfile::new_female();
        let mut renderer = BatchRenderer::new(&voice, 44100.0).unwrap();
        renderer.push(Phoneme::VowelE, 0.08, Stress::Primary);
        renderer.push(Phoneme::FricativeS, 0.05, Stress::Unstressed);

        let mut progress_count = 0u32;
        let output = renderer
            .render_with_progress(|p| {
                progress_count += 1;
                assert!(p.fraction() <= 1.0);
            })
            .unwrap();

        assert_eq!(progress_count, 2);
        assert!(!output.samples.is_empty());
    }

    #[test]
    fn render_progress_fraction() {
        let p = RenderProgress {
            phoneme_index: 5,
            total_phonemes: 10,
            samples_rendered: 22050,
        };
        assert!((p.fraction() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn trajectory_planner_basic() {
        let voice = VoiceProfile::new_male();
        let phonemes = [Phoneme::VowelA, Phoneme::NasalN, Phoneme::VowelI];
        let durations = [0.1, 0.06, 0.1];
        let plan = TrajectoryPlanner::plan(&phonemes, &durations, &voice, 44100.0);

        assert!(plan.total_samples() > 0);
        assert!(plan.num_keypoints() >= 3);

        // Verify formants are finite throughout
        for sample in (0..plan.total_samples()).step_by(441) {
            let target = plan.formants_at(sample);
            assert!(target.f1.is_finite());
            assert!(target.f2.is_finite());
            assert!(target.f1 > 0.0);
        }
    }

    #[test]
    fn trajectory_planner_speaking_rate() {
        let voice = VoiceProfile::new_male();
        let phonemes = [Phoneme::VowelA, Phoneme::VowelI];
        let durations = [0.1, 0.1];
        let mut plan = TrajectoryPlanner::plan(&phonemes, &durations, &voice, 44100.0);
        let kp_before = plan.keypoints()[1].resistance;
        plan.apply_speaking_rate(2.0);
        let kp_after = plan.keypoints()[1].resistance;
        // Faster speech should reduce resistance
        assert!(kp_after < kp_before);
    }

    #[test]
    fn formant_keypoint_accessible() {
        // FormantKeypoint is #[non_exhaustive], so test via TrajectoryPlanner
        let voice = VoiceProfile::new_male();
        let plan = TrajectoryPlanner::plan(&[Phoneme::VowelA], &[0.1], &voice, 44100.0);
        let kps = plan.keypoints();
        assert!(!kps.is_empty());
        assert!(kps[0].resistance >= 0.0);
        assert!(kps[0].target.f1 > 0.0);
    }
}
