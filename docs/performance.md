# Performance Guide

## Scalar-f64 kernels

The Cyrius port is **scalar-f64 only** — SIMD intrinsics (`simd/x86`,
`simd/aarch64`) are platform-blocked pending a Cyrius SIMD story, and all
sample math widened from f32 to f64. The following operations run as scalar f64
loops (no vector lanes):

| Operation | Kernel | Notes |
|-----------|--------|-------|
| `mix()` | scalar f64 | element-wise add/accumulate |
| `apply_gain()` | scalar f64 | element-wise multiply |
| `clamp()` / `dhvani_dsp_hard_limiter()` | scalar f64 | element-wise clamp |
| `peak()` | scalar f64 | running max |
| `rms()` | scalar f64 | running sum of squares |
| `dhvani_dsp_noise_gate()` | scalar f64 | element-wise threshold |
| i16/i24/i32 ↔ f64 convert | scalar f64 | format conversion |
| `resample_sinc()` | scalar f64 | weighted sum |

IIR-feedback effects are inherently sequential (they were never vectorized in
the Rust source either):
- `BiquadFilter::process()`, `Reverb`, `Compressor`, `Limiter`

> The scalar-only kernels are the source of the Rust-vs-Cyrius throughput gap —
> see [`benchmarks-rust-v-cyrius.md`](benchmarks-rust-v-cyrius.md).

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
