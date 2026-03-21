# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.20.4] — 2026-03-20

### Added

#### DSP
- `GainSmoother` — exponential moving average with configurable attack/release coefficients for smooth gain transitions. Prevents pumping in volume normalization workflows
- `GainSmootherParams` — serde-compatible parameters (default: attack 0.3, release 0.05)
- `GraphicEq` — 10-band ISO graphic equalizer (31 Hz–16 kHz) wrapping `ParametricEq` with per-band gain control
- `GraphicEqSettings` — settings with 9 named presets (rock, pop, jazz, classical, bass, treble, vocal, electronic, acoustic)
- `ISO_BANDS` constant — standard 10-band center frequencies

#### Analysis
- `suggest_gain(buf, target_rms) → f32` — per-buffer normalization gain suggestion with 0.1–10.0x clamping. Convenience for media player volume normalization

#### Crate Structure
- Feature flags for module-level compilation: `dsp`, `analysis`, `midi`, `graph` (all default-on)
- `analysis` feature implies `dsp` (R128 K-weighting needs biquad, dynamics needs dB conversion)
- `dsp::noise_reduction` gated behind `analysis` feature (needs FFT)
- Core always available: `buffer`, `capture`, `clock`, `ffi`, `error`
- Consumers can now select only what they need (e.g., `default-features = false, features = ["dsp", "simd"]`)

#### Documentation
- Comprehensive documentation audit and cleanup across all docs
- Updated roadmap: collapsed v0.21–v0.23 into 2 dense releases targeting v1.0
- Architecture overview updated with full module tree
- Migration guide updated with planned v0.21.3 breaking changes

### Fixed
- Sanskrit character: नाद (Nāda) → ध्वनि (Dhvani) in README and docs
- README Quick Start: replaced nonexistent `dsp::compress()` with `Compressor` struct
- README: `spectrum_dft` → `spectrum_fft` in examples
- Roadmap: marked already-completed items (oscillator, envelope, LFO, noise_reduction, waveform, anyhow removal, serde_json)
- Stale version references removed from capability table and roadmap

---

## [0.20.3] — 2026-03-20

### Added

#### Core
- `AudioBuffer` — f32 interleaved audio buffer with channels, sample_rate, frames
- `SampleFormat` (F32, I16, I32) and `Layout` (Interleaved, Planar) enums with `#[non_exhaustive]`
- `AudioClock` — sample-accurate transport with position, tempo, beats, PTS, seek
- `NadaError` enum with FormatMismatch, LengthMismatch, InvalidSampleRate, InvalidChannels, Dsp, Capture, InvalidParameter, Conversion variants

#### DSP
- `BiquadFilter` — 8 filter types (LP, HP, BP, notch, all-pass, peaking, shelf) using Bristow-Johnson cookbook
- `ParametricEq` — N-band biquad cascade with per-band enable/disable
- `Reverb` — Schroeder/Freeverb (4 combs + 2 allpasses, stereo decorrelation)
- `DelayLine` + `ModulatedDelay` — fixed and LFO-modulated for chorus/flanger
- `Compressor` — envelope follower with soft knee, attack/release, makeup gain
- `EnvelopeLimiter` — brick-wall limiter with instant attack, soft knee
- `DeEsser` — biquad sidechain sibilance detection with pre-allocated buffer
- `StereoPanner` — constant-power (sin/cos) panning law
- Stateless: noise gate, hard limiter, normalize, amplitude/dB conversion

#### Analysis
- Radix-2 Cooley-Tukey FFT (O(n log n)) + simple DFT for small windows
- STFT spectrograms with configurable window/hop size
- EBU R128 loudness (K-weighting, 400ms blocks, absolute + relative gating, LRA)
- `DynamicsAnalysis` — true peak (4x oversampled), crest factor, dynamic range
- `Chromagram` — 12 pitch classes mapped from FFT bins
- Onset detection via spectral flux with peak-picking
- Simplified LUFS and silence detection

#### MIDI
- MIDI 1.0: `NoteEvent`, `ControlChange`, `MidiEvent` enum, `MidiClip`
- MIDI 2.0 / UMP: `NoteOnV2`, `NoteOffV2`, `ControlChangeV2`, per-note expression, `UmpMessageType`
- Translation: velocity (7↔16 bit), CC (7↔32 bit), pitch bend (14↔32 bit) with roundtrip tests
- `VoiceManager` — polyphonic voice pool with 4 steal modes (Oldest, Quietest, Lowest, None)
- Routing: `VelocityCurve`, `MidiRoute`, `CcMapping`
- `MidiClip` operations: sorted insert, binary search range query, merge, transpose, quantize

#### SIMD
- SSE2 kernels (x86_64): mix, gain, clamp, peak, RMS, noise gate, i16/f32, weighted sum
- AVX2 kernels (x86_64): mix, gain, clamp, peak — runtime-detected
- NEON kernels (aarch64): mix, gain, clamp, peak, RMS, noise gate, weighted sum
- Platform dispatch module with scalar fallback

#### RT Infrastructure
- `PeakMeter` / `MeterBank` / `SharedMeterBank` — lock-free metering via AtomicU32
- `AudioNode` trait + `Graph` + `ExecutionPlan` + `GraphProcessor` (double-buffered swap)
- `RecordManager` / `LoopRecordManager` — ring-buffer recording with take splitting

#### Capture
- PipeWire capture/output (`PwCapture`, `PwOutput`, `enumerate_devices`)
- Device types, config structs, `CaptureEvent` hot-plug notifications

#### Format Conversion
- i16 ↔ f32, i32 ↔ f32 with clamping
- Interleaved ↔ planar
- Mono → stereo, stereo → mono
- 5.1 → stereo downmix (ITU-R BS.775)
- Sinc resampling (Blackman-Harris window, Draft/Good/Best quality)

#### Crate Quality
- FFI module — C-compatible `nada_buffer_*` API
- CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, deny.toml
- Fuzz targets (mix, resample, DSP chain)
- CI: cargo-vet, cargo-semver-checks, test-minimal, fuzz, bench jobs
- 265+ tests, 7 benchmark suites, 94%+ line coverage
