//! PipeWire audio capture and output.
//!
//! Requires the `pipewire` feature flag. Provides device enumeration,
//! per-source audio capture, output to PipeWire sinks, and hot-plug detection.
//!
//! # Architecture
//!
//! PipeWire runs an event loop on a dedicated thread. The capture module
//! communicates with it via channels:
//!
//! ```text
//! PipeWire thread ──[AudioBuffer]──► capture channel ──► consumer
//! ```

#[cfg(feature = "pipewire")]
mod pw;

#[cfg(feature = "pipewire")]
pub use pw::*;

use serde::{Deserialize, Serialize};

/// Description of an audio device (source or sink).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDevice {
    /// Device unique identifier.
    pub id: u32,
    /// Human-readable device name.
    pub name: String,
    /// Device type (source or sink).
    pub device_type: DeviceType,
    /// Number of channels.
    pub channels: u32,
    /// Sample rate in Hz.
    pub sample_rate: u32,
}

/// Whether a device is an audio source (input) or sink (output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DeviceType {
    /// Audio input (microphone, line-in, desktop capture).
    Source,
    /// Audio output (speakers, headphones).
    Sink,
}

/// Configuration for a capture session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    /// Target sample rate (Hz). 0 = use device default.
    pub sample_rate: u32,
    /// Number of channels. 0 = use device default.
    pub channels: u32,
    /// Buffer size in frames per callback.
    pub buffer_frames: u32,
    /// Device ID to capture from. None = default device.
    pub device_id: Option<u32>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            buffer_frames: 1024,
            device_id: None,
        }
    }
}

/// Configuration for an output session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Target sample rate (Hz).
    pub sample_rate: u32,
    /// Number of channels.
    pub channels: u32,
    /// Buffer size in frames per callback.
    pub buffer_frames: u32,
    /// Device ID to output to. None = default device.
    pub device_id: Option<u32>,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            buffer_frames: 1024,
            device_id: None,
        }
    }
}

/// Events from the capture/output system.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum CaptureEvent {
    /// A new device was connected.
    DeviceAdded(AudioDevice),
    /// A device was disconnected.
    DeviceRemoved { id: u32 },
    /// Capture buffer overflow (data was lost).
    Overflow,
    /// Output buffer underrun (silence was inserted).
    Underrun,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_config_default() {
        let cfg = CaptureConfig::default();
        assert_eq!(cfg.sample_rate, 48000);
        assert_eq!(cfg.channels, 2);
        assert_eq!(cfg.buffer_frames, 1024);
        assert!(cfg.device_id.is_none());
    }

    #[test]
    fn output_config_default() {
        let cfg = OutputConfig::default();
        assert_eq!(cfg.sample_rate, 48000);
        assert_eq!(cfg.channels, 2);
    }

    #[test]
    fn device_type_equality() {
        assert_eq!(DeviceType::Source, DeviceType::Source);
        assert_ne!(DeviceType::Source, DeviceType::Sink);
    }

    #[test]
    fn audio_device_serde() {
        let dev = AudioDevice {
            id: 42,
            name: "Built-in Mic".into(),
            device_type: DeviceType::Source,
            channels: 2,
            sample_rate: 48000,
        };
        let json = serde_json::to_string(&dev).unwrap();
        let back: AudioDevice = serde_json::from_str(&json).unwrap();
        assert_eq!(back.id, 42);
        assert_eq!(back.name, "Built-in Mic");
    }
}
