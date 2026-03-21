# Performance Guide

## SIMD acceleration

With the `simd` feature (default), the following operations are vectorized:

| Operation | SSE2 (x86_64) | AVX2 (x86_64) | NEON (aarch64) | Speedup |
|-----------|---------------|----------------|----------------|---------|
| `mix()` | 4 f32/op | 8 f32/op | 4 f32/op | 4-8x |
| `apply_gain()` | 4 f32/op | 8 f32/op | 4 f32/op | 4-8x |
| `clamp()` / `hard_limiter()` | 4 f32/op | 8 f32/op | 4 f32/op | 4-8x |
| `peak()` | 4 f32/op | 8 f32/op | 4 f32/op | 3-8x |
| `rms()` | 4 f32/op | — | 4 f32/op | 3-4x |
| `noise_gate()` | 4 f32/op | — | 4 f32/op | 2-4x |
| `i16_to_f32()` / `f32_to_i16()` | 4/op | — | scalar | 4x |
| `resample_sinc()` | weighted sum | — | weighted sum | 2-4x |

Operations NOT vectorized (IIR feedback dependencies):
- `BiquadFilter::process()`, `Reverb`, `Compressor`, `Limiter`

## Benchmarking

```bash
cargo bench                    # all suites
cargo bench --bench simd       # SIMD-specific
cargo bench --bench analysis   # FFT vs DFT comparison
```

## FFT performance

| Algorithm | Complexity | 4096-point | Use case |
|-----------|-----------|-----------|----------|
| `spectrum_dft()` | O(n^2) | ~slow | Testing, small windows |
| `spectrum_fft()` | O(n log n) | ~fast | Production, all sizes |

Always prefer `spectrum_fft()` for production code.

## Memory optimization

- **Pre-allocate buffers** before the audio processing loop
- **Reuse DSP effect instances** — create once, call `process()` repeatedly
- **Use `DeEsser`** — internally reuses its sidechain buffer (no per-call allocation)
- **Avoid `clone()`** in RT paths — use references where possible

## Real-time audio tips

1. **Never allocate** in the audio callback — pre-allocate everything
2. **Never lock** a mutex — use `try_lock()` or lock-free structures
3. **Use `GraphProcessor`** — it handles plan swapping without RT allocation
4. **Use `MeterBank`** — atomic peak levels, no mutex
5. **Buffer size trade-off**: smaller = lower latency, larger = less overhead
