# Nada Roadmap

> **Principle**: Correctness first, then SIMD, then capture backends. Every consumer gets the same audio math.

---

## v0.20.3 (current) — Complete Engine

Everything below was implemented in the initial build-out.

### Core
- [x] AudioBuffer (f32 interleaved), SampleFormat, Layout, mixing, linear + sinc resampling
- [x] AudioClock (position, tempo, beats, PTS, seek)
- [x] NadaError with `#[non_exhaustive]`, Result type alias

### DSP
- [x] Biquad filter (8 types — Bristow-Johnson cookbook)
- [x] Parametric EQ (N-band cascade), StereoPanner (constant-power)
- [x] Reverb (Schroeder/Freeverb), Delay (fixed + modulated chorus/flanger)
- [x] Compressor (envelope follower, soft knee, makeup), EnvelopeLimiter (brick-wall)
- [x] De-esser, noise gate, hard limiter, normalize, dB conversions

### Analysis
- [x] Radix-2 FFT (O(n log n)), STFT spectrograms
- [x] EBU R128 loudness (K-weighting + gating + LRA)
- [x] DynamicsAnalysis (true peak 4x oversampled, crest factor, dynamic range)
- [x] Chromagram (12 pitch classes), onset detection (spectral flux)

### MIDI
- [x] MIDI 1.0: NoteEvent, ControlChange, MidiEvent enum, MidiClip (sorted insert, binary search, merge, transpose, quantize)
- [x] MIDI 2.0 / UMP: NoteOnV2, NoteOffV2, ControlChangeV2, per-note expression, UmpMessageType
- [x] Translation: velocity/CC/pitch bend 1.0↔2.0 with roundtrip tests
- [x] Voice management: VoiceManager, 4 steal modes, polyphonic pool
- [x] Routing: VelocityCurve, MidiRoute, CcMapping

### SIMD
- [x] SSE2/AVX2/NEON kernels: mix, gain, clamp, peak, RMS, noise gate, i16/f32, weighted sum
- [x] Platform dispatch with scalar fallback, runtime AVX2 detection

### RT Infrastructure
- [x] Lock-free metering (PeakMeter, MeterBank, SharedMeterBank via AtomicU32)
- [x] Audio graph (AudioNode trait, Graph → ExecutionPlan → GraphProcessor with double-buffered swap)
- [x] Ring-buffer recording (RecordManager, LoopRecordManager with take splitting)

### Capture
- [x] PipeWire capture/output (PwCapture, PwOutput, enumerate_devices, hot-plug events)
- [x] Device types, config structs, CaptureEvent

### Crate Quality
- [x] Format conversion: i16/i32/f32, interleaved/planar, mono/stereo, 5.1 downmix
- [x] FFI module (C-compatible AudioBuffer API)
- [x] CONTRIBUTING.md, SECURITY.md, deny.toml
- [x] Fuzz targets (mix, resample, DSP), cargo-vet/semver-checks in CI
- [x] 225+ tests, 6 benchmark suites, zero clippy warnings

---

## v0.21.3 — Hardening & Refactoring

Pre-v1.0 audit findings. Ship quality.

### Critical fixes

- [ ] **FFT panic removal**: Replace `assert!(n.is_power_of_two())` in `fft_in_place()` with graceful fallback — production code must not panic
- [ ] **AudioBuffer encapsulation**: Make fields private, add accessor methods (`samples()`, `samples_mut()`, `channels()`, `sample_rate()`, `frames()`) — freeze API surface for v1.0
- [ ] **Remove deprecated API**: Delete `EqBand` struct and `apply_eq_band()` stub

### Memory & allocation

- [ ] **DeEsser sidechain pre-allocation**: Store reusable buffer in struct instead of `buf.clone()` per process call
- [ ] **Graph processor Vec-indexed outputs**: Replace `HashMap<NodeId, AudioBuffer>` with `Vec<AudioBuffer>` indexed by execution order
- [ ] **Graph input gather pre-allocation**: Pre-allocate input Vec per node instead of per-frame collect
- [ ] **Buffer pool**: Reusable `AudioBuffer` arena — effects borrow instead of allocating
- [ ] **Zero-copy buffer views**: `AudioBufferRef<'a>` for read-only DSP (analysis, metering)

### Performance

- [ ] **Parallel DSP chain**: rayon for independent graph branches
- [ ] **SIMD biquad cross-channel**: Process stereo L+R in single SSE2 register

### Cross-project audit findings (2026-03-20)

Items surfaced from reviewing shruti, jalwa, tazama, and SY audio paths:

- [ ] **Spectral noise reduction (STFT gating)** — from tazama `noise_reduction.rs`. General-purpose DSP used by video editors, DAWs, and stream audio. STFT → threshold magnitudes → ISTFT
- [ ] **Waveform peak extraction (downsampled min/max)** — from tazama `waveform.rs`. Every audio app needs this for UI visualization. Returns min/max pairs at configurable reduction ratio
- [ ] **Oscillator (PolyBLEP: sine, saw, square, triangle, noise)** — from shruti `oscillator.rs`. Synthesis primitive — nada provides voice management but no waveform generation
- [ ] **ADSR Envelope** — from shruti `envelope.rs`. Paired with oscillator for synthesis. Attack/decay/sustain/release with configurable curves
- [ ] **LFO (6 shapes + sample-and-hold)** — from shruti `lfo.rs`. Modulation primitive for effects and synthesis. Sync-to-tempo option via AudioClock

### Robustness

- [ ] **Parameter validation**: Range checks on CompressorParams, ReverbParams, LimiterParams (constructors or `validate()`)
- [ ] **Sample rate ceiling**: Guard > 384kHz to prevent integer overflow
- [ ] **NaN propagation audit**: All DSP effects handle NaN input → 0.0; add `debug_assert!(finite)` in test builds
- [ ] **Reverb dead field**: Remove unused `_sample_rate` from Reverb struct

### Cleanup

- [ ] Remove `anyhow` dep (replace `NadaError::Other` with `Box<dyn Error>`)
- [ ] Move `serde_json` to dev-dependencies if only used in tests
- [ ] Audit `tracing` — instrument key functions or remove

---

## v0.22.3 — Testing & Documentation

### Testing

- [ ] **Property-based tests**: proptest for buffer dimensions, sample rates, DSP invariants (finite output, energy bounds)
- [ ] **SIMD parity tests**: Explicit SIMD vs scalar output comparison for all kernels
- [ ] **Long-buffer stress tests**: 1-hour processing through full DSP chain
- [ ] **Graph concurrency test**: Multi-threaded plan swapping under RT load
- [ ] **EBU R128 reference vectors**: Validate against EBU tech 3341 test signals
- [ ] **90%+ code coverage** (cargo-llvm-cov)

### Cross-project audit — lower priority

- [ ] **SVF Filter (Cytomic topology)** — from shruti `filter.rs`. Alternative to biquad, preferred for modular synthesis (better modulation behavior)
- [ ] **Spectral rolloff** — from shruti `spectral.rs`. Analysis: frequency below which 95% of spectral energy is concentrated
- [ ] **Waveform display data** — from tazama. `compute_waveform()` returning display-ready peak data at configurable resolution
- [ ] **EQ presets (rock, pop, jazz, etc.)** — from jalwa `dsp.rs`. Convenience layer over parametric EQ. Low priority — consumers can define their own

### Documentation

- [ ] **RT safety docs**: Which types are RT-safe (no alloc, no lock) vs non-RT
- [ ] **SIMD module docs**: Vectorized operations and expected speedups
- [ ] **FFI usage guide**: C/Python integration examples
- [ ] **Thread-safety annotations**: Document non-Sync DSP types
- [ ] **Complete docs.rs**: Every public type has doc comment + example

---

## v0.23.3 — Consumer Integration

### Adoption

- [ ] shruti adopts nada (replace shruti-engine + shruti-dsp + shruti-session MIDI)
- [ ] jalwa adopts nada (replace playback buffer + EQ + normalization)
- [ ] aethersafta adopts nada (replace PipeWire capture + mixer)
- [ ] tazama uses nada DSP (replace tazama-media/dsp/)
- [ ] hoosh uses `nada::midi` for music token preprocessing

### Validation

- [ ] Cross-crate integration tests
- [ ] Benchmark regression: nada not slower than code it replaces

---

## v1.0.0 Criteria

All must be true:

- [ ] API frozen: AudioBuffer, AudioClock, Spectrum, MIDI, Graph, Meter
- [ ] AudioBuffer encapsulated (private fields, accessor methods)
- [ ] No panics in non-test code
- [ ] DSP effects within 0.01 dB of reference implementations
- [ ] SIMD parity verified on x86_64 + aarch64
- [ ] PipeWire capture/output tested with real hardware
- [ ] 3+ downstream consumers in production
- [ ] 90%+ test coverage
- [ ] docs.rs complete
- [ ] All `unsafe` has `// SAFETY:` comments
- [ ] Golden benchmark numbers published
- [ ] Zero clippy warnings
- [ ] Supply chain clean (audit + deny + vet)

---

## Post-v1

### Advanced DSP
- [ ] Convolution reverb (impulse response)
- [ ] Multiband compressor
- [ ] Noise suppression (RNNoise or custom)
- [ ] Pitch shifting (phase vocoder)
- [ ] Time stretching (WSOLA / phase vocoder)

### MIDI advanced
- [ ] SMF (Standard MIDI File) read/write
- [ ] MIDI clock / sync (MTC, SPP)
- [ ] SysEx handling
- [ ] MPE zone management
- [ ] MIDI tokenization for music LLMs

### Platform
- [ ] CoreAudio (macOS)
- [ ] WASAPI (Windows)
- [ ] JACK (pro audio)
- [ ] WASM (Web Audio API)

### Format
- [ ] 24-bit audio
- [ ] DSD (1-bit)
- [ ] Ambisonic (3D audio) channel layouts

---

## Non-goals

- **Audio I/O (file read/write)** — tarang / symphonia
- **Plugin hosting (VST/CLAP/LV2)** — shruti
- **Music composition / sequencing** — shruti
- **Streaming protocols (RTMP/SRT)** — aethersafta
- **Specific instruments** — shruti; nada provides voice management, consumers build on top
