# Dependency Watch

Direct dependencies to monitor for updates, CVEs, and breaking changes.

dhvani is a Cyrius engine. **As of 2.2.3 every bundle below is a real `[deps.X]`
entry** in `cyrius.cyml` (`git` + `path` + `tag`), resolved by `cyrius deps` into
`lib/` and hash-locked in `cyrius.lock` — 18 declared deps, 74 locked files, and
the closure is **100% tag-pinned** (no commit-pins). The `include "lib/<x>.cyr"`
lines still govern **compile order**; `[deps]` governs **vendoring**. Bump a
version by editing its `tag` and re-running `cyrius deps` — never by hand-copying
a bundle into `lib/`, which the next `cyrius build/test` silently overwrites.

⚠ **A green `cyrius test` does not clear a bump.** One flat namespace means a
deleted upstream symbol rebinds instead of erroring — see the Upgrade Policy below.

## Core Dependencies

| Bundle | Version | Role | Notes |
|--------|---------|------|-------|
| **abaco** | 2.4.5 | DSP math (amplitude/dB, poly_blep, panning, filters) | AGNOS bundle — the one `[deps]`-wired dep; coordinate upgrades with consumers |

## Synthesis-Stack Siblings (vendored in `lib/`, included in dep order)

| Bundle | Version | Layer | Notes |
|--------|---------|-------|-------|
| **naad** | 2.2.2 | `synthesis` | AGNOS synthesis — ⚠ 2.2.0 renamed `FILTER_*` → `NAAD_FILTER_*`; naad/svara/prani/ghurni/garjan must move TOGETHER |
| **svara** | 3.5.4 | `voice` | AGNOS voice synthesis — depends on naad; 3.5.4 fixes a crash at `sample_rate <= 1200 Hz` |
| **goonj** | 2.0.4 | `acoustics` | AGNOS acoustics — depends on hisab |
| **prani** | 2.0.12 | `creature` | AGNOS creature vocals |
| **garjan** | 2.5.1 | `environment` | AGNOS environmental sounds — depends on naad |
| **ghurni** | 2.6.0 | `mechanical` | AGNOS mechanical sounds — depends on naad |
| **nidhi** | 2.1.1 | `sampler` | AGNOS sample playback — needs shravan (WAV codec); render path alloc-free; `n_engine_note_on` is `#must_use` |
| **sakshi** | 2.4.12 | (support) | Logging used across the stack |
| **hisab** | 2.11.2 | (support) | HVec3 math for goonj |
| **shravan** | 2.8.0 | (support) | WAV codec, needed by nidhi |

Include order (dependency order — also the `[deps]` declaration order):
`sakshi → hisab → goonj → naad → shravan → svara → nidhi → ghurni → garjan → prani`,
then `varna → shabdakosh → shabda`, then `patra → yukti → vani`.

**sankoch 2.7.10** is declared explicitly too: it arrives transitively under
shravan/nidhi, and left implicit `cyrius deps` pins it to a bare commit hash
instead of a tag. dhvani makes no `sankoch_*` calls (it DCE-prunes).

## g2p Stack (2.2.0, vendored in `lib/`)

| Bundle | Version | Role | Notes |
|--------|---------|------|-------|
| **shabda** | 3.0.4 | G2P engine | 3.0.2+ uses real Unicode tables → pulls the `unicode` stdlib leaf; SSML depth capped at 256 |
| **shabdakosh** | 3.0.6 | pronunciation dictionary | string→phoneme `map_*` (the `hashmap` leaf) |
| **varna** | 2.4.1 | phoneme inventories | 2.4.1 made pre-built inventories shared singletons — treat as READ-ONLY, use `phoneme_clone` to mutate |

Include order: `… svara → varna → shabdakosh → shabda`.

## Device-I/O Bundles (2.1.x, vendored in `lib/`)

| Bundle | Version | Role | Notes |
|--------|---------|------|-------|
| **vani** | 1.2.2 | ALSA PCM via raw `/dev/snd` ioctls | dh(vani)'s sibling — no libpipewire/libasound, no FFI; drives `playback` (S16/S24/S32). 1.2.x caches ring scratch on the handle (no per-call `alloc()`) |
| **yukti** | 2.3.8 | PCM device discovery | backs `device` enumeration / default-open; 2.3.8 reads `/proc/asound` via patra |
| **patra** | 1.13.11 | procfs/text reads | pulled in by yukti 2.3.8 — must precede `lib/yukti.cyr` in the include order |

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
- **Minor versions**: Review changelog, test, apply within a week.
- **Major versions**: Plan migration, update CHANGELOG with breaking section.
- **AGNOS bundles**: Coordinate across the stack (abaco → naad/svara/goonj → dhvani → shruti/jalwa).
