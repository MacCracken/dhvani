# Performance Guide

## Buffer kernels (packed since 2.3.2)

All sample math is f64 (widened from the oracle's f32). Since 2.3.2 the buffer kernels
run on Cyrius's packed verbs: `f64v_*` (2 lanes; SSE2, NEON) and `f64v256_*` (4 lanes;
AVX2, chosen by CPUID). Through 2.3.1 they were scalar only. 512-sample block, x86_64
with AVX2:

| Operation | Kernel | vs 2.3.1 |
|-----------|--------|---------:|
| `mix()` | packed add | 58× |
| `apply_gain()` | packed scale | 61× |
| `rms()` | packed dot (sum of squares) | 14.7× |
| `resample_sinc()` | packed dot (weighted sum), weights once per frame | — |
| `peak()` | walked running max | 5.7× |
| `clamp()` / `dhvani_dsp_hard_limiter()` | walked clamp | 4.4× |
| `dhvani_dsp_noise_gate()` | walked threshold | 2.6× |
| i16/i24/u8 ↔ f64 convert | walked | 2.8–5× |
| `BiquadFilter::process()`, stereo at full wet | 2-lane L/R vector | 3.8× |

"Walked" kernels have no packed verb (no packed min/max, compare or convert), so they
stay scalar, but over the raw element storage instead of a `vec_get` call per sample.

The other IIR-feedback effects are sequential per channel and stay scalar, as in the
Rust source: `Reverb`, `Compressor`, `Limiter`, and mono or part-wet biquads. Per
second of stereo audio, gain is 54× and biquad lowpass 3.85×, while the compressor and
limiter are unchanged. See [`benchmarks-rust-v-cyrius.md`](benchmarks-rust-v-cyrius.md)
and `BENCHMARKS.md`.

## Benchmarking

```bash
cyrius test tests/hotpath.bcyr    # hot-path .bcyr benches
```

Captured numbers live in [`BENCHMARKS.md`](../BENCHMARKS.md); the Rust-vs-Cyrius
comparison lives in [`benchmarks-rust-v-cyrius.md`](benchmarks-rust-v-cyrius.md).

## FFT performance

| Algorithm | Complexity | Use case |
|-----------|-----------|-----------|
| `dhvani_analysis_spectrum_dft()` | O(n^2) | Testing, small windows |
| `dhvani_fft_spectrum()` | O(n log n) | Production, all sizes |

Always prefer `dhvani_fft_spectrum()` for production code.

## Memory optimization

The free-less bump allocator never reclaims — a per-sample or per-block
allocation leaks unboundedly across a render. Hot paths must allocate zero
bytes/sample.

- **Pre-allocate buffers** before the audio processing loop
- **Reuse DSP effect instances** — create once, call the process function
  repeatedly against scratch owned by the processor struct
- **Reuse sidechain/scratch buffers** — no per-call allocation in the effect
- **Avoid copies** in RT paths — pass buffers by handle where possible

## Real-time audio tips

1. **Never allocate** in the audio path — pre-allocate everything (the bump
   allocator does not free)
2. **Never lock** a mutex — use lock-free structures; the RT ring path is
   lock-free/alloc-free
3. **Use the RT ring player/recorder** (`dhvani_player_*` / `dhvani_recorder_*`)
   — zero per-block allocation
4. **Use the meter/graph** — the graph swaps plans without RT allocation
5. **Buffer size trade-off**: smaller = lower latency, larger = less overhead
