# dhvani — Roadmap

> Milestone plan for the Rust → Cyrius port (**→ 2.0.0**). State lives in
> [`state.md`](state.md); per-module parity in [`port-audit.md`](port-audit.md);
> this file is the **sequencing** — what ships, in what order, against what
> dependency gates.

## 2.0.0 criteria (port complete) — 2.0.0 released 2026-07-05

- [x] All portable Rust modules ported function-for-function vs `rust-old/`
      (54 of 64 at 2.0.0, g2p in 2.2.0; the blocked list below has the rest)
- [x] Every ported module has a `tests/<mod>.tcyr` suite, all green (each Rust
      `#[test]` ported one-for-one, minus serde/Display tests)
- [x] `[lib]` distlib bundle `dist/dhvani.cyr` assembled; symbol-collision audit
      clean (one flat namespace)
- [x] Hot-path benchmarks captured (`BENCHMARKS.md`, `tests/hotpath.bcyr`,
      `docs/benchmarks-rust-v-cyrius.md`)
- [x] `abaco`-vs-`hisab` math decision recorded as an ADR: **ADR 006** (2026-11-19),
      keep abaco. hisab is not pruned: it is the siblings' dependency, not a second
      math base (goonj's geometry is built on its HVec3).
- [x] CHANGELOG `[2.0.0]` complete; `VERSION` = 2.0.0
- [ ] At least one downstream consumer (shruti / jalwa / aethersafta) builds
      against the Cyrius bundle. **Still open**: no consumer is on the bundle yet.
- [x] Clean gate: `cyrius fmt` + `lint` + tests + bench green (fmt/lint gate since 2.2.3)

## Milestones

### M0 — Port scaffold (2.0.0-dev) — ✅ shipped 2026-07-04

- `cyrius port` scaffold; Rust source frozen at `rust-old/` (23,695 lines).
- `cyrius.cyml` set up (stdlib + DSP-math set; dep wiring commented per-wave).
- `VERSION` → 2.0.0; smoke binary builds (`cyrius build`).
- Tracking docs: state.md, port-audit.md, roadmap.md.

### M1 — Foundation (Wave A) — core, always-on — ✅ COMPLETE

`error` · `clock` · `simd` (scalar kernels only; packed arms since 2.3.2) ·
`buffer/{mod,convert,resample,dither,ops}` — **163 parity assertions green**.
- `BufferPool` landed (the alloc-free convention for the whole crate).
- Math decision: **keep abaco** (its DSP helpers are ready-made). abaco wiring
  itself is the first task of Wave B (where it's actually consumed).
- Deferred: `AudioBufferRef`, `normalize_to_lufs` (→ Wave D).

### M2 — DSP core (Waves B–C) — `dsp` — ✅ COMPLETE

- **Wave B**: `oscillator`, `gain_smoother`, `envelope`, `lfo`, `automation`,
  `pan`, `svf`, `biquad`, `dsp/mod`, `routing`, `buffer/ops`.
- **Wave C**: `eq`, `deesser`, `compressor`, `limiter`, `delay`, `reverb`,
  `graphic_eq`.
- Fix the bump-allocator hot paths (eq dry-clone, routing/graphic_eq rebuild)
  before porting their render loops.
- Gate: `dsp`. Each module green before its dependents start.

### M3 — Analysis (Wave D) — `analysis` — ✅ COMPLETE

- `waveform`, `zcr`, `analysis/mod`, `fft`, `dynamics`, `loudness` (needs
  biquad from M2), `stft`, `chroma`, `convolution` + `noise_reduction` (need
  fft), `key`, `onset`, `beat`.
- Gate: `analysis`. Unlocks the analysis-gated buffer fns (`normalize_to_lufs`).

### M4 — MIDI · graph · meter · capture (Wave E) — `midi` / `graph` / `pipewire` — ✅ COMPLETE

- `midi/{mod,voice,routing,v2,translate}` (3 missing abaco note constants defined
  locally), `meter` (atomics → plain fields), `graph` (largest module, 1174 LOC;
  `AudioNode` trait → fn-ptr node dispatch, NodeId atomic → module-level `var`,
  `process_parallel`/rayon dropped), `capture/{mod,record}` (portable; **not**
  `capture/pw`). **9 modules, 480 assertions.**
- Gate: `midi`, `graph`. Note: graph's per-block scratch (input gather + output
  slots) currently allocates a fresh scratch vec per node cycle — revisit the
  alloc-free-per-block acceptance target in Wave G hardening (parity first).

### M5 — Synthesis stack (Wave F) — feature-gated sibling wrappers — ✅ COMPLETE

- **Consumption pattern (solved):** siblings vendored into `lib/` (committed) and
  `include`d in dependency order `sakshi → hisab → goonj → naad → shravan → svara
  → ghurni → garjan → prani` — **not** `[deps]` (mis-orders cross-bundle types +
  force-includes the 136 KB `bayan`, overflowing the LEXID cap). Every dist
  externalizes its deps; the consumer assembles the set. Rationale in `port-audit.md`.
- ✅ All 7 wrappers: `synthesis`(naad), `sampler`(nidhi), `voice_synth/mod`(svara),
  `creature`(prani), `environment`(garjan), `mechanical`(ghurni), `acoustics`(goonj).
  **47 tests / 117 assertions.** Traits/closures → fn-ptr; goonj IR → dhvani
  ConvolutionReverb. Dep-blocked: `voice_synth/bhava_bridge` (bhava).
- Gate: `synthesis`, `voice`, `creature`, `environment`, `mechanical`,
  `sampler`, `acoustics`.

### M6 — Assembly & release (Wave G) — 2.0.0 tag — ✅ READY (awaiting git tag)

- ✅ Re-enabled the two Wave-D-deferred analysis-gated tests (normalize_to_lufs,
  sine_frequency_preserved).
- ✅ `lib` facade → `[lib] modules` L0→L4 order; `cyrius distlib` → `dist/dhvani.cyr`
  (externalizes abaco + siblings). Collision audit clear (naad 2.1.1 fixed
  amplitude_to_db). Validated by `tests/bundle{,_synth}.tcyr`.
- ✅ Ported `tests/{mod,proptest}` (dropped `serde_tests`) against the assembled
  bundle — 67 tests / 414 assertions (integration + dsp-reference + proptest).
- ✅ abaco 2.3.2: numerical dsp-reference port caught & fixed abaco's dB constants
  (wrong `ln(10)`); re-vendored, ripple-free.
- ✅ Hot-path `.bcyr` benches (`tests/hotpath.bcyr` + `BENCHMARKS.md`): 2.0.0
  baseline for osc/biquad/svf (per-sample), SIMD kernels + FFT (per-block), scalar.
- ✅ Rust-vs-Cyrius comparison (`tests/bench_compare.bcyr` +
  `docs/benchmarks-rust-v-cyrius.md`): scalar-f64 runs 6–250× the Rust f32-SIMD.
- ✅ CHANGELOG `[2.0.0]` finalized; `VERSION` = 2.0.0, deps pinned git+tag.
- ✅ **Tagged 2.0.0** (2026-07-05).

## Blocked from porting — deferred past 2.0.0

The audit ([`port-audit.md`](port-audit.md)) found nine modules that cannot port
now. Two kinds:

### Waiting on an unported Cyrius dependency

| Feature | Module(s) | Blocks on | Unblocks when |
|---------|-----------|-----------|---------------|
| `bhava-voice` | `voice_synth/bhava_bridge` (881 LOC, 38 tests) | **bhava** (still Rust) | bhava ports to Cyrius |

> ✅ **`g2p` unblocked + ported in 2.2.0** (2026-07-06) once **shabda 3.0.0** landed —
> `g2p/mod` (269 LOC, 14 tests) → `src/g2p.cyr` over vendored shabda 3.0.1 + shabdakosh 3.0.2
> + varna 2.0.0. **bhava** is now the only unported-dep holdout (for `bhava-voice`).
> All *other* dhvani deps are already ported (abaco, naad, svara, prani, nidhi,
> garjan, ghurni, goonj, shabda).

### Waiting on a Cyrius platform primitive (no equivalent yet)

(SIMD acceleration left this table in 2.3.2: see the backlog entry below.)

| Area | Module(s) | Reason | Path forward |
|------|-----------|--------|--------------|
| C-ABI FFI | `ffi` | `extern "C"`/`#[no_mangle]`/raw-pointer handles/`CString`; free-less allocator breaks `*_free` | defer; consumers are Cyrius-native. Re-architect as an in-language handle table only if a C boundary is needed. |
| PipeWire capture | `capture/pw` | PipeWire/`spa` unsafe FFI | defer behind the `pipewire` gate until Cyrius has an audio-device backend. |

## Post-2.0.0 — deferred backlog (carried from the Rust roadmap)

Parity first; these resume once the port is green. Demand-gated.

- **Consumer adoption**: shruti (DAW), jalwa (player), aethersafta (compositor),
  kiran (game audio) build against the Cyrius bundle.
- **Advanced DSP**: multiband compressor, noise suppression, pitch shift /
  time stretch (phase vocoder / WSOLA).
- **MIDI advanced**: SMF read/write, MIDI clock/MTC/SPP, SysEx, MPE.
- **Platform backends**: JACK, and (pending Cyrius FFI) CoreAudio / WASAPI /
  WASM — plus the PipeWire capture unblock above.
- **High sample rate**: validated 44.1k↔…↔768k paths, multi-stage resampling,
  oversampled DSP.
- **Formats — niche**: a-law/µ-law (G.711), i8, DSD, ambisonic layouts.
- ✅ **SIMD re-acceleration**: shipped in **2.3.2**, on Cyrius's flat-array packed
  verbs (`f64v_*` SSE2/NEON, `f64v256_*` AVX2). The intrinsic arms (`simd/x86`,
  `simd/aarch64`) are superseded rather than ported. The 14 kernels plus the oracle's
  two callers (the stereo biquad fast path, the resampler's weight row) are restored,
  each with its scalar form kept as the parity oracle. On a 512 block the kernels run
  2.6–61× their scalar form. See `docs/guides/simd.md`.
  - Next in this line, demand-gated: packed min/max/compare verbs would let `peak_abs`,
    `clamp` and `noise_gate` vectorize too (today they are walked), and an f32 lane
    path (`f32v8_*`, 8 lanes) only if a consumer wants f32 throughput over f64 precision.

- **Open from the 2.3.3 P(-1) sweep** (each needs a decision; `state.md`, ADR 005):
  - (decided in 2.3.3: convolution `process` streams with one block of latency, so a
    host block of any size is fully processed; ADR 005.)
  - MIDI 2.0 upscaling is bit replication, not min-center-max, so the pitch-bend centre
    8192 does not map to `0x80000000`.
  - upstream (cyrius stdlib): `simd_has_avx2` should also check OSXSAVE/XCR0 and the
    maximum CPUID leaf. Filed 2026-11-19 as cyrius
    `docs/development/issues/2026-11-19-simd-has-avx2-skips-the-os-check.md`, with a
    qemu repro: under `-cpu max,-xsave` the gate says yes and the ymm kernel SIGILLs.
  - ✅ streaming onset/beat flux (memory proportional to bins, not to the track length):
    done after 2.3.3. `dhvani_spectral_flux_from_buffer` computes the flux one STFT
    frame at a time, bit for bit with the spectrogram path; onset and tempo detection
    use it.

## Out of scope (unchanged from Rust)

- Audio file I/O (shravan / tarang), plugin hosting (shruti), composition /
  sequencing / timeline (shruti), streaming protocols (aethersafta), DAW UI
  (shruti), neural TTS / text-to-phoneme ML models (hoosh).
