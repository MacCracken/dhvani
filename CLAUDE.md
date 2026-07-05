# dhvani — Claude Code Instructions

> **Core rule**: this file is **preferences, process, and procedures** —
> durable rules that change rarely. Volatile state (current version,
> module line counts, port progress, test counts, consumers) lives in
> [`docs/development/state.md`](docs/development/state.md).
> Do not inline state here.

## Project Identity

**dhvani** — Cyrius port of a Rust project (23695 lines preserved at `rust-old/`).

- **Type**: Port (Rust → Cyrius)
- **License**: GPL-3.0-only
- **Language**: Cyrius (toolchain pinned in `cyrius.cyml [package].cyrius`)
- **Version**: `VERSION` at the project root is the source of truth — do not inline the number here
- **Standards**: [First-Party Standards](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-standards.md) · [First-Party Documentation](https://github.com/MacCracken/agnosticos/blob/main/docs/development/applications/first-party-documentation.md)

## Goal

dhvani (ध्वनि — "sound") **owns the core audio engine for AGNOS**: sample
buffers, format conversion (i16/i24/i32/f32/f64/u8), mixing, resampling, DSP
effects (biquad/SVF/EQ/dynamics/reverb/delay/…), MIDI (v1/v2, voice, routing),
spectral & temporal analysis (FFT/STFT/chroma/key/beat/onset/loudness), a
processing graph, metering, and ALSA device I/O via vani. Feature-gated layers wrap the
sibling synthesis engines — naad (synthesis), svara (voice), prani (creature),
garjan (environment), ghurni (mechanical), nidhi (sampler), goonj (acoustics).
Consumers: **shruti** (DAW), **jalwa** (media player), **aethersafta**
(compositor audio), **kiran** (game audio).

## Current State

> Volatile state lives in [`docs/development/state.md`](docs/development/state.md) —
> port progress, surface parity, in-flight work. Refreshed every release.

This file (`CLAUDE.md`) is durable rules.

## Scaffolding

Project was scaffolded with `cyrius port`. Original Rust at `rust-old/` is the reference oracle — do not modify it; cross-check the port against it.

## Quick Start

```sh
cyrius deps                              # resolve dependencies
cyrius build src/main.cyr build/dhvani    # compile
cyrius test                              # run tests/*.tcyr (CI does this)
cyrius test tests/hw/device.tcyr         # HARDWARE tests — local only, explicit path
```

**Hardware tests live in `tests/hw/`.** Bare `cyrius test` (what CI runs) only
auto-discovers top-level `tests/*.tcyr`, so `tests/hw/` is skipped in CI — those
suites need real audio hardware (`/dev/snd`, e.g. device enumeration) and would
fail headless. Run them locally by explicit path.

## Key Principles

- **Cross-check against `rust-old/`** — the port's correctness bar is "matches what Rust did". Diverge only with an ADR.
- **Correctness over cleverness** — if the Cyrius behavior diverges silently from Rust, the bugs win
- Test after every change, not after the feature is "done"
- ONE change at a time — never bundle unrelated changes
- Build with `cyrius build`, not raw `cat file | cc5` — the manifest auto-resolves deps
- Source files only need project includes — stdlib auto-resolves from `cyrius.cyml`
- `var buf[N]` = N **bytes**, not N entries

## Port Conventions (audio engine)

Full per-module ledger + established conventions live in
[`docs/development/port-audit.md`](docs/development/port-audit.md). The load-bearing ones:

- **Alloc-free hot paths.** Cyrius runs a **free-less bump allocator** — a
  per-sample or per-block heap allocation leaks unboundedly across a render.
  Every process loop must allocate zero bytes/sample; reuse scratch owned by the
  processor struct. (See naad 2.1.0: 4 hot paths fixed to 0 bytes/sample.)
- **f32 → f64** throughout (the hisab/ganita math is f64-only; widening is
  forced and improves precision). Loosen f32-oracle test tolerances where
  bit-exactness isn't meaningful.
- **enums → integer `var` codes**; **`Result`/`Option` → sentinel/error-code
  returns** (`ERR_*`, NaN, `-1`, null) — payloads via `lib/tagged.cyr`. No
  unwinding, no `panic`.
- **closures → fn-ptr** (`fnptr`/`callback`); **`Vec`/`SmallVec` → stdlib `vec`**;
  free fns get a `<module>_` prefix (one flat bundle namespace).
- **serde round-trip + Display-string tests dropped** (no serde; integer codes).
  All other `#[test]` blocks ported one-for-one into `tests/<mod>.tcyr`.
- **Never skip benchmarks** — hot-path `.bcyr` numbers are the proof a port
  didn't regress. Capture before claiming a win.
- **Parallel-porting concurrency**: every `cyrius build/test/deps` call
  re-resolves deps and races on `cyrius.lock` (concurrent runs corrupt it).
  Serialize toolchain calls behind a file lock:
  `flock <scratch>/dhvani-build.lock cyrius test …`.

## Rules (Hard Constraints)

- **Do not commit or push** — the user handles all git operations
- **Never use `gh` CLI** — use `curl` to the GitHub API if needed
- Do not modify `rust-old/` — it's the parity oracle
- Do not skip tests before claiming changes work
- Do not modify `lib/` files (vendored stdlib / dep symlinks)
- Do not hardcode toolchain versions in CI YAML — `cyrius = "X.Y.Z"` in `cyrius.cyml` is the source of truth

## Documentation

- [`docs/adr/`](docs/adr/) — Architecture Decision Records (*why X over Y?*)
- [`docs/architecture/`](docs/architecture/) — Non-obvious constraints
- [`docs/guides/`](docs/guides/) — Task-oriented how-tos
- [`docs/examples/`](docs/examples/) — Runnable examples
- [`docs/development/state.md`](docs/development/state.md) — Live state
- [`docs/development/roadmap.md`](docs/development/roadmap.md) — Milestones through 2.0.0 (then the 2.1.x device-I/O line)

