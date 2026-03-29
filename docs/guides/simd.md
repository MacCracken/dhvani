# SIMD Acceleration Guide

## Overview

Dhvani uses platform-specific SIMD instructions to accelerate audio processing. The `simd` feature (enabled by default) selects the optimal implementation at compile time (architecture) and runtime (CPU feature detection for AVX2).

## Platform Coverage

| Kernel | SSE2 (x86_64) | AVX2 (x86_64) | NEON (aarch64) | Scalar fallback |
|--------|:---:|:---:|:---:|:---:|
| `add_buffers` | 4-wide | 8-wide | 4-wide | yes |
| `apply_gain` | 4-wide | 8-wide | 4-wide | yes |
| `clamp` | 4-wide | 8-wide | 4-wide | yes |
| `peak_abs` | 4-wide | 8-wide | 4-wide | yes |
| `sum_of_squares` | 4-wide f64 | 8-wide f64 | 4-wide | yes |
| `noise_gate` | 4-wide | 8-wide | 4-wide | yes |
| `weighted_sum` | 4-wide | 8-wide | 4-wide | yes |
| `i16_to_f32` | 4-wide | 8-wide | 4-wide | yes |
| `f32_to_i16` | 4-wide | 8-wide | 4-wide | yes |
| `i24_to_f32` | 4-wide | 8-wide | 4-wide | yes |
| `f32_to_i24` | 4-wide | 8-wide | 4-wide | yes |
| `u8_to_f32` | 4-wide | 8-wide | 4-wide | yes |
| `f32_to_u8` | 4-wide | 8-wide | 4-wide | yes |
| `biquad_stereo` | 2×f64 | — | 2×f64 | yes |

## Runtime Dispatch (x86_64)

On x86_64, SSE2 is always available (baseline). AVX2 is detected at runtime:

```rust
pub fn apply_gain(samples: &mut [f32], gain: f32) {
    if is_x86_feature_detected!("avx2") {
        unsafe { apply_gain_avx2(samples, gain) };
    } else {
        unsafe { apply_gain_sse2(samples, gain) };
    }
}
```

## Expected Speedups

Measured on x86_64 with AVX2, stereo 1-second buffer (88,200 samples):

| Operation | SIMD Time | Notes |
|-----------|-----------|-------|
| `apply_gain` | ~5.5 µs | 8-wide AVX2 |
| `clamp` | ~5.5 µs | 8-wide AVX2 |
| `peak_abs` | ~3.2 µs | 8-wide AVX2 with horizontal max |
| `sum_of_squares` (RMS) | ~7.8 µs | f64 accumulation to avoid precision loss |
| `noise_gate` | ~6.2 µs | Branchless via SIMD compare + mask |
| `biquad_stereo` | ~147 µs | 2×f64 SSE2 cross-channel (−42% vs scalar) |

## Biquad Cross-Channel Optimization

The stereo biquad processes L and R channels simultaneously using 2×f64 SIMD:

- **SSE2**: `__m128d` (2 × f64) — processes both channels per sample
- **NEON**: `float64x2_t` — same approach on aarch64
- Activated automatically for stereo buffers at full wet mix

## Disabling SIMD

```toml
[dependencies]
dhvani = { version = "0.22", default-features = false, features = ["dsp", "analysis"] }
```

Without the `simd` feature, all kernels use scalar fallbacks. Useful for debugging or platforms without SIMD support.

## Benchmarking SIMD vs Scalar

```bash
# With SIMD (default)
cargo bench --bench simd

# Without SIMD (scalar fallback)
cargo bench --bench simd --no-default-features --features dsp
```

## Adding New SIMD Kernels

1. Add scalar implementation in `src/simd/mod.rs`
2. Add SSE2 + AVX2 in `src/simd/x86.rs`
3. Add NEON in `src/simd/aarch64.rs`
4. Add dispatch in `mod.rs` with `#[cfg(target_arch)]`
5. Add parity test (SIMD vs scalar) in `mod.rs` tests
6. Wire into the calling module with `#[cfg(feature = "simd")]` gate
