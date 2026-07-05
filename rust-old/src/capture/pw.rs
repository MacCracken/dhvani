//! PipeWire backend — device enumeration, capture, output, hot-plug.
//!
//! This module is only compiled when the `pipewire` feature is enabled.
//! Uses the `pipewire` crate (v0.9) Rust bindings.

use std::cell::Cell;
use std::convert::TryInto;
use std::mem;
use std::rc::Rc;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use pipewire as pw;
use pw::spa;
use pw::spa::pod::Pod;
use spa::param::format::{MediaSubtype, MediaType};
use spa::param::format_utils;

use crate::NadaError;
use crate::buffer::AudioBuffer;

use super::{AudioDevice, CaptureConfig, CaptureEvent, DeviceType, OutputConfig};

// ── Capture ─────────────────────────────────────────────────────────

/// A PipeWire capture stream that receives audio buffers.
pub struct PwCapture {
    config: CaptureConfig,
    receiver: mpsc::Receiver<AudioBuffer>,
    sender: mpsc::Sender<AudioBuffer>,
    event_receiver: mpsc::Receiver<CaptureEvent>,
    event_sender: mpsc::Sender<CaptureEvent>,
    running: Arc<Mutex<bool>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl PwCapture {
    /// Create a new capture stream.
    pub fn new(config: CaptureConfig) -> Result<Self, NadaError> {
        let (sender, receiver) = mpsc::channel();
        let (event_sender, event_receiver) = mpsc::channel();

        Ok(Self {
            config,
            receiver,
            sender,
            event_receiver,
            event_sender,
            running: Arc::new(Mutex::new(false)),
            thread: None,
        })
    }

    /// Start capturing audio from PipeWire.
    pub fn start(&mut self) -> Result<(), NadaError> {
        {
            let mut running = self
                .running
                .lock()
                .map_err(|e| NadaError::Capture(format!("lock poisoned: {e}")))?;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        let sender = self.sender.clone();
        let event_sender = self.event_sender.clone();
        let running = self.running.clone();
        let sample_rate = self.config.sample_rate;
        let channels = self.config.channels;
        let device_id = self.config.device_id;

        let handle = std::thread::spawn(move || {
            if let Err(e) = run_capture_loop(
                sender,
                event_sender,
                running.clone(),
                sample_rate,
                channels,
                device_id,
            ) {
                tracing::error!("PipeWire capture error: {e}");
            }
            if let Ok(mut r) = running.lock() {
                *r = false;
            }
        });

        self.thread = Some(handle);
        Ok(())
    }

    /// Stop capturing.
    pub fn stop(&mut self) -> Result<(), NadaError> {
        {
            let mut running = self
                .running
                .lock()
                .map_err(|e| NadaError::Capture(format!("lock poisoned: {e}")))?;
            *running = false;
        }
        // The mainloop will quit when it checks the running flag
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
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

impl Drop for PwCapture {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn run_capture_loop(
    sender: mpsc::Sender<AudioBuffer>,
    _event_sender: mpsc::Sender<CaptureEvent>,
    running: Arc<Mutex<bool>>,
    sample_rate: u32,
    channels: u32,
    device_id: Option<u32>,
) -> Result<(), NadaError> {
    pw::init();

    let mainloop =
        pw::main_loop::MainLoopRc::new(None).map_err(|e| NadaError::Capture(e.to_string()))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| NadaError::Capture(e.to_string()))?;
    let core = context
        .connect_rc(None)
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    let mut props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Capture",
        *pw::keys::MEDIA_ROLE => "Music",
    };
    if let Some(id) = device_id {
        props.insert("target.object", id.to_string());
    }

    let stream = pw::stream::StreamBox::new(&core, "dhvani-capture", props)
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    // User data for the process callback
    struct CaptureData {
        format: spa::param::audio::AudioInfoRaw,
        sender: mpsc::Sender<AudioBuffer>,
        channels: u32,
        sample_rate: u32,
    }

    let data = CaptureData {
        format: Default::default(),
        sender: sender.clone(),
        channels,
        sample_rate,
    };

    let loop_clone = mainloop.clone();
    let running_clone = running.clone();

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .param_changed(|_stream, user_data, id, param| {
            let Some(param) = param else { return };
            if id != spa::param::ParamType::Format.as_raw() {
                return;
            }
            let Ok((media_type, media_subtype)) = format_utils::parse_format(param) else {
                return;
            };
            if media_type != MediaType::Audio || media_subtype != MediaSubtype::Raw {
                return;
            }
            if let Err(e) = user_data.format.parse(param) {
                tracing::warn!("Failed to parse audio format: {e}");
                return;
            }
            user_data.channels = user_data.format.channels();
            user_data.sample_rate = user_data.format.rate();
        })
        .process(move |stream, user_data| {
            // Check if we should stop
            if let Ok(r) = running_clone.lock()
                && !*r
            {
                loop_clone.quit();
                return;
            }

            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };

            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }

            let data = &mut datas[0];
            let n_channels = user_data.channels.max(1);
            let chunk_size = data.chunk().size() as usize;
            let n_samples = chunk_size / mem::size_of::<f32>();

            if n_samples == 0 {
                return;
            }

            if let Some(raw_bytes) = data.data() {
                let mut samples = Vec::with_capacity(n_samples);
                for i in 0..n_samples {
                    let start = i * mem::size_of::<f32>();
                    let end = start + mem::size_of::<f32>();
                    if end <= raw_bytes.len() {
                        let bytes: [u8; 4] = raw_bytes[start..end].try_into().unwrap_or([0; 4]);
                        samples.push(f32::from_le_bytes(bytes));
                    }
                }

                let frames = samples.len() / n_channels as usize;
                if frames > 0
                    && let Ok(buf) =
                        AudioBuffer::from_interleaved(samples, n_channels, user_data.sample_rate)
                {
                    let _ = user_data.sender.send(buf);
                }
            }
        })
        .register()
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    // Set up audio format: F32LE, with requested rate and channels
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    if sample_rate > 0 {
        audio_info.set_rate(sample_rate);
    }
    if channels > 0 {
        audio_info.set_channels(channels);
    }

    let obj = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .map_err(|e| NadaError::Capture(format!("pod serialize: {e}")))?
    .0
    .into_inner();

    let mut params =
        [Pod::from_bytes(&values).ok_or_else(|| NadaError::Capture("invalid pod".into()))?];

    stream
        .connect(
            spa::utils::Direction::Input,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    mainloop.run();

    Ok(())
}

// ── Output ──────────────────────────────────────────────────────────

/// A PipeWire output stream that plays audio buffers.
pub struct PwOutput {
    config: OutputConfig,
    sender: mpsc::Sender<AudioBuffer>,
    receiver: Arc<Mutex<mpsc::Receiver<AudioBuffer>>>,
    running: Arc<Mutex<bool>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl PwOutput {
    /// Create a new output stream.
    pub fn new(config: OutputConfig) -> Result<Self, NadaError> {
        let (sender, receiver) = mpsc::channel();

        Ok(Self {
            config,
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            running: Arc::new(Mutex::new(false)),
            thread: None,
        })
    }

    /// Start the output stream.
    pub fn start(&mut self) -> Result<(), NadaError> {
        {
            let mut running = self
                .running
                .lock()
                .map_err(|e| NadaError::Capture(format!("lock poisoned: {e}")))?;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        let receiver = self.receiver.clone();
        let running = self.running.clone();
        let sample_rate = self.config.sample_rate;
        let channels = self.config.channels;
        let device_id = self.config.device_id;

        let handle = std::thread::spawn(move || {
            if let Err(e) =
                run_output_loop(receiver, running.clone(), sample_rate, channels, device_id)
            {
                tracing::error!("PipeWire output error: {e}");
            }
            if let Ok(mut r) = running.lock() {
                *r = false;
            }
        });

        self.thread = Some(handle);
        Ok(())
    }

    /// Stop the output stream.
    pub fn stop(&mut self) -> Result<(), NadaError> {
        {
            let mut running = self
                .running
                .lock()
                .map_err(|e| NadaError::Capture(format!("lock poisoned: {e}")))?;
            *running = false;
        }
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
        Ok(())
    }

    /// Send an audio buffer to the output.
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

impl Drop for PwOutput {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn run_output_loop(
    receiver: Arc<Mutex<mpsc::Receiver<AudioBuffer>>>,
    running: Arc<Mutex<bool>>,
    sample_rate: u32,
    channels: u32,
    device_id: Option<u32>,
) -> Result<(), NadaError> {
    pw::init();

    let mainloop =
        pw::main_loop::MainLoopRc::new(None).map_err(|e| NadaError::Capture(e.to_string()))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| NadaError::Capture(e.to_string()))?;
    let core = context
        .connect_rc(None)
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    let mut props = pw::properties::properties! {
        *pw::keys::MEDIA_TYPE => "Audio",
        *pw::keys::MEDIA_CATEGORY => "Playback",
        *pw::keys::MEDIA_ROLE => "Music",
    };
    if let Some(id) = device_id {
        props.insert("target.object", id.to_string());
    }

    let stream = pw::stream::StreamBox::new(&core, "dhvani-output", props)
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    struct OutputData {
        receiver: Arc<Mutex<mpsc::Receiver<AudioBuffer>>>,
        pending_samples: Vec<f32>,
        write_pos: usize,
        channels: u32,
    }

    let data = OutputData {
        receiver,
        pending_samples: Vec::new(),
        write_pos: 0,
        channels,
    };

    let loop_clone = mainloop.clone();
    let running_clone = running.clone();

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .process(move |stream, user_data| {
            if let Ok(r) = running_clone.lock()
                && !*r
            {
                loop_clone.quit();
                return;
            }

            // Refill pending buffer from channel
            if user_data.write_pos >= user_data.pending_samples.len()
                && let Ok(rx) = user_data.receiver.lock()
                && let Ok(buf) = rx.try_recv()
            {
                user_data.pending_samples = buf.samples;
                user_data.write_pos = 0;
            }

            let Some(mut buffer) = stream.dequeue_buffer() else {
                return;
            };

            let datas = buffer.datas_mut();
            if datas.is_empty() {
                return;
            }

            let data = &mut datas[0];
            let Some(slice) = data.data() else {
                return;
            };

            let stride = mem::size_of::<f32>() * user_data.channels as usize;
            let n_frames = slice.len() / stride;

            for i in 0..n_frames {
                for c in 0..user_data.channels as usize {
                    let sample = if user_data.write_pos < user_data.pending_samples.len() {
                        let s = user_data.pending_samples[user_data.write_pos];
                        user_data.write_pos += 1;
                        s
                    } else {
                        0.0f32 // Silence if no data
                    };
                    let start = i * stride + c * mem::size_of::<f32>();
                    let end = start + mem::size_of::<f32>();
                    if end <= slice.len() {
                        slice[start..end].copy_from_slice(&sample.to_le_bytes());
                    }
                }
            }

            let chunk = data.chunk_mut();
            *chunk.offset_mut() = 0;
            *chunk.stride_mut() = stride as _;
            *chunk.size_mut() = (stride * n_frames) as _;
        })
        .register()
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    // F32LE output format
    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    audio_info.set_rate(sample_rate);
    audio_info.set_channels(channels);

    let obj = spa::pod::Object {
        type_: spa::utils::SpaTypes::ObjectParamFormat.as_raw(),
        id: spa::param::ParamType::EnumFormat.as_raw(),
        properties: audio_info.into(),
    };
    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(obj),
    )
    .map_err(|e| NadaError::Capture(format!("pod serialize: {e}")))?
    .0
    .into_inner();

    let mut params =
        [Pod::from_bytes(&values).ok_or_else(|| NadaError::Capture("invalid pod".into()))?];

    stream
        .connect(
            spa::utils::Direction::Output,
            None,
            pw::stream::StreamFlags::AUTOCONNECT
                | pw::stream::StreamFlags::MAP_BUFFERS
                | pw::stream::StreamFlags::RT_PROCESS,
            &mut params,
        )
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    mainloop.run();

    Ok(())
}

// ── Device Enumeration ──────────────────────────────────────────────

/// Enumerate available audio devices via PipeWire.
pub fn enumerate_devices() -> Result<Vec<AudioDevice>, NadaError> {
    pw::init();

    let mainloop =
        pw::main_loop::MainLoopRc::new(None).map_err(|e| NadaError::Capture(e.to_string()))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| NadaError::Capture(e.to_string()))?;
    let core = context
        .connect_rc(None)
        .map_err(|e| NadaError::Capture(e.to_string()))?;
    let registry = core
        .get_registry()
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    let devices: Rc<std::cell::RefCell<Vec<AudioDevice>>> =
        Rc::new(std::cell::RefCell::new(Vec::new()));
    let done = Rc::new(Cell::new(false));

    let done_clone = done.clone();
    let loop_clone = mainloop.clone();
    let pending = core
        .sync(0)
        .map_err(|e| NadaError::Capture(e.to_string()))?;

    let _listener_core = core
        .add_listener_local()
        .done(move |id, seq| {
            if id == pw::core::PW_ID_CORE && seq == pending {
                done_clone.set(true);
                loop_clone.quit();
            }
        })
        .register();

    let devices_clone = devices.clone();
    let _listener_reg = registry
        .add_listener_local()
        .global(move |global| {
            // We only care about Node objects
            if global.type_ != pw::types::ObjectType::Node {
                return;
            }

            let Some(props) = &global.props else {
                return;
            };

            let media_class = props.get("media.class").unwrap_or("");

            let device_type = if media_class.contains("Source") || media_class.contains("Input") {
                DeviceType::Source
            } else if media_class.contains("Sink") || media_class.contains("Output") {
                DeviceType::Sink
            } else {
                return; // Not an audio device we care about
            };

            // Only audio nodes
            if !media_class.contains("Audio") {
                return;
            }

            let name = props
                .get("node.description")
                .or_else(|| props.get("node.name"))
                .unwrap_or("Unknown")
                .to_string();

            let channels = props
                .get("audio.channels")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(2);

            let sample_rate = props
                .get("audio.rate")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(48000);

            devices_clone.borrow_mut().push(AudioDevice {
                id: global.id,
                name,
                device_type,
                channels,
                sample_rate,
            });
        })
        .register();

    while !done.get() {
        mainloop.run();
    }

    let result = devices.borrow().clone();
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_create_and_config() {
        let cap = PwCapture::new(CaptureConfig::default()).unwrap();
        assert!(!cap.is_running());
        assert_eq!(cap.config().sample_rate, 48000);
        assert_eq!(cap.config().channels, 2);
    }

    #[test]
    fn output_create_and_config() {
        let out = PwOutput::new(OutputConfig::default()).unwrap();
        assert!(!out.is_running());
        assert_eq!(out.config().sample_rate, 48000);
    }

    #[test]
    fn capture_try_recv_empty() {
        let cap = PwCapture::new(CaptureConfig::default()).unwrap();
        assert!(cap.try_recv().is_none());
    }

    #[test]
    fn enumerate_devices_runs() {
        // This test requires PipeWire to be running.
        // It should not panic even if PipeWire is not available.
        match enumerate_devices() {
            Ok(devices) => {
                for dev in &devices {
                    println!(
                        "  {} [{}] {:?} {}ch {}Hz",
                        dev.name, dev.id, dev.device_type, dev.channels, dev.sample_rate
                    );
                }
            }
            Err(e) => {
                // PipeWire might not be running in CI
                println!("enumerate_devices failed (expected in CI): {e}");
            }
        }
    }

    #[test]
    fn output_start_send_stop() {
        // Test the full output lifecycle against PipeWire.
        // Works even with only a Dummy Output sink.
        let mut out = PwOutput::new(OutputConfig::default()).unwrap();
        assert!(!out.is_running());

        // Start
        if out.start().is_err() {
            println!("PipeWire output start failed (expected without PW daemon)");
            return;
        }
        assert!(out.is_running());

        // Send a short sine buffer
        let sr = 48000u32;
        let frames = 4800; // 100ms
        let samples: Vec<f32> = (0..frames * 2)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * (i / 2) as f32 / sr as f32).sin() * 0.3)
            .collect();
        let buf = AudioBuffer::from_interleaved(samples, 2, sr).unwrap();
        let send_result = out.send(buf);
        assert!(send_result.is_ok(), "send should succeed while running");

        // Brief wait for PipeWire to process
        std::thread::sleep(std::time::Duration::from_millis(150));

        // Stop
        out.stop().unwrap();
        assert!(!out.is_running());
    }

    #[test]
    fn capture_start_stop() {
        // Test capture lifecycle. Without a real audio source,
        // we won't receive buffers, but start/stop should not panic.
        let mut cap = PwCapture::new(CaptureConfig::default()).unwrap();

        if cap.start().is_err() {
            println!("PipeWire capture start failed (expected without PW daemon)");
            return;
        }
        assert!(cap.is_running());

        // Try to receive — likely empty without a real source
        std::thread::sleep(std::time::Duration::from_millis(100));
        let buf = cap.try_recv();
        if let Some(b) = &buf {
            assert!(b.frames() > 0);
            assert!(b.samples().iter().all(|s| s.is_finite()));
            println!("Captured {} frames", b.frames());
        } else {
            println!("No capture data (expected with Dummy sink only)");
        }

        cap.stop().unwrap();
        assert!(!cap.is_running());
    }

    #[test]
    fn enumerate_returns_at_least_dummy() {
        // On a system with PipeWire running, there should be at least one node.
        match enumerate_devices() {
            Ok(devices) => {
                assert!(
                    !devices.is_empty(),
                    "PipeWire running but no audio devices found"
                );
                // At minimum, the Dummy Output should be present
                let has_sink = devices.iter().any(|d| d.device_type == DeviceType::Sink);
                assert!(has_sink, "Expected at least one sink device");
            }
            Err(_) => {
                println!("PipeWire not available, skipping");
            }
        }
    }

    #[test]
    fn double_start_idempotent() {
        let mut out = PwOutput::new(OutputConfig::default()).unwrap();
        if out.start().is_err() {
            return;
        }
        // Second start should be a no-op, not an error
        assert!(out.start().is_ok());
        assert!(out.is_running());
        out.stop().unwrap();
    }

    #[test]
    fn stop_without_start() {
        let mut out = PwOutput::new(OutputConfig::default()).unwrap();
        // Stopping without starting should not error
        assert!(out.stop().is_ok());
        assert!(!out.is_running());
    }
}
