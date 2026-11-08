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

## 2.2.2 re-measure (cycc 6.5.41, same x86_64 Linux box)

Captured after the toolchain 6.4.12 → 6.5.41 bump and the sibling-dep sweep, per
the "never skip benchmarks" rule. 6.5.41's SESTYPE normalization (6.4.56) changed
the emitted code for the `f64_gt(x, y) == 1` idiom dhvani uses at 40+ DSP sites,
so this is the check that the codegen change cost nothing. **No path regressed.**

| hot path | 2.0.0 (6.4.5) | 2.2.2 (6.5.41) | delta |
|----------|--------------:|---------------:|------:|
| `osc_sample` (sine, per-sample) | 63 ns | **59 ns** | −6 % |
| `biquad_process_sample` (per-sample) | 51 ns | **50 ns** | −2 % |
| `svf_process_sample` (per-sample) | 55 ns | **51 ns** | −7 % |
| `simd_peak_abs` (512-block) | 5.71 µs | **4.16 µs** | −27 % |
| `simd_apply_gain` (512-block) | 6.72 µs | **5.00 µs** | −26 % |
| `simd_i16_to_f32` (512-block) | 6.65 µs | **4.88 µs** | −27 % |
| `fft_spectrum` (1024-radix-2) | 713 µs | **689 µs** | −3 % |

⚠ Read the deltas as "no regression", not as a precise speedup: the two runs are
months apart on a box whose load was not pinned, and 6.5.41 subtracts a measured
1.34 µs timer floor per clock read. The three 512-block SIMD kernels moving
together by ~27 % is the only delta large enough to be structural.

## 2.2.7 re-measure (cycc 6.6.20 vs 6.6.10, same box, back to back)

Captured for the toolchain 6.6.10 → 6.6.20 bump and the 6.6.20 dependency batch.
Unlike the 2.2.2 row this is a same-session A/B: a clean `git archive` of 2.2.6 on its
own pin (6.6.10) against the 2.2.7 tree, interleaved base/new/base/new; each cell is the
mean of the two runs. **No path regressed.**

| hot path (`tests/hotpath.bcyr`) | 2.2.6 (6.6.10) | 2.2.7 (6.6.20) | delta |
|----------|--------------:|---------------:|------:|
| `osc_sample` (sine, per-sample) | 17.4 ns | **14.0 ns** | −20 % |
| `biquad_process_sample` (per-sample) | 25.2 ns | 25.9 ns | +3 % (run 2: equal) |
| `svf_process_sample` (per-sample) | 28.3 ns | 28.2 ns | 0 % |
| `simd_peak_abs` (512-block) | 5.92 µs | 5.89 µs | 0 % |
| `simd_apply_gain` (512-block) | 5.14 µs | 5.15 µs | 0 % |
| `simd_i16_to_f32` (512-block) | 5.14 µs | 5.07 µs | −1 % |
| `fft_spectrum` (1024-radix-2) | 666 µs | 680 µs | +2 % (within max spread) |

| per second of audio (`tests/bench_compare.bcyr`) | 2.2.6 (6.6.10) | 2.2.7 (6.6.20) | delta |
|----------|--------------:|---------------:|------:|
| compressor stereo_1s | 5.40 ms | **5.18 ms** | −4 % |
| limiter stereo_1s | 3.58 ms | **3.47 ms** | −3 % |
| gain / peak / biquad_lp / r128 / dynamics stereo_1s | — | — | each within ±2 % |
| fft 4096 mono | 3.18 ms | 3.26 ms | +2.5 % avg, +1 % min |

The compressor and limiter rows matter most: 2.2.7 also fixes their attack/release
coefficients (see CHANGELOG), and abaco's `amplitude_to_db`, which both call per frame,
gained input checks upstream. They still came out faster. The coefficients are computed
once per block, so fixing them costs nothing per sample.

The per-sample figures sit well under the 2.2.2 table's (59 → 14 ns for `osc_sample`).
That gap spans ~60 toolchain releases and different box load, so compare only within a
table.

## 2.3.0 re-measure (cycc 6.7.9 vs 6.6.20, same box, back to back)

Captured for the toolchain 6.6.20 → 6.7.9 bump and the 6.7.9 dependency batch: a clean
`git archive` of 2.2.7 on its own pin against the 2.3.0 tree, interleaved twice per tree;
each cell is the mean of the two runs. **No path regressed**; every delta is within ±2.3 %.

| hot path (`tests/hotpath.bcyr`) | 2.2.7 (6.6.20) | 2.3.0 (6.7.9) | delta |
|----------|--------------:|---------------:|------:|
| `osc_sample` (sine, per-sample) | 14.0 ns | 14.0 ns | 0 % |
| `biquad_process_sample` (per-sample) | 25.5 ns | 25.0 ns | −2.0 % |
| `svf_process_sample` (per-sample) | 28.0 ns | 28.0 ns | 0 % |
| `simd_peak_abs` (512-block) | 5.83 µs | 5.81 µs | −0.3 % |
| `simd_apply_gain` (512-block) | 5.09 µs | 5.17 µs | +1.5 % |
| `simd_i16_to_f32` (512-block) | 4.96 µs | 5.08 µs | +2.3 % |
| `fft_spectrum` (1024-radix-2) | 672 µs | 663 µs | −1.4 % |

| per second of audio (`tests/bench_compare.bcyr`) | 2.2.7 (6.6.20) | 2.3.0 (6.7.9) | delta |
|----------|--------------:|---------------:|------:|
| gain stereo_1s | 889 µs | 903 µs | +1.6 % |
| peak stereo_1s | 1.02 ms | 1.03 ms | +1.2 % |
| biquad_lp stereo_1s | 2.64 ms | 2.63 ms | −0.3 % |
| compressor stereo_1s | 5.39 ms | 5.33 ms | −1.1 % |
| limiter stereo_1s | 3.54 ms | 3.47 ms | −2.0 % |
| r128 stereo_1s | 8.87 ms | 8.67 ms | −2.2 % |
| dynamics mono_1s | 4.71 ms | 4.64 ms | −1.3 % |
| fft 4096 mono | 3.27 ms | 3.21 ms | −1.7 % |

## 2.3.1 re-measure (cycc 6.7.18 vs 6.7.9, same box, back to back)

Captured for the toolchain 6.7.9 → 6.7.18 bump and the newest dependency tags: a clean
`git archive` of 2.3.0 on its own pin against the 2.3.1 tree, interleaved twice per tree;
each cell is the mean of the two runs. **No path regressed**; every delta is within ±2.4 %.

| hot path (`tests/hotpath.bcyr`) | 2.3.0 (6.7.9) | 2.3.1 (6.7.18) | delta |
|----------|--------------:|---------------:|------:|
| `osc_sample` (sine, per-sample) | 13.5 ns | 13.5 ns | 0 % |
| `biquad_process_sample` (per-sample) | 24.5 ns | 25.0 ns | +2.0 % (1 ns resolution) |
| `svf_process_sample` (per-sample) | 27.5 ns | 27.5 ns | 0 % |
| `simd_peak_abs` (512-block) | 5.73 µs | 5.78 µs | +0.9 % |
| `simd_apply_gain` (512-block) | 4.96 µs | 5.04 µs | +1.6 % |
| `simd_i16_to_f32` (512-block) | 4.92 µs | 5.04 µs | +2.4 % |
| `fft_spectrum` (1024-radix-2) | 656 µs | 651 µs | −0.7 % |

| per second of audio (`tests/bench_compare.bcyr`) | 2.3.0 (6.7.9) | 2.3.1 (6.7.18) | delta |
|----------|--------------:|---------------:|------:|
| gain stereo_1s | 862 µs | 856 µs | −0.6 % |
| peak stereo_1s | 997 µs | 998 µs | +0.1 % |
| biquad_lp stereo_1s | 2.53 ms | 2.52 ms | −0.7 % |
| compressor stereo_1s | 5.09 ms | 5.05 ms | −0.7 % |
| limiter stereo_1s | 3.36 ms | 3.36 ms | 0 % |
| r128 stereo_1s | 8.45 ms | 8.51 ms | +0.8 % |
| dynamics mono_1s | 4.52 ms | 4.47 ms | −1.2 % |
| fft 4096 mono | 3.08 ms | 3.10 ms | +0.6 % |

Notes:
- Per-sample DSP (osc/biquad/svf) benched with `bench_run_batch` (200k calls,
  batched to amortize the ~240 ns/pair `clock_gettime` overhead).
- The FFT is the scalar radix-2 Cooley-Tukey; a per-block analysis cost, not a
  per-sample hot path. Room for a real-FFT / split-radix optimization later.
- All these paths are alloc-free per sample (the free-less bump allocator requires
  it); `fft_spectrum` allocates its output `Spectrum` per block by design.
