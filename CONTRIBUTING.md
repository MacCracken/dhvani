# Contributing to Dhvani

Thank you for considering contributing to dhvani. This guide will help you get started.

## Getting started

```bash
git clone https://github.com/MacCracken/dhvani
cd dhvani
cyrius deps    # resolve/vendor deps into lib/
cyrius fmt && cyrius lint && cyrius test   # full local gate
```

## Development workflow

| Command | What it does |
|---------|-------------|
| `cyrius fmt` | Check formatting |
| `cyrius lint` | Lint with warnings-as-errors |
| `cyrius build src/main.cyr build/dhvani` | Compile |
| `cyrius test` | Run all `tests/*.tcyr` (what CI runs) |
| `cyrius test tests/hotpath.bcyr` | Run hot-path benchmarks |
| `cyrius distlib` | (Re)build the `dist/dhvani.cyr` consumer bundle |

## What to contribute

- Bug fixes with regression tests
- New DSP effects (biquad-based, reverb variants, dynamics processing)
- Platform support (audio device I/O via vani, OS-specific features)
- Documentation improvements and examples
- Benchmark improvements

## Code style

- `cyrius fmt` — required (run locally before pushing)
- `cyrius lint` — zero warnings
- Explicit types on public API boundaries
- Doc comments on all public types and functions
- Minimal dependencies — prefer Cyrius-native code over FFI
- Cross-check ported behavior against the `rust-old/` parity oracle
- Keep hot paths alloc-free (the bump allocator never frees within a render)

## Project layout

```
src/
├── main.cyr                    # entry point
├── error.cyr                   # error codes (error base)
├── buffer.cyr, convert.cyr, resample.cyr, dither.cyr   # AudioBuffer, format conversion, mixing, resampling
├── dsp.cyr, biquad.cyr, svf.cyr, eq.cyr, graphic_eq.cyr, reverb.cyr, delay.cyr,
│   compressor.cyr, limiter.cyr, deesser.cyr, pan.cyr, oscillator.cyr,
│   lfo.cyr, envelope.cyr, noise_reduction.cyr          # DSP effects
├── analysis.cyr, fft.cyr, stft.cyr, loudness.cyr, dynamics.cyr,
│   chroma.cyr, key.cyr, beat.cyr, onset.cyr, waveform.cyr, zcr.cyr   # spectral & temporal analysis
├── clock.cyr                   # sample-accurate transport clock
├── midi.cyr, midi_v2.cyr, voice.cyr, midi_routing.cyr, translate.cyr   # MIDI 1.0/2.0, voice, routing
├── graph.cyr                   # RT-safe audio graph
├── meter.cyr                   # lock-free peak metering
├── simd.cyr                    # SIMD abstraction (scalar-only — see below)
├── capture.cyr, record.cyr     # capture + ring-buffer recording
├── device.cyr, playback.cyr    # ALSA device I/O via vani (enumeration, blocking + RT ring player/recorder)
├── synthesis.cyr, voice_synth.cyr, sampler.cyr, creature.cyr,
│   environment.cyr, mechanical.cyr, acoustics.cyr      # synthesis-sibling wrappers
tests/          # *.tcyr suites (unit + property-based) and *.bcyr benchmarks
tests/hw/       # hardware tests (local-only, explicit path)
```

> **SIMD is scalar-only.** `simd.cyr` is a scalar abstraction — the Rust
> SSE2/AVX2/NEON intrinsics did not port (no Cyrius SIMD story yet), which is the
> source of the Rust-vs-Cyrius throughput gap. Contributions here are gated on a
> Cyrius SIMD story landing; see `docs/development/port-audit.md`.

> **No FFI.** Cyrius consumers link the native `dist/dhvani.cyr` bundle; there is
> no C-ABI surface. Device I/O goes through vani (raw `/dev/snd` ALSA PCM via
> ioctls), not PipeWire — see ADR 004.

## Adding a new DSP effect

1. Create `src/your_effect.cyr` with the effect struct and a `dhvani_your_effect_process` function over an `AudioBuffer`
2. Add the module to `[lib].modules` in `cyrius.cyml` (in dependency order) and rebuild the bundle with `cyrius distlib`
3. Add unit tests in `tests/your_effect.tcyr` (silence passthrough, known-signal verification, parameter edge cases)
4. Add a benchmark case to `tests/hotpath.bcyr`
5. Keep the process loop alloc-free — reuse scratch owned by the processor struct

## Commit messages

- Use imperative mood: "add reverb effect" not "added reverb effect"
- Keep subject under 72 characters
- Reference issues where applicable: "fix #42: handle zero-length buffers"

## Pull requests

- One logical change per PR
- Include tests for new functionality
- Update docs if the public API changes
- CI must pass (`cyrius build` + `cyrius test`); run `cyrius fmt` + `cyrius lint` locally first

## Versioning

Dhvani uses SemVer. Version bumps are managed by maintainers.

## License

By contributing, you agree that your contributions will be licensed under GPL-3.0-only.
