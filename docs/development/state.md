# dhvani — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.2.0** (in progress) — **g2p** (grapheme-to-phoneme over the newly-ported shabda
3.0.0) + toolchain bump to **6.4.11** + dep refresh (hisab 2.6.8, vani 1.0.0).
**2.1.2** — capture ring path. **2.0.0** (Rust→Cyrius parity port),
**2.1.0** (vani device I/O), **2.1.1** (RT ring player + multi-format S16/S24/S32)
released. The `playback` module (`src/playback.cyr`) bridges `AudioBuffer` ↔ vani
ALSA PCM; 2.1.2 adds the recorder (`dhvani_recorder_*` over `vani_record_to_ring`), the
capture mirror of the RT player, plus **device enumeration** (`src/device.cyr`:
`dhvani_devices_list` + default-device open, over yukti's PCM discovery — a
separate module, not in the dist). The 2.1.1 ring path (`dhvani_player_*` over
`vani_play_from_ring`) gives real-time-safe playback —
zero per-block allocation, as the free-less bump allocator requires.

## Toolchain

- **Cyrius pin**: `6.4.11` (in `cyrius.cyml [package].cyrius`).
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
  `fnptr`, `callback`, `bench`) + the g2p set (`hashmap`, `atomic`, `mmap` — for
  shabdakosh's dictionary; **not** `bayan`, 0 reachable calls).
- **Math**: ✅ **abaco 2.3.2 wired** (`[deps.abaco]`, `path=../abaco` local +
  git/tag for CI; vendored `lib/abaco.cyr`, `cyrius.lock` written). DSP helpers
  verified (poly_blep/amplitude_to_db/constant_power_pan/…). abaco's unused
  json/http/net helpers DCE-prune (benign warnings).
- **Synthesis stack** (Wave F): naad 2.1.1, svara 3.0.1, prani 2.0.1,
  nidhi 2.0.0, garjan 2.0.0, ghurni 2.0.0, goonj 2.0.0 + sakshi 2.4.4 (logging)
  + hisab **2.6.8** (HVec3 for goonj) + shravan 2.6.7 (WAV codec for nidhi) —
  **vendored into `lib/` (committed), included in dependency order**, NOT `[deps]`.
- **g2p stack** (2.2.0): **shabda 3.0.0** (G2P engine) + shabdakosh 3.0.1
  (dictionary) + varna 2.0.0 (phoneme inventories) — same vendored-include pattern
  as Wave F (`… svara → varna → shabdakosh → shabda`), externalized from the dist.
- **Device I/O**: vani **1.0.0** (ALSA PCM, re-vendored — first stable) + yukti
  2.2.8 (device enumeration, separate module — not in the dist).
- **Blocked** (not ported): bhava (→ bhava-voice). shabda is now ported ✅.

## Consumers

_None on the Cyrius bundle yet._ Rust consumers (shruti, jalwa, aethersafta,
kiran) migrate up the stack after the port is green (post-2.0.0).

## Port progress

**2.0.0 RELEASED** — the full Rust→Cyrius parity port (54/64 modules; 9 deferred on
blocked deps/platform): Waves A–F + Wave G assembly (dist bundle, integration/
dsp-reference/proptest suites, hot-path benches + Rust-vs-Cyrius comparison).

**2.1.x (in progress):** device I/O via vani. `playback` bridges `AudioBuffer` ↔
little-endian PCM (**S16/S24/S32**) + device glue (open/write/close/capture) + the
alloc-free RT ring **player** (`dhvani_player_*`, 2.1.1) and **recorder**
(`dhvani_recorder_*`, 2.1.2); bundled into the dist (DCE-prunes for vani-free
consumers). **Device enumeration** (`src/device.cyr`, 2.1.2) over yukti — separate
module, not in the dist, tested against real HW.

**g2p (2.2.0):** `src/g2p.cyr` bridges text → phonemes → speech over shabda 3.0.0
(`dhvani_g2p_text_to_phonemes` → svara `PhonemeEvent` vec; `dhvani_g2p_speak` →
`AudioBuffer`). The Rust module was otherwise `pub use` re-exports (no-ops in the
flat namespace). shabda 3.0.0 + shabdakosh 3.0.1 + varna 2.0.0 vendored and
externalized from the dist like the Wave F siblings; stdlib gained only `hashmap`
(the dictionary `map_*`) — not `mmap`/`bayan` (unreachable, DCE-prune). Parity `tests/g2p.tcyr` (14 one-for-one) +
`tests/bundle_g2p.tcyr`. **Full CI suite: 1692 assertions across 64 top-level
suites, all green on 6.4.11** (+ `tests/hw/device.tcyr`, HW-gated, excluded from CI).

**abaco 2.3.2** (dep bump): the numerical dsp-reference port caught abaco's dB
constants (`DB_SCALE`/`DB_EXP`/`DB_GAIN_EXP`) encoding a wrong `ln(10)` — ~0.04%/dB
error across 8 dhvani DSP modules. Fixed upstream (abaco 2.3.2), re-vendored;
`dsp_reference` now tests the real shipped dB path.

**Bundle:** `[lib].modules` = all 54 modules in L0→L4 order; `cyrius distlib` →
`dist/dhvani.cyr` (9.4k lines / 340 KB). It **externalizes abaco + the 10 siblings**
— dhvani ships only its own auditable code; a consumer links the siblings for the
features it uses (unused refs DCE-prune). Validated by `tests/bundle.tcyr` (core:
abaco+dist) + `tests/bundle_synth.tcyr` (synthesis feature: naad chain+abaco+dist).

**Wave F consumption pattern:** the 7 synthesis-stack siblings + sakshi + hisab +
shravan are vendored into `lib/` (committed) and included **in dependency order**
(`sakshi → hisab → goonj → naad → shravan → svara → ghurni → garjan → prani`) —
NOT wired as `[deps]` (which mis-orders cross-bundle types and force-includes the
136 KB `bayan`, overflowing the compiler's identifier cap). Every dist externalizes
its deps (nidhi→naad+shravan, naad→goonj, …); the consumer assembles the full set.
See the manifest and [`port-audit.md`](port-audit.md).

| Layer / Wave | Modules | Status |
|--------------|---------|--------|
| A — Foundation (core) | ✅ error, ✅ clock, ✅ simd(scalar), ✅ buffer/{mod,convert,resample,dither,ops} | ✅ |
| B — DSP L0/L1 (dsp) | ✅ oscillator, pan, gain_smoother, lfo, envelope, automation, svf, biquad, dsp(facade), routing | ✅ |
| C — DSP dependents (dsp) | ✅ compressor, limiter, delay, reverb, eq, deesser, graphic_eq | ✅ |
| D — Analysis (analysis) | ✅ waveform, zcr, analysis, fft, dynamics, loudness, stft, chroma, convolution, noise_reduction, key, onset, beat | ✅ |
| E — MIDI/meter/capture/graph | ✅ midi, voice, midi_routing, midi_v2, translate, meter, capture, record, graph | ✅ |
| F — Synthesis stack | ✅ synthesis(naad), sampler(nidhi), voice_synth(svara), creature(prani), environment(garjan), mechanical(ghurni), acoustics(goonj) | ✅ |
| G — Assembly | lib facade, dist/dhvani.cyr bundle, tests/{mod,proptest}, benches | ⬜ |
| g2p (2.2.0) | g2p (shabda 3.0.0 + shabdakosh + varna) | ✅ ported |
| ⛔ Blocked (dep) | voice_synth/bhava_bridge (bhava) | deferred |
| ⛔ Blocked (platform) | simd/{x86,aarch64}, ffi, capture/pw | deferred |
| ✂ Dropped | tests/serde_tests (no serde) | n/a |

Per-module parity ledger + conventions: [`port-audit.md`](port-audit.md).

## Tests

No parity suites ported yet. Target: one `tests/<module>.tcyr` per ported
module, each Rust `#[test]` ported one-for-one (serde round-trip + Display
tests dropped). ~660 `#[test]` blocks in the oracle; ~570 portable (excludes
the serde_tests suite and the blocked/platform modules).

## Next

See [`roadmap.md`](roadmap.md). **The port is complete and release-ready** — Waves
A–G all done: ✅ deferred analysis-gated tests, ✅ `dist/dhvani.cyr` bundle +
validation, ✅ integration/dsp-reference/proptest suites, ✅ abaco 2.3.2 dB fix,
✅ hot-path benches + `BENCHMARKS.md` + `docs/benchmarks-rust-v-cyrius.md`,
✅ CHANGELOG `[2.0.0]` finalized. **Only the git tag remains** (user). Post-2.0.0:
the blocked modules (g2p/bhava-voice on shabda/bhava; ffi/simd-intrinsics/pipewire
on platform) unblock as their deps/toolchain land.
`voice_synth/bhava_bridge` + `g2p` stay dep-blocked (bhava/shabda); `ffi`,
`simd/{x86,aarch64}`, `capture/pw` stay platform-blocked.
Then **Wave G** — lib facade + `dist/dhvani.cyr` bundle + integration tests, and
re-enable the deferred `ops::normalize_to_lufs` + `resample` sine tests (now
unblocked by fft/loudness). (The abaco↔naad `amplitude_to_db` collision is already
resolved upstream — naad 2.1.1 dropped its copies.)
