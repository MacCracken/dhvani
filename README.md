# dhvani

**Core audio engine for Cyrius.**

Buffers, DSP, resampling, mixing, analysis, synthesis, and device I/O — in a single bundle. The Cyrius port of a 23,695-line Rust library (frozen at `rust-old/` as the parity oracle).

> **Name**: Dhvani (ध्वनि, Sanskrit) — sound, resonance.

[![CI](https://github.com/MacCracken/dhvani/actions/workflows/ci.yml/badge.svg)](https://github.com/MacCracken/dhvani/actions/workflows/ci.yml)
[![License: GPL-3.0](https://img.shields.io/badge/license-GPL--3.0-blue.svg)](LICENSE)

---

## What it does

dhvani is the **audio processing core** — it owns the audio math so nobody else has to. Applications build their audio features on top of dhvani.

| Capability | Details |
|------------|---------|
| **Audio buffers** | `AudioBuffer` — f64 interleaved, channel-aware, sample-rate-aware, buffer pool |
| **Mixing** | Sum N sources with channel/rate validation |
| **Resampling** | Linear + sinc (Blackman-Harris window, Draft/Good/Best quality) |
| **Format conversion** | i16, i24, i32, f32, f64, u8 with roundtrip fidelity; dithering (TPDF + noise-shaped) |
| **DSP effects** | Biquad EQ, SVF filter, parametric/graphic EQ, compressor, limiter, reverb, convolution reverb, delay, de-esser, panner, noise gate, automation curves, routing matrix |
| **Analysis** | FFT spectrum, STFT, EBU R128 loudness, dynamics (true peak), chromagram, onset/beat/key detection |
| **Synthesis** | Subtractive, FM, additive, wavetable, granular, physical modeling, drum, vocoder, sampler (via [naad](https://github.com/MacCracken/naad)/[nidhi](https://github.com/MacCracken/nidhi)) |
| **Voice synthesis** | Glottal source, formant filtering, phoneme sequencing, prosody (via [svara](https://github.com/MacCracken/svara)) |
| **Acoustics** | Room IR generation, convolution/FDN reverb, ambisonics decode, room presets (via [goonj](https://github.com/MacCracken/goonj)) |
| **MIDI** | MIDI 1.0/2.0, voice management, clip operations, routing, translation |
| **Transport clock** | Sample-accurate position, tempo/beats, PTS timestamps for A/V sync |
| **Audio graph** | RT-safe graph with topological execution, latency compensation, double-buffered plan swap |
| **Metering** | Lock-free peak/RMS/LUFS metering via atomics, peak hold with decay |
| **Device I/O** | ALSA PCM via [vani](https://github.com/MacCracken/vani) (raw `/dev/snd` ioctls, no FFI) — device enumeration, blocking play/record, lock-free RT ring player/recorder |

---

## Quick start

Build against the consumer bundle:

```sh
cyrius deps                              # resolve/vendor deps into lib/
cyrius build src/main.cyr build/dhvani   # compile
cyrius test                              # run tests/*.tcyr
```

```cyr
include "dist/dhvani.cyr"

# Create buffers (f64 interleaved) from sample vecs
var vocals = dhvani_buffer_from_interleaved(samples_a, 2, 44100)
var drums  = dhvani_buffer_from_interleaved(samples_b, 2, 44100)

# Mix — dhvani_buffer_mix takes a vec of buffer handles
var sources = vec_new()
vec_push(sources, vocals)
vec_push(sources, drums)
var mixed = dhvani_buffer_mix(sources)

# Compress: (threshold_db, ratio, attack_ms, release_ms, makeup_db, knee_db, mix).
# Negative float literals use f64_neg (a bare -18.0 mis-parses in Cyrius).
var params = dhvani_compparams_new(f64_neg(18.0), 4.0, 10.0, 100.0, 3.0, 6.0, 1.0)
var comp = dhvani_comp_new(params, 44100)
dhvani_comp_process(comp, mixed)
dhvani_dsp_normalize(mixed, 0.95)

# Analyze
var spectrum = dhvani_fft_spectrum(mixed, 4096)
var lufs = dhvani_ops_normalize_to_lufs(mixed, f64_neg(16.0))

# Resample for output
var output = dhvani_buffer_resample_linear(mixed, 48000)
```

---

## Layers

There are no Cargo-style feature flags. Every ported module ships in the single
`dist/dhvani.cyr` bundle; the synthesis/voice/acoustics layers are wrappers that
**externalize** their sibling bundles (naad, svara, …). A consumer links only the
sibling bundles it uses, and unused wrapper refs DCE-prune. The layers:

| Layer | Sibling bundle | Provides |
|-------|----------------|----------|
| Core | — | Buffers, mixing, resampling, clock, format conversion |
| DSP | — | EQ, compressor, limiter, reverb, convolution, delay, de-esser, panner, oscillator, LFO, envelope, SVF, automation, routing |
| Analysis | — | FFT, STFT, R128 loudness, dynamics, chromagram, onset/beat/key detection |
| MIDI | — | MIDI 1.0/2.0 events, voice management, routing, translation |
| Graph | — | RT-safe audio graph, lock-free metering |
| Device I/O | [vani](https://github.com/MacCracken/vani) | ALSA PCM play/record, RT ring player/recorder, device enumeration |
| Synthesis | [naad](https://github.com/MacCracken/naad) | Subtractive, FM, additive, wavetable, granular, drum, vocoder |
| Voice | [svara](https://github.com/MacCracken/svara) | Glottal source, formant, phoneme, prosody |
| Creature | [prani](https://github.com/MacCracken/prani) | Creature/animal vocals |
| Environment | [garjan](https://github.com/MacCracken/garjan) | Environmental sounds |
| Mechanical | [ghurni](https://github.com/MacCracken/ghurni) | Mechanical sounds |
| Sampler | [nidhi](https://github.com/MacCracken/nidhi) | Sample playback |
| Acoustics | [goonj](https://github.com/MacCracken/goonj) | Room IR, convolution/FDN reverb, ambisonics, presets |

> **Blocked** (dependency still Rust, not yet ported to Cyrius): `g2p` (via
> shabda), personality/mood voice mapping (via bhava). SIMD acceleration is a
> post-port non-goal — the port is scalar-only pending a Cyrius SIMD story.

---

## Architecture

```
src/                (flat bundle — one namespace, dhvani_* prefix)
├── buffer.cyr       AudioBuffer, format conversion, mixing, resampling, dithering
├── clock.cyr        Sample-accurate transport, tempo, beats, PTS
├── dsp.cyr …        Biquad, SVF, EQ, compressor, limiter, reverb, convolution, delay, automation, routing
├── analysis.cyr …   FFT, STFT, R128 loudness, dynamics, chromagram, onset/beat/key detection
├── midi.cyr …       MIDI 1.0/2.0, voice management, routing, translation
├── graph.cyr        RT-safe audio graph, topological execution, latency compensation
├── meter.cyr        Lock-free peak/RMS/LUFS metering
├── playback.cyr     AudioBuffer ↔ vani PCM (S16/S24/S32) play/record + RT ring player/recorder
├── device.cyr       ALSA device enumeration + default-open (via vani)
├── synthesis.cyr    Synth engines via naad (subtractive, FM, additive, wavetable, granular, drum, vocoder)
├── voice_synth.cyr  Voice synthesis via svara (glottal, formant, phoneme, prosody)
├── acoustics.cyr    Room acoustics via goonj (IR generation, convolution, FDN, ambisonics, presets)
├── creature.cyr     Animal vocals via prani
├── environment.cyr  Nature sounds via garjan
├── mechanical.cyr   Mechanical sounds via ghurni
└── sampler.cyr      Sample playback via nidhi
```

Full details: [docs/architecture/overview.md](docs/architecture/overview.md)

---

## Consumers

| Project | Usage |
|---------|-------|
| **[shruti](https://github.com/MacCracken/shruti)** | DAW — all audio math (mix, DSP, analysis, transport, synthesis) |
| **[jalwa](https://github.com/MacCracken/jalwa)** | Media player — playback EQ, spectrum visualizer, resampling, normalization |
| **[aethersafta](https://github.com/MacCracken/aethersafta)** | Compositor — ALSA capture (via vani), audio mixing for streams |
| **[kiran](https://github.com/MacCracken/kiran)** | Game engine — game audio, spatial sound, creature/environment synthesis |

---

## Dependency stack

Deps are vendored into `lib/` (committed) and `include`d in dependency order —
not wired as auto-resolved `[deps]`. Consumers supply the sibling bundles for the
layers they use, alongside `dist/dhvani.cyr`.

```
dhvani (audio engine)
├── abaco  (DSP math: amplitude/dB, poly_blep, panning, filters)
├── vani   (ALSA PCM device I/O) + yukti          → device I/O
├── naad   (synthesis engines)                    → synthesis
├── svara  (voice synthesis)                       → voice
├── goonj  (room acoustics)                        → acoustics
├── prani  (creature vocals)                        → creature
├── garjan (environmental sounds)                   → environment
├── ghurni (mechanical sounds)                      → mechanical
├── nidhi  (sample playback)                        → sampler
├── sakshi · hisab · shravan  (shared support: WAV codec, math)
└── shabda · bhava   (still Rust — g2p / bhava-voice blocked)
```

---

## Building from source

```bash
git clone https://github.com/MacCracken/dhvani.git
cd dhvani

cyrius deps                              # resolve/vendor deps into lib/
cyrius build src/main.cyr build/dhvani   # compile
cyrius test                              # run tests/*.tcyr (CI)
cyrius distlib                           # (re)build dist/dhvani.cyr consumer bundle
```

Device I/O is via [vani](https://github.com/MacCracken/vani) — raw `/dev/snd`
ALSA PCM through ioctls, no libpipewire/libasound, no FFI. Hardware tests live
in `tests/hw/` and are local-only (they touch real devices); CI runs `tests/*.tcyr`.

---

## License

GPL-3.0-only. See [LICENSE](LICENSE) for details.
