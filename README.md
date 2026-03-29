# dhvani

**Core audio engine for Rust.**

Buffers, DSP, resampling, mixing, analysis, synthesis, and capture — in a single crate. The audio equivalent of [ranga](https://crates.io/crates/ranga) (image processing) and [tarang](https://crates.io/crates/tarang) (media framework).

> **Name**: Dhvani (ध्वनि, Sanskrit) — sound, resonance.

[![Crates.io](https://img.shields.io/crates/v/dhvani.svg)](https://crates.io/crates/dhvani)
[![docs.rs](https://docs.rs/dhvani/badge.svg)](https://docs.rs/dhvani)
[![CI](https://github.com/MacCracken/dhvani/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/dhvani/actions/workflows/ci.yml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg)](LICENSE)

---

## What it does

dhvani is the **audio processing core** — it owns the audio math so nobody else has to. Applications build their audio features on top of dhvani.

| Capability | Details |
|------------|---------|
| **Audio buffers** | `AudioBuffer` — f32 interleaved, channel-aware, sample-rate-aware, buffer pool |
| **Mixing** | Sum N sources with channel/rate validation |
| **Resampling** | Linear + sinc (Blackman-Harris window, Draft/Good/Best quality) |
| **Format conversion** | i16, i24, i32, f32, f64, u8 with roundtrip fidelity; dithering (TPDF + noise-shaped) |
| **DSP effects** | Biquad EQ, SVF filter, parametric/graphic EQ, compressor, limiter, reverb, convolution reverb, delay, de-esser, panner, noise gate, automation curves, routing matrix |
| **Analysis** | FFT spectrum, STFT, EBU R128 loudness, dynamics (true peak), chromagram, onset/beat/key detection |
| **Synthesis** | Subtractive, FM, additive, wavetable, granular, physical modeling, drum, vocoder, sampler (via [naad](https://crates.io/crates/naad)/[nidhi](https://crates.io/crates/nidhi)) |
| **Voice synthesis** | Glottal source, formant filtering, phoneme sequencing, prosody (via [svara](https://crates.io/crates/svara)) |
| **Acoustics** | Room IR generation, convolution/FDN reverb, ambisonics decode, room presets (via [goonj](https://crates.io/crates/goonj)) |
| **MIDI** | MIDI 1.0/2.0, voice management, clip operations, routing, translation |
| **Transport clock** | Sample-accurate position, tempo/beats, PTS timestamps for A/V sync |
| **Audio graph** | RT-safe graph with topological execution, latency compensation, double-buffered plan swap |
| **Metering** | Lock-free peak/RMS/LUFS metering via atomics, peak hold with decay |
| **PipeWire capture** | Device enumeration, per-source capture, output, hot-plug detection |
| **SIMD** | SSE2/AVX2/NEON acceleration — mixing, gain, clamp, peak, RMS, format conversion, biquad stereo |

---

## Quick start

```toml
[dependencies]
dhvani = "0.22"
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

// Analyze
let spectrum = analysis::spectrum_fft(&mixed, 4096)?;
let r128 = analysis::measure_r128(&mixed)?;
println!("Peak: {:.2}, LUFS: {:.1}", mixed.peak(), r128.integrated_lufs());

// Resample for output
let output = resample_linear(&mixed, 48000)?;
```

---

## Features

| Flag | Default | Description |
|------|---------|-------------|
| `dsp` | Yes | DSP effects (EQ, compressor, limiter, reverb, convolution, delay, de-esser, panner, oscillator, LFO, envelope, SVF, automation, routing) |
| `analysis` | Yes | Audio analysis (FFT, STFT, R128 loudness, dynamics, chromagram, onset/beat/key detection). Implies `dsp` |
| `midi` | Yes | MIDI 1.0/2.0 events, voice management, routing, translation |
| `graph` | Yes | RT-safe audio graph, lock-free metering |
| `simd` | Yes | SSE2/AVX2/NEON acceleration |
| `synthesis` | No | Synthesis engines via [naad](https://crates.io/crates/naad) |
| `voice` | No | Voice synthesis via [svara](https://crates.io/crates/svara). Implies `synthesis` |
| `creature` | No | Creature/animal vocals via [prani](https://crates.io/crates/prani). Implies `synthesis` |
| `environment` | No | Environmental sounds via [garjan](https://crates.io/crates/garjan). Implies `synthesis` |
| `mechanical` | No | Mechanical sounds via [ghurni](https://crates.io/crates/ghurni). Implies `synthesis` |
| `sampler` | No | Sample playback via [nidhi](https://crates.io/crates/nidhi) |
| `acoustics` | No | Room acoustics via [goonj](https://crates.io/crates/goonj). Implies `analysis` |
| `g2p` | No | Grapheme-to-phoneme via [shabda](https://crates.io/crates/shabda). Implies `voice` |
| `pipewire` | No | PipeWire audio capture/output (Linux) |
| `parallel` | No | Parallel graph execution via rayon. Implies `graph` |
| `full` | No | All features |

```toml
# Everything (default: dsp + analysis + midi + graph + simd)
dhvani = "0.22"

# Core only — buffers, mixing, resampling, clock
dhvani = { version = "0.22", default-features = false }

# Media player — DSP + analysis, no MIDI or graph
dhvani = { version = "0.22", default-features = false, features = ["dsp", "analysis", "simd"] }

# Full synthesis + acoustics
dhvani = { version = "0.22", features = ["full"] }
```

---

## Architecture

```
dhvani
├── buffer/        AudioBuffer, format conversion, mixing, resampling, dithering
├── clock/         Sample-accurate transport, tempo, beats, PTS
├── dsp/           Biquad, SVF, EQ, compressor, limiter, reverb, convolution, delay, automation, routing
├── analysis/      FFT, STFT, R128 loudness, dynamics, chromagram, onset/beat/key detection
├── midi/          MIDI 1.0/2.0, voice management, clips, routing, translation
├── graph/         RT-safe audio graph, topological execution, latency compensation
├── meter/         Lock-free peak/RMS/LUFS metering
├── capture/       PipeWire capture/output, device enumeration
├── simd/          SSE2/AVX2/NEON kernels with scalar fallback
├── synthesis/     Synth engines via naad (subtractive, FM, additive, wavetable, granular, drum, vocoder)
├── voice_synth/   Voice synthesis via svara (glottal, formant, phoneme, prosody)
├── acoustics/     Room acoustics via goonj (IR generation, convolution, FDN, ambisonics, presets)
├── creature/      Animal vocals via prani
├── environment/   Nature sounds via garjan
├── mechanical/    Mechanical sounds via ghurni
├── sampler/       Sample playback via nidhi
├── g2p/           Text-to-phoneme via shabda
└── ffi/           C-compatible API
```

Full details: [docs/architecture/overview.md](docs/architecture/overview.md)

---

## Consumers

| Project | Usage |
|---------|-------|
| **[shruti](https://github.com/MacCracken/shruti)** | DAW — all audio math (mix, DSP, analysis, transport, synthesis) |
| **[jalwa](https://github.com/MacCracken/jalwa)** | Media player — playback EQ, spectrum visualizer, resampling, normalization |
| **[aethersafta](https://github.com/MacCracken/aethersafta)** | Compositor — PipeWire capture, audio mixing for streams |
| **[kiran](https://github.com/MacCracken/kiran)** | Game engine — game audio, spatial sound, creature/environment synthesis |

---

## Dependency stack

```
dhvani (audio engine)
├── abaco (DSP math: amplitude/dB, poly_blep, panning, filters)
├── naad (synthesis engines)         [feature: synthesis]
├── svara (voice synthesis)          [feature: voice]
├── goonj (room acoustics)           [feature: acoustics]
├── prani (creature vocals)          [feature: creature]
├── garjan (environmental sounds)    [feature: environment]
├── ghurni (mechanical sounds)       [feature: mechanical]
├── nidhi (sample playback)          [feature: sampler]
└── shabda (grapheme-to-phoneme)     [feature: g2p]
```

---

## Building from source

```bash
git clone https://github.com/MacCracken/dhvani.git
cd dhvani

cargo build                          # default features
cargo build --features full          # everything
cargo build --features pipewire      # with PipeWire (Linux, requires libpipewire-dev)
cargo test --features full           # 597 tests + 22 doctests
cargo bench --features full          # 51 benchmarks
```

---

## License

AGPL-3.0-only. See [LICENSE](LICENSE) for details.
