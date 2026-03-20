# Nada Roadmap

> **Principle**: Correctness first, then SIMD, then capture backends. Every consumer gets the same audio math.

Completed items are in [CHANGELOG.md](../../CHANGELOG.md).

---

## v0.20.3 — Foundation (current)

- [x] AudioBuffer (f32 interleaved, channels, sample_rate, frames)
- [x] SampleFormat enum (F32, I16, I32) with bytes_per_sample
- [x] Buffer operations: peak, RMS, gain, clamp, silence
- [x] Mixing: sum N buffers with channel/rate validation
- [x] Resampling: linear interpolation (44.1k ↔ 48k ↔ 96k)
- [x] DSP: noise gate, hard limiter, compressor, normalize
- [x] DSP: amplitude ↔ dB conversion
- [x] Analysis: DFT spectrum, dominant frequency detection
- [x] Analysis: LUFS loudness (simplified), silence detection
- [x] AudioClock: sample-accurate position, tempo, beats, PTS, seek
- [x] 40+ tests, 3 benchmark suites

---

## v0.21.3 — DSP & Format Conversion

### DSP effects
- [ ] Biquad filter (low-pass, high-pass, band-pass, notch, all-pass, peaking, shelf)
- [ ] Parametric EQ (N-band biquad cascade)
- [ ] Reverb (Schroeder or Freeverb algorithm)
- [ ] Delay line (fixed + modulated for chorus/flanger)
- [ ] De-esser (sibilance reduction)

### Format conversion
- [ ] i16 ↔ f32 interleaved conversion
- [ ] i32 ↔ f32 interleaved conversion
- [ ] Interleaved ↔ planar conversion
- [ ] Mono → stereo (duplicate) and stereo → mono (sum/average)
- [ ] 5.1 → stereo downmix

### Resampling
- [ ] Sinc resampling (windowed sinc interpolation — higher quality than linear)
- [ ] Configurable quality levels (draft/good/best)

---

## v0.22.3 — SIMD & Capture

### SIMD acceleration
- [ ] SSE2 mixing (4 samples/iter)
- [ ] AVX2 mixing (8 samples/iter)
- [ ] NEON mixing (aarch64)
- [ ] SIMD gain application
- [ ] SIMD resampling inner loop
- [ ] Benchmarks: SIMD vs scalar per operation

### PipeWire capture (requires `pipewire` feature)
- [ ] Device enumeration (sources, sinks)
- [ ] Per-source audio capture (mic, desktop, per-app)
- [ ] Capture → AudioBuffer conversion
- [ ] Output to PipeWire sink
- [ ] Hot-plug device detection

---

## v0.23.3 — Integration & Performance

### Consumer integration
- [ ] shruti adopts nada (replace shruti-engine + shruti-dsp audio math)
- [ ] jalwa adopts nada (replace internal playback buffer + EQ)
- [ ] aethersafta adopts nada (replace PipeWire capture + mixer stub)

### Performance
- [ ] Zero-copy buffer views (borrow slices for read-only DSP)
- [ ] Buffer pool (reuse allocations across frames — arena allocator)
- [ ] Parallel DSP chain (rayon for independent effects)
- [ ] Lock-free ring buffer for capture → processing handoff

### Analysis
- [ ] rustfft backend (O(n log n) FFT — replace DFT for production)
- [ ] STFT (short-time Fourier transform) for spectrograms
- [ ] Full EBU R128 loudness (K-weighting + gating)
- [ ] Chromagram (pitch class distribution)
- [ ] Onset detection (transient analysis)

### Quality
- [ ] Property-based tests (proptest: random buffers, sample rates, channels)
- [ ] Fuzz targets (mix, resample, DSP chain)
- [ ] 90%+ code coverage
- [ ] cargo-semver-checks in CI

---

## v1.0.0 Criteria

- [ ] AudioBuffer, AudioClock, Spectrum APIs frozen
- [ ] All DSP effects match reference implementations (within 0.01 dB)
- [ ] SIMD on x86_64 (SSE2+AVX2) and aarch64 (NEON)
- [ ] PipeWire capture/output stable
- [ ] Sinc resampling passing SRC quality tests
- [ ] At least 3 downstream consumers (shruti, jalwa, aethersafta)
- [ ] 90%+ test coverage
- [ ] docs.rs documentation complete
- [ ] No `unsafe` without `// SAFETY:` comments
- [ ] Benchmarks establish golden numbers

---

## Post-v1

### Advanced DSP
- [ ] Convolution reverb (impulse response loading)
- [ ] Multiband compressor
- [ ] Noise suppression (RNNoise integration or custom)
- [ ] Pitch shifting (phase vocoder)
- [ ] Time stretching (WSOLA or phase vocoder)

### Platform
- [ ] CoreAudio backend (macOS)
- [ ] WASAPI backend (Windows)
- [ ] JACK backend (pro audio)
- [ ] WASM target (Web Audio API)

### Format
- [ ] 24-bit audio support
- [ ] DSD (1-bit) support
- [ ] Ambisonic (3D audio) channel layouts

---

## Non-goals

- **Audio I/O (file read/write)** — that's tarang (decode) and symphonia (pure Rust decode)
- **MIDI** — that's shruti's domain
- **Plugin hosting (VST/CLAP/LV2)** — that's shruti
- **Music composition / sequencing** — that's shruti
- **Streaming protocols (RTMP/SRT)** — that's aethersafta
