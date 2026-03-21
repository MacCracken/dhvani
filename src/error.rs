//! Error types for nada.

use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
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

    #[error("invalid parameter: {name} = {value} ({reason})")]
    InvalidParameter {
        name: String,
        value: String,
        reason: String,
    },

    #[error("conversion error: {0}")]
    Conversion(String),

    #[error("{0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for NadaError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Other(err)
    }
}
