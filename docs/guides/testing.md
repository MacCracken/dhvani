# Testing Guide

## Running tests

```bash
# All tests (default features, includes SIMD)
cargo test

# Without SIMD (scalar fallback)
cargo test --no-default-features

# With PipeWire (Linux only)
cargo test --features pipewire

# Specific module
cargo test dsp::biquad
cargo test midi::translate

# Show output
cargo test -- --nocapture
```

## Test categories

| Category | Location | Count | What it tests |
|----------|----------|-------|---------------|
| Unit tests | `src/*/mod.rs` | ~200 | Individual functions, edge cases, error paths |
| Integration | `src/tests/mod.rs` | ~10 | Multi-module workflows (DSP chains, format pipelines) |
| Doc tests | `src/lib.rs` | 7 | Code examples in documentation |
| Fuzz targets | `fuzz/fuzz_targets/` | 3 | Random input robustness (mix, resample, DSP) |
| Benchmarks | `benches/` | 7 suites | Performance regression detection |

## Coverage

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --summary-only   # quick summary
cargo llvm-cov --html           # detailed HTML report
```

Current coverage: 94%+ line coverage.

## Benchmarks

```bash
cargo bench                      # all benchmarks
cargo bench --bench simd         # SIMD vs scalar comparison
cargo bench --bench analysis     # FFT, STFT, R128, dynamics
cargo bench --bench dsp          # all DSP effects
```

Compare SIMD vs scalar:
```bash
cargo bench --bench simd                          # with SIMD
cargo bench --bench simd --no-default-features    # scalar only
```

## Fuzzing

Requires nightly Rust:
```bash
cargo install cargo-fuzz
cargo +nightly fuzz run fuzz_mix -- -max_total_time=300
cargo +nightly fuzz run fuzz_resample -- -max_total_time=300
cargo +nightly fuzz run fuzz_dsp -- -max_total_time=300
```

## CI

The CI pipeline runs on every push/PR:
- Format check, clippy (zero warnings)
- Tests on Linux (x86_64) and macOS (aarch64)
- Scalar-only test (`--no-default-features`)
- MSRV check (Rust 1.89)
- Security audit, cargo-deny, cargo-vet
- Coverage upload to codecov
- Benchmarks (non-regression)
- Fuzzing (5 min per target, nightly)
