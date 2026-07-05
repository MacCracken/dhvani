# Benchmarks: Rust vs Cyrius

> dhvani first-port (2.0.0) benchmark comparison — Rust 1.1.0 oracle vs the Cyrius port.
>
> - **Rust**: criterion 0.8, `cargo bench --release`, run 2026-07-05 on `rust-old/`
>   (dhvani 1.1.0, abaco 1.1.0). f32 samples, autovectorized (AVX2).
> - **Cyrius**: cc 6.4.5, `cyrius bench` (`tests/bench_compare.bcyr`), run 2026-07-05
>   against `abaco + dist/dhvani.cyr`. f64 samples, **scalar fallbacks only** (the
>   x86/aarch64 SIMD arms are platform-blocked — an accepted regression until Cyrius
>   has a SIMD story; dhvani is the ecosystem's SIMD owner).
> - **Platform**: x86_64 Linux. Inputs: 1 s @ 44100 (`stereo_1s` = 88200 samples;
>   `mono_1s` = 44100), 4096-point mono FFT.

The port trades speed for correctness and portability: everything is f64 (the
hisab/ganita math is f64-only) and scalar, on a young toolchain. The numbers below
are the honest cost of that, and the baseline every future optimization measures
against.

## Head-to-Head (same operation, same size)

| Operation (stereo_1s unless noted) | Rust | Cyrius | Ratio | Why |
|-----------|-----:|-------:|------:|------|
| **SIMD kernels** | | | | |
| `apply_gain` | 4.86 µs | 865 µs | **178×** | f32 AVX2 vs f64 scalar + per-element `vec` access |
| `peak_abs` | 2.76 µs | 694 µs | **251×** | pure reduction — best case for SIMD, worst ratio |
| **DSP (per-sample)** | | | | |
| `biquad_lp` | 124 µs | 4.21 ms | 34× | Direct-Form-II per sample; some Rust vectorization |
| `compressor` | 884 µs | 5.43 ms | **6.1×** | envelope math per sample — doesn't vectorize, so f32 gains little |
| `limiter` | 558 µs | 3.44 ms | **6.2×** | peak-hold + release per sample; same story |
| **Analysis** | | | | |
| `fft` (4096) | 347 µs | 3.25 ms | 9.4× | scalar radix-2 Cooley-Tukey |
| `measure_r128` | 414 µs | 12.5 ms | 30× | K-weighting biquads + block RMS |
| `dynamics` (mono_1s) | 357 µs | 5.68 ms | 16× | crest/true-peak (cubic interp) |

**Takeaways.** The gap is widest exactly where Rust's f32 autovectorization wins
most — flat SIMD reductions (`peak`, `gain`) run 180–250× faster in Rust. It
narrows sharply for per-sample DSP whose inner loop is branchy envelope/gain math
that doesn't vectorize: the compressor and limiter are only ~6× off, i.e. Cyrius is
already competitive there. Restoring the SIMD kernels (when Cyrius gains SIMD
intrinsics) would close most of the top rows; a real-FFT / split-radix would help
the analysis rows. Correctness parity is exact (1625 assertions); this is purely
the throughput cost of scalar-f64-on-a-young-toolchain.

## Full Rust benchmark set (2026-07-05)

### SIMD (`benches/simd.rs`)

| Benchmark | Rust |
|-----------|-----:|
| `simd_peak_stereo_1s` | 2.76 µs |
| `simd_clamp_stereo_1s` | 3.54 µs |
| `simd_rms_stereo_1s` | 7.07 µs |
| `simd_mix_2_stereo_1s` | 18.68 µs |
| `gain_channels/1` | 1.63 µs |
| `gain_channels/2` | 4.86 µs |
| `gain_channels/6` | 16.6 µs |
| `gain_channels/8` | 23.2 µs |
| `gain_buffer_sizes/256` | 15.7 ns |
| `gain_buffer_sizes/4096` | 248 ns |
| `gain_buffer_sizes/65536` | 7.51 µs |
| `peak_buffer_sizes/4096` | 244 ns |
| `peak_buffer_sizes/65536` | 4.63 µs |

### DSP (`benches/dsp.rs`)

| Benchmark | Rust |
|-----------|-----:|
| `noise_gate_stereo_1s` | 3.91 µs |
| `normalize_stereo_1s` | 6.14 µs |
| `panner_stereo_1s` | 21.7 µs |
| `biquad_lp_stereo_1s` | 124 µs |
| `limiter_stereo_1s` | 558 µs |
| `compress_legacy_stereo_1s` | 655 µs |
| `compressor_stereo_1s` | 884 µs |
| `reverb_stereo_1s` | 890 µs |

### Analysis (`benches/analysis.rs`)

| Benchmark | Rust |
|-----------|-----:|
| `fft_4096_mono` | 347 µs |
| `dynamics_mono_1s` | 357 µs |
| `chromagram_4096_mono` | 370 µs |
| `r128_stereo_1s` | 414 µs |
| `stft_2048_512_mono_1s` | 11.4 ms |
| `onset_2048_512_mono_1s` | 11.9 ms |
| `dft_4096_mono` | 116 ms |

Cyrius-side numbers: `tests/bench_compare.bcyr` (matched sizes) and
`tests/hotpath.bcyr` (per-sample inner loops); summarized in `BENCHMARKS.md`.
