//! Sample playback engine — key/velocity zones, loop modes, time-stretching.
//!
//! Re-exports from [`nidhi`](https://crates.io/crates/nidhi), the AGNOS sample playback crate.
//!
//! # Feature: `sampler`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["sampler"] }
//! ```
//!
//! # Architecture
//!
//! ```text
//! SampleBank (loaded samples) → Instrument (zones) → SamplerEngine (voices) → AudioBuffer
//! ```

use crate::buffer::AudioBuffer;

// ── Core types ──────────────────────────────────────────────────────

/// A loaded audio sample.
pub use nidhi::sample::{Sample, SampleBank, SampleId};

/// Key/velocity zone mapping.
pub use nidhi::zone::Zone;

/// Loop modes (one-shot, forward, ping-pong, reverse).
pub use nidhi::loop_mode::LoopMode;

/// A sampled instrument (collection of zones).
pub use nidhi::instrument::Instrument;

/// Polyphonic sampler engine.
pub use nidhi::engine::{SamplerEngine, SamplerVoice};

/// Time-stretch algorithm selection.
pub use nidhi::stretch::StretchMode;

/// Nidhi error type.
pub use nidhi::error::NidhiError;

// ── Bridge: nidhi engine → dhvani AudioBuffer ───────────────────────

/// Render the sampler engine output to a dhvani [`AudioBuffer`].
///
/// Generates `frames` of mono audio from all active voices.
pub fn render_to_buffer(
    engine: &mut SamplerEngine,
    channels: u32,
    frames: usize,
    sample_rate: u32,
) -> AudioBuffer {
    let ch = channels as usize;
    let mut samples = Vec::with_capacity(frames * ch);
    for _ in 0..frames {
        let s = engine.next_sample();
        for _ in 0..ch {
            samples.push(s);
        }
    }
    AudioBuffer::from_interleaved(samples, channels, sample_rate)
        .unwrap_or_else(|_| AudioBuffer::silence(channels.max(1), frames, sample_rate.max(1)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> SamplerEngine {
        let mut bank = SampleBank::new();
        let sine: Vec<f32> = (0..44100)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        let id = bank.add(Sample::from_mono(sine, 44100));

        let mut inst = Instrument::new("test");
        inst.add_zone(Zone::new(id).with_key_range(0, 127).with_root_note(69));

        let mut engine = SamplerEngine::new(8, 44100.0);
        engine.set_bank(bank);
        engine.set_instrument(inst);
        engine
    }

    #[test]
    fn sampler_to_buffer() {
        let mut engine = make_engine();
        engine.note_on(69, 100);
        let buf = render_to_buffer(&mut engine, 2, 4410, 44100);
        assert_eq!(buf.channels(), 2);
        assert_eq!(buf.frames(), 4410);
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn sampler_pitch_shift() {
        let mut engine = make_engine();
        engine.note_on(81, 100); // octave up
        let buf = render_to_buffer(&mut engine, 1, 4410, 44100);
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn sampler_note_off() {
        let mut engine = make_engine();
        engine.note_on(69, 100);
        engine.note_off(69);
        // Process through release
        let buf = render_to_buffer(&mut engine, 1, 44100, 44100);
        // Should have faded out
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }
}
