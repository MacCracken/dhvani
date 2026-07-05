//! Synthesis engines — subtractive, FM, additive, wavetable, granular, physical modeling, drum, vocoder.
//!
//! Re-exports from [`naad`](https://crates.io/crates/naad), the AGNOS synthesis primitives crate.
//! All synthesis types are sample-based (`next_sample()`) and composable with dhvani's
//! buffer-based DSP via [`render_to_buffer`].
//!
//! # Feature: `synthesis`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["synthesis"] }
//! ```

use crate::buffer::AudioBuffer;

// ── Core primitives ─────────────────────────────────────────────────

/// Oscillator — band-limited waveform generation with PolyBLEP.
pub use naad::oscillator::{Oscillator, Waveform, polyblep};

/// Filters — biquad (Direct Form II) and state-variable (Cytomic SVF).
pub use naad::filter::{BiquadFilter, FilterType, StateVariableFilter, SvfOutput};

/// ADSR envelope and multi-stage envelope generators.
pub use naad::envelope::{Adsr, EnvelopeSegment, EnvelopeState, MultiStageEnvelope};

/// Wavetable synthesis with morphing between tables.
pub use naad::wavetable::{MorphWavetable, Wavetable, WavetableOscillator};

/// LFO and FM modulation.
pub use naad::modulation::{Lfo, LfoMode, LfoShape};

/// Noise generators (white, pink, brown).
pub use naad::noise::{NoiseGenerator, NoiseType};

/// Modulation routing matrix (8×8 source→destination).
pub use naad::mod_matrix::{ModDestination, ModMatrix, ModRouting, ModSource};

/// Polyphonic voice management.
pub use naad::voice::{PolyMode, StealMode, Voice, VoiceManager};

/// Parameter smoothing for glitch-free automation.
pub use naad::smoothing::ParamSmoother;

/// Pitch and tuning utilities.
pub use naad::tuning::{
    TuningSystem, TuningTable, cents, equal_temperament_freq, freq_to_midi, midi_to_freq,
};

/// Stereo panning.
pub use naad::panning::{PanGains, PanLaw, pan_gains, pan_mono, stereo_balance};

/// DSP utilities (dB conversion, interpolation, crossfade).
pub use naad::dsp_util;

/// Delay line and comb filter primitives.
pub use naad::delay::{CombFilter, DelayLine as SynthDelayLine};

/// Dynamics processing (compressor, limiter, gate).
pub use naad::dynamics::{
    Compressor as SynthCompressor, EnvelopeDetector, Limiter as SynthLimiter, NoiseGate,
};

/// Parametric EQ.
pub use naad::eq::{EqBand, ParametricEq as SynthParametricEq};

/// Algorithmic reverb.
pub use naad::reverb::Reverb as SynthReverb;

/// Effects (chorus, flanger, phaser, distortion).
pub use naad::effects::{Chorus, Distortion, DistortionType, Flanger, Phaser};

/// Denormal flushing utility.
pub use naad::flush_denormal;

/// Naad error type.
pub use naad::NaadError;

// ── Synthesis engines ───────────────────────────────────────────────

/// Subtractive synthesis — oscillators + filter + ADSR.
pub use naad::synth::subtractive::SubtractiveSynth;

/// FM synthesis — multi-operator with algorithm selection.
pub use naad::synth::fm::{FmAlgorithm, FmOperator, FmSynthEngine as FmSynth};

/// Additive synthesis — harmonic partials with individual control.
pub use naad::synth::additive::{AdditiveSynth, Partial};

/// Drum synthesis — kick, snare, hi-hat.
pub use naad::synth::drum::{HiHat, KickDrum, SnareDrum};

/// Formant synthesis — vowel filtering for vocal-like sounds.
pub use naad::synth::formant::{FormantFilter as SynthFormantFilter, Vowel as SynthVowel};

/// Channel vocoder — analysis/synthesis filter bank.
pub use naad::synth::vocoder::Vocoder;

/// Granular synthesis — grain cloud engine.
pub use naad::synth::granular::{GrainWindow, GranularEngine};

/// Physical modeling — Karplus-Strong plucked string.
pub use naad::synth::physical::KarplusStrong;

// ── Bridge: sample-based synthesis → dhvani AudioBuffer ─────────────

/// Render a sample-generating closure into a dhvani [`AudioBuffer`].
///
/// The closure is called once per frame and should return one sample.
/// For stereo output, the mono signal is duplicated to both channels.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::synthesis::{Oscillator, Waveform, render_to_buffer};
///
/// let mut osc = Oscillator::new(Waveform::Saw, 440.0, 44100.0).unwrap();
/// let buf = render_to_buffer(|| osc.next_sample(), 1, 44100, 44100);
/// assert_eq!(buf.frames(), 44100);
/// ```
pub fn render_to_buffer(
    mut sample_fn: impl FnMut() -> f32,
    channels: u32,
    frames: usize,
    sample_rate: u32,
) -> AudioBuffer {
    let ch = channels as usize;
    let mut samples = Vec::with_capacity(frames * ch);
    for _ in 0..frames {
        let s = sample_fn();
        for _ in 0..ch {
            samples.push(s);
        }
    }
    // channels/sample_rate are validated by caller context; silence fallback is safe
    AudioBuffer::from_interleaved(samples, channels, sample_rate)
        .unwrap_or_else(|_| AudioBuffer::silence(channels.max(1), frames, sample_rate.max(1)))
}

/// Render a stereo sample-generating closure into a dhvani [`AudioBuffer`].
///
/// The closure returns `(left, right)` per frame.
pub fn render_stereo_to_buffer(
    mut sample_fn: impl FnMut() -> (f32, f32),
    frames: usize,
    sample_rate: u32,
) -> AudioBuffer {
    let mut samples = Vec::with_capacity(frames * 2);
    for _ in 0..frames {
        let (l, r) = sample_fn();
        samples.push(l);
        samples.push(r);
    }
    AudioBuffer::from_interleaved(samples, 2, sample_rate)
        .unwrap_or_else(|_| AudioBuffer::silence(2, frames, sample_rate.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_sine_to_buffer() {
        let mut osc = Oscillator::new(Waveform::Sine, 440.0, 44100.0).unwrap();
        let buf = render_to_buffer(|| osc.next_sample(), 2, 4410, 44100);
        assert_eq!(buf.channels(), 2);
        assert_eq!(buf.frames(), 4410);
        assert!(buf.peak() > 0.9);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn render_fm_synth() {
        let mut fm = FmSynth::new(2, 44100.0).unwrap();
        fm.set_algorithm(FmAlgorithm::Serial2);
        fm.operator_mut(0).unwrap().set_frequency(440.0);
        fm.operator_mut(0).unwrap().set_level(1.0);
        fm.operator_mut(1).unwrap().set_frequency(880.0);
        fm.operator_mut(1).unwrap().set_level(0.5);
        fm.note_on();
        let buf = render_to_buffer(|| fm.next_sample(), 1, 4410, 44100);
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn render_stereo_panned() {
        let mut osc = Oscillator::new(Waveform::Saw, 220.0, 44100.0).unwrap();
        let buf = render_stereo_to_buffer(
            || {
                let s = osc.next_sample();
                let gains = pan_gains(0.7, PanLaw::EqualPower);
                (s * gains.left, s * gains.right)
            },
            4410,
            44100,
        );
        assert_eq!(buf.channels(), 2);
        assert!(buf.peak() > 0.5);
    }

    #[test]
    fn subtractive_synth_works() {
        let mut synth = SubtractiveSynth::new(Waveform::Saw, 440.0, 2000.0, 0.7, 44100.0).unwrap();
        synth.note_on();
        let buf = render_to_buffer(|| synth.next_sample(), 1, 4410, 44100);
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn granular_engine_works() {
        // Source: 1 second of sine
        let source: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        let mut engine = GranularEngine::new(44100.0);
        engine.set_source(source);
        engine.set_grain_rate(20.0);
        engine.set_grain_duration(50.0);
        let buf = render_to_buffer(|| engine.next_sample(), 1, 4410, 44100);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn karplus_strong_works() {
        let mut ks = KarplusStrong::new(440.0, 0.99, 0.5, 44100.0).unwrap();
        ks.pluck();
        let buf = render_to_buffer(|| ks.next_sample(), 1, 4410, 44100);
        assert!(buf.peak() > 0.0);
    }

    #[test]
    fn drum_kick_works() {
        let mut kick = KickDrum::new(150.0, 40.0, 200.0, 0.8, 44100.0).unwrap();
        kick.trigger();
        let buf = render_to_buffer(|| kick.next_sample(), 1, 4410, 44100);
        assert!(buf.peak() > 0.0);
    }
}
