# Nada Roadmap

> **Principle**: Correctness first, then SIMD, then capture backends. Every consumer gets the same audio math.

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## Completed

### v0.20.3 — Foundation

- [x] AudioBuffer (f32 interleaved, channels, sample_rate, frames)
- [x] SampleFormat, Layout enums with `#[non_exhaustive]`
- [x] Buffer ops: peak, RMS, gain, clamp, silence, mixing, linear resampling
- [x] DSP: noise gate, hard limiter, compressor, normalize, dB conversions
- [x] Analysis: DFT spectrum, LUFS loudness, silence detection
- [x] AudioClock: position, tempo, beats, PTS, seek
- [x] 40+ tests, 3 benchmark suites

### v0.21.3 — DSP & Format Conversion

- [x] Biquad filter (LP, HP, BP, notch, all-pass, peaking, shelf — Bristow-Johnson cookbook)
- [x] Parametric EQ (N-band biquad cascade)
- [x] Reverb (Schroeder/Freeverb: 4 combs + 2 allpasses, stereo decorrelation)
- [x] Delay line (fixed + modulated for chorus/flanger)
- [x] De-esser (biquad sidechain sibilance detection)
- [x] Compressor (envelope follower, soft knee, makeup gain)
- [x] Format conversion: i16/i32/f32, interleaved/planar, mono/stereo, 5.1 downmix
- [x] Sinc resampling (Blackman-Harris window, draft/good/best quality)
- [x] 100+ tests, 6 benchmark suites

### v0.22.3 — SIMD, Capture, MIDI & Crate Quality

#### SIMD acceleration
- [x] SSE2 kernels: mix, gain, clamp, peak, RMS, noise gate, i16/f32 conversion, weighted sum (4 f32/op)
- [x] AVX2 kernels: mix, gain, clamp, peak (8 f32/op, runtime-detected)
- [x] NEON kernels: mix, gain, clamp, peak, RMS, noise gate, weighted sum (aarch64)
- [x] SIMD sinc resampling (pre-computed kernel + SIMD dot product)
- [x] Platform dispatch module (`src/simd/`) with scalar fallback
- [x] Dedicated SIMD benchmark suite (`benches/simd.rs`)

#### PipeWire capture (`pipewire` feature)
- [x] Device types: AudioDevice, DeviceType (Source/Sink), CaptureConfig, OutputConfig
- [x] PwCapture: channel-based capture stream with start/stop/recv
- [x] PwOutput: channel-based output stream with start/stop/send
- [x] enumerate_devices() API
- [x] CaptureEvent: DeviceAdded, DeviceRemoved, Overflow, Underrun (hot-plug)
- [ ] Full PipeWire event loop integration (TODO stubs for pw::MainLoop)

#### MIDI (`midi` module)
- [x] Core types: NoteEvent, ControlChange, MidiEvent enum, MidiClip
- [x] MidiClip: sorted insert, binary search range query, merge, transpose, quantize
- [x] MIDI 2.0 / UMP: NoteOnV2, NoteOffV2, ControlChangeV2, per-note pitch bend/CC, 32-bit pressure/bend, UmpMessageType
- [x] Translation: velocity 7↔16 bit, CC 7↔32 bit, pitch bend 14↔32 bit, event/CC conversion with roundtrip tests
- [x] Voice management: Voice, VoiceState, VoiceStealMode (Oldest/Quietest/Lowest/None), VoiceManager with polyphonic pool
- [x] Routing: VelocityCurve (Linear/Soft/Hard/Fixed), MidiRoute (channel/note/velocity filtering), CcMapping (7-bit + 32-bit)

#### Crate quality
- [x] CONTRIBUTING.md
- [x] SECURITY.md
- [x] deny.toml (license/advisory/source/bans validation)
- [x] FFI module (`src/ffi.rs`) — C-compatible API for AudioBuffer ops
- [x] Fuzz targets (mix, resample, DSP chain)
- [x] cargo-vet in CI
- [x] cargo-semver-checks in CI
- [x] Test-minimal job in CI (no features)
- [x] Benchmark job in CI
- [x] Fuzz job in CI (nightly, 5 min)
- [x] 168+ tests, 6 benchmark suites

---

## Next

### v0.23.3 — RT Infrastructure & DSP Gaps

#### Lock-free metering (`meter`)
- [ ] `PeakMeter` — stereo peak levels via `AtomicU32` (f32 bit patterns, no mutex)
- [ ] `MeterBank` — growable slot bank, pre-allocated
- [ ] `SharedMeterBank` — `Arc`-wrapped for multi-thread sharing
      _Source: shruti-engine/src/meter.rs_

#### Audio graph (`graph`)
- [ ] `AudioNode` trait — name, num_inputs, num_outputs, `process()`, `is_finished()`
- [ ] `Graph` — non-RT builder: add nodes, connect edges
- [ ] `ExecutionPlan` — compiled topological order (Kahn's algorithm, cycle detection)
- [ ] `GraphProcessor` — RT-thread processor with double-buffered plan swapping
- [ ] `NodeId` — atomic ID generator
      _Source: shruti-engine/src/graph/_

#### Ring-buffer recording (`capture`)
- [ ] `RecordManager` — lock-free ring buffer (rtrb) → accumulator thread → output
- [ ] `LoopRecordManager` — loop-aware recording with sentinel-based take splitting
      _Source: shruti-engine/src/record.rs_

#### DSP (completed)
- [x] `StereoPanner` — constant-power (sin/cos) panning
- [x] `EnvelopeLimiter` — soft-knee brick-wall limiter with instant attack, configurable release
- [x] `DynamicsAnalysis` — peak, RMS, true peak (4x oversampled), crest factor, dynamic range
- [x] FFT spectrum — radix-2 Cooley-Tukey FFT replacing O(n^2) DFT
- [x] Full PipeWire event loop integration (capture + output + enumerate_devices)

#### Analysis (completed)
- [x] STFT (short-time Fourier transform) for spectrograms — Spectrogram struct with time-frequency matrix
- [x] Full EBU R128 loudness — K-weighting (high shelf + high pass), 400ms blocks, absolute + relative gating, LRA
- [x] Chromagram — 12 pitch classes mapped from FFT bins
- [x] Onset detection — spectral flux with peak-picking

---

### v0.24.3 — Integration & Performance

#### Consumer integration
- [ ] shruti adopts nada (replace shruti-engine audio math, shruti-dsp, shruti-session MIDI)
- [ ] jalwa adopts nada (replace internal playback buffer + EQ)
- [ ] aethersafta adopts nada (replace PipeWire capture + mixer stub)
- [ ] hoosh uses `nada::midi` for music token preprocessing

#### Performance
- [ ] Zero-copy buffer views (borrow slices for read-only DSP)
- [ ] Buffer pool (reuse allocations across frames — arena allocator)
- [ ] Parallel DSP chain (rayon for independent effects)

#### Quality
- [ ] Property-based tests (proptest: random buffers, sample rates, channels)
- [ ] 90%+ code coverage
- [ ] Complete docs.rs documentation (all public types + examples)

---

### v1.0.0 Criteria

- [ ] AudioBuffer, AudioClock, Spectrum, MIDI APIs frozen
- [ ] All DSP effects match reference implementations (within 0.01 dB)
- [ ] SIMD on x86_64 (SSE2+AVX2) and aarch64 (NEON)
- [ ] PipeWire capture/output stable
- [ ] Sinc resampling passing SRC quality tests
- [ ] MIDI 1.0 + 2.0 types, translation, voice management stable
- [ ] At least 3 downstream consumers (shruti, jalwa, aethersafta)
- [ ] 90%+ test coverage
- [ ] docs.rs documentation complete
- [ ] No `unsafe` without `// SAFETY:` comments
- [ ] Benchmarks establish golden numbers

---

### Post-v1

#### Advanced DSP
- [ ] Convolution reverb (impulse response loading)
- [ ] Multiband compressor
- [ ] Noise suppression (RNNoise integration or custom)
- [ ] Pitch shifting (phase vocoder)
- [ ] Time stretching (WSOLA or phase vocoder)

#### MIDI advanced
- [ ] SMF (Standard MIDI File) read/write
- [ ] MIDI clock / sync (MTC, SPP)
- [ ] SysEx message handling
- [ ] MPE (MIDI Polyphonic Expression) zone management
- [ ] MIDI tokenization for music LLMs (port from shruti-ml `tokenizer.rs`)

#### Platform
- [ ] CoreAudio backend (macOS)
- [ ] WASAPI backend (Windows)
- [ ] JACK backend (pro audio)
- [ ] WASM target (Web Audio API)

#### Format
- [ ] 24-bit audio support
- [ ] DSD (1-bit) support
- [ ] Ambisonic (3D audio) channel layouts

---

### Non-goals

- **Audio I/O (file read/write)** — tarang / symphonia
- **Plugin hosting (VST/CLAP/LV2)** — shruti
- **Music composition / sequencing** — shruti
- **Streaming protocols (RTMP/SRT)** — aethersafta
- **Specific instruments (synth/sampler/drums)** — shruti; nada provides voice management, consumers build instruments on top
