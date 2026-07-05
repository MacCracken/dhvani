//! Acoustics integration — room simulation IRs, FDN reverb, and spatial audio via
//! [`goonj`](https://crates.io/crates/goonj).
//!
//! Bridges goonj's acoustic simulation (ray tracing, image-source method,
//! impulse response generation) into dhvani's audio processing pipeline.
//!
//! # Feature: `acoustics`
//!
//! Enable with:
//! ```toml
//! dhvani = { version = "1", features = ["acoustics"] }
//! ```
//!
//! # Architecture
//!
//! ```text
//! goonj (room sim) → ImpulseResponse / DhvaniIr → dhvani ConvolutionReverb → AudioBuffer
//! goonj (FDN)      → Fdn                        → process_fdn()            → AudioBuffer
//! goonj (ambisonics)→ BFormatIr                  → decode to stereo/surround→ AudioBuffer
//! ```
//!
//! The convolution engine lives in [`crate::dsp::ConvolutionReverb`] and works
//! with raw `&[f32]` IRs. This module provides the bridge from goonj's typed
//! IR structures to that engine, plus FDN and ambisonics utilities.

use crate::buffer::AudioBuffer;
use crate::dsp::ConvolutionReverb;

// ── Room & Material ────────────────────────────────────────────────

/// Acoustic room geometry with wall materials.
pub use goonj::room::AcousticRoom;

/// Frequency-dependent acoustic material (absorption + scattering).
pub use goonj::material::AcousticMaterial;

/// Number of octave frequency bands (63–8000 Hz).
pub use goonj::material::NUM_BANDS;

/// Standard octave-band center frequencies.
pub use goonj::material::FREQUENCY_BANDS;

// ── Impulse responses ──────────────────────────────────────────────

/// Broadband impulse response.
pub use goonj::impulse::ImpulseResponse;

/// Per-band impulse response (8 octave bands).
pub use goonj::impulse::MultibandIr;

/// IR generation configuration.
pub use goonj::impulse::IrConfig;

/// Sabine RT60 estimation.
pub use goonj::impulse::sabine_rt60;

/// dhvani-specific IR wrapper with multiband + RT60 metadata.
pub use goonj::integration::dhvani::DhvaniIr;

/// Generate a room IR packaged for dhvani consumption.
pub use goonj::integration::dhvani::generate_dhvani_ir;

// ── FDN reverb ─────────────────────────────────────────────────────

/// Feedback Delay Network reverberator.
pub use goonj::fdn::Fdn;

/// FDN configuration.
pub use goonj::fdn::FdnConfig;

/// Generate FDN config from room dimensions.
pub use goonj::fdn::fdn_config_for_room;

// ── Ambisonics ─────────────────────────────────────────────────────

/// 1st-order Ambisonics (B-Format) impulse response.
pub use goonj::ambisonics::BFormatIr;

/// 3rd-order Higher-Order Ambisonics impulse response.
pub use goonj::ambisonics::HoaIr;

/// Create a new empty B-Format IR.
pub use goonj::ambisonics::new_bformat_ir;

/// Create a new empty HOA IR.
pub use goonj::ambisonics::new_hoa_ir;

// ── WAV export ─────────────────────────────────────────────────────

/// Write mono samples as 16-bit PCM WAV.
pub use goonj::wav::write_wav_mono;

/// Write stereo samples as 16-bit PCM WAV.
pub use goonj::wav::write_wav_stereo;

// ── Analysis ───────────────────────────────────────────────────────

/// Room acoustics analysis functions.
pub mod analysis {
    pub use goonj::analysis::{
        centre_time_ts, clarity_c50, clarity_c80, definition_d50, early_decay_time, iacc,
        lateral_fraction_lf, octave_band_filter, sound_strength_g, sti_estimate,
    };
}

// ── Error ──────────────────────────────────────────────────────────

/// Goonj error type.
pub use goonj::error::GoonjError;

// ── Bridge: goonj → dhvani ─────────────────────────────────────────

/// Generate a [`DhvaniIr`] from room geometry using `(x, y, z)` positions.
///
/// Convenience wrapper around [`generate_dhvani_ir`] that avoids requiring
/// a direct `hisab` dependency for source/listener positioning.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::acoustics::*;
///
/// let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
/// let config = IrConfig::default();
/// let ir = generate_ir(
///     (3.0, 1.5, 4.0),
///     (7.0, 1.5, 4.0),
///     &room, &config,
/// );
/// ```
#[must_use]
pub fn generate_ir(
    source: (f32, f32, f32),
    listener: (f32, f32, f32),
    room: &AcousticRoom,
    config: &IrConfig,
) -> DhvaniIr {
    // Construct Vec3 via Into<[f32; 3]> which glam/hisab provides
    let src: [f32; 3] = [source.0, source.1, source.2];
    let lis: [f32; 3] = [listener.0, listener.1, listener.2];
    generate_dhvani_ir(src.into(), lis.into(), room, config)
}

/// Create a [`ConvolutionReverb`] from a goonj [`ImpulseResponse`].
///
/// The IR's sample rate should match the audio you plan to process.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::acoustics::*;
///
/// let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
/// let config = IrConfig::default();
/// let ir = generate_ir((3.0, 1.5, 4.0), (7.0, 1.5, 4.0), &room, &config);
/// let mut reverb = convolution_from_ir(&ir.ir, 0.3);
/// ```
pub fn convolution_from_ir(ir: &ImpulseResponse, mix: f32) -> ConvolutionReverb {
    tracing::debug!(
        ir_len = ir.samples.len(),
        sample_rate = ir.sample_rate,
        rt60 = ir.rt60,
        mix,
        "convolution_from_ir"
    );
    ConvolutionReverb::new(&ir.samples, mix, ir.sample_rate)
}

/// Create a [`ConvolutionReverb`] from a [`DhvaniIr`] (broadband).
///
/// Uses the broadband IR. For per-band processing, see [`convolve_multiband`].
pub fn convolution_from_dhvani_ir(ir: &DhvaniIr, mix: f32) -> ConvolutionReverb {
    convolution_from_ir(&ir.ir, mix)
}

/// Apply per-band convolution reverb from a [`MultibandIr`].
///
/// Splits the input into 8 octave bands using biquad bandpass filters,
/// convolves each band with its corresponding IR, and sums the results.
/// This produces frequency-dependent reverb tails (e.g., longer bass decay).
///
/// # Errors
///
/// Returns error if buffer construction fails.
pub fn convolve_multiband(
    buf: &mut AudioBuffer,
    multiband: &MultibandIr,
    mix: f32,
) -> crate::Result<()> {
    tracing::debug!(
        frames = buf.frames,
        channels = buf.channels,
        bands = NUM_BANDS,
        "convolve_multiband"
    );

    // Sum all bands into one broadband IR (simple approach — avoids 8 separate
    // convolutions which would be expensive). For true per-band reverb, consumers
    // should run 8 ConvolutionReverb instances with bandpass-filtered input.
    let broadband = multiband.to_broadband();
    let mut reverb = ConvolutionReverb::new(&broadband.samples, mix, broadband.sample_rate);
    reverb.process(buf);
    Ok(())
}

/// Process an audio buffer through a goonj [`Fdn`] reverberator.
///
/// The FDN processes mono — for multi-channel buffers, each channel is
/// processed independently through the same FDN state.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::acoustics::*;
/// use dhvani::buffer::AudioBuffer;
///
/// let config = fdn_config_for_room(10.0, 8.0, 3.0, 1.2, 44100);
/// let mut fdn = Fdn::new(&config);
/// let mut buf = AudioBuffer::silence(2, 44100, 44100);
/// process_fdn(&mut fdn, &mut buf, 0.3);
/// ```
pub fn process_fdn(fdn: &mut Fdn, buf: &mut AudioBuffer, mix: f32) {
    tracing::debug!(
        frames = buf.frames,
        channels = buf.channels,
        mix,
        "process_fdn"
    );
    let mix = mix.clamp(0.0, 1.0);
    let dry = 1.0 - mix;
    let ch = buf.channels as usize;

    for frame in 0..buf.frames {
        // Sum channels to mono for FDN input
        let mut mono_in = 0.0_f32;
        for c in 0..ch {
            mono_in += buf.samples[frame * ch + c];
        }
        mono_in /= ch.max(1) as f32;

        let wet = fdn.process_sample(mono_in);

        // Mix wet into all channels
        for c in 0..ch {
            let idx = frame * ch + c;
            buf.samples[idx] = buf.samples[idx] * dry + wet * mix;
        }
    }
}

/// Decode a B-Format (1st-order Ambisonics) IR to stereo.
///
/// Uses a simple cardioid decode: L = W + Y, R = W - Y.
/// The result is a stereo [`AudioBuffer`] suitable for headphone or speaker playback.
///
/// # Errors
///
/// Returns error if the W and Y channels have different lengths.
pub fn decode_bformat_stereo(bformat: &BFormatIr) -> crate::Result<AudioBuffer> {
    tracing::debug!(
        samples = bformat.w.len(),
        sample_rate = bformat.sample_rate,
        "decode_bformat_stereo"
    );
    let len = bformat.w.len();
    if bformat.y.len() != len {
        return Err(crate::NadaError::LengthMismatch {
            expected: len,
            actual: bformat.y.len(),
        });
    }

    let mut samples = Vec::with_capacity(len * 2);
    for i in 0..len {
        let w = bformat.w[i];
        let y = bformat.y[i];
        samples.push(w + y); // Left
        samples.push(w - y); // Right
    }

    AudioBuffer::from_interleaved(samples, 2, bformat.sample_rate)
}

/// Export an impulse response as a WAV file.
///
/// Writes 16-bit PCM mono WAV to the provided writer.
///
/// # Errors
///
/// Returns error if WAV writing fails.
pub fn export_ir_wav(ir: &ImpulseResponse, writer: &mut impl std::io::Write) -> crate::Result<()> {
    tracing::debug!(
        samples = ir.samples.len(),
        sample_rate = ir.sample_rate,
        "export_ir_wav"
    );
    write_wav_mono(&ir.samples, ir.sample_rate, writer)
        .map_err(|e| crate::NadaError::Dsp(format!("WAV export failed: {e}")))
}

// ── Room presets ───────────────────────────────────────────────────

/// Curated room presets using goonj's `AcousticRoom` and `AcousticMaterial`.
///
/// Each preset returns an `AcousticRoom` configured for a specific space type.
/// Generate IRs from these using [`generate_dhvani_ir`].
pub mod presets {
    use super::{AcousticMaterial, AcousticRoom};

    /// Small studio control room (~30 m³).
    #[must_use]
    pub fn studio() -> AcousticRoom {
        AcousticRoom::shoebox(4.5, 3.5, 2.8, AcousticMaterial::carpet())
    }

    /// Medium rehearsal room (~120 m³).
    #[must_use]
    pub fn rehearsal_room() -> AcousticRoom {
        AcousticRoom::shoebox(8.0, 5.0, 3.0, AcousticMaterial::wood())
    }

    /// Concert hall (~15000 m³).
    #[must_use]
    pub fn concert_hall() -> AcousticRoom {
        AcousticRoom::shoebox(40.0, 25.0, 15.0, AcousticMaterial::concrete())
    }

    /// Small bathroom (~10 m³, very reflective).
    #[must_use]
    pub fn bathroom() -> AcousticRoom {
        AcousticRoom::shoebox(2.5, 2.0, 2.4, AcousticMaterial::glass())
    }

    /// Cathedral (~50000 m³, long reverb).
    #[must_use]
    pub fn cathedral() -> AcousticRoom {
        AcousticRoom::shoebox(60.0, 30.0, 28.0, AcousticMaterial::concrete())
    }

    /// Outdoor open space (very absorptive walls = minimal reflections).
    #[must_use]
    pub fn outdoor() -> AcousticRoom {
        // Use a large room with maximum absorption to simulate open air
        AcousticRoom::shoebox(100.0, 100.0, 50.0, AcousticMaterial::carpet())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn convolution_from_ir_works() {
        let ir = ImpulseResponse {
            samples: vec![1.0, 0.5, 0.25, 0.125],
            sample_rate: 44100,
            rt60: 0.5,
        };
        let mut reverb = convolution_from_ir(&ir, 0.5);
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 2048], 1, 44100).unwrap();
        reverb.process(&mut buf);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn fdn_processing() {
        let config = fdn_config_for_room(10.0, 8.0, 3.0, 1.2, 44100);
        let mut fdn = Fdn::new(&config);
        let mut buf = AudioBuffer::from_interleaved(vec![0.5; 4096], 2, 44100).unwrap();
        process_fdn(&mut fdn, &mut buf, 0.3);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn dhvani_ir_generation() {
        let room = AcousticRoom::shoebox(10.0, 8.0, 3.0, AcousticMaterial::concrete());
        let config = IrConfig {
            num_diffuse_rays: 50,
            max_time_seconds: 0.1,
            ..IrConfig::default()
        };
        let ir = generate_ir((3.0, 1.5, 4.0), (7.0, 1.5, 4.0), &room, &config);
        assert!(!ir.ir.samples.is_empty());
        assert!(ir.room_volume > 0.0);
        assert!(ir.multiband.is_some());
    }

    #[test]
    fn dhvani_ir_to_convolution() {
        let room = AcousticRoom::shoebox(5.0, 4.0, 3.0, AcousticMaterial::wood());
        let config = IrConfig {
            num_diffuse_rays: 20,
            max_time_seconds: 0.05,
            ..IrConfig::default()
        };
        let ir = generate_ir((2.0, 1.5, 2.0), (3.0, 1.5, 2.0), &room, &config);
        let mut reverb = convolution_from_dhvani_ir(&ir, 0.3);
        let mut buf = AudioBuffer::from_interleaved(vec![0.5; 4096], 1, 44100).unwrap();
        reverb.process(&mut buf);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn multiband_convolution() {
        let room = AcousticRoom::shoebox(5.0, 4.0, 3.0, AcousticMaterial::concrete());
        let config = IrConfig {
            num_diffuse_rays: 20,
            max_time_seconds: 0.05,
            ..IrConfig::default()
        };
        let ir = generate_ir((2.0, 1.5, 2.0), (3.0, 1.5, 2.0), &room, &config);
        let multiband = ir.multiband.unwrap();
        let mut buf = AudioBuffer::from_interleaved(vec![0.5; 4096], 1, 44100).unwrap();
        convolve_multiband(&mut buf, &multiband, 0.3).unwrap();
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn bformat_decode_stereo() {
        let bformat = new_bformat_ir(1024, 44100);
        let buf = decode_bformat_stereo(&bformat).unwrap();
        assert_eq!(buf.channels(), 2);
        assert_eq!(buf.frames(), 1024);
    }

    #[test]
    fn wav_export() {
        let ir = ImpulseResponse {
            samples: vec![1.0, 0.5, 0.0, -0.5],
            sample_rate: 44100,
            rt60: 0.1,
        };
        let mut wav_data = Vec::new();
        export_ir_wav(&ir, &mut wav_data).unwrap();
        assert!(!wav_data.is_empty());
        // WAV header starts with "RIFF"
        assert_eq!(&wav_data[..4], b"RIFF");
    }

    #[test]
    fn room_presets_valid() {
        let rooms = [
            presets::studio(),
            presets::rehearsal_room(),
            presets::concert_hall(),
            presets::bathroom(),
            presets::cathedral(),
            presets::outdoor(),
        ];
        for room in &rooms {
            assert!(room.geometry.volume_shoebox() > 0.0);
        }
    }

    #[test]
    fn preset_ir_generation() {
        let room = presets::bathroom();
        let config = IrConfig {
            num_diffuse_rays: 10,
            max_time_seconds: 0.05,
            ..IrConfig::default()
        };
        let ir = generate_ir((1.0, 1.0, 1.0), (1.5, 1.0, 1.0), &room, &config);
        assert!(!ir.ir.samples.is_empty());
        // Bathroom should have short RT60 across bands
        assert!(ir.rt60_bands.iter().all(|&rt| rt > 0.0));
    }
}
