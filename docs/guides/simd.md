# SIMD Acceleration Guide

## Overview

The Rust dhvani accelerated its buffer kernels with platform intrinsics (SSE2/AVX2
on x86_64, NEON on aarch64) and kept scalar fallbacks beside them. Through 2.3.1 the
Cyrius port shipped only those fallbacks, because Cyrius had no vector story. **Since
2.3.2 `src/simd.cyr` has both again**, on the compiler's flat-array packed verbs. These
are the same builtins abaco and goonj use:

| verb family | lanes | x86_64 | aarch64 | cx |
|-------------|------:|--------|---------|----|
| `f64v_add/sub/mul/scale/dot/…(dst, a, b, n)` | 2 f64 | SSE2 | NEON | per-lane loop |
| `f64v256_add/scale/…(dst, a, b, n)` | 4 f64 | AVX2 (VEX) | — | — |

Every kernel has two forms, named as in the oracle:

- `dhvani_simd_<k>`: the accelerated kernel. `buffer`, `convert`, `resample`, `dsp`
  and `biquad` call this one.
- `dhvani_simd_<k>_scalar`: the scalar fallback (`rust-old/src/simd/mod.rs`
  `<k>_scalar`). It is kept as the **parity oracle** the accelerated form is tested
  against.

## Kernel coverage

512-sample block on x86_64 with AVX2 (`tests/hotpath.bcyr`, mean of two interleaved
runs; see `BENCHMARKS.md`):

| Kernel | How | vs `_scalar` | Bits vs `_scalar` |
|--------|-----|-------------:|-------------------|
| `add_buffers` | packed add, 256-bit from 64 elements | **58×** | identical |
| `apply_gain` | packed scale, 256-bit from 64 elements | **61×** | identical |
| `sum_of_squares` | packed dot (128-bit) | **14.7×** | lane order (see below) |
| `weighted_sum` | packed dot ×2 (128-bit) | **12.7×** | lane order |
| `biquad_stereo` | one interleaved L/R frame = one 2-lane vector | **4.3×** | identical |
| `peak_abs` | walked, NaN-ignoring fold inline | 5.7× | identical |
| `i16_to_f32` / `i24_to_f32` / `u8_to_f32` | walked | 5.0× (i16) | identical |
| `clamp` | walked, bounds inline | 4.4× | identical |
| `f32_to_i16` / `f32_to_i24` / `f32_to_u8` | walked, clamp inline | 2.8× (i16) | identical |
| `noise_gate` | walked | 2.6× | identical |

"Walked" means there is no packed verb for the operation: Cyrius has no packed
min/max, compare-and-mask or int↔float convert. These kernels stay scalar. What they
gain is the per-element `vec_get`/`vec_set` call (two bounds checks each), replaced by
a walk over the vec's element storage.

Through 2.3.1 two of the oracle's callers ran their `not(simd)` code. Both are back on
the default path:

- `dhvani_biquad_process` takes the oracle's stereo fast path again. A 2-channel
  buffer at full wet runs through `biquad_stereo`, on the filter's own coefficients
  and states (no vec is built): **3.8×** on a 256-frame block.
- `dhvani_resample_sinc` computes each frame's kernel weights once, shares them across
  channels, and runs `weighted_sum` on gathered taps. The old loop is kept as
  `dhvani_resample_sinc_scalar`.

## Parity

- **Element-wise kernels are bit-identical to `_scalar`**, on both lane widths. Each
  lane does the same IEEE operation the scalar loop does. `biquad_stereo` keeps the
  scalar grouping, mul then add, and never uses fused multiply-add.
- **The two reductions add in lane order.** `sum_of_squares` and `weighted_sum` keep
  two partial sums and then add them, as Rust's SSE2 arm did. So they differ from the
  left-to-right scalar sum in the last bits. The oracle's own parity tests tolerate
  this (1e-3 and 1e-2). They use only the 128-bit kernel, because `f64v256_dot` reduces
  in a third order, and the answer must not depend on whether the CPU has AVX2.

`tests/simd.tcyr` holds the five oracle parity tests (dropped through 2.3.1), plus a
sweep. The sweep covers every kernel at sizes 0–20, 63–67, 511–513 and 1025, with NaN,
±Inf and −0.0 inputs. It checks two things: the exact bits `_scalar` gives, and no
write past `vec_len`, using six canary slots inside the vec's capacity. It runs once
on the widest path and once forced onto 128-bit lanes (`dh_simd_wide = 0`).

## The no-tail trap

The packed verbs step 2 (`f64v_*`) or 4 (`f64v256_*`) lanes and have **no scalar
tail**. `f64v_add(out, a, b, 1)` writes *two* elements, one past a 1-long row. abaco
shipped exactly that heap overrun through 2.4.8. Never call a verb with a raw length.
Go through the row helpers in `src/simd.cyr` (`dh_simd_row_add`, `_scale`, `_dot`,
`_sum`): they run the 256-bit prefix, then the 128-bit middle, then the odd last
element in scalar code with the same rounding.

## Runtime dispatch

`f64v256_*` is an illegal instruction without AVX2 (SIGILL), so the 256-bit path is
chosen from CPUID: `simd_has_avx2()` from the stdlib `simd` leaf, probed once and
cached in `dh_simd_wide`. It also answers 0 on every non-x86 target, which then takes
the 2-lane NEON or portable path. Rows under 64 elements stay on 128-bit lanes, since
each 256-bit call pays a `vzeroupper` that loses on short rows (goonj ADR-0008).

## Consuming the bundle

`dist/dhvani.cyr` now calls `simd_has_avx2`, so a consumer that includes the bundle
needs the stdlib `simd` leaf (`stdlib = [..., "simd"]`). The packed verbs are compiler
builtins and need nothing.

## Benchmarking

```sh
cyrius bench tests/hotpath.bcyr          # per-sample + per-block kernels
cyrius bench tests/bench_compare.bcyr    # per second of audio, the Rust bench sizes
```

Capture numbers before claiming a throughput win.
