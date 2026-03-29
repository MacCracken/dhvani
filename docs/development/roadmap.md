# Dhvani Roadmap

> **Principle**: Correctness first, then SIMD, then capture backends. Every consumer gets the same audio math.

---

## v1.0.0 Criteria — All Met

- [x] API frozen: all fields private, accessors only
- [x] No panics in non-test code
- [x] All unsafe blocks have `// SAFETY:` comments
- [x] DSP effects within 0.01 dB of reference implementations
- [x] SIMD parity verified on x86_64 (SSE2 + AVX2) and aarch64 (NEON)
- [x] Format conversion: i16, i24, i32, f32, f64, u8 — all with roundtrip tests
- [x] PipeWire capture/output tested with live daemon
- [x] 90%+ test coverage (90.02% line coverage via cargo-llvm-cov)
- [x] docs.rs complete — every public type documented
- [x] Golden benchmark numbers published
- [x] Zero clippy warnings
- [x] Supply chain clean (audit + deny + vet)

---

## Consumer Adoption (post-v1)

- [ ] shruti adopts dhvani (replace shruti-engine + shruti-dsp + shruti-session MIDI)
- [ ] jalwa adopts dhvani (replace playback buffer + EQ + normalization)
- [ ] aethersafta adopts dhvani (replace PipeWire capture + mixer)
- [ ] tazama uses dhvani DSP (replace tazama-media/dsp/)
- [ ] hoosh uses `dhvani::midi` for music token preprocessing
- [ ] Cross-crate integration tests
- [ ] Benchmark regression: dhvani not slower than code it replaces

---

## Backlog — Demand-Gated

### Advanced DSP

- [ ] Multiband compressor
- [ ] Noise suppression (RNNoise or custom)
- [ ] Pitch shifting (phase vocoder)
- [ ] Time stretching (WSOLA / phase vocoder)

### MIDI Advanced

- [ ] SMF (Standard MIDI File) read/write
- [ ] MIDI clock / sync (MTC, SPP)
- [ ] SysEx handling
- [ ] MPE zone management
- [ ] MIDI tokenization for music LLMs

### Platform Backends

- [ ] CoreAudio (macOS)
- [ ] WASAPI (Windows)
- [ ] JACK (pro audio)
- [ ] WASM (Web Audio API)

### High Sample Rate

- [ ] Validated resampling paths: 44.1k ↔ 48k ↔ 88.2k ↔ 96k ↔ 176.4k ↔ 192k ↔ 352.8k ↔ 384k ↔ 768k
- [ ] Multi-stage resampling for large ratio conversions
- [ ] Oversampled DSP mode — 2x/4x internal rate for reduced aliasing
- [ ] Sinc resampler optimization for high-rate conversions

### Format — Niche

- [ ] u8 a-law / u-law (G.711) — telephony codecs
- [ ] i8 (signed 8-bit) — embedded audio
- [ ] DSD (1-bit) — SACD / audiophile playback
- [ ] Ambisonic (3D audio) channel layouts

### Voice Synthesis (v2.0 scope)

- [ ] Bhava integration — personality-to-prosody mapping
- [ ] Articulatory modeling — graduate from formant when demand justifies

---

## Non-goals

- **Audio I/O (file read/write)** — shravan / tarang
- **Plugin hosting (VST/CLAP/LV2)** — shruti
- **Music composition / sequencing / timeline** — shruti
- **Streaming protocols (RTMP/SRT)** — aethersafta
- **DAW UI / preset management** — shruti
- **Neural TTS / ML-based voice** — hoosh
- **Text-to-phoneme ML models** — hoosh
