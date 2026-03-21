# Contributing to Nada

Thank you for considering contributing to nada. This guide will help you get started.

## Getting started

```bash
git clone https://github.com/MacCracken/nada
cd nada
make check   # runs fmt + clippy + test + audit
```

## Development workflow

| Command | What it does |
|---------|-------------|
| `make check` | Full CI locally (fmt, clippy, test, audit) |
| `make fmt` | Check formatting |
| `make clippy` | Lint with `-D warnings` |
| `make test` | Run all tests |
| `make bench` | Run benchmarks |
| `make doc` | Build docs |
| `cargo test --no-default-features` | Verify scalar fallback (no SIMD) |

## What to contribute

- Bug fixes with regression tests
- New DSP effects (biquad-based, reverb variants, dynamics processing)
- SIMD kernel optimizations (SSE2/AVX2/NEON)
- Platform support (audio backends, OS-specific features)
- Documentation improvements and examples
- Benchmark improvements

## Code style

- `cargo fmt` — required, checked in CI
- `cargo clippy -- -D warnings` — zero warnings
- Explicit types on public API boundaries
- Doc comments (`///`) on all public types and functions
- Minimal dependencies — prefer pure Rust over C bindings
- `#[non_exhaustive]` on public enums
- `#[serde(default)]` on parameter structs for forward compatibility

## Project layout

```
src/
├── lib.rs              # Crate root, public API
├── error.rs            # NadaError enum
├── buffer/             # AudioBuffer, mixing, resampling, format conversion
├── dsp/                # Biquad, EQ, reverb, delay, compressor, de-esser
├── analysis/           # Spectrum (DFT), loudness (LUFS), silence detection
├── clock/              # Sample-accurate transport clock
├── midi/               # MIDI 1.0/2.0 types, voice management, routing
├── simd/               # Platform SIMD kernels (SSE2, AVX2, NEON)
├── capture/            # PipeWire audio capture/output (feature-gated)
└── tests/              # Integration tests
```

## Adding a new DSP effect

1. Create `src/dsp/your_effect.rs` with the effect struct and `process(&mut AudioBuffer)` method
2. Add `pub mod your_effect;` and re-exports in `src/dsp/mod.rs`
3. Add unit tests in the module (silence passthrough, known-signal verification, parameter edge cases)
4. Add a benchmark in `benches/dsp.rs`
5. If the effect has SIMD-friendly inner loops, add kernels in `src/simd/`

## Commit messages

- Use imperative mood: "add reverb effect" not "added reverb effect"
- Keep subject under 72 characters
- Reference issues where applicable: "fix #42: handle zero-length buffers"

## Pull requests

- One logical change per PR
- Include tests for new functionality
- Update docs if the public API changes
- All CI checks must pass (fmt, clippy, test, audit, deny)

## Versioning

Nada uses `day.month` SemVer (e.g., `0.22.3`). Version bumps are managed by maintainers.

## License

By contributing, you agree that your contributions will be licensed under AGPL-3.0-only.
