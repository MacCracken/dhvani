# SIMD Acceleration Guide

## Overview

The Rust dhvani used platform-specific SIMD intrinsics (SSE2/AVX2 on x86_64,
NEON on aarch64) to accelerate audio processing. In the Cyrius port these
intrinsics have **no equivalent** — Cyrius has no SIMD story yet — so
`src/simd.cyr` ports **only the scalar fallbacks**. Every kernel below runs
scalar. This is the primary source of the Rust-vs-Cyrius throughput gap and is
tracked as a post-port non-goal (see `docs/development/port-audit.md`).

## Kernel Coverage

The scalar kernels ported from `rust-old/src/simd/mod.rs` (the SSE2/AVX2/NEON
arms in `x86.rs` / `aarch64.rs` were dropped):

| Kernel | Cyrius symbol | Status |
|--------|---------------|:---:|
| `add_buffers` | `dhvani_simd_add_buffers` | scalar |
| `apply_gain` | `dhvani_simd_apply_gain` | scalar |
| `clamp` | `dhvani_simd_clamp` | scalar |
| `peak_abs` | `dhvani_simd_peak_abs` | scalar |
| `sum_of_squares` | `dhvani_simd_sum_of_squares` | scalar (f64 accum) |
| `noise_gate` | `dhvani_simd_noise_gate` | scalar |
| `weighted_sum` | `dhvani_simd_weighted_sum` | scalar |
| `i16_to_f32` | `dhvani_simd_i16_to_f32` | scalar |
| `f32_to_i16` | `dhvani_simd_f32_to_i16` | scalar |
| `i24_to_f32` | `dhvani_simd_i24_to_f32` | scalar |
| `f32_to_i24` | `dhvani_simd_f32_to_i24` | scalar |
| `u8_to_f32` | `dhvani_simd_u8_to_f32` | scalar |
| `f32_to_u8` | `dhvani_simd_f32_to_u8` | scalar |
| `biquad_stereo` | `dhvani_simd_biquad_stereo` | scalar |

These scalar kernels are the public compute surface that `buffer`, `convert`,
and `resample` call. Samples are `vec` handles holding f64 in each 8-byte slot
(integer-PCM sources hold their integer value per slot); everything operates in
place and is alloc-free.

## No Runtime Dispatch

There is no `is_x86_feature_detected!`-style dispatch: there is exactly one
implementation per kernel — the scalar one — selected at build time by
whichever unit includes `src/simd.cyr`. Nothing to feature-gate, nothing to
detect at runtime.

## Benchmarking

Hot-path kernels have `.bcyr` benchmarks; the numbers are the proof a port
didn't regress relative to the scalar Rust path:

```sh
cyrius bench                             # run tests/*.bcyr
```

`tests/hotpath.bcyr` and `tests/bench_compare.bcyr` cover the compute-heavy
paths. Capture numbers before claiming a throughput win.

## Restoring SIMD Later

If Cyrius grows SIMD intrinsics, the port would re-add architecture-specific
arms behind the scalar kernels in `src/simd.cyr` and dispatch to them, keeping
the scalar path as the portable fallback and parity oracle. Until then, scalar
is the only path.
