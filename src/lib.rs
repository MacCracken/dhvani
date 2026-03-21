//! # Dhvani — Core Audio Engine
//!
//! Dhvani (ध्वनि, Sanskrit: sound, resonance) provides shared audio processing
//! primitives for the AGNOS ecosystem. It eliminates duplicate implementations
//! across [shruti](https://github.com/MacCracken/shruti) (DAW),
//! [jalwa](https://github.com/MacCracken/jalwa) (media player),
//! [aethersafta](https://github.com/MacCracken/aethersafta) (compositor),
//! and [tarang](https://crates.io/crates/tarang) (media framework).
//!
//! Every downstream consumer gets the same audio math — buffers, DSP,
//! analysis, MIDI, metering, and an RT-safe audio graph.
//!
//! # Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`buffer`] | Audio buffers, mixing, resampling (linear + sinc), format conversion |
//! | [`clock`] | Sample-accurate transport clock, tempo, beats, PTS, A/V sync |
//! | [`ffi`] | C-compatible FFI for AudioBuffer operations |
//! | [`dsp`] | Biquad filters, parametric EQ, compressor, limiter, reverb, delay, de-esser, panner *(feature: `dsp`)* |
//! | [`analysis`] | FFT, STFT spectrograms, EBU R128 loudness, dynamics, chromagram, onset detection *(feature: `analysis`)* |
//! | [`midi`] | MIDI 1.0/2.0 events, clips, translation, voice management, routing *(feature: `midi`)* |
//! | [`graph`] | RT-safe audio graph with topological execution and double-buffered plan swap *(feature: `graph`)* |
//! | [`meter`] | Lock-free peak metering via atomics (no mutex) *(feature: `graph`)* |
//! | [`capture`] | PipeWire capture/output, ring-buffer recording *(feature: `pipewire`)* |
//!
//! # Quick Start
//!
//! ```rust
//! use dhvani::buffer::{AudioBuffer, mix};
//! use dhvani::dsp::{self, ParametricEq, EqBandConfig, BandType, Compressor, CompressorParams};
//! use dhvani::analysis;
//!
//! // Create and mix buffers
//! let vocals = AudioBuffer::from_interleaved(vec![0.5; 4096], 2, 44100).unwrap();
//! let drums = AudioBuffer::from_interleaved(vec![0.3; 4096], 2, 44100).unwrap();
//! let mut mixed = mix(&[&vocals, &drums]).unwrap();
//!
//! // 3-band parametric EQ
//! let mut eq = ParametricEq::new(vec![
//!     EqBandConfig { band_type: BandType::HighPass, freq_hz: 80.0, gain_db: 0.0, q: 0.707, enabled: true },
//!     EqBandConfig { band_type: BandType::Peaking, freq_hz: 3000.0, gain_db: 3.0, q: 1.5, enabled: true },
//!     EqBandConfig { band_type: BandType::HighShelf, freq_hz: 10000.0, gain_db: -2.0, q: 0.707, enabled: true },
//! ], 44100, 2);
//! eq.process(&mut mixed);
//!
//! // Compress and normalize
//! let mut comp = Compressor::new(CompressorParams {
//!     threshold_db: -18.0, ratio: 4.0, attack_ms: 10.0, release_ms: 100.0,
//!     makeup_gain_db: 3.0, knee_db: 6.0,
//! }, 44100);
//! comp.process(&mut mixed);
//! dsp::normalize(&mut mixed, 0.95);
//!
//! println!("Peak: {:.2}, LUFS: {:.1}", mixed.peak(), analysis::loudness_lufs(&mixed));
//! ```
//!
//! # Guide
//!
//! ## Step 1: Create and manipulate buffers
//!
//! [`AudioBuffer`](buffer::AudioBuffer) is the core type. All audio is f32 interleaved internally.
//!
//! ```rust
//! use dhvani::buffer::{AudioBuffer, mix, resample_linear};
//! use dhvani::buffer::convert::{i16_to_f32, mono_to_stereo};
//!
//! // From raw samples
//! let buf = AudioBuffer::from_interleaved(vec![0.5; 2048], 2, 44100).unwrap();
//! assert_eq!(buf.channels, 2);
//! assert_eq!(buf.frames, 1024);
//!
//! // Format conversion
//! let i16_data: Vec<i16> = vec![16384; 1024];
//! let f32_data = i16_to_f32(&i16_data);
//!
//! // Mono to stereo
//! let mono = AudioBuffer::from_interleaved(vec![0.5; 1024], 1, 44100).unwrap();
//! let stereo = mono_to_stereo(&mono).unwrap();
//!
//! // Resample
//! let resampled = resample_linear(&buf, 48000).unwrap();
//! ```
//!
//! ## Step 2: Apply DSP effects
//!
//! All effects operate on [`AudioBuffer`](buffer::AudioBuffer) in-place.
//! Stateful effects (EQ, reverb, compressor) have `process()` methods.
//! Stateless operations (gate, limiter, normalize) are free functions.
//!
//! ```rust
//! use dhvani::buffer::AudioBuffer;
//! use dhvani::dsp::{self, BiquadFilter, FilterType, Reverb, ReverbParams, StereoPanner};
//!
//! let mut buf = AudioBuffer::from_interleaved(vec![0.5; 4096], 2, 44100).unwrap();
//!
//! // Biquad low-pass filter
//! let mut lp = BiquadFilter::new(FilterType::LowPass, 5000.0, 0.707, 44100, 2);
//! lp.process(&mut buf);
//!
//! // Reverb
//! let mut reverb = Reverb::new(ReverbParams { room_size: 0.6, damping: 0.4, mix: 0.3 }, 44100);
//! reverb.process(&mut buf);
//!
//! // Panning
//! let panner = StereoPanner::new(0.3); // slightly right
//! panner.process(&mut buf);
//!
//! // Gate and normalize
//! dsp::noise_gate(&mut buf, 0.01);
//! dsp::normalize(&mut buf, 0.95);
//! ```
//!
//! ## Step 3: Analyze audio
//!
//! Analysis functions are non-destructive — they read the buffer without modifying it.
//!
//! ```rust
//! use dhvani::buffer::AudioBuffer;
//! use dhvani::analysis::{self, spectrum_fft, analyze_dynamics, measure_r128, chromagram, detect_onsets, compute_stft};
//!
//! let buf = AudioBuffer::from_interleaved(
//!     (0..44100).map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin()).collect(),
//!     1, 44100,
//! ).unwrap();
//!
//! // FFT spectrum (radix-2, O(n log n))
//! let spec = spectrum_fft(&buf, 4096);
//! println!("Dominant freq: {:?} Hz", spec.dominant_frequency());
//!
//! // Dynamics (true peak, crest factor, dynamic range)
//! let dyn_ = analyze_dynamics(&buf);
//! println!("True peak: {:.2} dB, Crest: {:.1} dB", dyn_.true_peak_db, dyn_.crest_factor_db);
//!
//! // EBU R128 loudness (K-weighted, gated)
//! let r128 = measure_r128(&buf);
//! println!("Integrated: {:.1} LUFS", r128.integrated_lufs);
//!
//! // Chromagram (pitch class detection)
//! let chroma = chromagram(&buf, 4096);
//! println!("Dominant pitch: {}", chroma.dominant_name());
//!
//! // Onset detection
//! let onsets = detect_onsets(&buf, 2048, 512, 0.3);
//! println!("Found {} onsets", onsets.positions.len());
//! ```
//!
//! ## Step 4: Work with MIDI
//!
//! ```rust
//! use dhvani::midi::{MidiClip, NoteEvent, MidiEvent};
//! use dhvani::midi::voice::{VoiceManager, VoiceStealMode};
//!
//! // Create a clip with notes
//! let mut clip = MidiClip::new("melody", 0, 44100);
//! clip.add_note(0, 22050, 60, 100, 0);     // C4
//! clip.add_note(22050, 22050, 64, 90, 0);   // E4
//!
//! // Query notes at a position
//! let active = clip.notes_at(11025);
//! assert_eq!(active.len(), 1);
//!
//! // Voice management for polyphonic synths
//! let mut voices = VoiceManager::new(16, VoiceStealMode::Oldest);
//! let slot = voices.note_on(60, 100, 0).unwrap();
//! println!("Voice {} playing {:.1} Hz", slot, voices.voice(slot).unwrap().frequency());
//! ```
//!
//! ## Step 5: Build an audio graph
//!
//! ```rust,no_run
//! use dhvani::graph::{Graph, GraphProcessor, NodeId, AudioNode};
//! use dhvani::buffer::AudioBuffer;
//!
//! // Define a custom node
//! struct ToneGenerator { freq: f32, phase: f64 }
//! impl AudioNode for ToneGenerator {
//!     fn name(&self) -> &str { "tone" }
//!     fn num_inputs(&self) -> usize { 0 }
//!     fn num_outputs(&self) -> usize { 1 }
//!     fn process(&mut self, _inputs: &[&AudioBuffer], output: &mut AudioBuffer) {
//!         for s in &mut output.samples {
//!             *s = (self.phase as f32).sin() * 0.5;
//!             self.phase += 2.0 * std::f64::consts::PI * self.freq as f64 / 44100.0;
//!         }
//!     }
//! }
//!
//! // Build and compile graph
//! let mut graph = Graph::new();
//! let tone_id = NodeId::next();
//! graph.add_node(tone_id, Box::new(ToneGenerator { freq: 440.0, phase: 0.0 }));
//! let plan = graph.compile().unwrap();
//!
//! // Process on RT thread
//! let mut processor = GraphProcessor::new(2, 44100, 1024);
//! let handle = processor.swap_handle();
//! handle.swap(plan);
//! let output = processor.process(); // returns Option<&AudioBuffer>
//! ```
//!
//! # Error Handling
//!
//! All fallible operations return [`Result<T, NadaError>`](NadaError).
//!
//! ```rust
//! use dhvani::buffer::AudioBuffer;
//! use dhvani::NadaError;
//!
//! match AudioBuffer::from_interleaved(vec![], 0, 44100) {
//!     Ok(_) => unreachable!(),
//!     Err(NadaError::InvalidChannels(0)) => println!("zero channels rejected"),
//!     Err(e) => println!("other error: {e}"),
//! }
//! ```
//!
//! # Cargo Features
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `dsp` | Yes | DSP effects (EQ, compressor, limiter, reverb, delay, de-esser, panner, oscillator, LFO, envelope) |
//! | `analysis` | Yes | Audio analysis (FFT, STFT, R128, dynamics, chromagram, onsets). Implies `dsp` |
//! | `midi` | Yes | MIDI 1.0/2.0 events, voice management, routing, translation |
//! | `graph` | Yes | RT-safe audio graph and lock-free metering |
//! | `simd` | Yes | SSE2/AVX2 (x86_64) and NEON (aarch64) acceleration |
//! | `pipewire` | No | PipeWire audio capture/output backend (Linux only) |
//! | `full` | No | All features including PipeWire |
//!
//! Core-only build (buffers, mixing, resampling, clock — no DSP/MIDI/analysis):
//! ```toml
//! dhvani = { version = "0.20", default-features = false }
//! ```

// Core (always available)
pub mod buffer;
pub mod capture;
pub mod clock;
pub mod ffi;

// Feature-gated modules
#[cfg(feature = "dsp")]
pub mod dsp;
#[cfg(feature = "analysis")]
pub mod analysis;
#[cfg(feature = "midi")]
pub mod midi;
#[cfg(feature = "graph")]
pub mod graph;
#[cfg(feature = "graph")]
pub mod meter;

#[cfg(feature = "simd")]
pub(crate) mod simd;

mod error;
pub use error::NadaError;

/// Result type alias for dhvani operations.
pub type Result<T> = std::result::Result<T, NadaError>;

#[cfg(test)]
mod tests;
