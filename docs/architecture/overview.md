# Nada Architecture

> Core audio engine — buffers, DSP, resampling, mixing, analysis, and capture.
>
> **Name**: Nada (नाद, Sanskrit) — primordial sound, cosmic vibration.
> Extracted from [shruti](https://github.com/MacCracken/shruti) (DAW) as a standalone, reusable crate.

---

## Design Principles

1. **f32 internally** — all processing in 32-bit float; format conversion at I/O boundaries only
2. **Sample-accurate** — clock, mixing, and DSP operate at sample granularity
3. **Zero-allocation hot path** — mixing and DSP reuse buffers; no alloc per frame
4. **SIMD where it matters** — mixing, resampling, gain — the inner loops
5. **PipeWire-native** — first-class Linux audio capture/output (feature-gated)

---

## Module Structure

```
src/
├── lib.rs              Public API, Result type
├── error.rs            NadaError enum
├── buffer/
│   └── mod.rs          AudioBuffer, SampleFormat, Layout, mix(), resample_linear()
├── dsp/
│   └── mod.rs          noise_gate, hard_limiter, compress, normalize, EQ, dB conversions
├── analysis/
│   └── mod.rs          spectrum_dft, loudness_lufs, is_silent, Spectrum type
├── clock/
│   └── mod.rs          AudioClock (position, tempo, beats, PTS, seek)
└── tests/
    └── mod.rs          Integration tests
```

---

## Pipeline

```
Input (file, capture, synthesis)
    │
    ▼
AudioBuffer (f32 interleaved, channels, sample_rate)
    │
    ├──▶ DSP chain (EQ → compress → gate → limit)
    │
    ├──▶ Analysis (spectrum, loudness, silence detection)
    │
    ├──▶ Mix (sum multiple sources with gain)
    │
    ├──▶ Resample (44.1k ↔ 48k ↔ 96k)
    │
    ▼
Output (encode via tarang, play via PipeWire, sync via clock PTS)
```

---

## Key Types

### AudioBuffer
Core sample buffer. Holds f32 interleaved samples with channel count, sample rate, and frame count. Provides peak/RMS/gain/clamp operations.

### AudioClock
Sample-accurate transport. Tracks position in samples, converts to seconds/ms/beats/PTS. Tempo-aware for DAW integration. Generates PTS timestamps for A/V sync with aethersafta.

### Spectrum
DFT magnitude analysis. Provides frequency bins, dominant frequency detection, and per-bin access. Simple O(n^2) DFT for correctness; rustfft backend planned for production.

---

## Consumers

| Project | Usage |
|---------|-------|
| **shruti** | DAW — drops engine/dsp crates, uses nada for all audio math |
| **jalwa** | Media player — playback EQ, visualizer spectrum, resampling |
| **aethersafta** | Compositor — PipeWire capture, audio mixing for streams |
| **tarang** | Media framework — audio analysis, fingerprinting input |
| **hoosh** | Inference gateway — audio preprocessing for whisper STT |
| **Streaming app** | Live broadcast — mic processing, desktop audio capture |
