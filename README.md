# nada

**Core audio engine for Rust.**

Buffers, DSP, resampling, mixing, analysis, and capture — in a single crate. The audio equivalent of [ranga](https://crates.io/crates/ranga) (image processing) and [tarang](https://crates.io/crates/tarang) (media framework).

> **Name**: Nada (नाद, Sanskrit) — primordial sound, cosmic vibration.
> Extracted from [shruti](https://github.com/MacCracken/shruti) (DAW) as a standalone, reusable engine.

[![Crates.io](https://img.shields.io/crates/v/nada.svg)](https://crates.io/crates/nada)
[![CI](https://github.com/MacCracken/nada/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/nada/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

---

## What it does

nada is the **audio processing core** — it owns the audio math so nobody else has to. Applications build their audio features on top of nada.

| Capability | Details |
|------------|---------|
| **Audio buffers** | Unified `AudioBuffer` type — f32 interleaved, channel-aware, sample-rate-aware |
| **Mixing** | Sum N sources with channel/rate validation |
| **Resampling** | Linear interpolation (sinc in v0.22); 44.1k ↔ 48k ↔ 96k |
| **DSP effects** | Noise gate, hard limiter, compressor, normalize, EQ (biquad in v0.21) |
| **Analysis** | DFT spectrum, dominant frequency, LUFS loudness, silence detection |
| **Transport clock** | Sample-accurate position, tempo/beats, PTS timestamps for A/V sync |
| **PipeWire capture** | Per-source audio capture and output (feature-gated) |
| **SIMD** | SSE2/AVX2/NEON acceleration for mixing and DSP (v0.22) |

---

## Quick start

```toml
[dependencies]
nada = "0.20"
```

```rust
use nada::buffer::{AudioBuffer, mix, resample_linear};
use nada::dsp;
use nada::analysis;
use nada::clock::AudioClock;

// Create buffers
let vocals = AudioBuffer::from_interleaved(samples_a, 2, 44100)?;
let drums = AudioBuffer::from_interleaved(samples_b, 2, 44100)?;

// Mix
let mut mixed = mix(&[&vocals, &drums])?;

// Process
dsp::compress(&mut mixed, 0.5, 4.0);
dsp::normalize(&mut mixed, 0.95);
dsp::noise_gate(&mut mixed, 0.01);

// Analyze
let spectrum = analysis::spectrum_dft(&mixed, 4096);
let loudness = analysis::loudness_lufs(&mixed);
println!("Peak: {:.2}, LUFS: {:.1}", mixed.peak(), loudness);

// Resample for output
let output = resample_linear(&mixed, 48000)?;

// Sync with video via clock
let mut clock = AudioClock::new(48000);
clock.start();
clock.advance(output.frames as u64);
println!("PTS: {} us", clock.pts_us());
```

---

## Features

| Flag | Default | Description |
|------|---------|-------------|
| `simd` | Yes | SSE2/AVX2/NEON acceleration for mixing and DSP |
| `pipewire` | No | PipeWire audio capture/output backend |
| `full` | No | All features |

```toml
# Minimal (no system deps)
nada = { version = "0.20", default-features = false }

# With PipeWire capture (Linux)
nada = { version = "0.20", features = ["pipewire"] }
```

---

## Key types

### `AudioBuffer`

Core sample buffer. Holds f32 interleaved samples with channel count and sample rate.

```rust
let mut buf = AudioBuffer::silence(2, 44100, 44100); // 1 second stereo
buf.apply_gain(0.5);
buf.clamp();
println!("Peak: {:.3}, RMS: {:.3}, Duration: {:.2}s",
    buf.peak(), buf.rms(), buf.duration_secs());
```

### `AudioClock`

Sample-accurate transport with tempo awareness and PTS generation for A/V sync.

```rust
let mut clock = AudioClock::with_tempo(44100, 120.0); // 120 BPM
clock.start();
clock.advance(44100); // 1 second
println!("Position: {:.2}s, Beat: {:.1}, PTS: {} us",
    clock.position_secs(),
    clock.position_beats().unwrap(),
    clock.pts_us());
```

### DSP

```rust
dsp::noise_gate(&mut buf, 0.01);        // silence below threshold
dsp::compress(&mut buf, 0.5, 4.0);      // reduce dynamic range
dsp::hard_limiter(&mut buf, 0.95);       // prevent clipping
dsp::normalize(&mut buf, 1.0);           // peak normalize

let db = dsp::amplitude_to_db(0.5);      // -6.02 dB
let amp = dsp::db_to_amplitude(-6.0);    // ~0.501
```

### Analysis

```rust
let spectrum = analysis::spectrum_dft(&buf, 4096);
if let Some(freq) = spectrum.dominant_frequency() {
    println!("Dominant: {:.0} Hz", freq);
}

let lufs = analysis::loudness_lufs(&buf);
let silent = analysis::is_silent(&buf, -60.0);
```

---

## The Sanskrit Stack

```
shruti (श्रुति — that which is heard) creates music
  └── with nada (नाद — primordial sound) as its audio engine
       └── carried by tarang (तरंग — wave) as its media framework
            └── colored by ranga (रंग — color) for visual processing
```

---

## Who uses this

| Project | Usage |
|---------|-------|
| **[shruti](https://github.com/MacCracken/shruti)** | DAW — all audio math (mix, DSP, analysis, transport) |
| **[jalwa](https://github.com/MacCracken/jalwa)** | Media player — playback EQ, spectrum visualizer, resampling |
| **[aethersafta](https://github.com/MacCracken/aethersafta)** | Compositor — PipeWire capture, audio mixing for streams |
| **[tarang](https://crates.io/crates/tarang)** | Media framework — audio analysis, fingerprint input |
| **[hoosh](https://github.com/MacCracken/hoosh)** | Inference gateway — audio preprocessing for whisper STT |

---

## Roadmap

| Version | Milestone | Key features |
|---------|-----------|--------------|
| **0.20.3** | Foundation | Buffers, mix, resample, DSP, analysis, clock, 40+ tests |
| **0.21.3** | DSP & conversion | Biquad EQ, reverb, delay, format conversion, sinc resample |
| **0.22.3** | SIMD & capture | SSE2/AVX2/NEON mixing, PipeWire capture/output |
| **0.23.3** | Integration | shruti/jalwa/aethersafta adoption, rustfft, buffer pool |
| **1.0.0** | Stable API | Frozen types, 90%+ coverage, reference-quality DSP |

Full details: [docs/development/roadmap.md](docs/development/roadmap.md)

---

## Building from source

```bash
git clone https://github.com/MacCracken/nada.git
cd nada

# Build (no system deps needed)
cargo build

# Build with PipeWire (Linux, requires libpipewire-dev)
sudo apt install libpipewire-0.3-dev
cargo build --features pipewire

# Run tests
cargo test

# Run benchmarks
cargo bench

# Run all CI checks locally
make check
```

---

## Versioning

Pre-1.0 releases use `0.D.M` (day.month) SemVer — e.g. `0.20.3` = March 20th.
Post-1.0 follows standard SemVer.

---

## License

AGPL-3.0-only. See [LICENSE](LICENSE) for details.
