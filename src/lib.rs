//! # Nada — Core Audio Engine
//!
//! Nada (नाद, Sanskrit: primordial sound) provides shared audio processing
//! primitives for the AGNOS ecosystem. It eliminates duplicate implementations
//! across [shruti](https://github.com/MacCracken/shruti) (DAW),
//! [jalwa](https://github.com/MacCracken/jalwa) (media player),
//! [aethersafta](https://github.com/MacCracken/aethersafta) (compositor),
//! and [tarang](https://crates.io/crates/tarang) (media framework).
//!
//! ## Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`buffer`] | Audio buffers, mixing, resampling, format conversion |
//! | [`dsp`] | Biquad filters, parametric EQ, compressor, reverb, delay, de-esser, gate, limiter |
//! | [`analysis`] | DFT spectrum, loudness (LUFS), silence detection |
//! | [`clock`] | Sample-accurate transport clock, tempo, beats, PTS, A/V sync |
//! | [`midi`] | MIDI 1.0/2.0 events, clips, voice management, routing |
//! | [`capture`] | PipeWire audio capture/output (requires `pipewire` feature) |
//! | [`ffi`] | C-compatible FFI for key types |
//!
//! ## Quick Start
//!
//! ```rust
//! use nada::buffer::{AudioBuffer, mix};
//! use nada::dsp::{self, ParametricEq, EqBandConfig, BandType, Compressor, CompressorParams, Reverb, ReverbParams};
//! use nada::analysis;
//!
//! // Create buffers
//! let vocals = AudioBuffer::from_interleaved(vec![0.5; 4096], 2, 44100).unwrap();
//! let drums = AudioBuffer::from_interleaved(vec![0.3; 4096], 2, 44100).unwrap();
//!
//! // Mix sources
//! let mut mixed = mix(&[&vocals, &drums]).unwrap();
//!
//! // Parametric EQ (3-band)
//! let mut eq = ParametricEq::new(vec![
//!     EqBandConfig { band_type: BandType::HighPass, freq_hz: 80.0, gain_db: 0.0, q: 0.707, enabled: true },
//!     EqBandConfig { band_type: BandType::Peaking, freq_hz: 3000.0, gain_db: 3.0, q: 1.5, enabled: true },
//!     EqBandConfig { band_type: BandType::HighShelf, freq_hz: 10000.0, gain_db: -2.0, q: 0.707, enabled: true },
//! ], 44100, 2);
//! eq.process(&mut mixed);
//!
//! // Compress
//! let mut comp = Compressor::new(CompressorParams {
//!     threshold_db: -18.0, ratio: 4.0, attack_ms: 10.0, release_ms: 100.0,
//!     makeup_gain_db: 3.0, knee_db: 6.0,
//! }, 44100);
//! comp.process(&mut mixed);
//!
//! // Normalize and analyze
//! dsp::normalize(&mut mixed, 0.95);
//! let loudness = analysis::loudness_lufs(&mixed);
//! println!("Peak: {:.2}, LUFS: {:.1}", mixed.peak(), loudness);
//! ```
//!
//! ## Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `simd` | Yes | SIMD acceleration for mixing, resampling, DSP |
//! | `pipewire` | No | PipeWire capture/output backend (Linux) |
//! | `full` | No | All features enabled |

pub mod analysis;
pub mod buffer;
pub mod capture;
pub mod clock;
pub mod dsp;
pub mod ffi;
pub mod midi;

#[cfg(feature = "simd")]
pub(crate) mod simd;

mod error;
pub use error::NadaError;

/// Result type alias for nada operations.
pub type Result<T> = std::result::Result<T, NadaError>;

#[cfg(test)]
mod tests;
