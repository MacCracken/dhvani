# dhvani — Roadmap

> Milestone plan for the Rust → Cyrius port (**→ 2.0.0**). State lives in
> [`state.md`](state.md); per-module parity in [`port-audit.md`](port-audit.md);
> this file is the **sequencing** — what ships, in what order, against what
> dependency gates.

## 2.0.0 criteria (port complete)

- [ ] All portable Rust modules ported function-for-function vs `rust-old/`
      (~55 of 64 — see the blocked list below for the remainder)
- [ ] Every ported module has a `tests/<mod>.tcyr` suite, all green (each Rust
      `#[test]` ported one-for-one, minus serde/Display tests)
- [ ] `[lib]` distlib bundle `dist/dhvani.cyr` assembled; symbol-collision audit
      clean (one flat namespace)
- [ ] Hot-path benchmarks captured (buffer mix/convert/resample, biquad/SVF,
      FFT, graph render) in `docs/benchmarks.md`
- [ ] `abaco`-vs-`hisab` math decision recorded as an ADR; loser pruned from
      `cyrius.cyml`
- [ ] CHANGELOG `[2.0.0]` complete; `VERSION` = 2.0.0
- [ ] At least one downstream consumer (shruti / jalwa / aethersafta) builds
      against the Cyrius bundle
- [ ] Clean gate: `cyrius fmt` + `lint` + tests + bench green

## Milestones

### M0 — Port scaffold (2.0.0-dev) — ✅ shipped 2026-07-04

- `cyrius port` scaffold; Rust source frozen at `rust-old/` (23,695 lines).
- `cyrius.cyml` set up (stdlib + DSP-math set; dep wiring commented per-wave).
- `VERSION` → 2.0.0; smoke binary builds (`cyrius build`).
- Tracking docs: state.md, port-audit.md, roadmap.md.

### M1 — Foundation (Wave A) — core, always-on — ✅ COMPLETE

`error` · `clock` · `simd` (scalar kernels only) ·
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
- ⬜ **Tag 2.0.0** (user handles git) — everything else is release-ready.

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

| Area | Module(s) | Reason | Path forward |
|------|-----------|--------|--------------|
| SIMD acceleration | `simd/x86`, `simd/aarch64` | raw SSE2/AVX2/NEON intrinsics, `#[target_feature]` unsafe, CPU feature detection | scalar kernels ship in 2.0.0; accept the throughput regression until Cyrius has a SIMD story. dhvani owns SIMD dispatch for the ecosystem, so this is the natural home for it later. |
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
- **SIMD re-acceleration** once Cyrius exposes vector intrinsics.

## Out of scope (unchanged from Rust)

- Audio file I/O (shravan / tarang), plugin hosting (shruti), composition /
  sequencing / timeline (shruti), streaming protocols (aethersafta), DAW UI
  (shruti), neural TTS / text-to-phoneme ML models (hoosh).

---

## Moving the cyrius pin to 6.6.6

**Current pin: `cyrius = "6.6.3"`.** Nothing must change first — dhvani needs the pin
bump and a rebuild, nothing else.

**What was checked** (58 `.cyr` under `src/`; vendored `lib/` excluded throughout):

- **Windows write corruption (the 6.6.6 headline): not a dhvani exposure.** `O_APPEND` /
  `O_TRUNC` appear **39 times, all inside the vendored `lib/` fold and zero times in
  `src/`** — dhvani opens no files at all (no `file_open` / `sys_open` / `file_read_all` /
  `file_exists` call site anywhere in `src/`). Device I/O is ALSA ioctls, not file writes.
- **Struct copies / by-value struct params: nothing to change.** dhvani declares **82
  `Dh*` structs**, and **not one** of them is a by-value fn parameter, a fn return type, or
  a `var x: DhT = …` declaration — every one is a heap-offset layout reached through a raw
  pointer. So 6.6.6's new "copying between two different struct types is a compile error"
  and "a by-value struct parameter over 8 B is now deep-copied" both touch zero sites here.
- **Also clean:** no `async fn`, no `operator` fn, no pair-return fn with a mismatched
  `return` (checked every fn for mixed pair/scalar returns — zero), no `var` inside a
  top-level block (zero top-level `{` / `if (` / `while (` at column 0, so the new
  block-scoping rule is a no-op), no raw `SYS_STATFS`, no `lib/regression.cyr` consumer, no
  `vec_*` of dhvani's own (so the new `assert.cyr` → `vec.cyr` transitive include cannot
  collide), no duplicate top-level global.
- **Platform:** CI is `ubuntu-latest` only and `src/` carries no `CYRIUS_TARGET_*` branch
  at all. No Windows or Mach-O build exists to be affected by the PE or Darwin changes.

**What it gains:** 6.6.5's aggregate-layout fix (silently wrong since 5.8.17 — for a
library whose whole job is dense buffer layout, this is the one to want) and the three
corrected ENTRY stack bases, plus 6.6.6's nine new refusals that turn previously-silent
miscompiles into named compile errors. For a library this size the refusals are the real
win — they are a free audit of `src/`. **Not** a gain here: 6.6.4's ≥64 KB string-literal
fix — dhvani's longest literal is 22 bytes (`src/error.cyr`), measured, so that defect was
never reachable.

**Verify after bumping:** `cyrius deps` (re-vendors the whole `lib/` fold from the 6.6.6
store, which is where dhvani's 39 `O_APPEND`/`O_TRUNC` sites actually live) → `cyrius build`
→ `cyrius test` → **regenerate `dist/dhvani.cyr`**. The dist bundle is compiled by each
consumer's own toolchain, so a consumer still on an older cyrius must keep compiling it;
confirm the regenerated bundle still builds under the oldest pin in the consumer set before
tagging.
