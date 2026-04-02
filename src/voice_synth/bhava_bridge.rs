//! Bhava personality/mood → svara voice parameter mapping.
//!
//! Pure stateless functions that translate emotional and personality state
//! from [`bhava`](https://crates.io/crates/bhava) into svara synthesis
//! parameters. All mappings are grounded in psychoacoustic research:
//!
//! - Scherer (2003) — vocal correlates of speaker personality
//! - Juslin & Laukka (2003) — emotional expression in speech prosody
//! - Banse & Scherer (1996) — acoustic profiles of vocal emotion expression
//! - Traunmüller & Eriksson (2000) — vocal effort effects
//! - Gobl & Ní Chasaide (2003) — voice quality and f0 in affect expression
//! - Mendoza & Carballo (1998) — vocal quality changes under stress
//!
//! # Feature: `bhava-voice`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["bhava-voice"] }
//! ```

use bhava::energy::EnergyState;
use bhava::mood::MoodVector;
use bhava::stress::StressState;
use bhava::traits::{PersonalityProfile, TraitKind};

use super::{IntonationPattern, ProsodyContour, Quality, VocalEffort, VoiceProfile};

// ── Mapping constants ──────────────────────────────────────────────
// Named coefficients make the psychoacoustic mapping auditable and tunable.

// voice_from_personality
const WARMTH_BREATHINESS: f32 = 0.08;
const WARMTH_FORMANT: f32 = 0.03;
const CONFIDENCE_RANGE: f32 = 0.25;
const CONFIDENCE_PERTURB: f32 = 0.3;
const EMPATHY_VIBRATO_DEPTH: f32 = 0.02;
const PATIENCE_VIBRATO_RATE: f32 = 0.5;
const FORMALITY_BREATHINESS: f32 = 0.04;
const DIRECTNESS_RANGE: f32 = 0.1;

// prosody_from_mood
const JOY_F0: f32 = 0.08;
const AROUSAL_F0: f32 = 0.12;
const AROUSAL_DURATION: f32 = 0.15;
const SADNESS_DURATION: f32 = 0.1;
const AROUSAL_AMPLITUDE: f32 = 0.15;
const DOMINANCE_AMPLITUDE: f32 = 0.1;
const FRUSTRATION_CONTOUR_JAG: f32 = 0.05;

// effort_from_mood
const EFFORT_AROUSAL_WEIGHT: f32 = 0.6;
const EFFORT_DOMINANCE_WEIGHT: f32 = 0.3;
const EFFORT_FRUSTRATION_WEIGHT: f32 = 0.1;

// apply_mood_to_voice
const MOOD_JOY_F0: f32 = 0.05;
const MOOD_AROUSAL_RANGE: f32 = 0.3;
const MOOD_AROUSAL_BREATHINESS: f32 = 0.05;
const MOOD_FRUSTRATION_JITTER: f32 = 0.01;
const MOOD_FRUSTRATION_SHIMMER: f32 = 0.02;
const MOOD_TRUST_VIBRATO: f32 = 0.01;
const MOOD_INTEREST_RANGE_HZ: f32 = 5.0;

// apply_stress_to_voice
const STRESS_JITTER: f32 = 0.015;
const STRESS_SHIMMER: f32 = 0.03;
const STRESS_BREATHINESS: f32 = 0.1;
const STRESS_F0_RAISE: f32 = 0.08;
const STRESS_RANGE_NARROW: f32 = 0.3;
const STRESS_VIBRATO_RATE: f32 = 1.0;
const BURNOUT_BANDWIDTH: f32 = 0.5;

// quality_from_energy
const ENERGY_RAW_WEIGHT: f32 = 0.6;
const ENERGY_PERFORMANCE_WEIGHT: f32 = 0.4;

// Safety floor for f0-related outputs
const F0_FLOOR_HZ: f32 = 50.0;
const F0_MULTIPLIER_FLOOR: f32 = 0.5;

// ── Mapping functions ──────────────────────────────────────────────

/// Derive a voice baseline from personality traits.
///
/// Applies stable trait-based adjustments to a base voice profile.
/// Call once when personality is assigned or changes — not per-frame.
///
/// # Mapping
///
/// - **Warmth** → breathiness (warmer = breathier phonation), formant scale (warmer = slightly lower)
/// - **Confidence** → f0 range (wider), jitter/shimmer (less perturbation)
/// - **Empathy** → vibrato depth (more expressive)
/// - **Patience** → vibrato rate (slower, more relaxed)
/// - **Formality** → breathiness (more formal = more pressed/modal)
/// - **Directness** → f0 range (more decisive pitch movements)
#[must_use]
#[inline]
pub fn voice_from_personality(profile: &PersonalityProfile, base: &VoiceProfile) -> VoiceProfile {
    tracing::trace!("voice_from_personality");

    let warmth = profile.get_trait(TraitKind::Warmth).normalized();
    let confidence = profile.get_trait(TraitKind::Confidence).normalized();
    let empathy = profile.get_trait(TraitKind::Empathy).normalized();
    let patience = profile.get_trait(TraitKind::Patience).normalized();
    let formality = profile.get_trait(TraitKind::Formality).normalized();
    let directness = profile.get_trait(TraitKind::Directness).normalized();

    let breathiness = (base.breathiness + warmth * WARMTH_BREATHINESS
        - formality * FORMALITY_BREATHINESS)
        .clamp(0.0, 0.3);

    base.clone()
        .with_breathiness(breathiness)
        .with_formant_scale(base.formant_scale + warmth * WARMTH_FORMANT)
        .with_f0_range(
            base.f0_range
                * (1.0 + confidence * CONFIDENCE_RANGE)
                * (1.0 + directness * DIRECTNESS_RANGE),
        )
        .with_jitter(base.jitter * (1.0 - confidence * CONFIDENCE_PERTURB))
        .with_shimmer(base.shimmer * (1.0 - confidence * CONFIDENCE_PERTURB))
        .with_vibrato_depth(base.vibrato_depth + empathy * EMPATHY_VIBRATO_DEPTH)
        .with_vibrato_rate((base.vibrato_rate - patience * PATIENCE_VIBRATO_RATE).max(3.0))
}

/// Generate a prosody contour from current mood.
///
/// Produces an f0 trajectory, duration scale, and amplitude scale
/// reflecting the speaker's emotional state. Call per utterance.
///
/// # Mapping
///
/// - **Joy + Arousal** → f0 level (positive raises pitch)
/// - **Joy polarity** → contour shape (positive = rise-fall, negative = sagging)
/// - **Arousal** → duration (high arousal = faster), amplitude (louder)
/// - **Dominance** → amplitude (more dominant = louder)
/// - **Frustration** → contour jaggedness (tension adds irregularity)
#[must_use]
pub fn prosody_from_mood(mood: &MoodVector) -> ProsodyContour {
    tracing::trace!("prosody_from_mood");

    let f0_base = 1.0 + mood.joy * JOY_F0 + mood.arousal * AROUSAL_F0;

    // Contour shape: 5 points from flat, shaped by joy polarity
    let joy_abs = mood.joy.abs();
    let (shape_offsets, flat) = if mood.joy >= 0.0 {
        // Happy: slight rise-fall
        ([0.98_f32, 1.04, 1.02, 0.98, 0.96], [1.0; 5])
    } else {
        // Sad: sagging
        ([1.0_f32, 0.99, 0.97, 0.96, 0.94], [1.0; 5])
    };

    // Blend between flat and shaped by joy intensity, floor at F0_MULTIPLIER_FLOOR
    let mut points: Vec<(f32, f32)> = (0..5)
        .map(|i| {
            let t = i as f32 * 0.25;
            let v = (f0_base * (flat[i] + (shape_offsets[i] - flat[i]) * joy_abs))
                .max(F0_MULTIPLIER_FLOOR);
            (t, v)
        })
        .collect();

    // Frustration adds jagged perturbation at t=0.3 and t=0.7
    if mood.frustration > 0.3 {
        let jag = mood.frustration * FRUSTRATION_CONTOUR_JAG;
        // Insert perturbation points
        points.push((0.3, (f0_base + jag).max(F0_MULTIPLIER_FLOOR)));
        points.push((0.7, (f0_base - jag).max(F0_MULTIPLIER_FLOOR)));
        points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(core::cmp::Ordering::Equal));
    }

    let duration_scale = (1.0 - mood.arousal * AROUSAL_DURATION
        + (-mood.joy).max(0.0) * SADNESS_DURATION)
        .clamp(0.7, 1.4);

    let amplitude_scale =
        (1.0 + mood.arousal * AROUSAL_AMPLITUDE + mood.dominance * DOMINANCE_AMPLITUDE)
            .clamp(0.7, 1.5);

    ProsodyContour {
        f0_points: points,
        duration_scale,
        amplitude_scale,
    }
}

/// Map emotional state to vocal effort level.
///
/// Arousal is the primary driver (Traunmüller & Eriksson 2000),
/// with dominance and frustration as secondary contributors.
///
/// # Mapping
///
/// Computes `drive = arousal×0.6 + dominance×0.3 + frustration×0.1`,
/// then classifies: ≥0.7 Shout, ≥0.4 Loud, ≥−0.2 Normal, ≥−0.5 Soft, else Whisper.
#[must_use]
#[inline]
pub fn effort_from_mood(mood: &MoodVector) -> VocalEffort {
    tracing::trace!("effort_from_mood");

    let drive = mood.arousal * EFFORT_AROUSAL_WEIGHT
        + mood.dominance * EFFORT_DOMINANCE_WEIGHT
        + mood.frustration * EFFORT_FRUSTRATION_WEIGHT;

    if drive >= 0.7 {
        VocalEffort::Shout
    } else if drive >= 0.4 {
        VocalEffort::Loud
    } else if drive >= -0.2 {
        VocalEffort::Normal
    } else if drive >= -0.5 {
        VocalEffort::Soft
    } else {
        VocalEffort::Whisper
    }
}

/// Modify a voice profile from current mood.
///
/// Applies real-time emotional coloring to an existing voice.
/// Call per utterance or per phrase for dynamic expression.
///
/// # Mapping
///
/// - **Joy** → base f0 (+5% at max)
/// - **Arousal** → f0 range (wider), breathiness (less)
/// - **Frustration** → jitter, shimmer (more perturbation)
/// - **Trust** → vibrato depth (freer vibrato)
/// - **Interest** → f0 range (+5 Hz at max)
#[must_use]
#[inline]
pub fn apply_mood_to_voice(mood: &MoodVector, voice: &VoiceProfile) -> VoiceProfile {
    tracing::trace!("apply_mood_to_voice");

    voice
        .clone()
        .with_f0((voice.base_f0 * (1.0 + mood.joy * MOOD_JOY_F0)).max(F0_FLOOR_HZ))
        .with_f0_range(
            voice.f0_range * (1.0 + mood.arousal * MOOD_AROUSAL_RANGE)
                + mood.interest * MOOD_INTEREST_RANGE_HZ,
        )
        .with_breathiness(voice.breathiness - mood.arousal * MOOD_AROUSAL_BREATHINESS)
        .with_jitter(voice.jitter + mood.frustration.max(0.0) * MOOD_FRUSTRATION_JITTER)
        .with_shimmer(voice.shimmer + mood.frustration.max(0.0) * MOOD_FRUSTRATION_SHIMMER)
        .with_vibrato_depth(voice.vibrato_depth + mood.trust * MOOD_TRUST_VIBRATO)
}

/// Apply chronic stress effects to a voice profile.
///
/// Stress degrades voice quality through laryngeal tension, constriction,
/// and fatigue. Call when stress state updates.
///
/// # Mapping
///
/// - **Load** → jitter/shimmer (instability), breathiness (strained phonation)
/// - **Load** → base f0 (raised via tension), f0 range (narrowed via fatigue)
/// - **Burnout** → bandwidth widening (loss of articulatory precision)
#[must_use]
#[inline]
pub fn apply_stress_to_voice(stress: &StressState, voice: &VoiceProfile) -> VoiceProfile {
    tracing::trace!("apply_stress_to_voice");

    let load = stress.load.get();

    let mut result = voice
        .clone()
        .with_jitter(voice.jitter + load * STRESS_JITTER)
        .with_shimmer(voice.shimmer + load * STRESS_SHIMMER)
        .with_breathiness(voice.breathiness + load * STRESS_BREATHINESS)
        .with_f0((voice.base_f0 * (1.0 + load * STRESS_F0_RAISE)).max(F0_FLOOR_HZ))
        .with_f0_range(voice.f0_range * (1.0 - load * STRESS_RANGE_NARROW))
        .with_vibrato_rate(voice.vibrato_rate + load * STRESS_VIBRATO_RATE);

    // Burnout: loss of articulatory precision
    if stress.is_burned_out() {
        result = result.with_bandwidth_widening(voice.bandwidth_widening + BURNOUT_BANDWIDTH);
    }

    result
}

/// Select synthesis quality level from energy state.
///
/// Low energy maps to reduced LOD — tired speakers articulate less precisely,
/// and this also serves as a resource hint for multi-voice rendering.
///
/// Blends raw energy (60%) with Banister performance (40%) for smoother
/// transitions that account for supercompensation.
#[must_use]
#[inline]
pub fn quality_from_energy(energy: &EnergyState) -> Quality {
    tracing::trace!("quality_from_energy");

    let effective =
        energy.energy.get() * ENERGY_RAW_WEIGHT + energy.performance() * ENERGY_PERFORMANCE_WEIGHT;

    if effective >= 0.5 {
        Quality::Full
    } else if effective >= 0.2 {
        Quality::Reduced
    } else {
        Quality::Minimal
    }
}

/// Select intonation pattern from emotional state.
///
/// Priority-ordered heuristic — real intonation depends on syntax,
/// but this provides a mood-flavored default that callers can override.
///
/// # Mapping
///
/// 1. High joy + arousal → Exclamatory
/// 2. High frustration + arousal → Exclamatory
/// 3. High interest + low dominance → Interrogative
/// 4. Moderate arousal + dominance → Continuation
/// 5. Default → Declarative
#[must_use]
#[inline]
pub fn intonation_from_mood(mood: &MoodVector) -> IntonationPattern {
    tracing::trace!("intonation_from_mood");

    // Two distinct triggers for exclamatory — joy-based excitement vs frustration outburst.
    // Intentionally separate branches for clarity despite identical result.
    #[allow(clippy::if_same_then_else)]
    if mood.joy > 0.5 && mood.arousal > 0.5 {
        IntonationPattern::Exclamatory
    } else if mood.frustration > 0.5 && mood.arousal > 0.3 {
        IntonationPattern::Exclamatory
    } else if mood.interest > 0.4 && mood.dominance < 0.0 {
        IntonationPattern::Interrogative
    } else if mood.arousal > 0.3 && mood.dominance > 0.0 {
        IntonationPattern::Continuation
    } else {
        IntonationPattern::Declarative
    }
}

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use bhava::stress::StressLevel;
    use bhava::traits::TraitLevel;

    fn neutral_mood() -> MoodVector {
        MoodVector::neutral()
    }

    fn male_voice() -> VoiceProfile {
        VoiceProfile::new_male()
    }

    fn balanced_profile() -> PersonalityProfile {
        PersonalityProfile::new("test")
    }

    // ── voice_from_personality ──────────────────────────────────────

    #[test]
    fn personality_balanced_preserves_base() {
        let base = male_voice();
        let result = voice_from_personality(&balanced_profile(), &base);
        // All-balanced should produce near-identical voice
        assert!((result.base_f0 - base.base_f0).abs() < 1.0);
        assert!((result.breathiness - base.breathiness).abs() < 0.01);
        assert!((result.f0_range - base.f0_range).abs() < 1.0);
    }

    #[test]
    fn warm_personality_increases_breathiness() {
        let base = male_voice();
        let mut profile = balanced_profile();
        profile.set_trait(TraitKind::Warmth, TraitLevel::Highest);
        let result = voice_from_personality(&profile, &base);
        assert!(result.breathiness > base.breathiness);
    }

    #[test]
    fn confident_personality_widens_range() {
        let base = male_voice();
        let mut profile = balanced_profile();
        profile.set_trait(TraitKind::Confidence, TraitLevel::Highest);
        let result = voice_from_personality(&profile, &base);
        assert!(result.f0_range > base.f0_range);
        assert!(result.jitter < base.jitter);
        assert!(result.shimmer < base.shimmer);
    }

    #[test]
    fn formal_personality_reduces_breathiness() {
        let base = male_voice();
        let mut profile = balanced_profile();
        profile.set_trait(TraitKind::Formality, TraitLevel::Highest);
        let result = voice_from_personality(&profile, &base);
        assert!(result.breathiness <= base.breathiness);
    }

    #[test]
    fn personality_output_finite() {
        let base = male_voice();
        let mut profile = balanced_profile();
        // Set all traits to extreme
        for &kind in bhava::traits::TraitKind::ALL {
            profile.set_trait(kind, TraitLevel::Highest);
        }
        let result = voice_from_personality(&profile, &base);
        assert!(result.base_f0.is_finite());
        assert!(result.breathiness.is_finite());
        assert!(result.f0_range.is_finite());
        assert!(result.jitter.is_finite());
        assert!(result.shimmer.is_finite());
        assert!(result.vibrato_rate.is_finite());
        assert!(result.vibrato_depth.is_finite());
    }

    #[test]
    fn personality_all_lowest_valid() {
        let base = male_voice();
        let mut profile = balanced_profile();
        for &kind in bhava::traits::TraitKind::ALL {
            profile.set_trait(kind, TraitLevel::Lowest);
        }
        let result = voice_from_personality(&profile, &base);
        // All outputs must be finite and within svara's valid ranges
        assert!(result.base_f0.is_finite() && result.base_f0 > 0.0);
        assert!(result.breathiness >= 0.0 && result.breathiness <= 1.0);
        assert!(result.f0_range >= 0.0);
        assert!(result.jitter >= 0.0 && result.jitter <= 0.05);
        assert!(result.shimmer >= 0.0 && result.shimmer <= 0.1);
        assert!(result.vibrato_rate >= 0.0);
        assert!(result.vibrato_depth >= 0.0 && result.vibrato_depth <= 0.5);
        assert!(result.formant_scale >= 0.1);
    }

    #[test]
    fn personality_lowest_confidence_increases_jitter() {
        let base = male_voice();
        let mut profile = balanced_profile();
        profile.set_trait(TraitKind::Confidence, TraitLevel::Lowest);
        let result = voice_from_personality(&profile, &base);
        // Low confidence → more perturbation
        assert!(result.jitter > base.jitter);
        assert!(result.shimmer > base.shimmer);
        assert!(result.f0_range < base.f0_range);
    }

    // ── prosody_from_mood ──────────────────────────────────────────

    #[test]
    fn neutral_mood_flat_prosody() {
        let contour = prosody_from_mood(&neutral_mood());
        assert!((contour.duration_scale - 1.0).abs() < 0.01);
        assert!((contour.amplitude_scale - 1.0).abs() < 0.01);
        // f0 points should be near 1.0
        for &(_, v) in &contour.f0_points {
            assert!((v - 1.0).abs() < 0.15);
        }
    }

    #[test]
    fn sad_mood_lowers_f0_and_slows() {
        let mut mood = neutral_mood();
        mood.joy = -0.8;
        mood.arousal = -0.5;
        let contour = prosody_from_mood(&mood);
        let avg_f0: f32 =
            contour.f0_points.iter().map(|(_, v)| v).sum::<f32>() / contour.f0_points.len() as f32;
        assert!(avg_f0 < 1.0); // lower pitch
        assert!(contour.duration_scale > 1.0); // slower
    }

    #[test]
    fn extreme_negative_mood_contour_positive() {
        // Worst-case: all dimensions at negative extreme
        let mut mood = MoodVector::neutral();
        mood.joy = -1.0;
        mood.arousal = -1.0;
        mood.dominance = -1.0;
        mood.trust = -1.0;
        mood.interest = -1.0;
        mood.frustration = -1.0;
        let contour = prosody_from_mood(&mood);
        for &(t, v) in &contour.f0_points {
            assert!((0.0..=1.0).contains(&t), "time out of range: {t}");
            assert!(v >= F0_MULTIPLIER_FLOOR, "f0 below floor: {v}");
            assert!(v.is_finite(), "f0 not finite: {v}");
        }
        assert!(contour.duration_scale > 0.0);
        assert!(contour.amplitude_scale > 0.0);
    }

    #[test]
    fn extreme_positive_mood_contour_valid() {
        let mut mood = MoodVector::neutral();
        mood.joy = 1.0;
        mood.arousal = 1.0;
        mood.dominance = 1.0;
        mood.trust = 1.0;
        mood.interest = 1.0;
        mood.frustration = 1.0;
        let contour = prosody_from_mood(&mood);
        for &(t, v) in &contour.f0_points {
            assert!((0.0..=1.0).contains(&t));
            assert!(v > 0.0 && v.is_finite());
        }
    }

    #[test]
    fn happy_mood_raises_f0() {
        let mut mood = neutral_mood();
        mood.joy = 0.8;
        mood.arousal = 0.5;
        let contour = prosody_from_mood(&mood);
        // f0 base should be above 1.0
        let avg_f0: f32 =
            contour.f0_points.iter().map(|(_, v)| v).sum::<f32>() / contour.f0_points.len() as f32;
        assert!(avg_f0 > 1.0);
    }

    #[test]
    fn aroused_mood_faster_louder() {
        let mut mood = neutral_mood();
        mood.arousal = 0.8;
        let contour = prosody_from_mood(&mood);
        assert!(contour.duration_scale < 1.0); // faster
        assert!(contour.amplitude_scale > 1.0); // louder
    }

    #[test]
    fn prosody_points_valid() {
        let mut mood = neutral_mood();
        mood.joy = -0.9;
        mood.frustration = 0.9;
        mood.arousal = 0.7;
        let contour = prosody_from_mood(&mood);
        for &(t, v) in &contour.f0_points {
            assert!((0.0..=1.0).contains(&t), "time {t} out of range");
            assert!(v > 0.0 && v.is_finite(), "f0 value {v} invalid");
        }
        assert!(contour.duration_scale > 0.0);
        assert!(contour.amplitude_scale > 0.0);
    }

    // ── effort_from_mood ───────────────────────────────────────────

    #[test]
    fn neutral_mood_normal_effort() {
        assert_eq!(effort_from_mood(&neutral_mood()), VocalEffort::Normal);
    }

    #[test]
    fn high_arousal_loud_or_shout() {
        let mut mood = neutral_mood();
        mood.arousal = 0.9;
        mood.dominance = 0.5;
        let effort = effort_from_mood(&mood);
        assert!(effort == VocalEffort::Loud || effort == VocalEffort::Shout);
    }

    #[test]
    fn low_arousal_soft_or_whisper() {
        let mut mood = neutral_mood();
        mood.arousal = -0.8;
        mood.dominance = -0.5;
        let effort = effort_from_mood(&mood);
        assert!(effort == VocalEffort::Soft || effort == VocalEffort::Whisper);
    }

    #[test]
    fn effort_monotonic_with_arousal() {
        let efforts: Vec<VocalEffort> = (-10..=10)
            .map(|i| {
                let mut mood = neutral_mood();
                mood.arousal = i as f32 / 10.0;
                effort_from_mood(&mood)
            })
            .collect();
        // Effort should never decrease as arousal increases
        let as_num = |e: &VocalEffort| match e {
            VocalEffort::Whisper => 0,
            VocalEffort::Soft => 1,
            VocalEffort::Normal => 2,
            VocalEffort::Loud => 3,
            VocalEffort::Shout => 4,
            _ => 2, // unknown future variants treated as Normal
        };
        for w in efforts.windows(2) {
            assert!(as_num(&w[1]) >= as_num(&w[0]));
        }
    }

    // ── apply_mood_to_voice ────────────────────────────────────────

    #[test]
    fn neutral_mood_preserves_voice() {
        let voice = male_voice();
        let result = apply_mood_to_voice(&neutral_mood(), &voice);
        assert!((result.base_f0 - voice.base_f0).abs() < 0.1);
        assert!((result.f0_range - voice.f0_range).abs() < 0.1);
    }

    #[test]
    fn joy_raises_f0() {
        let voice = male_voice();
        let mut mood = neutral_mood();
        mood.joy = 0.8;
        let result = apply_mood_to_voice(&mood, &voice);
        assert!(result.base_f0 > voice.base_f0);
    }

    #[test]
    fn frustration_increases_perturbation() {
        let voice = male_voice();
        let mut mood = neutral_mood();
        mood.frustration = 0.8;
        let result = apply_mood_to_voice(&mood, &voice);
        assert!(result.jitter > voice.jitter);
        assert!(result.shimmer > voice.shimmer);
    }

    #[test]
    fn mood_voice_output_finite() {
        let voice = male_voice();
        let mut mood = neutral_mood();
        mood.joy = 1.0;
        mood.arousal = 1.0;
        mood.dominance = 1.0;
        mood.trust = 1.0;
        mood.interest = 1.0;
        mood.frustration = 1.0;
        let result = apply_mood_to_voice(&mood, &voice);
        assert!(result.base_f0.is_finite());
        assert!(result.f0_range.is_finite());
        assert!(result.breathiness.is_finite());
        assert!(result.jitter.is_finite());
        assert!(result.shimmer.is_finite());
    }

    // ── apply_stress_to_voice ──────────────────────────────────────

    #[test]
    fn zero_stress_preserves_voice() {
        let voice = male_voice();
        let stress = StressState::new();
        let result = apply_stress_to_voice(&stress, &voice);
        assert!((result.base_f0 - voice.base_f0).abs() < 0.1);
        assert!((result.jitter - voice.jitter).abs() < 0.001);
    }

    #[test]
    fn stress_increases_jitter_monotonically() {
        let voice = male_voice();
        let mut prev_jitter = voice.jitter;
        for load_pct in [10, 30, 50, 70, 90] {
            let mut stress = StressState::new();
            // Tick stress up to desired load
            let aggressive_mood = MoodVector {
                frustration: 1.0,
                arousal: 1.0,
                ..MoodVector::neutral()
            };
            for _ in 0..(load_pct * 5) {
                stress.tick(&aggressive_mood);
            }
            let result = apply_stress_to_voice(&stress, &voice);
            assert!(
                result.jitter >= prev_jitter - 0.001,
                "jitter decreased at load {}",
                load_pct
            );
            prev_jitter = result.jitter;
        }
    }

    #[test]
    fn high_stress_raises_f0() {
        let voice = male_voice();
        let aggressive = MoodVector {
            frustration: 1.0,
            arousal: 1.0,
            ..MoodVector::neutral()
        };
        let mut stress = StressState::new();
        for _ in 0..500 {
            stress.tick(&aggressive);
        }
        let result = apply_stress_to_voice(&stress, &voice);
        assert!(result.base_f0 > voice.base_f0);
    }

    #[test]
    fn burnout_widens_bandwidth() {
        let voice = male_voice();
        let aggressive = MoodVector {
            frustration: 1.0,
            arousal: 1.0,
            ..MoodVector::neutral()
        };
        let mut stress = StressState::new();
        // Drive to burnout
        for _ in 0..2000 {
            stress.tick(&aggressive);
        }
        assert_eq!(stress.level(), StressLevel::Burnout);
        let result = apply_stress_to_voice(&stress, &voice);
        assert!(result.bandwidth_widening > voice.bandwidth_widening);
    }

    // ── quality_from_energy ────────────────────────────────────────

    #[test]
    fn full_energy_full_quality() {
        let energy = EnergyState::default();
        assert_eq!(quality_from_energy(&energy), Quality::Full);
    }

    #[test]
    fn depleted_energy_minimal_quality() {
        let mut energy = EnergyState::default();
        // Drain energy
        for _ in 0..200 {
            energy.tick(1.0);
        }
        assert!(energy.energy.get() < 0.15);
        let q = quality_from_energy(&energy);
        assert!(q == Quality::Reduced || q == Quality::Minimal);
    }

    // ── intonation_from_mood ───────────────────────────────────────

    #[test]
    fn neutral_mood_declarative() {
        assert_eq!(
            intonation_from_mood(&neutral_mood()),
            IntonationPattern::Declarative
        );
    }

    #[test]
    fn excited_happy_exclamatory() {
        let mood = MoodVector {
            joy: 0.8,
            arousal: 0.8,
            ..MoodVector::neutral()
        };
        assert_eq!(intonation_from_mood(&mood), IntonationPattern::Exclamatory);
    }

    #[test]
    fn frustrated_aroused_exclamatory() {
        let mood = MoodVector {
            frustration: 0.8,
            arousal: 0.5,
            ..MoodVector::neutral()
        };
        assert_eq!(intonation_from_mood(&mood), IntonationPattern::Exclamatory);
    }

    #[test]
    fn curious_yielding_interrogative() {
        let mood = MoodVector {
            interest: 0.6,
            dominance: -0.3,
            ..MoodVector::neutral()
        };
        assert_eq!(
            intonation_from_mood(&mood),
            IntonationPattern::Interrogative
        );
    }

    #[test]
    fn assertive_continuation() {
        let mood = MoodVector {
            arousal: 0.5,
            dominance: 0.3,
            ..MoodVector::neutral()
        };
        assert_eq!(intonation_from_mood(&mood), IntonationPattern::Continuation);
    }

    // ── proptest ───────────────────────────────────────────────────

    mod proptest_tests {
        use super::*;
        use proptest::prelude::*;

        fn arb_mood() -> impl Strategy<Value = MoodVector> {
            (
                -1.0f32..=1.0,
                -1.0f32..=1.0,
                -1.0f32..=1.0,
                -1.0f32..=1.0,
                -1.0f32..=1.0,
                -1.0f32..=1.0,
            )
                .prop_map(|(j, a, d, t, i, f)| MoodVector {
                    joy: j,
                    arousal: a,
                    dominance: d,
                    trust: t,
                    interest: i,
                    frustration: f,
                })
        }

        fn arb_profile() -> impl Strategy<Value = PersonalityProfile> {
            proptest::collection::vec(
                prop_oneof![
                    Just(TraitLevel::Lowest),
                    Just(TraitLevel::Low),
                    Just(TraitLevel::Balanced),
                    Just(TraitLevel::High),
                    Just(TraitLevel::Highest),
                ],
                15,
            )
            .prop_map(|levels| {
                let mut p = PersonalityProfile::new("proptest");
                for (kind, level) in bhava::traits::TraitKind::ALL.iter().zip(levels) {
                    p.set_trait(*kind, level);
                }
                p
            })
        }

        proptest! {
            #[test]
            fn mood_to_voice_always_valid(mood in arb_mood()) {
                let voice = VoiceProfile::new_male();
                let result = apply_mood_to_voice(&mood, &voice);
                prop_assert!(result.base_f0.is_finite() && result.base_f0 >= F0_FLOOR_HZ);
                prop_assert!(result.breathiness >= 0.0 && result.breathiness <= 1.0);
                prop_assert!(result.f0_range >= 0.0);
                prop_assert!(result.jitter >= 0.0 && result.jitter <= 0.05);
                prop_assert!(result.shimmer >= 0.0 && result.shimmer <= 0.1);
                prop_assert!(result.vibrato_depth >= 0.0 && result.vibrato_depth <= 0.5);
            }

            #[test]
            fn mood_to_prosody_always_valid(mood in arb_mood()) {
                let contour = prosody_from_mood(&mood);
                for &(t, v) in &contour.f0_points {
                    prop_assert!((0.0..=1.0).contains(&t), "time out of range: {}", t);
                    prop_assert!(v >= F0_MULTIPLIER_FLOOR && v.is_finite(),
                        "f0 multiplier invalid: {}", v);
                }
                prop_assert!(contour.duration_scale >= 0.7 && contour.duration_scale <= 1.4);
                prop_assert!(contour.amplitude_scale >= 0.7 && contour.amplitude_scale <= 1.5);
            }

            #[test]
            fn mood_to_effort_always_valid(mood in arb_mood()) {
                let _effort = effort_from_mood(&mood);
                // No panic = valid variant returned
            }

            #[test]
            fn mood_to_intonation_always_valid(mood in arb_mood()) {
                let _pattern = intonation_from_mood(&mood);
            }

            #[test]
            fn personality_to_voice_always_valid(profile in arb_profile()) {
                let base = VoiceProfile::new_male();
                let result = voice_from_personality(&profile, &base);
                prop_assert!(result.base_f0.is_finite() && result.base_f0 > 0.0);
                prop_assert!(result.breathiness >= 0.0 && result.breathiness <= 1.0);
                prop_assert!(result.f0_range >= 0.0);
                prop_assert!(result.jitter >= 0.0 && result.jitter <= 0.05);
                prop_assert!(result.shimmer >= 0.0 && result.shimmer <= 0.1);
                prop_assert!(result.vibrato_rate >= 0.0);
                prop_assert!(result.vibrato_depth >= 0.0 && result.vibrato_depth <= 0.5);
                prop_assert!(result.formant_scale >= 0.1);
            }
        }
    }
}
