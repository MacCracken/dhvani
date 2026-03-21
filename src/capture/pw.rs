//! PipeWire backend — device enumeration, capture, output, hot-plug.
//!
//! This module is only compiled when the `pipewire` feature is enabled.

use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use crate::buffer::AudioBuffer;
use crate::NadaError;

use super::{AudioDevice, CaptureConfig, CaptureEvent, DeviceType, OutputConfig};

/// A PipeWire capture stream that receives audio buffers.
pub struct PwCapture {
    config: CaptureConfig,
    /// Receiver for captured audio buffers.
    receiver: mpsc::Receiver<AudioBuffer>,
    /// Sender used by the PipeWire callback (held to keep channel alive).
    _sender: mpsc::Sender<AudioBuffer>,
    /// Event receiver for hot-plug notifications.
    event_receiver: mpsc::Receiver<CaptureEvent>,
    _event_sender: mpsc::Sender<CaptureEvent>,
    running: Arc<Mutex<bool>>,
}

impl PwCapture {
    /// Create a new capture stream with the given configuration.
    ///
    /// The stream does not start capturing until [`start()`](Self::start) is called.
    pub fn new(config: CaptureConfig) -> Result<Self, NadaError> {
        let (sender, receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();

        Ok(Self {
            config,
            receiver,
            _sender: sender,
            event_receiver,
            _event_sender: event_sender,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start capturing audio.
    pub fn start(&mut self) -> Result<(), NadaError> {
        let mut running = self.running.lock().map_err(|e| {
            NadaError::Capture(format!("lock poisoned: {e}"))
        })?;
        if *running {
            return Ok(());
        }

        // TODO: Initialize PipeWire stream with self.config
        // - Create pw::main_loop::MainLoop
        // - Create pw::stream::Stream with audio format
        // - Set process callback that sends AudioBuffers via self._sender
        // - Register hot-plug listener that sends CaptureEvents
        // - Spawn thread running main_loop.run()

        *running = true;
        Ok(())
    }

    /// Stop capturing audio.
    pub fn stop(&mut self) -> Result<(), NadaError> {
        let mut running = self.running.lock().map_err(|e| {
            NadaError::Capture(format!("lock poisoned: {e}"))
        })?;
        *running = false;
        Ok(())
    }

    /// Try to receive a captured audio buffer (non-blocking).
    pub fn try_recv(&self) -> Option<AudioBuffer> {
        self.receiver.try_recv().ok()
    }

    /// Receive the next captured audio buffer (blocking).
    pub fn recv(&self) -> Result<AudioBuffer, NadaError> {
        self.receiver
            .recv()
            .map_err(|e| NadaError::Capture(format!("channel closed: {e}")))
    }

    /// Try to receive a hot-plug event (non-blocking).
    pub fn try_recv_event(&self) -> Option<CaptureEvent> {
        self.event_receiver.try_recv().ok()
    }

    /// Whether the capture stream is running.
    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }

    /// Current capture configuration.
    pub fn config(&self) -> &CaptureConfig {
        &self.config
    }
}

/// A PipeWire output stream that plays audio buffers.
pub struct PwOutput {
    config: OutputConfig,
    /// Sender for audio buffers to be played.
    sender: mpsc::Sender<AudioBuffer>,
    _receiver: mpsc::Receiver<AudioBuffer>,
    running: Arc<Mutex<bool>>,
}

impl PwOutput {
    /// Create a new output stream.
    pub fn new(config: OutputConfig) -> Result<Self, NadaError> {
        let (sender, receiver) = mpsc::channel();

        Ok(Self {
            config,
            sender,
            _receiver: receiver,
            running: Arc::new(Mutex::new(false)),
        })
    }

    /// Start the output stream.
    pub fn start(&mut self) -> Result<(), NadaError> {
        let mut running = self.running.lock().map_err(|e| {
            NadaError::Capture(format!("lock poisoned: {e}"))
        })?;
        if *running {
            return Ok(());
        }

        // TODO: Initialize PipeWire output stream
        // - Create pw::stream::Stream for playback
        // - Set process callback that reads from self._receiver
        // - Spawn thread

        *running = true;
        Ok(())
    }

    /// Stop the output stream.
    pub fn stop(&mut self) -> Result<(), NadaError> {
        let mut running = self.running.lock().map_err(|e| {
            NadaError::Capture(format!("lock poisoned: {e}"))
        })?;
        *running = false;
        Ok(())
    }

    /// Send an audio buffer to the output (non-blocking).
    pub fn send(&self, buf: AudioBuffer) -> Result<(), NadaError> {
        self.sender
            .send(buf)
            .map_err(|e| NadaError::Capture(format!("output channel closed: {e}")))
    }

    /// Whether the output stream is running.
    pub fn is_running(&self) -> bool {
        self.running.lock().map(|r| *r).unwrap_or(false)
    }

    /// Current output configuration.
    pub fn config(&self) -> &OutputConfig {
        &self.config
    }
}

/// Enumerate available audio devices via PipeWire.
///
/// Returns a list of sources (inputs) and sinks (outputs).
pub fn enumerate_devices() -> Result<Vec<AudioDevice>, NadaError> {
    // TODO: Use PipeWire registry to enumerate devices
    // - Create a temporary MainLoop + Context + Core
    // - Listen for registry events (type = Node, media.class = Audio/*)
    // - Collect device info (name, channels, sample rate)
    // - Return after a short timeout or when initial enumeration completes

    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_lifecycle() {
        let mut cap = PwCapture::new(CaptureConfig::default()).unwrap();
        assert!(!cap.is_running());
        cap.start().unwrap();
        assert!(cap.is_running());
        cap.stop().unwrap();
        assert!(!cap.is_running());
    }

    #[test]
    fn output_lifecycle() {
        let mut out = PwOutput::new(OutputConfig::default()).unwrap();
        assert!(!out.is_running());
        out.start().unwrap();
        assert!(out.is_running());
        out.stop().unwrap();
        assert!(!out.is_running());
    }

    #[test]
    fn enumerate_returns_vec() {
        let devices = enumerate_devices().unwrap();
        // May be empty if no PipeWire running, but should not error
        let _ = devices;
    }

    #[test]
    fn capture_try_recv_empty() {
        let cap = PwCapture::new(CaptureConfig::default()).unwrap();
        assert!(cap.try_recv().is_none());
    }
}
