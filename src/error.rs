//! Error types for dhvani.

use thiserror::Error;

/// Errors that can occur during dhvani audio operations.
#[must_use]
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum NadaError {
    /// Buffer format (sample rate, channels) doesn't match expected.
    #[error("buffer format mismatch: expected {expected}, got {actual}")]
    FormatMismatch {
        /// Expected format description.
        expected: String,
        /// Actual format description.
        actual: String,
    },

    /// Buffer sample count doesn't match expected.
    #[error("buffer length mismatch: expected {expected} samples, got {actual}")]
    LengthMismatch {
        /// Expected sample count.
        expected: usize,
        /// Actual sample count.
        actual: usize,
    },

    /// Sample rate is zero or out of supported range.
    #[error("invalid sample rate: {0} Hz")]
    InvalidSampleRate(u32),

    /// Channel count is zero or exceeds maximum.
    #[error("invalid channel count: {0}")]
    InvalidChannels(u32),

    /// DSP processing error.
    #[error("DSP error: {0}")]
    Dsp(String),

    /// Audio capture/output error.
    #[error("capture error: {0}")]
    Capture(String),

    /// A parameter value is out of valid range.
    #[error("invalid parameter: {name} = {value} ({reason})")]
    InvalidParameter {
        /// Parameter name.
        name: String,
        /// Invalid value (as string).
        value: String,
        /// Why the value is invalid.
        reason: String,
    },

    /// Format conversion error.
    #[error("conversion error: {0}")]
    Conversion(String),

    /// Wrapped external error.
    #[error("{0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for NadaError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::Other(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let errors = [
            NadaError::FormatMismatch {
                expected: "44100Hz".into(),
                actual: "48000Hz".into(),
            },
            NadaError::LengthMismatch {
                expected: 1024,
                actual: 512,
            },
            NadaError::InvalidSampleRate(0),
            NadaError::InvalidChannels(0),
            NadaError::Dsp("test".into()),
            NadaError::Capture("test".into()),
            NadaError::InvalidParameter {
                name: "ratio".into(),
                value: "-1".into(),
                reason: "must be positive".into(),
            },
            NadaError::Conversion("test".into()),
        ];
        for e in &errors {
            let msg = e.to_string();
            assert!(!msg.is_empty());
        }
    }

    #[test]
    fn error_from_boxed() {
        let err: Box<dyn std::error::Error + Send + Sync> = "test error".into();
        let nada: NadaError = err.into();
        assert!(nada.to_string().contains("test error"));
    }
}
