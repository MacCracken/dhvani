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

### M5 — Synthesis stack (Wave F) — feature-gated sibling wrappers — 🟡 IN FLIGHT

- **Consumption pattern (solved):** siblings vendored into `lib/` (committed) and
  `include`d in dependency order `sakshi → hisab → goonj → naad → svara → ghurni
  → garjan → prani` — **not** `[deps]` (mis-orders cross-bundle types + force-
  includes the 136 KB `bayan`, overflowing the LEXID cap). Full rationale in
  `port-audit.md`.
- ✅ `synthesis/mod`(naad) — 7 tests. Remaining unblocked: `creature`(prani),
  `environment`(garjan), `mechanical`(ghurni), `voice_synth/mod`(svara),
  `acoustics`(goonj).
- ⛔ `sampler`(nidhi) — nidhi's dist omits its `STREAM_EVT_*` enum
  (`STREAM_EVT_HEADER` undefined); blocked pending a nidhi re-release.
- Gate: `synthesis`, `voice`, `creature`, `environment`, `mechanical`,
  `sampler`, `acoustics`.

### M6 — Assembly & release (Wave G) — 2.0.0 tag

- `lib` facade → `[lib] modules` order; assemble `dist/dhvani.cyr`; collision
  audit; port `tests/{mod,proptest}` (drop `serde_tests`); hot-path benches;
  pin deps to git+tag; **bump/confirm `VERSION` = 2.0.0**.

## Blocked from porting — deferred past 2.0.0

The audit ([`port-audit.md`](port-audit.md)) found nine modules that cannot port
now. Two kinds:

### Waiting on an unported Cyrius dependency

| Feature | Module(s) | Blocks on | Unblocks when |
|---------|-----------|-----------|---------------|
| `g2p` | `g2p/mod` (269 LOC, 14 tests) | **shabda** (still Rust) | shabda ports to Cyrius |
| `bhava-voice` | `voice_synth/bhava_bridge` (881 LOC, 38 tests) | **bhava** (still Rust) | bhava ports to Cyrius |

> All *other* dhvani deps are already ported (abaco, naad, svara, prani, nidhi,
> garjan, ghurni, goonj). shabda and bhava are the only two holdouts — porting
> them upstream is the prerequisite for these two dhvani features.

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
