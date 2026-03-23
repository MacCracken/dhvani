# dhvani

**Core audio engine for Rust.**

Buffers, DSP, resampling, mixing, analysis, and capture — in a single crate. The audio equivalent of [ranga](https://crates.io/crates/ranga) (image processing) and [tarang](https://crates.io/crates/tarang) (media framework).

> **Name**: Dhvani (ध्वनि, Sanskrit) — sound, resonance.
> Extracted from [shruti](https://github.com/MacCracken/shruti) (DAW) as a standalone, reusable engine.

[![Crates.io](https://img.shields.io/crates/v/dhvani.svg)](https://crates.io/crates/dhvani)
[![CI](https://github.com/MacCracken/dhvani/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/dhvani/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

---

## What it does

dhvani is the **audio processing core** — it owns the audio math so nobody else has to. Applications build their audio features on top of dhvani.

| Capability | Details |
|------------|---------|
| **Audio buffers** | Unified `AudioBuffer` type — f32 interleaved, channel-aware, sample-rate-aware |
| **Mixing** | Sum N sources with channel/rate validation |
| **Resampling** | Linear + sinc (Blackman-Harris window); 44.1k ↔ 48k ↔ 96k |
| **DSP effects** | Biquad EQ, compressor, limiter, reverb, delay, de-esser, panner, noise gate, normalize |
| **Analysis** | FFT spectrum, STFT, EBU R128 loudness, dynamics, chromagram, onset detection |
| **MIDI** | MIDI 1.0/2.0, voice management, clip operations, routing |
| **Transport clock** | Sample-accurate position, tempo/beats, PTS timestamps for A/V sync |
| **Audio graph** | RT-safe graph with topological execution and double-buffered plan swap |
| **PipeWire capture** | Per-source audio capture and output (feature-gated) |
| **SIMD** | SSE2/AVX2/NEON acceleration for mixing, gain, clamp, peak, RMS |

---

## Quick start

```toml
[dependencies]
dhvani = "0.20"
```

```rust
use dhvani::buffer::{AudioBuffer, mix, resample_linear};
use dhvani::dsp::{self, Compressor, CompressorParams};
use dhvani::analysis;
use dhvani::clock::AudioClock;

// Create buffers
let vocals = AudioBuffer::from_interleaved(samples_a, 2, 44100)?;
let drums = AudioBuffer::from_interleaved(samples_b, 2, 44100)?;

// Mix
let mut mixed = mix(&[&vocals, &drums])?;

// Process
let mut comp = Compressor::new(CompressorParams {
    threshold_db: -18.0, ratio: 4.0, attack_ms: 10.0, release_ms: 100.0,
    makeup_gain_db: 3.0, knee_db: 6.0, ..Default::default()
}, 44100)?;
comp.process(&mut mixed);
dsp::normalize(&mut mixed, 0.95);
dsp::noise_gate(&mut mixed, 0.01);

// Analyze
let spectrum = analysis::spectrum_fft(&mixed, 4096).unwrap();
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
| `dsp` | Yes | DSP effects (EQ, compressor, limiter, reverb, delay, de-esser, panner, oscillator, LFO, envelope) |
| `analysis` | Yes | Audio analysis (FFT, STFT, R128 loudness, dynamics, chromagram, onset detection). Implies `dsp` |
| `midi` | Yes | MIDI 1.0/2.0 events, voice management, routing, translation |
| `graph` | Yes | RT-safe audio graph, lock-free metering |
| `simd` | Yes | SSE2/AVX2/NEON acceleration for mixing, gain, peak, RMS, format conversion |
| `pipewire` | No | PipeWire audio capture/output backend (Linux only) |
| `full` | No | All features including PipeWire |

```toml
# Everything (default)
dhvani = "0.20"

# Core only — buffers, mixing, resampling, clock (no DSP/MIDI/analysis/graph)
dhvani = { version = "0.20", default-features = false }

# Media player — DSP + analysis, no MIDI or graph
dhvani = { version = "0.20", default-features = false, features = ["dsp", "analysis", "simd"] }

# With PipeWire capture (Linux)
dhvani = { version = "0.20", features = ["pipewire"] }
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
dsp::hard_limiter(&mut buf, 0.95);       // prevent clipping
dsp::normalize(&mut buf, 1.0);           // peak normalize

let db = dsp::amplitude_to_db(0.5);      // -6.02 dB
let amp = dsp::db_to_amplitude(-6.0);    // ~0.501
```

### Analysis

```rust
let spectrum = analysis::spectrum_fft(&buf, 4096).unwrap();
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
  └── with dhvani (ध्वनि — sound, resonance) as its audio engine
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
| **0.20.3** | Complete engine | Buffers, DSP (EQ/reverb/compressor/delay), MIDI 1.0/2.0, SIMD, FFT, R128, graph, PipeWire, 265+ tests |
| **0.21.3** | Hardening & API freeze | Safety comments on all unsafe, API encapsulation, 24-bit/f64/u8 formats, panic elimination, buffer pool |
| **0.22.3** | Testing, SIMD & adoption | SIMD completeness (AVX2/NEON gaps), 90%+ coverage, docs.rs, consumer integration, golden benchmarks |
| **1.0.0** | Stable | Frozen API, 3+ consumers, reference-quality DSP, full platform parity |

Full details: [docs/development/roadmap.md](docs/development/roadmap.md)

---

## Building from source

```bash
git clone https://github.com/MacCracken/dhvani.git
cd dhvani

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
