# dhvani — Rust → Cyrius Port Audit

Per-module parity ledger for the **2.0.0** port. The Rust oracle is frozen at
`rust-old/` (23,695 lines, 64 modules, ~660 `#[test]` blocks); every Cyrius
module must match it function-for-function. Update the relevant row whenever a
module's status changes. Sequencing (waves, gates) lives in
[`roadmap.md`](roadmap.md); this file is the **per-module reference**.

**Status:** ✅ ported & tested · 🟡 partial · ⬜ pending · ⛔ blocked
**LOC** = Rust lines (incl. tests) at `rust-old/src/`. **T** = `#[test]` count.

---

## Conventions established (apply to every module)

Same discipline naad/svara used (they are the parity model — both are audio
libraries dhvani consumes):

- **f32 → f64** everywhere. The audio-stack math (`hisab`/`ganita`, and abaco)
  is f64-only; widening is forced and improves precision. Loosen f32-oracle
  test tolerances where bit-exactness isn't meaningful.
- **Float literals**: integers via `f64_from(n)`; non-integers as module-top
  `var` constants holding the IEEE-754 hex bit pattern (decimal in a comment).
- **`enum` → integer `var` codes.** dhvani has many: `SampleFormat`, `Layout`,
  `CrossfadeType`, `FadeCurve`, `ResampleQuality`, `FilterType`, `SvfMode`,
  `BandType`, `Waveform`, MIDI message kinds, graph node kinds, … all become
  integer const codes + `if/else` dispatch (no `match`-on-variant).
  - **Data-carrying variants** (`FilterType::Peaking{gain_db}`, LowShelf,
    HighShelf) can't ride an i64 code → thread `gain_db` as a **separate field/
    param** through `BiquadCoeffs::design` and every call site.
- **`enum` errors → integer codes.** `NadaError` collapses to `ERR_*` codes in
  `src/error.cyr`; validators return `ERR_NONE` (0) or a negative code. Rich
  struct-variant context (`String expected/actual`) can't ride an i64 — drop it
  or route through a last-error buffer.
- **`Result<T>` / `Option<T>` → sentinel returns** (error code, NaN, `-1`, null
  handle) unless a real payload is needed (then `lib/tagged.cyr`). No unwinding,
  no `panic`. `clock::position_beats`/`samples_per_beat` `Option<f64>` → NaN
  sentinel (matches abaco's NEG_INF convention).
- **closures → fn-ptr** (`fnptr`/`callback`) or inline loops. Every
  `.iter().map()/.fold()/.collect()` closure becomes an explicit index loop.
- **tuple returns → out-params.** Cyrius has no tuples. `abaco::dsp` ports the
  DSP helpers to out-pointer form: `equal_power_crossfade(t, &out_a, &out_b)`,
  `constant_power_pan(pan, &out_l, &out_r)` — return 0, store f64 via `store64`.
  Same for `weighted_sum`/`simd` `(f32,f32)` returns.
- **`Vec<T>` / `SmallVec<T>` → stdlib `vec`**; f64 elements store directly in
  the 8-byte slots. **`Vec<Vec<T>>` → a flat `vec` arena** indexed `out*N+in`
  (routing matrix, planar buffers) — Vec-arena-over-nested.
- **structs** via `#derive(accessors)` + `alloc(sizeof(T))`; methods →
  free functions `TypeName_verb(self, …)`. **Free fns get a `<module>_` prefix**
  — the bundle is one flat namespace; front-load collision avoidance.
- **u32 xorshift RNG** (oscillator, dither, noise) → i64 slot with explicit
  `& 0xFFFF_FFFF` mask after each shift/xor, or the sequence diverges from the
  oracle. `u32::MAX` normalization → `4294967295.0` divisor.
- **Module files do NOT `include` each other** — the build/test entry includes
  them in dependency order; stdlib auto-prepends; deps via `include "lib/…"`.
- **serde round-trip + Display-string tests dropped** (no serde; integer codes).
  All other `#[test]` blocks ported one-for-one into `tests/<mod>.tcyr`.
- **Alloc-free hot paths** (see the dedicated section below).
- **Cross-check every module against `rust-old/`** — the correctness bar is
  "matches what Rust did".

### Alloc-free hot paths (free-less bump allocator)

Cyrius's allocator never frees; a per-sample/per-block heap allocation leaks
monotonically across a render. The audit found these **hot allocations** that
must become struct-owned scratch or caller-supplied out-buffers **before** the
render loop is ported:

| Module | Hot allocation | Fix |
|--------|----------------|-----|
| `buffer/mod` | `mix()`, `resample_linear()`, `silence()` alloc fresh output `Vec` | caller-out-buffer / `BufferPool` acquire-release (already present) |
| `buffer/convert` | every converter + `interleaved↔planar` allocs `samples.len()` | in-place / caller-out-slice |
| `buffer/resample` | `resample_sinc()` output `Vec` | caller-out-buffer |
| `buffer/ops`, `buffer/dither` | `crossfade`, `tpdf/noise_shaped` `collect()` | out-slice |
| `dsp/eq` | `process()` `buf.samples.clone()` for dry/wet (mix<1) | struct-owned dry scratch |
| `dsp/graphic_eq` | `rebuild()` allocs a whole new `ParametricEq` per set_band | in-place `set_params` |
| `dsp/routing` | `apply()` allocs output `Vec` + new `AudioBuffer` per call | caller-out-buffer |
| `analysis/*`, `graph`, `meter` | per-frame scratch (STFT frames, graph node bufs) | preallocate in constructor |

`BufferPool` (`buffer/mod.rs`) is the model mitigation — acquire/release from a
pre-sized arena; port it early and make it the universal convention.

---

## Toolchain & commands

- cycc pin: **6.4.3** (`cyrius.cyml [package].cyrius`).
- Build: `cyrius build src/main.cyr build/dhvani`
- Test ONE suite: `cyrius test tests/<mod>.tcyr` (explicit path — no discovery).
- **Concurrency**: `cyrius build/test/deps` re-resolve deps and race on
  `cyrius.lock`. Parallel-porting agents MUST serialize every `cyrius …` call
  behind a shared file lock: `flock <scratch>/dhvani-build.lock cyrius …`.

## Math dependency — open decision (ADR candidate)

Rust dhvani used `abaco::dsp` (`amplitude_to_db`, `db_to_amplitude`, `poly_blep`,
`constant_power_pan`, `equal_power_crossfade`, `angular_frequency`,
`db_gain_factor`, `time_constant`, `sanitize_sample`). Every audio sibling
dhvani consumes (naad/svara/goonj/…) pulls **`hisab`** instead, so dhvani pulls
hisab transitively regardless. **Decide when the Foundation wave lands** whether
dhvani's own math stays on abaco or standardizes on hisab; record an ADR and
prune the loser from `cyrius.cyml`. Transcendentals (`f64_sin/cos/tan/sqrt/pow`)
come from `ganita` (already in the stdlib list), **not** `dist/abaco.cyr` —
confirm the link.

**Sibling-API gaps found** (must be handled, not blockers):
- `midi/voice.rs` re-exports `abaco::dsp::{A4_FREQUENCY, A4_MIDI_NOTE,
  SEMITONES_PER_OCTAVE}` — these are **internal** `var DSP_*` in abaco, not
  public. Define them locally in the midi port.
- `analysis/loudness.rs` K-weighting needs biquad coeff design not in abaco →
  satisfied by dhvani's **own** ported `dsp::biquad`; gates loudness on the DSP
  wave, not on abaco.

---

## Ledger

### L0 — foundation leaves (no non-error internal deps)

| Module | LOC | Wave | T | Notes |
|--------|----:|:----:|--:|-------|
| `error` | 108 | A | 2 | `NadaError` → `ERR_*` codes + `flush_denormal`, finite checks, EPSILON/INF consts. Universal base — every entry includes it first. Folds `lib.rs` crate-root helpers. |
| `clock` | 209 | A | 9 | `AudioClock`; `Option<f64>` → NaN sentinels; u64→i64; drop serde. |
| `simd` (scalar) | 739 | A | 18 | **Port ONLY the scalar fallbacks** (`*_scalar`) as the public kernels. `(f32,f32)` returns → out-params. Accelerated arms are ⛔ (see Blocked). Parity tests re-anchor to oracle outputs, not SIMD-vs-scalar. |
| `dsp/oscillator` | 245 | B | 8 | Cleanest hot path — no alloc, register-only. `Waveform`→codes; `poly_blep` (abaco); u32 xorshift mask. Doesn't even use `buffer`. |
| `dsp/gain_smoother` | 207 | B | 7 | Scalar EMA. Drop serde (has a serde_roundtrip test → remove). |
| `dsp/envelope` | 324 | B | 7 | ADSR; `Result` only in `set_params`. |
| `dsp/lfo` | 268 | B | 8 | LFO shapes → codes. |
| `dsp/automation` | 349 | B | 10 | Automation curves; closures → loops. |
| `analysis/waveform` | 155 | D | 5 | Peak/RMS envelope; out-slice. |
| `analysis/zcr` | 121 | D | 4 | Zero-crossing rate. |
| `midi/mod` | 642 | E | 16 | Message/status enums → codes; parser; 5 alloc scratch to preallocate. |
| `midi/voice` | 413 | E | 11 | Voice alloc/steal; **3 abaco const gaps → define locally**. |
| `capture/mod` | 162 | E | 4 | Config + capture trait; portable (no FFI). |
| `capture/record` | 319 | E | 12 | WAV/ring recorder; portable (no PipeWire). |

### L1 — depend on L0

| Module | LOC | Wave | T | Deps | Notes |
|--------|----:|:----:|--:|------|-------|
| `buffer/mod` | 561 | A | 13 | error, simd | `SampleFormat`/`Layout`→codes; `BufferPool` (port early). 5 alloc hot paths. |
| `buffer/convert` | 499 | A | 20 | error, buffer, simd | i16/i24/i32/u8↔f32 sign-extend + saturating casts in i64; every converter allocs → out-slice. |
| `buffer/resample` | 287 | A | 8 | error, buffer, simd, (analysis) | linear + windowed-sinc; scalar path only. Spectrum-DFT test gated on analysis. |
| `buffer/dither` | 114 | A | 5 | — | TPDF + noise-shaped; u32 xorshift mask; out-slice. |
| `dsp/mod` | 335 | B | 19 | buffer, simd, clock | facade re-exports → flat namespace; `soft_knee_gain`; scalar path only. |
| `dsp/biquad` | 512 | B | 15 | buffer, simd | RBJ design; `FilterType` payload (`gain_db`) threaded separately. States alloc at ctor only. |
| `dsp/svf` | 515 | B | 16 | buffer | TPT SVF; `SvfMode`→codes; cleanest after pan. |
| `dsp/pan` | 119 | B | 6 | buffer | `constant_power_pan` → out-params. No alloc. |
| `dsp/routing` | 232 | B | 8 | buffer, error | `Vec<Vec>`→flat arena; `apply()` → caller-out-buffer. |
| `dsp/compressor` | 381 | C | 6 | buffer, dsp/mod, error | `time_constant`/dB (abaco); alloc-free loop. |
| `dsp/limiter` | 349 | C | 8 | buffer, dsp/mod, error | peak detect + clamp; alloc-free. |
| `dsp/delay` | 384 | C | 6 | buffer | delay line / ring buffer. |
| `dsp/reverb` | 492 | C | 6 | buffer, error | Schroeder/FDN combs. |
| `analysis/mod` | 434 | D | 14 | buffer, error, dsp | spectrum/DFT facade. |
| `analysis/fft` | 198 | D | 5 | analysis, buffer, error | `fft_in_place` (needed by convolution/noise_reduction). |
| `analysis/loudness` | 313 | D | 4 | buffer, **dsp/biquad**, error | R128 K-weighting — gated on DSP wave (biquad). |
| `analysis/dynamics` | 363 | D | 10 | buffer, dsp(amplitude_to_db) | crest/loudness-range. |
| `midi/routing` | 315 | E | 11 | midi | channel routing. |
| `midi/v2` | 194 | E | 5 | midi | MIDI 2.0 UMP. |
| `meter` | 589 | E | 17 | buffer | level/loudness meters; preallocate scratch. |
| `synthesis/mod` | 238 | F | 7 | buffer, **naad** | synthesis facade over naad. |
| `sampler` | 114 | F | 3 | buffer, **nidhi** | sample playback over nidhi. |
| `creature` | 157 | F | 6 | buffer, error, **prani** | creature vocal over prani. |
| `environment` | 316 | F | 6 | buffer, error, **garjan** | nature sound over garjan. |
| `mechanical` | 193 | F | 5 | buffer, error, **ghurni** | mechanical sound over ghurni. |
| `voice_synth/mod` | 463 | F | 21 | buffer, error, **svara** | glottal/formant/tract over svara. |

### L2 — depend on L1

| Module | LOC | Wave | T | Deps | Notes |
|--------|----:|:----:|--:|------|-------|
| `buffer/ops` | 236 | B | 7 | buffer, error, (analysis) | crossfade → out-slice; `normalize_to_lufs` gated on analysis (defer that fn). |
| `dsp/eq` | 465 | C | 14 | buffer, dsp/biquad | `BandType`→codes; `process()` dry clone → struct scratch. |
| `dsp/deesser` | 323 | C | 7 | buffer, dsp/biquad | `mem::take` swap → explicit reuse; needs `FilterType::BandPass` code. |
| `dsp/convolution` | 424 | D | 8 | analysis/fft, buffer | partitioned convolution; needs fft. |
| `dsp/noise_reduction` | 283 | D | 6 | analysis/fft, buffer | spectral subtraction; needs fft. |
| `analysis/stft` | 262 | D | 4 | analysis, buffer, error | 4 per-frame scratch → preallocate. |
| `analysis/chroma` | 139 | D | 4 | analysis, buffer | chromagram. |
| `midi/translate` | 210 | E | 10 | midi | note/CC translation; no alloc. |
| `graph` | 1174 | E | 21 | buffer | RT-safe node graph (largest module); 5 scratch to preallocate. See `docs/decisions/003-rt-safe-graph-design.md`. |
| `acoustics` | 467 | F | 9 | buffer, error, dsp, **goonj** | convolution reverb / FDN / ambisonics over goonj. |

### L3 — depend on L2

| Module | LOC | Wave | T | Deps | Notes |
|--------|----:|:----:|--:|------|-------|
| `dsp/graphic_eq` | 377 | C | 13 | buffer, dsp/eq | 10-band ISO; preset string table; in-place rebuild. |
| `analysis/key` | 213 | D | 5 | analysis/chroma | Krumhansl key detection. |
| `analysis/onset` | 183 | D | 5 | analysis | spectral-flux onset. |

### L4 — top of the graph

| Module | LOC | Wave | T | Deps | Notes |
|--------|----:|:----:|--:|------|-------|
| `analysis/beat` | 284 | D | 4 | analysis/onset,key | tempo/beat tracking. |
| `lib` (crate root) | 348 | G | 3 | (all) | facade → `[lib] modules` order; drop trait assertions/doctests. |
| `tests/mod` | 1102 | G | 47 | (all) | integration parity suite → `tests/*.tcyr`. |
| `tests/proptest_tests` | 232 | G | 20 | (all) | property tests → deterministic Cyrius equivalents. |

---

## Blocked / deferred

### Dep-blocked — unblock when the sibling ports to Cyrius

| Module | LOC | T | Blocks on | Feature |
|--------|----:|--:|-----------|---------|
| `g2p/mod` | 269 | 14 | **shabda** (G2P engine/rules/dictionaries — still Rust) | `g2p` |
| `voice_synth/bhava_bridge` | 881 | 38 | **bhava** (energy/mood/stress/traits — still Rust) | `bhava-voice` |

Port shabda and bhava to Cyrius first; then these land in a follow-up wave. All
their upstream deps (svara for voice) are already ported.

### Platform-blocked — no Cyrius primitive exists

| Module | LOC | T | Reason | Path forward |
|--------|----:|--:|--------|--------------|
| `simd/x86` | 1002 | 0 | SSE2/AVX2 `std::arch` intrinsics + `#[target_feature]` unsafe + `is_x86_feature_detected!` | **Not ported.** Scalar kernels in `simd/mod` supersede. Accepted throughput regression vs oracle. |
| `simd/aarch64` | 482 | 0 | NEON intrinsics, same class | **Not ported.** Scalar supersedes. |
| `ffi` | 417 | 13 | C-ABI `extern "C"` / `#[no_mangle]` / raw-pointer opaque handles / `CString`; free-less allocator also makes `*_free` no-ops (breaks the C ownership model) | **Deferred.** Consumers are Cyrius-native. Re-architect as an in-language handle-table API only if a C boundary is ever needed. |
| `capture/pw` | 788 | 9 | PipeWire/`spa` unsafe FFI (RT client, POD serialization) | **Deferred** behind the `pipewire` gate until Cyrius has an audio-device story. `capture/mod` + `capture/record` port without it. See `docs/decisions/004-pipewire-feature-gated.md`. |

### Dropped (not ported)

- `tests/serde_tests` (206 LOC, 32 tests) — no serde in Cyrius; the whole suite
  is round-trip tests. Drop.

---

## Progress

**8 / 64 modules ported — Wave A (Foundation) COMPLETE. 163 parity assertions green.**

| Module | Cyrius | Tests | Notes |
|--------|--------|------:|-------|
| ✅ `error` | `src/error.cyr` | 20 | 9 codes + names + finite/sentinel helpers |
| ✅ `clock` | `src/clock.cyr` | 14 | `AudioClock` struct; Option→NaN sentinel; int↔f64 |
| ✅ `simd` | `src/simd.cyr` | 34 | 14 scalar kernels; SIMD arms dropped (platform-blocked) |
| ✅ `buffer/mod` | `src/buffer.cyr` | 23 | `AudioBuffer` + `BufferPool`; mix/resample_linear |
| ✅ `buffer/dither` | `src/dither.cyr` | 6 | TPDF + noise-shaped; u32 xorshift masked |
| ✅ `buffer/convert` | `src/convert.cyr` | 44 | i32/planar/mono-stereo/i24-packed/downmix |
| ✅ `buffer/resample` | `src/resample.cyr` | 12 | windowed-sinc (scalar); Blackman-Harris window |
| ✅ `buffer/ops` | `src/ops.cyr` | 10 | crossfade + fades (equal-power matched inline) |

**Deferred out of Wave A** (not blockers):
- `AudioBuffer::AudioBufferRef` (borrowed view — no tests; a "ref" is the same handle).
- `buffer/ops::normalize_to_lufs` + `resample::sine_frequency_preserved` test →
  Wave D (need `analysis::measure_r128` / `spectrum_dft`).
- `dither` defines `dh_xorshift32_next`; Wave B `oscillator`/`noise` must **reuse**
  it (share, don't redefine) to keep the dist bundle collision-free.

**Wave B (DSP core) — in flight** (9/64 total; 177 parity assertions):
- ✅ **abaco wired** — `[deps.abaco]` (`path=../abaco` + git/tag), vendored
  `lib/abaco.cyr`, `cyrius.lock` written. DSP helpers verified; abaco's unused
  json/http/net DCE-prune (benign warnings). No stdlib expansion needed.
- ✅ `oscillator` → `src/oscillator.cyr` — Waveform→codes; PolyBLEP via
  `abaco::poly_blep`; u32 xorshift inlined; `F64_TAU` exact. **14 green**.
- ✅ `pan` → `src/pan.cyr` — `abaco::constant_power_pan` (out-pointers). **11 green**.
- ✅ `gain_smoother` → `src/gain_smoother.cyr` — EMA attack/release; nested
  params struct; serde test dropped. **9 green**.
- ✅ `lfo` → `src/lfo.cyr` — 6 shapes; S&H xorshift; `F64_TAU`. **15 green**.
- ✅ `envelope` → `src/envelope.cyr` — ADSR state machine; params struct;
  set_params error code. **15 green**.
- ✅ `automation` → `src/automation.cyr` — breakpoint curves (step/linear/exp/
  smooth); `Exponential(f32)` payload → `exp` field; `f64_pow(0)` guarded. **22 green**.
- ✅ `svf` → `src/svf.cyr` — Cytomic SVF, 8 modes; per-channel state vec;
  `dhvani_tan` (added to error.cyr — no `f64_tan` in stdlib). **16 green**.
- ✅ `biquad` → `src/biquad.cyr` — RBJ cookbook, 8 types; data-carrying
  `FilterType{gain_db}` → code + separate field; `abaco::angular_frequency`/
  `db_gain_factor`; SIMD stereo fast-path dropped (scalar loop). **16 green**.
- ✅ `dsp/mod` → `src/dsp.cyr` — facade free fns (noise_gate/hard_limiter/
  normalize) + `soft_knee_gain`; abaco db-math re-exports. **24 green** (12
  tests; 7 deferred to Wave C — need compressor/limiter/delay/eq).
- ✅ `routing` → `src/routing.cyr` — N×M matrix; `Vec<Vec>`→flat arena
  (`out*inputs+inp`); apply Result→null-handle. **19 green**.

**L1 layer complete.** Wave C (DSP dependents) — in flight:
- ✅ `compressor` → `src/compressor.cyr` — envelope follower; `dsp::soft_knee_gain`
  (slope=1/ratio−1); abaco `time_constant`/dB math. **8 green**.
- ✅ `limiter` → `src/limiter.cyr` — inf:1 (slope=−1), instant attack + hard-clamp
  safety net. **10 green**.
- ✅ `delay` → `src/delay.cyr` — delay line (per-channel ring buffers) + modulated
  delay (chorus/flanger, sine LFO, interpolated reads). **9 green**.
- ✅ `reverb` → `src/reverb.cyr` — Schroeder/Freeverb (4 combs → 2 allpass per
  channel; damped feedback; rate-scaled delays). **6 green**.
- ✅ `eq` → `src/eq.cyr` — parametric EQ; BandType→codes; parallel config/filter
  vecs; **struct-owned dry-scratch** (kills the per-call `.clone()` alloc). **18 green**.
- ✅ `deesser` → `src/deesser.cyr` — sidechain bandpass detector; struct-owned
  sidechain+scratch reuse (alloc-free). **9 green**.
- ✅ `graphic_eq` → `src/graphic_eq.cyr` — 10-band ISO; wraps eq; **in-place
  rebuild** (avoids per-preset eq realloc); preset tables via if-chains;
  hex consts for `1.4`/`0.01`. **19 green**.
- ✅ `dsp/mod` re-completed to **19/19 tests** (33 assertions) — the 7 deferred
  integration tests (compressor/limiter/delay/eq) re-enabled.

**Waves A+B+C COMPLETE — entire DSP surface ported (25/64, 412 assertions).**
Ported via a 4-agent workflow (eq+deesser parallel → graphic_eq → verify);
independently reviewed (hex constants, dry/wet blend, reduction formula) — parity holds.

**Wave D (analysis) — in flight (29/64, 453 assertions):**
- ✅ `zcr` → `src/zcr.cyr` — sign-change rate; ZcrResult. **6 green**.
- ✅ `waveform` → `src/waveform.cyr` — min/max peak windows (Vec<Vec<(f32,f32)>>
  → per-channel flat interleaved vecs). **10 green**.
- ✅ `analysis/mod` → `src/analysis.cyr` — `Spectrum` type + `spectrum_dft` +
  centroid/rolloff + loudness_lufs/is_silent/suggest_gain (`log10`=ln/F64_LN10). **19 green**.
- ✅ `fft` → `src/fft.cyr` — radix-2 Cooley-Tukey `fft_in_place` + Hann-windowed
  `spectrum_fft` (pow2 helpers; rounds window down). **6 green**.
- ✅ `dynamics`(30), `loudness`(5), `stft`(7), `chroma`(6), `convolution`(10),
  `noise_reduction`(8), `key`(7), `onset`(14), `beat`(7) — ported via a 10-agent
  workflow (6 parallel → key+onset → beat → verify). Independently reviewed:
  loudness K-weighting coeffs (`HighShelf 4dB/1681/0.707` + `HighPass 38/0.5`),
  convolution IFFT (conjugate trick, `real/N`), chroma pitch-mapping — parity holds.

**Waves A+B+C+D COMPLETE — DSP + analysis ported (38/64, 547 assertions).**

Now-unblocked (fft/loudness available), to re-enable during Wave G assembly:
`buffer/ops::normalize_to_lufs` (needs loudness `measure_r128`) + `resample`
`sine_frequency_preserved` (needs `spectrum_dft`).

Next: **Wave E** — MIDI (`midi/{mod,voice,routing,v2,translate}`), `meter`
(lock-free peak metering), `graph` (RT-safe node graph, 1174 LOC — largest
module), and `capture/{mod,record}` (portable; `capture/pw` stays platform-blocked).

### Cyrius idioms confirmed (this port)

- Symbol prefix `dhvani_`/`DHVANI_`/`DH_` on every top-level name (flat-namespace
  coexistence with naad/svara/goonj bundles — verified against svara's `SVARA_*`).
- **Decimal float literals parse** (`1.0`, `2.5`, `0.001`) — use them; hex bit
  patterns only where clarity demands. **Gotcha**: an integer literal `120` is
  NOT `120.0` as an f64 (it's a denormal bit pattern) — f64-typed args must be
  decimals; integer-count args (sample_rate, frames) stay integer literals.
- int↔f64: `f64_from(i)` (int→f64), `f64_to(x)` (f64→int), `f64_trunc(x)`
  (truncate toward zero, matches Rust `as u64`). Compare: `f64_lt/le/gt/ge`,
  `f64_max/min/clamp`, `f64_abs`. `0` == `+0.0` bit pattern (fine for f64 compare).
- structs: `#derive(accessors)` + `struct T { a; b; }` → `T_a(self)` / `T_set_a`;
  construct `var s = alloc(sizeof(T)); T_set_a(s, v);`. Methods → `module_verb(self, …)`.
- test suite `.tcyr`: `include` deps in order → `alloc_init()` → `test_group()` +
  `assert`/`assert_eq`/`assert_streq` → `var rc = assert_summary(); syscall(60, rc);`.
  Run: `flock <scratch>/dhvani-build.lock cyrius test tests/<mod>.tcyr`.
