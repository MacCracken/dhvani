# dhvani — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.0.0** (in progress) — Rust → Cyrius port, scaffolded 2026-07-04 via
`cyrius port`. The 23,695-line Rust source (64 modules, ~660 `#[test]` blocks) is
frozen at `rust-old/` as the parity oracle. The port lands as **2.0.0** (major
break: language change), matching the sibling audio libs (naad 2.x, svara 3.x).

## Toolchain

- **Cyrius pin**: `6.4.3` (in `cyrius.cyml [package].cyrius`).
- Build: `cyrius build src/main.cyr build/dhvani` (smoke binary — builds green).
- Test ONE suite: `cyrius test tests/<mod>.tcyr` (explicit path — no discovery).
- **Parallel-porting concurrency**: every `cyrius …` call re-resolves deps and
  races on `cyrius.lock` (concurrent runs corrupt it). Serialize toolchain calls
  behind `flock <scratch>/dhvani-build.lock cyrius …`.

## Source

- Rust reference: 23,695 lines across 64 modules at `rust-old/` (frozen — do
  not edit; it is the oracle).
- Cyrius port: `src/main.cyr` (smoke stub only so far) + per-module `src/*.cyr`
  as they land (subdir modules `buffer/`, `dsp/`, `analysis/`, `midi/`,
  `capture/` flatten to `src/<name>.cyr`).

## Dependencies

Direct (declared/planned in `cyrius.cyml`):

- **stdlib** — base set + the DSP-math set (`math`, `ganita`, `tagged`,
  `fnptr`, `callback`, `bench`).
- **Math**: ✅ **abaco 2.3.1 wired** (`[deps.abaco]`, `path=../abaco` local +
  git/tag for CI; vendored `lib/abaco.cyr`, `cyrius.lock` written). DSP helpers
  verified (poly_blep/amplitude_to_db/constant_power_pan/…). abaco's unused
  json/http/net helpers DCE-prune (benign warnings).
- **Synthesis stack** (Wave F): naad 2.1.1, svara 3.0.1, prani 2.0.1,
  nidhi 2.0.0, garjan 2.0.0, ghurni 2.0.0, goonj 2.0.0 + sakshi 2.4.4 (logging)
  + hisab 2.6.7 (HVec3 for goonj) + shravan 2.x (WAV codec for nidhi) —
  **vendored into `lib/` (committed), included in dependency order**, NOT `[deps]`.
- **Blocked** (not ported): shabda (→ g2p), bhava (→ bhava-voice).

## Consumers

_None on the Cyrius bundle yet._ Rust consumers (shruti, jalwa, aethersafta,
kiran) migrate up the stack after the port is green (post-2.0.0).

## Port progress

**49 / 64 modules ported** — Waves A+B+C+D+E complete; **Wave F in flight**
(synthesis + sampler landed; the sibling-bundle consumption pattern is solved).
**1083 parity assertions** across 50 suites (+ 1 scaffold smoke). Portable now:
~55 across A–G. Deferred: 9.

**Wave F consumption pattern (solved this cycle):** the 7 synthesis-stack siblings
+ sakshi + hisab + shravan are vendored into `lib/` (committed) and included **in
dependency order** (`sakshi → hisab → goonj → naad → shravan → svara → ghurni →
garjan → prani`) — NOT wired as `[deps]` (which mis-orders cross-bundle types and
force-includes the 136 KB `bayan`, overflowing the compiler's identifier cap).
Every dist externalizes its deps (nidhi→naad+shravan, naad→goonj, …); the consumer
assembles the full set. See the manifest and [`port-audit.md`](port-audit.md).

| Layer / Wave | Modules | Status |
|--------------|---------|--------|
| A — Foundation (core) | ✅ error, ✅ clock, ✅ simd(scalar), ✅ buffer/{mod,convert,resample,dither,ops} | ✅ |
| B — DSP L0/L1 (dsp) | ✅ oscillator, pan, gain_smoother, lfo, envelope, automation, svf, biquad, dsp(facade), routing | ✅ |
| C — DSP dependents (dsp) | ✅ compressor, limiter, delay, reverb, eq, deesser, graphic_eq | ✅ |
| D — Analysis (analysis) | ✅ waveform, zcr, analysis, fft, dynamics, loudness, stft, chroma, convolution, noise_reduction, key, onset, beat | ✅ |
| E — MIDI/meter/capture/graph | ✅ midi, voice, midi_routing, midi_v2, translate, meter, capture, record, graph | ✅ |
| F — Synthesis stack | ✅ synthesis(naad), sampler(nidhi) · ⬜ voice_synth(svara), creature(prani), environment(garjan), mechanical(ghurni), acoustics(goonj) | 🟡 |
| G — Assembly | lib facade, dist/dhvani.cyr bundle, tests/{mod,proptest}, benches | ⬜ |
| ⛔ Blocked (dep) | g2p (shabda), voice_synth/bhava_bridge (bhava) | deferred |
| ⛔ Blocked (platform) | simd/{x86,aarch64}, ffi, capture/pw | deferred |
| ✂ Dropped | tests/serde_tests (no serde) | n/a |

Per-module parity ledger + conventions: [`port-audit.md`](port-audit.md).

## Tests

No parity suites ported yet. Target: one `tests/<module>.tcyr` per ported
module, each Rust `#[test]` ported one-for-one (serde round-trip + Display
tests dropped). ~660 `#[test]` blocks in the oracle; ~570 portable (excludes
the serde_tests suite and the blocked/platform modules).

## Next

See [`roadmap.md`](roadmap.md). Continue **Wave F** — port the 5 remaining
wrappers using the established ordered-include pattern: `voice_synth` (svara),
`creature` (prani), `environment` (garjan), `mechanical` (ghurni), `acoustics`
(goonj + dhvani ConvolutionReverb). `voice_synth/bhava_bridge` + `g2p` stay
dep-blocked (bhava/shabda).
Then **Wave G** — lib facade + `dist/dhvani.cyr` bundle + integration tests, and
re-enable the deferred `ops::normalize_to_lufs` + `resample` sine tests (now
unblocked by fft/loudness). (The abaco↔naad `amplitude_to_db` collision is already
resolved upstream — naad 2.1.1 dropped its copies.)
