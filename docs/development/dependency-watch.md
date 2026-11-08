# Dependency Watch

Direct dependencies to monitor for updates, CVEs, and breaking changes.

dhvani is a Cyrius engine. **As of 2.2.3 every bundle below is a real `[deps.X]`
entry** in `cyrius.cyml` (`git` + `tag`; no committed `path` since 2.2.6), resolved by `cyrius deps` into
`lib/` and hash-locked in `cyrius.lock` — 18 declared deps, 77 locked files, and
the closure is **100% tag-pinned**: 19 commit pins, one per declared dep plus bayan, which
shravan 2.9.x declares. The `include "lib/<x>.cyr"`
lines still govern **compile order**; `[deps]` governs **vendoring**. Bump a
version by editing its `tag` and re-running `cyrius deps` — never by hand-copying
a bundle into `lib/`, which the next `cyrius build/test` silently overwrites.

⚠ **A green `cyrius test` does not clear a bump.** One flat namespace means a
deleted upstream symbol rebinds instead of erroring — see the Upgrade Policy below.

## Core Dependencies

| Bundle | Version | Role | Notes |
|--------|---------|------|-------|
| **abaco** | 2.7.0 | DSP math (amplitude/dB, poly_blep, panning, filters) | AGNOS bundle — coordinate upgrades with consumers. **Held at 2.7.0:** 2.8.0 collides with naad (see *Known collisions*) |

## Synthesis-Stack Siblings (vendored in `lib/`, included in dep order)

| Bundle | Version | Layer | Notes |
|--------|---------|-------|-------|
| **naad** | 2.3.3 | `synthesis` | AGNOS synthesis — ⚠ 2.2.0 renamed `FILTER_*` → `NAAD_FILTER_*`; naad/svara/prani/ghurni/garjan must move TOGETHER |
| **svara** | 3.6.3 | `voice` | AGNOS voice synthesis — depends on naad; 3.5.4 fixes a crash at `sample_rate <= 1200 Hz` |
| **goonj** | 2.3.1 | `acoustics` | AGNOS acoustics — depends on hisab |
| **prani** | 2.1.1 | `creature` | AGNOS creature vocals |
| **garjan** | 2.6.3 | `environment` | AGNOS environmental sounds — depends on naad |
| **ghurni** | 2.7.2 | `mechanical` | AGNOS mechanical sounds — depends on naad |
| **nidhi** | 2.2.1 | `sampler` | AGNOS sample playback — needs shravan (WAV codec); render path alloc-free; `n_engine_note_on` is `#must_use` |
| **sakshi** | 2.5.10 | (support) | Logging used across the stack — stdlib fold (see below) |
| **hisab** | 3.4.1 | (support) | HVec3 math for goonj — 3.x since dhvani 2.2.7; 3.4.0 retired 27 public names, none of which dhvani calls; dhvani itself uses only `hvec3_new` |
| **shravan** | 2.9.3 | (support) | WAV codec, needed by nidhi |

Include order (dependency order — also the `[deps]` declaration order):
`sakshi → hisab → goonj → naad → shravan → svara → nidhi → ghurni → garjan → prani`,
then `varna → shabdakosh → shabda`, then `patra → yukti → vani`.

**sankoch 2.8.9** is declared explicitly too: it arrives transitively under
shravan/nidhi, and left implicit `cyrius deps` pins it to a bare commit hash
instead of a tag. dhvani makes no `sankoch_*` calls (it DCE-prunes).

## g2p Stack (2.2.0, vendored in `lib/`)

| Bundle | Version | Role | Notes |
|--------|---------|------|-------|
| **shabda** | 3.1.1 | G2P engine | 3.0.2+ uses real Unicode tables → pulls the `unicode` stdlib leaf; SSML depth capped at 256 |
| **shabdakosh** | 3.1.1 | pronunciation dictionary | string→phoneme `map_*` (the `hashmap` leaf) |
| **varna** | 2.5.2 | phoneme inventories | 2.4.1 made pre-built inventories shared singletons — treat as READ-ONLY, use `phoneme_clone` to mutate |

Include order: `… svara → varna → shabdakosh → shabda`.

## Device-I/O Bundles (2.1.x, vendored in `lib/`)

| Bundle | Version | Role | Notes |
|--------|---------|------|-------|
| **vani** | 1.2.10 | ALSA PCM via raw `/dev/snd` ioctls | dh(vani)'s sibling — no libpipewire/libasound, no FFI; drives `playback` (S16/S24/S32). 1.2.x caches ring scratch on the handle (no per-call `alloc()`) |
| **yukti** | 2.3.18 | PCM device discovery | backs `device` enumeration / default-open; 2.3.8 reads `/proc/asound` via patra |
| **patra** | 1.16.6 | procfs/text reads | pulled in by yukti 2.3.8 — must precede `lib/yukti.cyr` in the include order |

## Stdlib folds — track the toolchain snapshot for these five

**sakshi, sankoch, patra, yukti and vani also ship inside the Cyrius stdlib snapshot**
(`~/.cyrius/versions/<pin>/lib/`), so set each `tag` to the version that snapshot
carries. At cyrius 6.7.18 that is sakshi 2.5.10, sankoch 2.8.9, patra 1.16.6, yukti
2.3.18, vani 1.2.10, each also the newest tag.

- **sakshi, patra, yukti:** `cyrius deps` keeps the snapshot's copy over the dep
  (`refusing to overwrite stdlib leaf`), so a different tag only makes the manifest
  wrong about what `lib/` holds.
- **sankoch, vani:** the tag's copy IS vendored, and a mismatch prints a shadowed-stdlib
  warning (`vani 1.2.5 (pinned: 1.2.7), …` at dhvani 2.2.6).
- **A newer tag can pin the same toolchain and still be the wrong choice.** At 6.7.9, sakshi
  2.5.9 / 2.5.10 and yukti 2.3.17 / 2.3.18 pinned it too, but the snapshot and the batch
  used 2.5.8 and 2.3.16.
- **bayan** is a sixth such copy (1.11.1). shravan 2.9.x declares it, so it takes the lock's
  19th pin. It stays out of `cyrius.cyml` because declaring it only adds a `refusing to
  overwrite stdlib leaf` warning.

On a toolchain bump, read the new snapshot's versions first and move these five tags to
match.

## Known flat-namespace collisions between deps

| Symbol | Defined in | Effect | Status |
|--------|------------|--------|--------|
| `power_spectrum` | abaco 2.8.0 (4 args) · naad 2.3.3 (1 arg) | Since 6.7.x cycc **refuses** a duplicate fn whose arity disagrees, so the unit does not compile. | **Blocks abaco 2.8.0.** dhvani holds 2.7.0 and uses neither. abaco is the math library and keeps the name, so the hold lifts when naad gives it up. **Filed 2026-11-08** in naad: `docs/development/issues/2026-11-08-power-spectrum-clashes-with-abaco-2-8-0.md`. |
| `_fft_pow2` | abaco 2.8.0 · shravan 2.9.3 | Same arity, so only a warning and the last definition wins. They differ at `n = 1` (abaco 1, shravan 0). | Not reachable while abaco is held. abaco keeps the name, so shravan's is the one to rename. With abaco first, shravan's currently wins. |
| `DH_SIZE` | goonj (`DhvaniLayout`, **88**) · yukti (`DeviceHealthField`, **24**) | yukti is included after goonj, so every dhvani unit compiles `DH_SIZE = 24`. goonj's `generate_dhvani_ir` (reached by `dhvani_acoustics_generate_ir`) then does `alloc(24)` and writes the rt60 array at offsets 24..88 — a 64-byte heap overrun. | **Open, pre-existing** (goonj 2.0.5 / yukti 2.3.11 carried the same pair; still present at goonj 2.3.1 / yukti 2.3.18). Fix is an upstream rename, then a tag bump here. Measured 24 bytes reserved. **Filed 2026-11-08** in goonj and yukti: `docs/development/issues/2026-11-08-dh-size-collides-*.md`. |
| `detect_format` | sankoch · shravan | shravan's (last) wins over sankoch's inside sankoch too. | Pre-existing; dhvani makes no `sankoch_*` calls. |
| `DcBlocker_*` accessors | garjan · prani | prani's win; both structs are `x_prev; y_prev; r;` (3 × 8 bytes), so the offsets agree. | Benign. |

The compiler prints the warning rows as `duplicate symbol` / `duplicate fn` on every
build; a NEW one after a bump is a finding, not noise.

## Blocked (not yet ported to Cyrius — still Rust)

| Bundle | Feature | Notes |
|--------|---------|-------|
| **bhava** | `bhava-voice` | emotion/personality engine — port bhava first. **The last unported-dep holdout.** |

> ✅ **shabda is ported** (3.0.0 landed 2026-07-06; `g2p` shipped in dhvani 2.2.0).

## Security Monitoring

- AGNOS bundles are maintained in-house — prioritize coordinated upgrades.
- Vendored `lib/` bundles are committed and auditable — re-vendor on upgrade and
  review the diff; `cyrius.lock` pins the resolved `[deps]` set (abaco).
- No third-party crate supply chain: dhvani is Cyrius-native, no Cargo/crates.io.

## Upgrade Policy

- **Patch versions**: Apply immediately if `cyrius test` passes.
- ⚠ **A green `cyrius test` is NOT sufficient on its own.** These are flat
  concatenated bundles in one namespace, so a *deleted* symbol can silently
  rebind to a same-named cycc builtin or another bundle's copy and still compile.
  dhvani 2.2.2 hit exactly this: abaco renamed `f64_round` → `f64_round_half_away`,
  the three call sites rebound to the ties-to-even **builtin**, every test stayed
  green, and the rounding quietly stopped matching `rust-old`. On any bump, diff
  the bundle's exported symbols against the previous version and grep dhvani for
  each removed name.
- **Resolve from an empty `lib/` in a scratch copy at every bump.** A tracked file the resolver
  stops vendoring is left in place, unrefreshed, and still shadows the toolchain's copy:
  `lib/thread*.cyr` sat at the 6.6.10 snapshot through 2.3.0. Diff the clean `lib/` against yours.
- **abaco has priority in a name clash.** It is the ecosystem's math library: abaco keeps the
  name and the other bundle yields. dhvani holds abaco back only until that happens.
- **Regenerate `cyrius.lock` after a tag bump.** `cyrius deps` (and `--relock`)
  keep each dep's OLD commit pin beside the new one, so an in-place resolve leaves
  two pins per bumped dep. Move the lock aside, re-run `cyrius deps`, check the
  file-hash lines are unchanged and `cyrius deps --verify` passes (2.2.7).
- **Minor versions**: Review changelog, test, apply within a week.
- **Major versions**: Plan migration, update CHANGELOG with breaking section.
- **AGNOS bundles**: Coordinate across the stack (abaco → naad/svara/goonj → dhvani → shruti/jalwa).
