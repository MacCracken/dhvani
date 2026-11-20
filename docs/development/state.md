# dhvani — Current State

> Refreshed every release. CLAUDE.md is preferences/process/procedures
> (durable); this file is **state** (volatile).

## Version

**2.3.4** — the **multiband compressor** (`src/multiband.cyr`, ADR 007), the first
"Advanced DSP" backlog item.
- It uses LR4 crossovers in a tree, with allpass compensation and one `DhCompressor` per
  band, for 2 to 8 bands.
- Uncompressed, it is a flat-magnitude allpass. It allocates 0 bytes per call at any host
  block size.

Also in 2.3.4:
- Onset and tempo detection compute the spectral flux one STFT frame at a time
  (`dhvani_spectral_flux_from_buffer`), bit for bit with the spectrogram path. 10 s of
  audio allocates 72.5 KB, not 7.1 MB.
- MIDI 1.0 → 2.0 up-scaling is the spec's min-center-max rule, an oracle divergence
  recorded in ADR 005.
- ADR 006 records abaco as the math base.
- **jalwa** is confirmed on the bundle, and its DSP suite passes against 2.3.4.
- The AVX2 OS-check gap is filed in cyrius.
- No dependency or toolchain change: every dep was already on its newest tag.
**2.3.3** — **P(-1) sweep**: a seven-group audit of all of `src/`. It fixed two
regressions in 2.3.2's new code (the biquad fast path's frame count, the resample default
path's unchecked raw stores), 9 P-1s, about 30 P-2s and the parity P-3s. It also added
allocation-free forms of every per-call-allocating RT path, and optimized loops that stay
bit-identical (FFT 6.2×, graph cycle 4–9×, LRA 600×). Convolution now streams with one
block of latency, so every frame is processed for any host block size. Deliberate oracle divergences are in
`docs/decisions/005-hardening-divergences.md`. No dependency change.
**2.3.2** — **SIMD kernels restored** and every dep on its newest tag, abaco included. The
14 `src/simd.cyr` kernels run on Cyrius's packed verbs (`f64v_*` / `f64v256_*`), each
`_scalar` form kept as its oracle. `biquad` takes the oracle's stereo fast path again, and
`resample_sinc` its default path. abaco's hold lifts (naad renamed `power_spectrum`), and
goonj 2.3.2 closes the `DH_SIZE` overrun. Toolchain stays **6.7.18**.
**2.3.1** — toolchain **6.7.18** + every dep on its newest tag, except **abaco, held at
2.7.0** because 2.8.0's `power_spectrum` collides with naad's. No dhvani source change.
**2.3.0** — toolchain **6.7.9** + the **6.7.9 dependency batch** (every dep on its
6.7.9 release; hisab 3.3.7 → **3.4.0**). No dhvani source change was needed.
**2.2.7** — toolchain **6.6.20** + the **6.6.20 dependency batch** (every dep on
its 6.6.20 release; hisab 2.11.2 → **3.3.7**), and the compressor/limiter
**time-constant fix**: the integer sample rate reached abaco's f64
`time_constant` raw, so every attack/release had collapsed to one sample (abaco
2.4.9+ answers NaN for it, which is how the bump surfaced it). **2.2.4–2.2.6** —
toolchain 6.6.3 → 6.6.10, vani device glue on the value form, and no committed
`path` deps (a local resolve reads the same tags as CI). **2.2.3** — **P-1 hardening sweep** (2 device-buffer overruns, 2
alloc-path leaks, 1 process-abort, + sentinel/saturation repairs) and the
**vendored → real `[deps]` migration**: all 16 sibling bundles are now declared,
tag-pinned and hash-locked instead of hand-copied. `cyrius audit` fmt+lint gates
pass. **2.2.2** — toolchain **6.5.41** + a **full 17-bundle dependency
sweep** (naad 2.2.2 / svara 3.5.4 / hisab 2.11.2 / vani 1.2.2 / …), the
`f64_round` → `f64_round_half_away` **rounding-parity fix**, and a self-gating
`tests/hw/device.tcyr` (6.5.x made test discovery recursive). **2.2.1** —
toolchain 6.4.12 + svara 3.1.0 (control-rate glide synthesis) + shabda 3.0.1 /
shabdakosh 3.0.2 refresh. **2.2.0** — **g2p**
(grapheme-to-phoneme over shabda) + hisab 2.6.8 / vani 1.0.0.
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

- **Cyrius pin**: `6.7.18` (in `cyrius.cyml [package].cyrius`). Five deps — sakshi,
  sankoch, patra, yukti, vani — also ship in this pin's stdlib snapshot; their tags
  are kept equal to it (see `dependency-watch.md`).
- Build: `cyrius build src/main.cyr build/dhvani` (smoke binary — builds green).
- Test ONE suite: `cyrius test tests/<mod>.tcyr` (explicit path — no discovery).
- ⚠ **`cyrius test` discovery is RECURSIVE as of 6.5.x** — it walks `tests/`
  subdirectories, so `tests/hw/` is no longer invisible to CI. `tests/hw/device.tcyr`
  self-gates instead: no PCM endpoints → a named SKIP for that group, while its
  hardware-free groups still assert (6.6.11+ fails a `.tcyr` with no `N ≥ 1` summary).
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

**As of 2.2.3 all of these are real `[deps.X]` entries** (`git`+`tag`; no committed `path` since 2.2.6),
resolved by `cyrius deps` into `lib/` and hash-locked in `cyrius.lock` — 18
declared, 77 locked, 100% tag-pinned (versions below as of 2.3.2). The ordered `include` lines still govern
compile order. Bump a version via its `tag`, never by hand-copying into `lib/`.
Cost of the migration: `lib/` grew ~5.4 MB → 8.5 MB, because the siblings'
declared stdlib closure (bayan, chrono, mmap, slice, sync, thread*, sankoch, …)
comes along; all of it is unreachable from dhvani and DCE-prunes.

Direct (declared in `cyrius.cyml`):

- **stdlib** — base set + the DSP-math set (`math`, `ganita`, `tagged`,
  `fnptr`, `callback`, `bench`) + the g2p set (`hashmap`, `unicode`; **not** `bayan`,
  0 reachable calls) + `simd` (2.3.2: the AVX2 probe the packed kernels gate on).
- **Math**: ✅ **abaco 2.8.0** (`[deps.abaco]`, git+tag; `lib/abaco.cyr`, hash-locked; 2.3.1's
  hold at 2.7.0 lifted once naad 2.3.4 renamed its `power_spectrum`). DSP helpers
  verified (poly_blep/amplitude_to_db/constant_power_pan/…). abaco's unused
  json/http/net helpers DCE-prune (benign warnings).
- **Synthesis stack** (Wave F): naad **2.3.5**, svara **3.14.0**, prani **2.3.3**,
  nidhi **2.3.0**, garjan **2.8.0**, ghurni **2.8.0**, goonj **2.3.2** + sakshi
  **2.5.10** (logging) + hisab **3.5.0** (HVec3 for goonj) + shravan **2.12.6**
  (WAV codec for nidhi) + sankoch **2.8.9** (codec substrate) — real `[deps]`
  since 2.2.3, still included in dependency order.
- **g2p stack**: **shabda 3.2.0** (G2P engine, ported 2.2.0) + shabdakosh **3.2.0**
  (dictionary) + varna **2.6.0** (phoneme inventories) — same vendored-include pattern
  as Wave F (`… svara → varna → shabdakosh → shabda`), externalized from the dist.
- **Device I/O**: vani **1.2.10** (ALSA PCM; 1.2.x dropped the per-call scratch
  `alloc()` in the ring paths — an unbounded leak under the bump allocator) + yukti
  **2.3.18** (device enumeration, separate module — not in the dist) + patra
  **1.16.6** (yukti reads `/proc/asound` through patra's `read_procfs_text`,
  so patra now leads the include chain in `tests/playback.tcyr` too).
- **Blocked** (not ported): bhava (→ bhava-voice). shabda is now ported ✅.

## Consumers

- **jalwa** (media player) is on the Cyrius bundle. It pins dhvani **2.2.4**'s
  `dist/dhvani.cyr`, with cyrius 6.6.3 and abaco 2.4.6.
  - **Its surface** is 17 functions: `dhvani_geq_*` (new, process, reset, set_enabled,
    set_settings, band_name, settings_default / band / set_band / enabled / is_flat),
    `dhvani_buffer_{from_interleaved, silence, samples, rms, peak}` and
    `dhvani_analysis_suggest_gain`. It also calls one accessor,
    `DhGraphicEqSettings_set_enabled`.
  - **Checked for 2.3.4** (2026-11-20):
    - Every one of those signatures, and the `DhGraphicEqSettings` layout, is unchanged
      since 2.2.4.
    - jalwa's `tests/dsp.tcyr` passes against the 2.3.4 bundle (42/42) on cyrius 6.7.18
      with abaco 2.8.0.
  - **Before changing that surface**, grep jalwa for the symbol.
- shruti, aethersafta and kiran are still Rust, or have no checkout here.

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

**g2p (2.2.0; deps refreshed 2.2.1):** `src/g2p.cyr` bridges text → phonemes → speech over shabda 3.0.1
(`dhvani_g2p_text_to_phonemes` → svara `PhonemeEvent` vec; `dhvani_g2p_speak` →
`AudioBuffer`). The Rust module was otherwise `pub use` re-exports (no-ops in the
flat namespace). shabda 3.0.0 + shabdakosh 3.0.1 + varna 2.0.0 vendored and
externalized from the dist like the Wave F siblings; stdlib gained only `hashmap`
(the dictionary `map_*`) — not `mmap`/`bayan` (unreachable, DCE-prune). Parity `tests/g2p.tcyr` (14 one-for-one) +
`tests/bundle_g2p.tcyr`. **Full CI suite: 1692 assertions across 64 top-level
suites, all green on 6.4.12** (+ `tests/hw/device.tcyr`, HW-gated, excluded from CI).
**As of 2.3.4: 66 suites green on 6.7.18**: 2,761 assertions headless (the CI shape).
The additions since 2.3.3:
- the new `tests/multiband.tcyr`, 99 assertions, and +2 in `tests/bundle.tcyr`;
- +22 onset and +3 beat for the streamed flux;
- +23 translate for min-center-max.
**As of 2.3.3: 65 suites green on 6.7.18**: 2,612 assertions headless (the CI shape).
Every new regression group fails on the 2.3.2 sources. **As of 2.3.2: 65 suites green on 6.7.18** — 1,759 assertions headless (the CI shape),
1,760 with audio hardware (+23 simd, +4 biquad, +5 resample, +1 g2p). **As of 2.3.1: 65 suites green on 6.7.18** — 1,726 assertions headless, 1,727 with
audio hardware. **As of 2.3.0: 65 suites green on 6.7.9** — 1,726 assertions headless (the CI
shape), 1,727 with audio hardware. **As of 2.2.7: 65 suites green on 6.6.20** — 1,727 assertions on a box with
audio hardware, 1,726 headless (CI: 1,716 across the other 64 suites + 10 in
`tests/hw/device.tcyr`, which runs 11 with hardware). New: the compressor-attack /
limiter-release time-constant regressions, and the hw suite's always-run groups (its
SKIP is per group since 6.6.11's summary rule). **As of 2.2.3: 65 suites / 1,712 assertions green on 6.5.41** (the +17 are the
hardening regression tests: S24 storage-width + round-trip, convolution
block-spanning / alloc-free / set_ir-grows-partitions, graph alloc-free, and the
NaN-ignoring peak fold). The alloc-free assertions are
mutation-proven — reverting the convolution fix makes the test report exactly
130240 leaked bytes per call. **As of 2.2.2: 65 suites / 1,695 assertions green on 6.5.41** — `tests/hw/device.tcyr`
is now discovered (recursive runner) and contributes its 3 assertions, skipping
cleanly where there is no audio hardware.

**abaco 2.3.2** (dep bump): the numerical dsp-reference port caught abaco's dB
constants (`DB_SCALE`/`DB_EXP`/`DB_GAIN_EXP`) encoding a wrong `ln(10)` — ~0.04%/dB
error across 8 dhvani DSP modules. Fixed upstream (abaco 2.3.2), re-vendored;
`dsp_reference` now tests the real shipped dB path.

**Bundle:** `[lib].modules` = all 55 modules in L0→L4 order (54 ports + the 2.3.4
multiband compressor); `cyrius distlib` → `dist/dhvani.cyr` (13.2k lines at 2.3.4). It **externalizes abaco + the 10 siblings**
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
| G — Assembly | lib facade, dist/dhvani.cyr bundle, tests/{mod,proptest}, benches | ✅ |
| g2p (2.2.0) | g2p (shabda 3.0.1 + shabdakosh 3.0.2 + varna 2.0.0) | ✅ ported |
| ⛔ Blocked (dep) | voice_synth/bhava_bridge (bhava) | deferred |
| SIMD (2.3.2) | simd kernels on packed verbs + biquad stereo fast path + resample default path (simd/{x86,aarch64} intrinsics superseded) | ✅ |
| ⛔ Blocked (platform) | ffi, capture/pw (superseded in practice by vani device I/O) | deferred |
| ✂ Dropped | tests/serde_tests (no serde) | n/a |

Per-module parity ledger + conventions: [`port-audit.md`](port-audit.md).

## Tests

One `tests/<module>.tcyr` per ported module, each Rust `#[test]` ported one-for-one
(serde round-trip + Display tests dropped), plus the integration / dsp-reference /
proptest / bundle suites and `tests/hw/device.tcyr` (self-gating). **66 suites, 2,761
assertions headless** (2,612 at 2.3.3, 1,759 at 2.3.2). The oracle's five SIMD-vs-scalar
tests, dropped while only the scalar path existed, are ported since 2.3.2.

## Next

See [`roadmap.md`](roadmap.md). The port shipped as 2.0.0 (2026-07-05); 2.1.x added device
I/O, 2.2.0 g2p, and 2.3.2 the SIMD kernels. What is left:

- **Dep-blocked:** `voice_synth/bhava_bridge`, until bhava ports (bhava 2.0.0 is still Rust).
- **Platform-blocked:** `ffi` (deliberate non-goal) and `capture/pw` (superseded in practice
  by vani device I/O; ADR 004).
- **Consumers:** jalwa is on the bundle (pinned at 2.2.4, verified against 2.3.4); moving
  its pins is jalwa-side work. shruti / aethersafta are still Rust.
- **Advanced DSP, demand-gated:** noise suppression, pitch shift / time stretch; multiband
  extras (linear-phase crossover, sidechain, band solo, a graph node), ADR 007.
- **SIMD, demand-gated:** `peak_abs` / `clamp` / `noise_gate` vectorize once Cyrius has
  packed min/max/compare verbs; today they are walked.
- **Open from the 2.3.3 sweep:**
  - The stdlib `simd_has_avx2` lacks an OSXSAVE/XCR0 check. Upstream, filed 2026-11-19 as
    cyrius `docs/development/issues/2026-11-19-simd-has-avx2-skips-the-os-check.md`.
  - Closed since: convolution streaming (one block of latency, ADR 005), the streamed
    onset/beat flux, and MIDI 2.0 min-center-max up-scaling (ADR 005).
