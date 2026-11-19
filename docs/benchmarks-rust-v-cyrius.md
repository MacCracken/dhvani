# Benchmarks: Rust vs Cyrius

> dhvani first-port (2.0.0) benchmark comparison — Rust 1.1.0 oracle vs the Cyrius port.
>
> - **Rust**: criterion 0.8, `cargo bench --release`, run 2026-07-05 on `rust-old/`
>   (dhvani 1.1.0, abaco 1.1.0). f32 samples, autovectorized (AVX2).
> - **Cyrius 2.0.0**: cc 6.4.5, `cyrius bench` (`tests/bench_compare.bcyr`), run
>   2026-07-05 against `abaco + dist/dhvani.cyr`. f64 samples, **scalar fallbacks only**
>   (the x86/aarch64 SIMD arms were platform-blocked).
> - **Cyrius 2.3.2**: cc 6.7.18, same bench, run 2026-11-18. The buffer kernels run on
>   Cyrius's packed verbs (`f64v_*` / `f64v256_*`, AVX2 here), and the stereo biquad takes
>   the oracle's fast path. That session ran ~25 % slow (see `BENCHMARKS.md`), so the
>   2.3.2 column is if anything pessimistic.
> - **Platform**: x86_64 Linux. Inputs: 1 s @ 44100 (`stereo_1s` = 88200 samples;
>   `mono_1s` = 44100), 4096-point mono FFT.

The port trades speed for correctness and portability: everything is f64 (the
hisab/ganita math is f64-only), on a young toolchain, and through 2.3.1 everything was
scalar too. The 2.0.0 column is the honest cost of that, and the 2.3.2 column is what
restoring the SIMD kernels bought back.

## Head-to-Head (same operation, same size)

| Operation (stereo_1s unless noted) | Rust | Cyrius 2.0.0 | ratio | Cyrius 2.3.2 | ratio | Why (2.3.2) |
|-----------|-----:|-------:|------:|-------:|------:|------|
| **SIMD kernels** | | | | | | |
| `apply_gain` | 4.86 µs | 865 µs | 178× | **16.3 µs** | **3.4×** | packed scale, 4 f64 lanes vs Rust's 8 f32 |
| `peak_abs` | 2.76 µs | 694 µs | 251× | **173 µs** | **63×** | walked: Cyrius has no packed max/compare verb |
| **DSP (per-sample)** | | | | | | |
| `biquad_lp` | 124 µs | 4.21 ms | 34× | **669 µs** | **5.4×** | the oracle's stereo fast path: L/R in one 2-lane vector |
| `compressor` | 884 µs | 5.43 ms | 6.1× | 6.57 ms | 7.4× | envelope math per sample; unchanged code, slow session |
| `limiter` | 558 µs | 3.44 ms | 6.2× | 4.77 ms | 8.5× | same story |
| **Analysis** | | | | | | |
| `fft` (4096) | 347 µs | 3.25 ms | 9.4× | 3.17 ms | 9.1× | scalar radix-2 Cooley-Tukey, unchanged |
| `measure_r128` | 414 µs | 12.5 ms | 30× | **4.87 ms** | **11.8×** | K-weighting biquads now take the stereo fast path |
| `dynamics` (mono_1s) | 357 µs | 5.68 ms | 16× | 4.60 ms | 13× | crest/true-peak (cubic interp), unchanged |

**Takeaways.** At 2.0.0 the gap was widest where Rust's f32 SIMD wins most: the flat
reductions (`peak`, `gain`) ran 180–250× faster in Rust. 2.3.2's packed kernels close most
of it. `apply_gain` is 3.4× off, about what 4 f64 lanes against 8 f32 lanes predicts.
`peak_abs` is still 63× off, because Cyrius has no packed max/compare verb, so it is a
tight scalar walk. The stereo biquad went from 34× to 5.4× off, and R128 from 30× to 12×
through its K-weighting. The compressor and limiter were already within ~6× and do not
vectorize. The analysis rows want a real-FFT / split-radix, not lanes. Correctness parity
is unchanged: every element-wise kernel is bit-identical to its scalar form.

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
