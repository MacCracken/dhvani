# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.2.3] — P-1 hardening sweep + vendored deps promoted to real `[deps]`

### Fixed — P-1

- **S24 device buffers were sized with the packed width, so the kernel read/wrote
  33% past the allocation.** ALSA's `SND_PCM_FORMAT_S24_LE` carries 24 significant
  bits in a **32-bit** word (`vani_bytes_per_sample` returns 4), but every
  vani-facing path sized its buffer with `dh_pcm_bytes(24)` — which is **3**, the
  *packed* layout that matches `rust-old`'s `f32_to_i24_packed`. A 24-bit capture
  therefore allocated `frames*channels*3` and handed it to the kernel, which wrote
  `frames*channels*4`. Introduced `dh_pcm_store_bytes` (the ALSA storage width)
  and split the two concepts: the pure bridge keeps the packed width (Rust parity,
  and `tests/playback.tcyr` pins it), while `dhvani_capture_read_fmt`, the RT
  player and the RT recorder all use the storage width. `dh_pack_le` /
  `dh_unpack_le` gained explicit-width `_w` variants; the unpack masks to
  `bit_depth` bits before the sign fold, so S24's sign padding no longer reads as
  a huge positive sample.
- **`dhvani_playback_write` always emitted S16 regardless of the device's
  negotiated depth.** It called `dhvani_playback_to_s16le` (2 bytes/sample) but
  passed `frames` straight to the kernel, which reads
  `frames*channels*<negotiated width>` — so a device opened via
  `dhvani_playback_open_fmt(..., 24)` or `32` read past the buffer. It now reads
  the depth back off the handle (`vani_format_get` → `vani_format_bit_depth`).
- **Convolution reverb leaked 130,240 bytes per `process` call.** `process_block`
  allocated two `fft_size` accumulators per channel per block. `rust-old` does the
  same and lets scope drop them — but under a **free-less bump allocator** there is
  no drop, so a stereo 44.1 kHz render leaked ~2.8 MB/s until the process died.
  The accumulators are now struct-owned and zeroed in place. The figure is measured,
  not estimated: `tests/convolution.tcyr` asserts `alloc_used()` is unchanged across
  a steady-state call, and reverting the fix makes it report exactly 130240.
- **Convolution aborted the process when a block spanned two `process` calls.**
  `write_pos` carries across calls, so `block_start = frame + to_copy - block_size`
  goes negative and reached `vec_get`, which calls `_vec_die()` → `exit(1)`. Those
  output frames belong to the previous call's buffer and are now skipped —
  reproducing what the oracle does in release mode, where the same `usize`
  expression wraps and fails its `src_idx < len` check.
- **The graph processor allocated a fresh input-gather vec per node per audio
  cycle.** `rust-old` carries an `input_scratch: Vec<AudioBuffer>` field on
  `GraphProcessor` and clears it per node; the port dropped the field. Restored, so
  the RT graph path is 0 bytes/cycle in steady state (asserted in `tests/graph.tcyr`).

- **`dhvani_conv_set_ir` with a longer IR aborted the process.** The per-channel
  frequency delay line is sized to the partition count, and
  `dhvani_conv_ensure_channels` — the only place that sizes it — early-returns
  when the channel count is unchanged. So: process once with a short IR, set a
  longer one, process again, and `process_block` indexed `ch_fdl` past its end
  (`vec_get` → `_vec_die` → `exit(1)`). `set_ir` now invalidates the channel cache
  so the next `process` rebuilds the FDL. Regression-tested; the test aborts with
  `vec: index out of bounds` without the fix.
- **NaN in a sample buffer silently under-reported every running peak.** Cyrius's
  `f64_max` is **asymmetric** on NaN — `f64_max(x, NaN)` → NaN, but
  `f64_max(NaN, x)` → x — so a naive fold first poisons the accumulator and then
  **resets** it to the sample after the NaN. `rust-old` uses `f32::max` /
  `.fold(0.0f32, f32::max)`, which *ignore* NaN. Measured: a buffer of
  `[0.8, NaN, 0.2]` reported a peak of **0.2 instead of 0.8**, so
  `dhvani_dsp_normalize` would over-amplify 4×. The threat model explicitly treats
  incoming samples as untrusted and expects NaN, so this was reachable from a
  public entry point. Added `dh_max_ignore_nan` and applied it to all six
  running-max folds: `simd` (`peak_abs`), `limiter`, `compressor`, `deesser`
  (sidechain peak), `analysis` (spectrum max), `chroma`.

### Fixed — P-2 / P-3

- **`dhvani_vsynth_render_vocal_tract` checked neither svara handle** before using
  them as struct bases (raw `load64`/`store64`, with no vec bounds check behind
  them). Both producers return negative error codes — `svara_glottal_new` →
  `SVARA_ERR_INVALID_PITCH`, `svara_tract_new` → a mapped naad error — so an
  unchecked handle forwarded by a consumer was a wild access. Rust took `&mut`
  references here, which made an error value unrepresentable; the flattened i64
  port has to state the guard. Every sibling bridge already did.
- **`dhvani_detect_tempo` could produce a negative lag.** Rust's float→int
  `as usize` **saturates** (negative → 0), so `min_lag`/`max_lag` could never go
  negative there; Cyrius's `f64_to` truncates, so a negative or NaN BPM reached
  `vec_get` as a negative index and aborted the process. Now clamped, reproducing
  the oracle's saturating cast.

### Changed — dependencies

- **All 16 vendored sibling bundles are now real `[deps]`**, declared in dependency
  order with `git` + `path` + `tag`, replacing the hand-copy-into-`lib/` step.
  Versions are declared and hash-locked in `cyrius.lock` (74 entries) instead of
  living in an uncommitted copy, CI resolves them from git tags, and the whole
  surface is auditable from one file. `cyrius deps` reproduced all 16 bundles
  **byte-identically**, and the `include "lib/<x>.cyr"` lines are unchanged —
  `[deps]` governs vendoring, the includes still govern compile order.
- The two defects that forced hand-vendoring are gone, so the workaround retired:
  transitive ordering is now **declared** (every sibling lists its full closure and
  ships a `.deps` sidecar `cyrius deps` consumes), and the LEXID identifier cap went
  16384 → 65536 (cyrius 6.4.21) with the pool 256 KB → 512 KB (6.4.76), so the
  documented "`bayan` overflows the cap" ceiling is no longer reachable.
- **`sankoch` 2.7.10 declared explicitly.** It arrives transitively under
  shravan/nidhi; left implicit, `cyrius deps` pinned it to a bare commit hash.
  Declaring it makes the closure **100% tag-pinned** (0 commit-pins). dhvani makes
  no `sankoch_*` calls — it DCE-prunes — but it is now auditable.
- `lib/` grows ~5.4 MB → 8.5 MB: promoting to `[deps]` pulls the siblings' full
  declared stdlib closure (bayan, chrono, mmap, slice, sync, thread*, sankoch, …)
  rather than only the leaves the hand-vendoring happened to need. These are
  unreachable from dhvani and DCE-prune; `bayan`'s size is an upstream matter.

### Changed — audit gate

- **`cyrius audit` fmt and lint gates now pass** (they were `FAIL: files need
  reformatting` + 33 warnings). All 58 `src/*.cyr` are canonically formatted, and
  the 33 long-line warnings and 1 untracked deferral are cleared. The line-wrapping
  was verified **token-identical** on 56 of 58 files; the two exceptions are
  `biquad.cyr` and `delay.cyr`, where long nested expressions were given
  intermediate names by hand rather than wrapped.
- Toolchain pin stays **6.5.41** deliberately (6.5.42 is in progress upstream), so
  a `toolchain drift` warning on 6.5.42 boxes is expected and benign.

Full suite **65 suites / 1,712 assertions green**; `cyrius audit` fmt+lint clean.
Every alloc-free and NaN assertion added here is **mutation-proven** — reverting
the fix makes the test fail with the exact wrong value (130240 leaked bytes;
peak 0.2 instead of 0.8).

## [2.2.2] — toolchain 6.5.41 + full dependency sweep

### Fixed

- **Rounding parity break (`f64_round` → `f64_round_half_away`)** — abaco 2.4.x
  **deleted** `f64_round`, renaming it to `f64_round_half_away`. dhvani's three
  call sites kept compiling because the bare name silently rebound to the **cycc
  builtin `f64_round`**, which rounds **ties-to-even** where abaco's (and Rust's)
  rounds **half away from zero**. Measured divergence: `0.5 → 0` (was 1),
  `2.5 → 2` (was 3), `4.5 → 4` (was 5). All three oracle sites use Rust
  `f32::round()` (half-away), so this was a silent parity regression against
  `rust-old/`:
  - `src/reverb.cyr:46` — comb/allpass delay length
    (`rust-old/src/dsp/reverb.rs:160`); ties are reachable at 22050 Hz, where
    `base * 0.5` lands on an exact `.5` for every odd `base`.
  - `src/dither.cyr:65` — quantization step
    (`rust-old/src/buffer/dither.rs:58`).
  - `src/chroma.cyr:83` — semitone → pitch-class
    (`rust-old/src/analysis/chroma.rs:72`); an exact `.5` is a quarter-tone.

  All three now call `f64_round_half_away` explicitly. The oracle has exactly
  three `.round()` calls and dhvani exactly three `f64_round` calls — a 1:1 map.

- **`tests/hw/device.tcyr` no longer breaks CI.** As of 6.5.x `cyrius test`
  auto-discovery **recurses into `tests/` subdirectories**, so the bare
  `cyrius test` CI runs now pick up the hardware suite that the old top-level-only
  discovery excluded (verified: 65 suites with `tests/hw/`, 64 without). The test
  now **gates itself at runtime** — when enumeration returns no PCM endpoints it
  prints a named SKIP and exits 0, so a headless runner stays green while a box
  with a sound card still gets the full `n > 0` assertion. Mutation-proven in both
  directions. The gate is deliberately a runtime check on the enumeration result,
  not a compile-time `#ifdef`/CI flag, matching the toolchain's own idiom for
  capability-gated tests (cyrius 6.5.27).

### Changed

- **Toolchain pin `6.4.12` → `6.5.41`** (`cyrius.cyml [package].cyrius`) — ~120
  releases. No source change was required to compile. The one broad-reach codegen
  change is **6.4.56 SESTYPE normalization**, which retypes `f64_lt(..)`-style
  builtin results as i64-boolean; dhvani uses the `if (f64_gt(x, y) == 1)` idiom
  at 40+ DSP sites, so every one of those comparisons changed emitted code.
  Re-benched to prove it cost nothing — see `BENCHMARKS.md`.
- **All 17 vendored bundles re-vendored** to current releases:

  | bundle | from | to | | bundle | from | to |
  |--------|------|----|-|--------|------|----|
  | abaco | 2.3.2 | **2.4.5** | | shabda | 3.0.1 | **3.0.4** |
  | naad | 2.1.1 | **2.2.2** | | shabdakosh | 3.0.2 | **3.0.6** |
  | svara | 3.1.0 | **3.5.4** | | varna | 2.0.0 | **2.4.1** |
  | prani | 2.0.1 | **2.0.12** | | hisab | 2.6.8 | **2.11.2** |
  | nidhi | 2.0.0 | **2.1.1** | | shravan | 2.6.7 | **2.8.0** |
  | garjan | 2.0.0 | **2.5.1** | | sakshi | 2.4.4 | **2.4.12** |
  | ghurni | 2.0.0 | **2.6.0** | | vani | 1.0.0 | **1.2.2** |
  | goonj | 2.0.0 | **2.0.4** | | yukti | 2.2.8 | **2.3.8** |
  | | | | | patra | 1.12.8 | **1.13.11** |

- **`FILTER_*` → `NAAD_FILTER_*` (naad 2.2.0) forced a coordinated bump.** naad's
  biquad filter-type constants were renamed, and garjan / ghurni / prani / svara
  all absorbed the rename upstream. naad, svara, prani, ghurni and garjan therefore
  had to move **together** — bumping any one alone leaves undefined constants. The
  already-published dists are all naad-2.2-ready, so no shim was needed. The bump
  also clears a latent duplicate-symbol hazard in `tests/sampler.tcyr`, which
  includes both `lib/naad.cyr` and `lib/nidhi.cyr` — both previously defined
  `FILTER_LOWPASS..NOTCH` at identical values.
- **stdlib gained `unicode`** (`cyrius.cyml [deps].stdlib`) — shabda 3.0.2's
  Unicode rewrite routes `is_alphabetic` / `to_lowercase` / `is_whitespace`
  through real Unicode tables (`unicode_category`, `unicode_to_lower`,
  `GC_LU`/`GC_LO`/`GC_NL`) instead of ASCII-only checks. It vendors as a
  **directory** (`lib/unicode/`, 7 files), unlike every other flat `lib/*.cyr`.
  `hashmap` is consequently no longer "the sole g2p addition"; the manifest
  comment was corrected.
- **`[deps.abaco] tag` `2.3.2` → `2.4.5`** — the tag had drifted out of sync with
  the vendored bundle. Because `path = "../abaco"` wins for local dev, `cyrius
  deps` had already resolved 2.4.5 into `lib/` while CI would still have fetched
  the 2.3.2 tag — the skew that hid the `f64_round` break above.
- **stdlib re-synced to the 6.5.41 snapshot** (19 files) plus the new
  `lib/hashseed.cyr` leaf (per-process hash seed shared by the hashmap variants).
  6.4.63's `./lib/ shadows version-pinned …` warning — which named sakshi / yukti
  / patra / vani skew on every invocation — is now silent.
- **`tests/playback.tcyr` include chain gained `lib/patra.cyr`** ahead of yukti:
  yukti 2.3.8 rewrote `_audio_load_drivers` to read `/proc/asound` via patra's
  `read_procfs_text` instead of raw `sys_open`/`sys_read`, newly externalizing
  `patra_result_get_str_len`.
- **`tests/sampler.tcyr` binds `n_engine_note_on`'s result** at three sites —
  nidhi 2.1.1 marked it `#must_use`.
- **`dist/dhvani.cyr` rebuilt** at 2.2.2 (9,538 lines), now emitted alongside a
  `dist/dhvani.deps` stdlib sidecar (`cyrius distlib` generates it; `cyrius deps`
  consumes it).
- `.gitignore` now excludes `lib/*.cyr.f` / `lib/*.cyr.syms`, the per-dep
  resolution caches `cyrius deps` writes next to each vendored bundle.

### Notable upstream behavior changes (no dhvani code change)

- **vani 1.2.x removed the per-call `alloc()`** in `vani_play_from_ring` /
  `vani_record_to_ring`, caching scratch on the device handle instead. On a
  free-less bump allocator that per-call allocation was an unbounded leak across a
  long render, so this directly hardens dhvani's RT ring path.
- **varna 2.4.1 made the pre-built phoneme inventories shared singletons**
  (build-once). Previously every `dhvani_g2p_text_to_phonemes` / `dhvani_g2p_speak`
  call rebuilt the 47-entry English inventory plus phonotactics — also an
  unbounded bump-allocator leak. Treat the returned inventories as read-only;
  `phoneme_clone` gives a mutable copy.
- **nidhi 2.1.1's render path is allocation-free**, and **svara 3.5.4** closes a
  crash for `sample_rate <= 1200 Hz` in the creature/vocal path (`tests/creature.tcyr`
  only ever passes 44100, which is why the suite never saw it).
- **shabda 3.0.2+** fixed a heap over-read in `_shabda_decode_chars` (UTF-8
  continuation bytes past the NUL terminator) and stopped `shabda_select_phonemes`
  terminating the host process on an empty pronunciation vector. SSML nesting is
  now capped at `SHABDA_SSML_MAX_DEPTH = 256` — a parse error where `rust-old`
  would SIGSEGV, and an intentional divergence from the oracle.
- **naad 2.2.2 / garjan 2.4.0 / ghurni 2.5.0** now range-check constructor
  arguments and return `*_ERR_INVALID_PARAMETER` instead of falling through to a
  last-enum default.

Full suite **65 suites / 1,695 assertions green** on 6.5.41 (1,692 at 2.2.1; the
+3 is `tests/hw/device.tcyr`, now discovered by the recursive runner). Hot paths
re-benched with no regression.

## [2.2.1] — toolchain 6.4.12 + svara 3.1.0 refresh (in progress)

### Changed

- **Toolchain pin `6.4.11` → `6.4.12`** (`cyrius.cyml [package].cyrius`) — current
  release; aligns dhvani with the shabda/shabdakosh/svara chain on one toolchain.
- **svara `3.0.1` → `3.1.0`** (re-vendored) — control-rate glide synthesis: diphthong
  formant coefficients recompute at a 64-sample control rate instead of per-sample
  (~5.8× faster diphthong synth), perceptually identical. Pure drop-in for dhvani
  (identical 246-symbol surface); voice_synth / creature / synthesis / g2p unchanged.
- **shabda `3.0.0` → `3.0.1`** (re-vendored) — dep/toolchain pin refresh (no shabda
  API change): picks up svara 3.1.0 + shabdakosh 3.0.2 end to end, so `dhvani_g2p_speak`
  of a diphthong word gets svara's faster render.
- **shabdakosh `3.0.1` → `3.0.2`** (re-vendored) — dictionary re-pinned to svara 3.1.0,
  keeping the vendored g2p chain consistent with what shabda 3.0.1 ships.

Full suite (64 suites / 1692 assertions) green on 6.4.12; drop-in surfaces confirmed
(zero public symbols removed across svara / shabda / shabdakosh).

## [2.2.0] — g2p (grapheme-to-phoneme) + toolchain/dep refresh (in progress)

### Added

- **`g2p` feature** (`src/g2p.cyr`) — grapheme-to-phoneme: text → phoneme
  sequences → speech, over the newly-ported **shabda 3.0.0**. Two bridges:
  `dhvani_g2p_text_to_phonemes(engine, text)` (→ vec of svara `PhonemeEvent`
  handles, or null on invalid input) and `dhvani_g2p_speak(engine, text, voice,
  sample_rate)` (→ mono `AudioBuffer`, G2P + voice synthesis in one call). The
  rest of the Rust `g2p` module was `pub use shabda::*` / `pub use
  svara::sequence::*` re-exports — no-ops in Cyrius's flat namespace, where a
  consumer calls the bare `shabda_*` / `svara_*` symbols directly. This unblocks
  the last dep-blocked module from the 2.0.0 port audit (only `bhava-voice`
  remains, on bhava). Parity: `tests/g2p.tcyr` (the 14 oracle `#[test]` blocks,
  one-for-one) + `tests/bundle_g2p.tcyr` (dist externalization).
- **shabda 3.0.0 + subtree vendored** — `lib/shabda.cyr` (G2P engine, rules,
  normalize, syllabify, prosody, SSML subset, heteronyms), plus its deps
  `lib/shabdakosh.cyr` (3.0.1, pronunciation dictionary) and `lib/varna.cyr`
  (2.0.0, phoneme inventories). Included in dependency order after the naad/svara
  chain and **externalized** from `dist/dhvani.cyr` like every Wave F sibling
  (unused refs DCE-prune for non-g2p consumers). Added one stdlib leaf, `hashmap`
  (the dictionary `map_*`); `mmap`/`bayan` stay out (unreachable — DCE-prune).

### Changed

- **Toolchain pin `6.4.3` → `6.4.11`** (`cyrius.cyml [package].cyrius`). The full
  suite (64 suites / 1692 assertions) is green on 6.4.11.
- **hisab `2.6.7` → `2.6.8`** (re-vendored) — upstream "collision hardening for
  co-compilation": the float-render scratch moved off an enum-sized stack array,
  and hisab's bare `ERR_*` error constants are now namespaced `HSB_ERR_*` (values
  unchanged). Both changes are transparent to dhvani (dhvani/goonj/naad keep their
  own `ERR_NONE == 0`; hisab simply exits that harmless collision).
- **vani `0.9.9` → `1.0.0`** (re-vendored) — vani's first stable release; a
  drop-in for `src/playback.cyr` (vani 1.0.0 certifies the full 106-symbol
  `vani_*` surface against dhvani 2.1.2's playback path). No consumer-facing
  breaking changes.

## [2.1.2] — capture ring path (in progress)

### Added

- **Lock-free, alloc-free ring capture** (`src/playback.cyr`) — the recorder
  mirror of the 2.1.1 player: `dhvani_recorder_new` / `dhvani_recorder_new_fmt` /
  `dhvani_recorder_fill` (device → ring via `vani_record_to_ring`) /
  `dhvani_recorder_read` (ring → decoded `AudioBuffer` at the recorder's bit
  depth) / `dhvani_recorder_ring`. Reusable scratch, no per-block allocation on
  the fill side; multi-format (S16/S24/S32). Unit-tested hardware-free (fill the
  ring with known PCM → read back → decoded samples match).

- **Device enumeration** (`src/device.cyr`) — `dhvani_devices_list` + descriptor
  accessors (`dhvani_device_card`/`_index`/`_direction`/`_name`/`_is_playback`/
  `_is_capture`) + default-device open (`dhvani_playback_open_default[_fmt]` /
  `dhvani_capture_open_default[_fmt]`: enumerate → open the first match). Wraps
  yukti's PCM discovery (udevadm subprocess + sysfs), which pulls yukti's
  `freelist`/`process`/`fs`/`patra` deps (vendored) — so it's a separate module,
  NOT in the core dist (it references yukti direction consts + the heavier stack;
  the bundle stays device-agnostic). Tested against real endpoints via
  `tests/hw/device.tcyr` — a **hardware test excluded from CI** (bare `cyrius test`
  only discovers top-level `tests/*.tcyr`; `tests/hw/` runs locally by explicit path).

## [2.1.1] — real-time playback path

### Added

- **Lock-free, alloc-free ring playback** (`src/playback.cyr`) —
  `dhvani_player_new` / `dhvani_player_push` / `dhvani_player_ring` /
  `dhvani_player_drain`. A player owns a vani ring plus a reusable scratch PCM
  buffer, so pushing a block converts into scratch and copies into the ring with
  **zero per-block allocation** (what the free-less bump allocator requires of an
  RT hot path); the RT loop pushes producer blocks and drains to the device via
  `vani_play_from_ring`. Unit-tested hardware-free (push → ring read-back matches
  the one-shot `to_s16le` bytes).
- **Multi-format PCM (S16 / S24 / S32 little-endian)** — generic alloc-free
  `dh_pack_le` / `dh_unpack_le` (+ `dh_pcm_bytes`/`dh_pcm_scale`) power new
  format-aware `dhvani_playback_to_pcm` / `dhvani_capture_from_pcm` and `_fmt`
  variants of the device open, capture read, and ring player
  (`dhvani_player_new_fmt`). The 2.1.0 S16 fns stay as thin wrappers (unchanged
  API). Byte-packing + round-trip unit-tested at all three depths.

## [2.1.0] — vani device I/O

Adds real-hardware audio I/O by bridging dhvani to **vani** — dh(vani)'s ALSA
sibling: raw `/dev/snd` PCM playback/capture via ioctls, no libasound, no FFI.
2.0.0 shipped headless (processing only); 2.1.0 gives the engine a device sink/
source without the platform-blocked PipeWire path.

### Added

- **`playback` module** (`src/playback.cyr`) — bridges the f64 `AudioBuffer` to
  vani's interleaved S16_LE PCM:
  - `dhvani_playback_to_s16le` / `dhvani_capture_from_s16le` — hardware-free format
    bridges (f64 interleaved ↔ little-endian i16 bytes), unit-tested (byte packing
    + round-trip within i16 quantization).
  - `dhvani_playback_open`/`_write`/`_close` + `dhvani_capture_open`/`_read` —
    device glue over vani (open → configure S16_LE → start → play/record); needs
    `/dev/snd`, validated on hardware.
  - Bundled into `dist/dhvani.cyr`, externalizing vani (+ its `yukti`/`sakshi`
    deps) and referencing it through functions only, so it DCE-prunes for consumers
    that don't link vani — the core bundle stays device-agnostic.

### Changed

- **Dependencies**: **vani** (ALSA I/O) + **yukti** (device descriptors) vendored
  into `lib/`; a consumer wanting device I/O links `yukti → sakshi → vani` ahead of
  dhvani. No collisions with dhvani/abaco.

## [2.0.0] — Cyrius port — 2026-07-05

Complete rewrite from Rust to **Cyrius**. dhvani's Rust line shipped through
1.1.0; the language port is a major break, so it lands as 2.0.0. The
23,695-line Rust source (64 modules, ~660 tests) is frozen at `rust-old/` as the
parity oracle — every Cyrius module cross-checked against it function-for-function.
**54 of 64 modules ported** (the remainder dependency- or platform-blocked, below);
**1625 parity assertions across 61 test suites, all green.** Per-module ledger in
[`docs/development/port-audit.md`](docs/development/port-audit.md); wave sequencing
in [`docs/development/roadmap.md`](docs/development/roadmap.md); throughput vs the
Rust oracle in [`docs/benchmarks-rust-v-cyrius.md`](docs/benchmarks-rust-v-cyrius.md).

### Added

- **The full portable engine, in Cyrius** — core (buffer/convert/resample/dither/
  ops), the DSP surface (oscillator/biquad/svf/eq/compressor/limiter/delay/reverb/
  deesser/graphic_eq/pan/lfo/envelope/automation/…), analysis (fft/stft/chroma/key/
  onset/beat/loudness/dynamics/convolution/noise_reduction/waveform/zcr), MIDI
  (v1/v2/voice/routing/translate), the RT-safe processing `graph` + `meter`,
  `capture`/`record`, and the feature-gated synthesis stack (synthesis/sampler/
  voice_synth/creature/environment/mechanical/acoustics) wrapping the ported siblings.
- **`dist/dhvani.cyr`** — the consumer distlib bundle (`cyrius distlib`): dhvani's
  own modules in dependency order, externalizing abaco + the synthesis siblings so
  the shipped surface stays auditable (consumers link siblings per feature).
- **Benchmarks** — `tests/hotpath.bcyr`, `tests/bench_compare.bcyr`, `BENCHMARKS.md`,
  and a Rust-vs-Cyrius comparison: scalar-f64 runs 6–250× the Rust f32-SIMD depending
  on vectorizability (per-sample DSP ~6×; flat SIMD reductions worst).

### Changed

- **Language**: Rust → Cyrius (`.cyr`). Toolchain pinned via `cyrius.cyml` (6.4.3).
  Build `cyrius build src/main.cyr build/dhvani`; test `cyrius test tests/<mod>.tcyr`.
- **f32 → f64** throughout (the audio-stack math is f64-only); **enums → integer
  codes**, **`Result`/`Option` → sentinel/error-code returns**, **closures + generic
  traits → fn-ptr**, **tuples → out-params**. No serde, no unwinding/panic.
- **Math**: kept **abaco** (2.3.2) for dhvani's DSP helpers — the numerical
  dsp-reference port caught and fixed abaco's dB constants (a wrong `ln(10)`)
  upstream. The synthesis siblings (naad 2.1.1, svara 3.0.1, prani, nidhi, garjan,
  ghurni, goonj + sakshi/hisab/shravan) are vendored and included in dependency
  order rather than wired as `[deps]`. `serde`/`thiserror`/`tracing`/`criterion`/
  `rayon`/`proptest` dropped.

### Removed / deferred

- **serde surface** + all serde round-trip tests (no serde in Cyrius).
- **SIMD acceleration** (`simd/x86`, `simd/aarch64`) — raw SSE2/AVX2/NEON
  intrinsics have no Cyrius equivalent; only the scalar kernels port (accepted
  throughput regression until Cyrius has a SIMD story).
- **C-ABI FFI** (`ffi`) — deferred; consumers are Cyrius-native.
- **PipeWire capture** (`capture/pw`) — platform FFI deferred behind the
  `pipewire` gate; `capture/{mod,record}` port.

### Blocked (dependency not yet ported to Cyrius)

- **`g2p`** feature — blocked on **shabda** (still Rust).
- **`bhava-voice`** feature (`voice_synth/bhava_bridge`) — blocked on **bhava**
  (still Rust). All other deps are ported.

## [1.1.0] — 2026-04-01

### Changed

#### Dependencies — BREAKING (minor)
- **svara** upgraded from 1.x to 2.0.0 — voice synthesis engine gains LOD, object pooling, batch rendering, trajectory planning, nasalization, vocal effort, tonal language support
- **shabda** upgraded from 1.x to 2.0.0 — G2P engine gains SSML parsing, heteronym disambiguation, conversion options, timing profiles
- **bhava** 2.0.0 added as optional dependency (feature: `bhava-voice`) — personality/mood to voice parameter mapping

### Added

#### Bhava Voice Bridge (feature: `bhava-voice`)
- **`voice_from_personality()`** — derive voice baseline from personality traits (warmth → breathiness, confidence → f0 range, empathy → vibrato, patience → vibrato rate, formality → pressed phonation)
- **`prosody_from_mood()`** — generate f0 contour, duration, and amplitude from emotional state (joy/arousal → pitch, arousal → tempo, frustration → contour jaggedness)
- **`effort_from_mood()`** — map arousal/dominance/frustration to vocal effort level (whisper → shout)
- **`apply_mood_to_voice()`** — real-time voice coloring from current mood (per-utterance)
- **`apply_stress_to_voice()`** — chronic stress degrades voice quality (jitter, shimmer, raised f0, narrowed range)
- **`quality_from_energy()`** — energy/performance → synthesis LOD (full/reduced/minimal)
- **`intonation_from_mood()`** — mood → intonation pattern selection (exclamatory, interrogative, continuation, declarative)

#### Voice Synthesis Bridge (feature: `voice`)
- **`VocalEffort`**, **`EffortParams`** — coordinated vocal effort levels (whisper → soft → normal → loud → shout) with physically-modeled parameter sets
- **`Quality`** — LOD control for multi-voice rendering (full/reduced/minimal pipeline complexity)
- **`Tone`** — lexical tone support for tonal languages (9 variants: Mandarin tones 1–5, Thai/Vietnamese/African patterns) with f0 contour generation
- **`Nasalization`**, **`detect_nasalization()`** — anticipatory nasalization detection and synthesis across phoneme sequences
- **`synthesize_phoneme_nasalized()`** — single-phoneme synthesis with nasal coupling
- **`VoiceOnsetTime`** — VOT parameters for plosive consonants (Lisker & Abramson model)
- **`SynthesisContext`** — reusable synthesis context for zero-allocation per-phoneme rendering
- **`phoneme_spectral_tilt()`**, **`height_adjusted_amplitudes()`** — phoneme-level spectral analysis functions
- **`SynthesisPool`** — pre-allocated object pool for zero-allocation batch phoneme rendering with diagnostic counters and pre-warmed buffer capacity
- **`BatchRenderer`**, **`RenderProgress`**, **`RenderOutput`** — non-real-time batch rendering with per-phoneme progress callbacks
- **`TrajectoryPlanner`**, **`FormantKeypoint`** — Catmull-Rom spline formant trajectory planning across multi-phoneme windows with coarticulation resistance and speaking rate adjustment (Lindblom undershoot)

#### G2P Bridge (feature: `g2p`)
- **`ConvertOptions`** — G2P conversion options: emphasis detection, speaking rate (WPM), timing profiles
- **`TimingProfile`** — independent scaling of vowel, consonant, and pause durations
- **`heteronym` module** — context-based heteronym disambiguation for 20 English heteronyms (read, lead, live, wind, etc.) with preceding-word trigger rules
- **`ssml` module** — SSML 1.1 subset parser (`<break>`, `<emphasis>`, `<prosody>`, `<phoneme>`, `<speak>`) with `no_std + alloc` compatibility

#### Tests
- 20 new voice synthesis tests: vocal effort, LOD quality, tone contours, nasalization detection, spectral tilt, height-adjusted amplitudes, VOT, synthesis context, nasalized phoneme synthesis, synthesis pool (render, capacity, batch), batch renderer (render, progress), trajectory planner (basic, speaking rate, keypoints)
- 12 new G2P tests: convert options builder, timing profile, heteronym lookup/select/trigger/miss, SSML parse (plain text, break, emphasis, prosody, speaking rate WPM)
- 625 unit + 22 doc-tests pass

#### Benchmarks
- **`benches/voice.rs`** (8 benchmarks): render_phoneme, render_sequence, synthesis_pool_render, synthesis_pool_batch, batch_renderer, trajectory_plan, trajectory_formants_at, detect_nasalization, glottal_vocal_tract
- **`benches/g2p.rs`** (6 benchmarks): g2p_hello_world, g2p_sentence, speak, heteronym_lookup/select_variant, ssml_parse (plain/break/complex)

### Fixed
- Doc version strings: `0.22` → `1` in 8 module doc comments (voice_synth, g2p, synthesis, acoustics, creature, environment, mechanical, sampler)
- Supply chain audits updated for all 16 bumped dependencies (svara 2.0.0, shabda 2.0.0, shabdakosh 2.0.0, garjan 1.1.0, hisab 1.4.0, winnow 1.0.1, + transitive bumps)
- `dependency-watch.md` updated with current svara/shabda versions

## [1.0.0] — 2026-03-29

### Changed — BREAKING

#### API Hardening
- `Compressor::set_params()`, `EnvelopeLimiter::set_params()`, `DeEsser::set_params()`, `Envelope::set_params()`, `Reverb::set_params()` now return `Result` — parameters are validated on update (previously bypassed constructor checks)
- `#[non_exhaustive]` added to all public parameter structs (`CompressorParams`, `LimiterParams`, `DeEsserParams`, `ReverbParams`, `AdsrParams`, `ModulatedDelayParams`, `EqBandConfig`, `GainSmootherParams`, `GraphicEqSettings`) and several data structs (`DynamicsAnalysis`, `Spectrogram`, `R128Loudness`, `OnsetResult`, `WaveformData`, `NoteEvent`, `ControlChange`, `MidiClip`, `CcMapping`, `Connection`, `CaptureConfig`, `OutputConfig`, `AudioDevice`)
- `#[non_exhaustive]` added to `EnvelopeState`, `VoiceState`, `CrossfadeType`, `FadeCurve` enums
- `AudioBuffer::from_interleaved()` now rejects sample counts not divisible by channel count (returns `LengthMismatch`)
- Builder constructors added: `CompressorParams::new().with_threshold().with_ratio()...`, `ReverbParams::new().with_room_size()...`, `LimiterParams::new().with_ceiling()...`, `EqBandConfig::new()`
- `NoteEvent::new()` and `ControlChange::new()` constructors added

#### API Encapsulation — v1.0 Freeze
- `DynamicsAnalysis` fields now private — use `peak()`, `peak_db()`, `true_peak()`, `true_peak_db()`, `rms()`, `rms_db()`, `crest_factor_db()`, `lufs()`, `dynamic_range_db()`, `frame_count()`, `channel_count()` accessors
- `R128Loudness` fields now private — use `integrated_lufs()`, `range_lu()`, `short_term_lufs()`, `momentary_lufs()` accessors
- `OnsetResult` fields now private — use `positions()`, `strengths()`, `count()` accessors
- `NoteEvent` fields now private — use `position()`, `duration()`, `note()`, `velocity()`, `channel()` accessors
- `ControlChange` fields now private — use `position()`, `controller()`, `value()`, `channel()` accessors
- `MidiClip` fields now private — use `name()`, `notes()`, `control_changes()`, `timeline_pos()`, `duration()` accessors
- `Connection` fields now private — use `from()`, `to()` accessors
- `NodeId` inner field now private — use `value()` accessor
- `LevelMeter` fields now private — use `peak()`, `rms()`, `lufs()` accessors (plus existing `peak_db()`, `rms_db()`, `peak_hold()`)

### Fixed

#### Correctness
- **SVF Peak/Shelf modes** — Peak mode now correctly boosts/cuts the bandpass component around cutoff (`input + (A²-1) * BP`) with modified `k = 1/(Q*A)` per Cytomic spec (was computing `HP * A²`, a scaled high-pass). Shelf modes: removed dead `* 0.0` terms; formulas were correct underneath
- **R128 loudness** — integrated, ungated, and short-term LUFS now averaged in linear power domain per EBU R128 spec (was incorrectly averaging in dB domain)
- **True peak detection** — upgraded from linear interpolation (which always equaled sample peak) to 4-point cubic Hermite interpolation for proper inter-sample peak detection
- **`AudioClock::set_tempo()`** — now clamps negative and NaN values to 0 (was storing invalid values)
- **`Lfo::set_rate()`** — now clamps negative values to 0 (was allowing negative phase drift)
- **`GainSmoother::new()`** — now clamps attack/release to 0.0–1.0 (was accepting any value)
- **FFI `nada_buffer_silence()`** — now returns null for `channels == 0` or `sample_rate == 0`
- **Oscillator triangle waveform** — now uses PolyBLEP-integrated leaky integrator for proper anti-aliasing (was using naive formula despite docs claiming PolyBLEP)
- **STFT `hop_size.max(1)`** dead code removed (error check already handled `hop_size == 0`)
- **`resample_linear()`** — early return for empty buffers and upper bound validation on target_rate

#### Performance
- **Graph processor** — output buffers reused across cycles (was allocating fresh `AudioBuffer::silence` per node per cycle)
- **STFT** — scratch `real`/`imag` buffers allocated once and reused across frames (was allocating per frame)
- **Noise reduction** — same scratch buffer reuse optimization
- **R128 K-weighting** — filters samples directly instead of cloning the entire `AudioBuffer` (avoids duplicating metadata + allocation overhead)
- **LevelMeter peak hold** — decay now scales by frame count (`powi(frames)`) for buffer-size-independent rate

### Added

#### Annotations
- `#[must_use]` on all public structs and pure functions across the crate (~50+ types), including `BiquadFilter::process_sample()`, `SvfFilter::process_sample()`
- `#[inline]` on hot-path functions: all DSP `process()` methods (BiquadFilter, Compressor, DeEsser, DelayLine, ModulatedDelay, EnvelopeLimiter, ParametricEq, GraphicEq, Reverb), `BiquadState::process`, `BiquadFilter::process_sample`, `Oscillator::sample`, `Envelope::tick`, `Lfo::tick`, `StereoPanner::process`, `GainSmoother::smooth`, SIMD dispatch functions, clock accessors, MIDI translate functions, meter store/load
- `AudioBuffer::silence()` now debug-asserts `channels > 0` and `sample_rate > 0`
- `// SAFETY:` comments on all unsafe blocks in NEON biquad_stereo (aarch64.rs)

#### Tracing
- `tracing::debug!` on all DSP effect constructors: BiquadFilter, SvfFilter, DelayLine, ModulatedDelay, StereoPanner, GainSmoother, GraphicEq, NoiseReducer (plus existing Compressor, Limiter, DeEsser, ParametricEq, Reverb)
- `tracing::debug!` on all analysis entry points (measure_r128, analyze_dynamics, stft, chromagram, detect_onsets)
- `tracing::warn!` on all error paths in conversion functions and FFI null returns
- `tracing::debug!` on noise_reduce, resample_sinc, Graph::compile

#### DSP
- **`ConvolutionReverb`** — FFT-based partitioned overlap-add convolution reverb engine. Works with any `&[f32]` impulse response. Supports mono/stereo, configurable block size, dry/wet mix, IR hot-swap
- **`NoiseReducer`** — stateful spectral noise reducer that reuses Hann window, FFT scratch, and magnitude buffers across calls (avoids 3 large allocations per call). `noise_reduce()` still available as convenience wrapper

#### Acoustics Integration (feature: `acoustics`)
- **`acoustics` module** — bridges [`goonj`](https://crates.io/crates/goonj) 1.1.1 acoustic simulation into dhvani's processing pipeline
- `convolution_from_ir()` / `convolution_from_dhvani_ir()` — create `ConvolutionReverb` from goonj impulse responses
- `convolve_multiband()` — frequency-dependent reverb from `MultibandIr` (8 octave bands)
- `process_fdn()` — FDN late reverb processing into `AudioBuffer` with mono-sum input
- `decode_bformat_stereo()` — decode 1st-order Ambisonics B-Format IR to stereo (W+Y / W-Y cardioid)
- `export_ir_wav()` — export IR as 16-bit PCM WAV
- `generate_ir()` — convenience wrapper for `generate_dhvani_ir` using `(f32, f32, f32)` tuples (no hisab dep needed)
- `acoustics::presets` — 6 curated room presets: studio, rehearsal room, concert hall, bathroom, cathedral, outdoor
- Re-exports: `AcousticRoom`, `AcousticMaterial`, `ImpulseResponse`, `MultibandIr`, `DhvaniIr`, `Fdn`, `FdnConfig`, `BFormatIr`, `HoaIr`, room acoustics analysis functions

#### Documentation
- Redundant explicit rustdoc link targets fixed in lib.rs

#### Synthesis Integration (feature: `synthesis`)
- **`synthesis` module** — re-exports from [`naad`](https://crates.io/crates/naad) 1.0.0: subtractive, FM, additive, wavetable, granular, physical modeling, drum, vocoder synthesis engines
- `render_to_buffer()` and `render_stereo_to_buffer()` — bridge functions from sample-based synthesis to dhvani `AudioBuffer`
- All naad core primitives available: oscillators, SVF/biquad filters, ADSR/multi-stage envelopes, LFO, noise generators, mod matrix, voice manager, parameter smoothing, tuning, panning, effects (chorus, flanger, phaser, distortion)

#### Voice Synthesis Integration (feature: `voice`)
- **`voice_synth` module** — re-exports from [`svara`](https://crates.io/crates/svara) 1.0.0: glottal source (Rosenberg/LF models), formant filtering, vocal tract modeling, phoneme inventory (~50 IPA phonemes), prosody contours, phoneme sequencing with coarticulation
- `render_sequence()` — render a `PhonemeSequence` to `AudioBuffer`
- `render_phoneme()` — render a single `Phoneme` to `AudioBuffer`
- `render_vocal_tract()` — low-level glottal source + vocal tract rendering
- Voice profiles: male, female, child with breathiness, vibrato, jitter, shimmer control

#### Tests
- 18 DSP reference accuracy tests (0.01 dB tolerance): biquad LP/HP/peaking/shelf/allpass/notch, limiter ceiling, compressor unity, panner constant power, coefficient DC/Nyquist gain, dB roundtrip
- 8 convolution reverb tests (identity, delay, stereo, long IR, reset, hot-swap)
- 9 acoustics integration tests (IR generation, FDN, multiband, B-Format decode, WAV export, room presets)
- 7 SVF peak/shelf correctness tests
- 5 PipeWire lifecycle tests (capture start/stop, output send, device enumeration, idempotent start)
- 4 MIDI accessor tests, 2 error Display tests
- 9 P(-1) hardening tests + 7 synthesis tests + 4 voice tests
- 90.02% line coverage via cargo-llvm-cov (597 unit + 22 doc = 619 total)

### Performance
- stereo_to_mono: 97µs → 54µs (−45%)
- mono_to_stereo: 97µs → 68µs (−30%)
- noise_gate: 17.5µs → 10.9µs (−38%)
- reverb: 1.14ms → 967µs (−15%)
- limiter: 689µs → 614µs (−11%)
- parametric_eq_10band: 2.96ms → 2.56ms (−14%)

### Tooling
- Dependencies updated (proptest 1.11, serde_spanned 1.1, cc 1.2.58, etc.)

## [0.22.4] — 2026-03-22

### Changed — BREAKING

#### API Encapsulation
- `AudioClock` fields (`position_samples`, `sample_rate`, `tempo_bpm`, `running`) are now private — use accessor methods `position_samples()`, `sample_rate()`, `tempo_bpm()`, `is_running()`, `set_tempo()`

### Added

#### Abaco Integration
- Added `abaco` 0.22.4 as dependency — shared DSP math crate for the AGNOS ecosystem
- `dsp::amplitude_to_db`, `db_to_amplitude`, `sanitize_sample` now re-exported from `abaco::dsp`
- Biquad filter design uses `abaco::dsp::{angular_frequency, db_gain_factor}`
- Compressor/limiter time constants use `abaco::dsp::time_constant`
- Oscillator uses `abaco::dsp::poly_blep`
- Panner uses `abaco::dsp::constant_power_pan`
- Crossfade uses `abaco::dsp::equal_power_crossfade`
- MIDI voice frequency uses `abaco::dsp::midi_to_freq`
- MIDI constants (`A4_FREQUENCY`, `A4_MIDI_NOTE`, `SEMITONES_PER_OCTAVE`) re-exported from abaco

#### DSP Consistency
- `set_sample_rate()` on all stateful DSP effects: BiquadFilter, Compressor, EnvelopeLimiter, ParametricEq, DeEsser, Reverb, ModulatedDelay, Envelope, Lfo
- `set_bypass()`/`is_bypassed()` on all dynamic effects: BiquadFilter, Compressor, EnvelopeLimiter, ParametricEq, DeEsser, Reverb, DelayLine, ModulatedDelay
- `ParametricEq::set_params()` — bulk band replacement
- `ModulatedDelay::set_params()` — runtime parameter updates
- `DelayLine::latency_frames()` and `ModulatedDelay::latency_frames()` — latency reporting for PDC
- `AudioClock::set_tempo()` — runtime BPM changes

#### Refactoring
- `dsp::soft_knee_gain()` — shared soft-knee gain computation used by Compressor and EnvelopeLimiter
- Removed duplicated `time_constant()` from Compressor and EnvelopeLimiter (now delegates to `abaco::dsp`)
- Removed inline `poly_blep()` from oscillator (now uses `abaco::dsp`)

#### Tooling
- `scripts/bench-history.sh` — benchmark runner with CSV history + 3-point Markdown tracking

#### Tests
- 15 new tests: bypass, set_sample_rate, soft_knee_gain, clock getters, latency_frames, set_params (431 total)

## [0.21.4] — 2026-03-21

### Changed — BREAKING

#### API Encapsulation
- `AudioBuffer` fields (`samples`, `channels`, `sample_rate`, `frames`) are now `pub(crate)` — use accessor methods `samples()`, `samples_mut()`, `channels()`, `sample_rate()`, `frames()` instead
- `Spectrum` fields are now private — use accessor methods `magnitudes()`, `magnitude_db()`, `freq_resolution()`, `sample_rate()`, `fft_size()`, `peak_frequency()`, `peak_magnitude_db()`
- `Chromagram.chroma` is now private — use `chroma()` accessor
- `Voice` fields are now `pub(crate)` — use accessor methods
- `MidiRoute` fields are now private — construct via `MidiRoute::new()`, use getters

#### Analysis Error Propagation
- `spectrum_fft()` now returns `Result<Spectrum, NadaError>` instead of default Spectrum on error
- `spectrum_dft()` now returns `Result<Spectrum, NadaError>`
- `compute_stft()` now returns `Result<Spectrogram, NadaError>`
- `measure_r128()` now returns `Result<R128Loudness, NadaError>`
- `chromagram()` now returns `Result<Chromagram, NadaError>`
- `detect_onsets()` now returns `Result<OnsetResult, NadaError>`

#### Constructor Validation
- `Compressor::new()`, `Reverb::new()`, `EnvelopeLimiter::new()`, `DeEsser::new()` now return `Result` — parameters are validated on construction

### Added

#### Format Conversion
- `SampleFormat::I24`, `SampleFormat::F64`, `SampleFormat::U8` variants
- `i24_to_f32()` / `f32_to_i24()` — 24-bit (i32-padded) conversion
- `i24_packed_to_f32()` / `f32_to_i24_packed()` — 24-bit packed 3-byte LE conversion
- `f64_to_f32()` / `f32_to_f64()` — double-precision conversion
- `u8_to_f32()` / `f32_to_u8()` — unsigned 8-bit PCM (centered at 128)

#### Dithering
- `buffer::dither::tpdf_dither()` — Triangular PDF dithering for bit-depth reduction
- `buffer::dither::noise_shaped_dither()` — first-order error feedback noise-shaped dithering

#### Buffer Utilities
- `buffer::ops::crossfade()` — linear and equal-power crossfade between two buffers
- `buffer::ops::fade_in()` / `fade_out()` — linear and exponential fade ramps
- `buffer::ops::normalize_to_lufs()` — normalize to target LUFS using EBU R128 measurement

#### Memory & Allocation
- `AudioBufferRef<'a>` — zero-copy read-only buffer view (borrows samples, no allocation)
- `BufferPool` — reusable buffer arena to reduce allocation pressure in RT paths
- `StftProcessor` — caches Hann window for repeated STFT computations
- `GraphProcessor` now uses Vec-indexed outputs (was HashMap) and pre-allocated input scratch

#### Parameter Validation
- `AdsrParams::validate()`, `ModulatedDelayParams::validate()`, `Oscillator::validate()`, `Lfo::validate()`
- Sample rate ceiling raised from 384 kHz to 768 kHz

#### Trait Derives
- `GraphicEq` now implements `Debug` and `Clone`

#### Robustness
- `dsp::sanitize_sample()` — NaN/Inf → 0.0 helper
- NaN guards added to reverb, delay, de-esser, limiter process paths
- `// SAFETY:` comments on all unsafe blocks in simd/x86.rs, simd/aarch64.rs, ffi.rs, meter/mod.rs

---

## [0.21.3] — 2026-03-21

### Changed

#### Analysis — BREAKING
- `DynamicsAnalysis` — all fields upgraded from scalar `f32` to per-channel `Vec<f32>`: `peak`, `peak_db`, `true_peak`, `true_peak_db`, `rms`, `rms_db`, `crest_factor_db`. Added `lufs: f32`, `frame_count: usize`, `channel_count: u32`. Convenience methods: `max_peak()`, `max_peak_db()`, `max_true_peak()`, `max_true_peak_db()`, `mean_rms()`, `mean_crest_factor_db()` for whole-buffer summaries
- `Spectrum` — added `magnitude_db: Vec<f32>`, `fft_size: usize`, `peak_frequency: f32`, `peak_magnitude_db: f32` fields. All constructed via internal `from_magnitudes()` which computes dB and peak fields automatically

### Added

#### Analysis
- `Spectrum::spectral_centroid()` — weighted mean frequency by magnitude (brightness indicator)
- `Spectrum::spectral_rolloff(threshold)` — frequency below which a given fraction of spectral energy sits (timbral shape descriptor)

#### Metering
- `LevelMeter` — block-accumulating audio level meter with peak, RMS, LUFS, and peak-hold tracking. Accumulates statistics across multiple `process()` calls and computes integrated LUFS using simplified EBU R128 gating (absolute gate at -70 LUFS, relative gate at mean-10 LU). Includes per-channel peak hold with configurable decay coefficient

---

## [0.20.5] — 2026-03-21

Yanked — superseded by 0.21.3 which includes the same features plus breaking API improvements.

---

## [0.20.4] — 2026-03-20

### Added

#### DSP
- `GainSmoother` — exponential moving average with configurable attack/release coefficients for smooth gain transitions. Prevents pumping in volume normalization workflows
- `GainSmootherParams` — serde-compatible parameters (default: attack 0.3, release 0.05)
- `GraphicEq` — 10-band ISO graphic equalizer (31 Hz–16 kHz) wrapping `ParametricEq` with per-band gain control
- `GraphicEqSettings` — settings with 9 named presets (rock, pop, jazz, classical, bass, treble, vocal, electronic, acoustic)
- `ISO_BANDS` constant — standard 10-band center frequencies

#### Analysis
- `suggest_gain(buf, target_rms) → f32` — per-buffer normalization gain suggestion with 0.1–10.0x clamping. Convenience for media player volume normalization

#### Crate Structure
- Feature flags for module-level compilation: `dsp`, `analysis`, `midi`, `graph` (all default-on)
- `analysis` feature implies `dsp` (R128 K-weighting needs biquad, dynamics needs dB conversion)
- `dsp::noise_reduction` gated behind `analysis` feature (needs FFT)
- Core always available: `buffer`, `capture`, `clock`, `ffi`, `error`
- Consumers can now select only what they need (e.g., `default-features = false, features = ["dsp", "simd"]`)

#### Documentation
- Comprehensive documentation audit and cleanup across all docs
- Updated roadmap: collapsed v0.21–v0.23 into 2 dense releases targeting v1.0
- Architecture overview updated with full module tree
- Migration guide updated with planned v0.21.3 breaking changes

### Fixed
- Sanskrit character: नाद (Nāda) → ध्वनि (Dhvani) in README and docs
- README Quick Start: replaced nonexistent `dsp::compress()` with `Compressor` struct
- README: `spectrum_dft` → `spectrum_fft` in examples
- Roadmap: marked already-completed items (oscillator, envelope, LFO, noise_reduction, waveform, anyhow removal, serde_json)
- Stale version references removed from capability table and roadmap

---

## [0.20.3] — 2026-03-20

### Added

#### Core
- `AudioBuffer` — f32 interleaved audio buffer with channels, sample_rate, frames
- `SampleFormat` (F32, I16, I32) and `Layout` (Interleaved, Planar) enums with `#[non_exhaustive]`
- `AudioClock` — sample-accurate transport with position, tempo, beats, PTS, seek
- `NadaError` enum with FormatMismatch, LengthMismatch, InvalidSampleRate, InvalidChannels, Dsp, Capture, InvalidParameter, Conversion variants

#### DSP
- `BiquadFilter` — 8 filter types (LP, HP, BP, notch, all-pass, peaking, shelf) using Bristow-Johnson cookbook
- `ParametricEq` — N-band biquad cascade with per-band enable/disable
- `Reverb` — Schroeder/Freeverb (4 combs + 2 allpasses, stereo decorrelation)
- `DelayLine` + `ModulatedDelay` — fixed and LFO-modulated for chorus/flanger
- `Compressor` — envelope follower with soft knee, attack/release, makeup gain
- `EnvelopeLimiter` — brick-wall limiter with instant attack, soft knee
- `DeEsser` — biquad sidechain sibilance detection with pre-allocated buffer
- `StereoPanner` — constant-power (sin/cos) panning law
- Stateless: noise gate, hard limiter, normalize, amplitude/dB conversion

#### Analysis
- Radix-2 Cooley-Tukey FFT (O(n log n)) + simple DFT for small windows
- STFT spectrograms with configurable window/hop size
- EBU R128 loudness (K-weighting, 400ms blocks, absolute + relative gating, LRA)
- `DynamicsAnalysis` — true peak (4x oversampled), crest factor, dynamic range
- `Chromagram` — 12 pitch classes mapped from FFT bins
- Onset detection via spectral flux with peak-picking
- Simplified LUFS and silence detection

#### MIDI
- MIDI 1.0: `NoteEvent`, `ControlChange`, `MidiEvent` enum, `MidiClip`
- MIDI 2.0 / UMP: `NoteOnV2`, `NoteOffV2`, `ControlChangeV2`, per-note expression, `UmpMessageType`
- Translation: velocity (7↔16 bit), CC (7↔32 bit), pitch bend (14↔32 bit) with roundtrip tests
- `VoiceManager` — polyphonic voice pool with 4 steal modes (Oldest, Quietest, Lowest, None)
- Routing: `VelocityCurve`, `MidiRoute`, `CcMapping`
- `MidiClip` operations: sorted insert, binary search range query, merge, transpose, quantize

#### SIMD
- SSE2 kernels (x86_64): mix, gain, clamp, peak, RMS, noise gate, i16/f32, weighted sum
- AVX2 kernels (x86_64): mix, gain, clamp, peak — runtime-detected
- NEON kernels (aarch64): mix, gain, clamp, peak, RMS, noise gate, weighted sum
- Platform dispatch module with scalar fallback

#### RT Infrastructure
- `PeakMeter` / `MeterBank` / `SharedMeterBank` — lock-free metering via AtomicU32
- `AudioNode` trait + `Graph` + `ExecutionPlan` + `GraphProcessor` (double-buffered swap)
- `RecordManager` / `LoopRecordManager` — ring-buffer recording with take splitting

#### Capture
- PipeWire capture/output (`PwCapture`, `PwOutput`, `enumerate_devices`)
- Device types, config structs, `CaptureEvent` hot-plug notifications

#### Format Conversion
- i16 ↔ f32, i32 ↔ f32 with clamping
- Interleaved ↔ planar
- Mono → stereo, stereo → mono
- 5.1 → stereo downmix (ITU-R BS.775)
- Sinc resampling (Blackman-Harris window, Draft/Good/Best quality)

#### Crate Quality
- FFI module — C-compatible `nada_buffer_*` API
- CONTRIBUTING.md, SECURITY.md, CODE_OF_CONDUCT.md, deny.toml
- Fuzz targets (mix, resample, DSP chain)
- CI: cargo-vet, cargo-semver-checks, test-minimal, fuzz, bench jobs
- 265+ tests, 7 benchmark suites, 94%+ line coverage
