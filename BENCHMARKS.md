# dhvani benchmarks

Hot-path throughput for the inner loops a real-time consumer runs every
sample/block. These are the **2.0.0 Cyrius-port baseline** — the port has no prior
Cyrius version to regress against, so future changes compare against these.

Run: `cyrius bench tests/hotpath.bcyr` (links `abaco` + the assembled
`dist/dhvani.cyr`, the same surface a consumer ships).

## Baseline (2.0.0, cycc 6.4.5, x86_64 Linux — **scalar fallbacks only**)

dhvani is the ecosystem's SIMD owner but ships only the scalar kernels in Cyrius
(the x86/aarch64 intrinsic arms are platform-blocked, an accepted regression until
Cyrius has a SIMD story). So these are scalar numbers.

| hot path | per call | ≈ per sample |
|----------|---------:|-------------:|
| `osc_sample` (sine, per-sample) | 63 ns | 63 ns |
| `biquad_process_sample` (per-sample) | 51 ns | 51 ns |
| `svf_process_sample` (per-sample) | 55 ns | 55 ns |
| `simd_peak_abs` (512-block) | 5.71 µs | ~11 ns |
| `simd_apply_gain` (512-block) | 6.72 µs | ~13 ns |
| `simd_i16_to_f32` (512-block) | 6.65 µs | ~13 ns |
| `fft_spectrum` (1024-radix-2) | 713 µs | — |

Notes:
- Per-sample DSP (osc/biquad/svf) benched with `bench_run_batch` (200k calls,
  batched to amortize the ~240 ns/pair `clock_gettime` overhead).
- The FFT is the scalar radix-2 Cooley-Tukey; a per-block analysis cost, not a
  per-sample hot path. Room for a real-FFT / split-radix optimization later.
- All these paths are alloc-free per sample (the free-less bump allocator requires
  it); `fft_spectrum` allocates its output `Spectrum` per block by design.
