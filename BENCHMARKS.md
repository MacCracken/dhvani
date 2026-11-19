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

## 2.3.3 — the P(-1) sweep (same box, one session, interleaved)

2.3.2 (the tagged tree) against 2.3.3, each run twice in the order 2.3.2 → 2.3.3 → 2.3.2
→ 2.3.3; every cell is the mean of its two runs. `tests/bench_dsp.bcyr` is new in 2.3.3.
It times the per-block DSP paths the sweep rewrote, through calls whose signatures 2.3.2
shares, so the same file runs on both trees.

Every rewrite below is bit-identical to the code it replaced: the tests compare against
kept reference implementations or golden checksums.

| per block (`tests/bench_dsp.bcyr`) | 2.3.2 | 2.3.3 | speedup |
|----------|------:|------:|--------:|
| `pan_process`, stereo 512 | 10.66 µs | **1.57 µs** | 6.8× |
| `noise_reducer_process`, stereo 4096 | 24.0 ms | **3.94 ms** | 6.1× |
| `conv_process`, 4096-tap IR, stereo 512 | 3.03 ms | **547 µs** | 5.5× (the one-block-latency stream; the zero-latency form measured 558 µs) |
| `reverb_process`, stereo 512 | 178.6 µs | **67.4 µs** | 2.7× |
| `delay_process`, stereo 512 | 39.2 µs | **15.6 µs** | 2.5× |
| `svf_process` (lowpass), stereo 512 | 33.5 µs | **15.0 µs** | 2.2× |
| `moddelay_process`, stereo 512 | 81.6 µs | 73.0 µs | 1.12× |
| `eq_process`, 3 bands, stereo 512 | 23.5 µs | 24.0 µs | 0.98× (noise) |
| `waveform_compute`, 1 s stereo | 1.20 ms | **447 µs** | 2.7× (see below) |
| `resample_sinc` 44.1 → 48 kHz, 1 s stereo | 153 ms | 156 ms | 0.98× (noise) |

| per second of audio (`tests/bench_compare.bcyr`) | 2.3.2 | 2.3.3 | speedup |
|----------|------:|------:|--------:|
| fft 4096 mono | 3.22 ms | **768 µs** | 4.2× |
| dynamics mono_1s | 4.72 ms | **2.88 ms** | 1.64× |
| limiter stereo_1s | 4.90 ms | **3.58 ms** | 1.37× |
| r128 stereo_1s | 4.98 ms | **3.69 ms** | 1.35× |
| compressor stereo_1s | 6.80 ms | **5.28 ms** | 1.29× |
| gain / peak / biquad_lp stereo_1s | — | — | each within ±1 % |

| hot path (`tests/hotpath.bcyr`) | 2.3.2 | 2.3.3 | speedup |
|----------|------:|------:|--------:|
| `fft_spectrum` (1024) | 669 µs | **155 µs** | 4.3× |
| per-sample osc / biquad / svf, the 512-block simd kernels, the stereo biquad | — | — | each within ±3 % |

- **Waveform:** the first A/B showed it 15 % *slower*. The new NaN-safe fold called
  `dh_min_ignore_nan` → `f64_min`, four calls per sample. Both folds are now inline, with
  exactly the helpers' selection rule (a test checks the bits, signed zeros included).
  The row is from a second interleaved A/B of `bench_dsp` run after that fix, which
  reproduced the other rows within 3 %.
- **FFT:** the twiddle table is cached per size and the butterflies walk raw storage. At
  1024 points the transform alone is 6.2× faster, and `fft_spectrum` adds its magnitude
  pass on top of that. Convolution and noise reduction gain mostly through the FFT, the
  rest from walked loops.
- **Graph and meter** are not in a bench file. The repair agent measured them with
  same-harness probes:
  - 32 no-op nodes, 2ch × 512: 209 → 48 µs per cycle.
  - 200 nodes, 1ch × 64: 298 → 33 µs per cycle.
  - meter block, 2ch × 512: 31.7 → 10.1 µs.
- **R128's LRA** sort was O(n²): it went from 1.2 s to 2 ms on 30 min of blocks, which
  this 1 s bench cannot show.
- **Memory:** `dh_vec_zeros` / `dhvani_buffer_clone` now allocate exactly. STFT, tempo
  and R128 halve their per-call footprint (R128 on 10 s stereo: 16.8 → 7.1 MB). Every
  steady-state RT path in the CHANGELOG's table now allocates 0 bytes.

## 2.3.2 — the packed kernels (same box, one session, interleaved)

Three trees, each run twice in the order 2.3.1 → deps → 2.3.2 → 2.3.1 → deps → 2.3.2, with
every cell the mean of its two runs:

- **2.3.1**: a clean `git archive` of 2.3.1.
- **deps**: 2.3.2's dependency bump with 2.3.1's kernels.
- **2.3.2**: the bump plus the packed kernels, the biquad stereo fast path and the
  resampler's default path.

All three ran the same two bench files, which 2.3.2 changes in two ways:

- **Eight new rows** (`add`, `sum_sq`, `clamp`, `gate`, `wsum`, `f32->i16`, the stereo
  biquad kernel, and a stereo biquad's `process`).
- **A fixture fix.** `dhvani_osc_new`, `dhvani_biquad_new` and `dhvani_svf_new` take an
  INTEGER sample rate. Both bench files passed `f64_from(44100)`, whose bit pattern is a
  ~4.7e18 Hz rate. That made the oscillator sit at phase ≈ 0 (13.5 ns at 2.3.1; 50 ns for
  a real 440 Hz sine) and the "1 kHz lowpass" a near-passthrough. The rows below are the
  fixed fixtures, so compare them only with each other.

**The dependency bump moved nothing**: every row within ±3.8 % of 2.3.1, inside run-to-run
spread (2.3.2's own two compressor runs differ by 4.9 %). The box ran ~25 % slower than in
the 2.3.1 session (the unchanged compressor: 5.05 → 6.52 ms), so read the ratios, not the
absolute times.

| hot path (`tests/hotpath.bcyr`) | 2.3.1 | deps | 2.3.2 | vs deps |
|----------|------:|-----:|------:|--------:|
| `osc_sample` (sine, per-sample) | 50 ns | 50 ns | 50 ns | 1.00× |
| `biquad_process_sample` (per-sample) | 25 ns | 25 ns | 25 ns | 1.00× |
| `svf_process_sample` (per-sample) | 28 ns | 28 ns | 28 ns | 1.00× |
| `simd_add_buffers` (512) | 7.34 µs | 7.38 µs | **127 ns** | **58×** |
| `simd_apply_gain` (512) | 5.12 µs | 5.18 µs | **85 ns** | **61×** |
| `simd_sum_of_squares` (512) | 2.84 µs | 2.84 µs | **194 ns** | **14.7×** |
| `simd_weighted_sum` (512) | 5.48 µs | 5.28 µs | **417 ns** | **12.7×** |
| `simd_peak_abs` (512) | 5.90 µs | 5.91 µs | **1.04 µs** | 5.7× |
| `simd_i16_to_f32` (512) | 5.11 µs | 5.22 µs | **1.04 µs** | 5.0× |
| `simd_clamp` (512) | 7.54 µs | 7.33 µs | **1.68 µs** | 4.4× |
| `simd_biquad_stereo` (256 frames) | 17.6 µs | 17.2 µs | **3.99 µs** | 4.3× |
| `biquad_process`, stereo full wet (256 frames) | 15.6 µs | 15.4 µs | **4.01 µs** | 3.8× |
| `simd_f32_to_i16` (512) | 7.84 µs | 7.62 µs | **2.71 µs** | 2.8× |
| `simd_noise_gate` (512) | 2.82 µs | 2.71 µs | **1.04 µs** | 2.6× |
| `fft_spectrum` (1024-radix-2) | 684 µs | 665 µs | 664 µs | 1.00× |

| per second of audio (`tests/bench_compare.bcyr`) | 2.3.1 | deps | 2.3.2 | vs deps |
|----------|------:|-----:|------:|--------:|
| gain stereo_1s | 879 µs | 878 µs | **16.3 µs** | **54×** |
| peak stereo_1s | 1.02 ms | 1.02 ms | **173 µs** | 5.9× |
| biquad_lp stereo_1s | 2.56 ms | 2.58 ms | **669 µs** | 3.85× |
| r128 stereo_1s | 8.72 ms | 8.87 ms | **4.87 ms** | 1.82× |
| compressor stereo_1s | 6.52 ms | 6.61 ms | 6.57 ms | 1.01× |
| limiter stereo_1s | 4.77 ms | 4.83 ms | 4.77 ms | 1.01× |
| dynamics mono_1s | 4.55 ms | 4.67 ms | 4.60 ms | 1.02× |
| fft 4096 mono | 3.10 ms | 3.22 ms | 3.17 ms | 1.02× |

- `add`, `gain`, `sum_sq` and `wsum` are packed (`f64v_*` / `f64v256_*`). `peak`, `clamp`,
  `gate` and the conversions have no packed verb and are walked over the element storage.
  The stereo biquad is one 2-lane vector per frame.
- r128 gains through its K-weighting, which runs `biquad_process` on the stereo buffer at
  full wet and so takes the fast path. The compressor, limiter, dynamics and FFT do not
  touch the kernels and are flat.
- The per-sample rows and the FFT are dhvani's own scalar code, unchanged, and flat.

Notes:
- Per-sample DSP (osc/biquad/svf) benched with `bench_run_batch` (200k calls,
  batched to amortize the ~240 ns/pair `clock_gettime` overhead).
- The FFT is the scalar radix-2 Cooley-Tukey; a per-block analysis cost, not a
  per-sample hot path. Room for a real-FFT / split-radix optimization later.
- All these paths are alloc-free per sample (the free-less bump allocator requires
  it); `fft_spectrum` allocates its output `Spectrum` per block by design.
