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
//! - [`buffer`] — Audio buffer types (interleaved/planar, f32/i16/i32)
//! - [`dsp`] — Effects: EQ, compressor, limiter, gate, reverb, noise suppression
//! - [`analysis`] — FFT, spectrum, loudness (LUFS), peak, RMS
//! - [`clock`] — Sample-accurate transport clock, tempo, A/V sync
//!
//! ## Feature Flags
//!
//! - `simd` (default) — SIMD acceleration for mixing, resampling, DSP
//! - `pipewire` — PipeWire capture/output backend

pub mod analysis;
pub mod buffer;
pub mod clock;
pub mod dsp;

mod error;
pub use error::NadaError;

/// Result type alias for nada operations.
pub type Result<T> = std::result::Result<T, NadaError>;

#[cfg(test)]
mod tests;
