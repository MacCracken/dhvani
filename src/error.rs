//! Error types for nada.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NadaError {
    #[error("buffer format mismatch: expected {expected}, got {actual}")]
    FormatMismatch { expected: String, actual: String },

    #[error("buffer length mismatch: expected {expected} samples, got {actual}")]
    LengthMismatch { expected: usize, actual: usize },

    #[error("invalid sample rate: {0} Hz")]
    InvalidSampleRate(u32),

    #[error("invalid channel count: {0}")]
    InvalidChannels(u32),

    #[error("DSP error: {0}")]
    Dsp(String),

    #[error("capture error: {0}")]
    Capture(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
