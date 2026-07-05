# Testing Guide

## Running tests

```sh
# All tests — auto-discovers tests/*.tcyr
cyrius test

# A specific suite
cyrius test tests/dsp.tcyr
cyrius test tests/biquad.tcyr
cyrius test tests/midi.tcyr
```

Every `#[test]` block from the Rust source was ported one-for-one into
`tests/<mod>.tcyr` (the serde round-trip and Display-string tests were dropped —
there is no serde, and values are integer codes). Cross-check behavior against
`rust-old/` — the port's correctness bar is "matches what Rust did".

> **Concurrency**: every `cyrius test`/`build`/`deps` call re-resolves deps and
> races on `cyrius.lock`. Serialize toolchain calls behind a file lock:
> `flock <scratch>/dhvani-build.lock cyrius test …`.

## Test categories

| Category | Location | What it tests |
|----------|----------|---------------|
| Unit tests | `tests/<mod>.tcyr` | Individual functions, edge cases, error paths |
| Integration | `tests/integration.tcyr`, `tests/integration_advanced.tcyr` | Multi-module workflows (DSP chains, format pipelines) |
| Bundle tests | `tests/bundle.tcyr`, `tests/bundle_synth.tcyr` | The `dist/dhvani.cyr` consumer surface |
| Property tests | `tests/proptest.tcyr` | Randomized input robustness |
| Reference/parity | `tests/dsp_reference.tcyr`, `tests/dhvani.fcyr` | Parity against `rust-old/` oracle values |
| Benchmarks | `tests/*.bcyr` | Performance regression detection |
| Hardware | `tests/hw/*.tcyr` | Device I/O against real `/dev/snd` |

## Hardware tests (local only)

`tests/hw/` (e.g. `tests/hw/device.tcyr`) exercises the vani ALSA device I/O
path against actual `/dev/snd` PCM. These are **local-only** — they need real
hardware and are not run in CI. Run them by hand on a machine with a sound card:

```sh
cyrius test tests/hw/device.tcyr
```

The pure format bridges (PCM pack/unpack) are unit-tested without hardware in
the CI suites; only the device glue is gated to `tests/hw/`.

## Benchmarks

```sh
cyrius bench                             # run tests/*.bcyr
cyrius test tests/hotpath.bcyr           # per-sample hot-path inner loops
```

`tests/hotpath.bcyr` and `tests/bench_compare.bcyr` are the hot-path proof a
port didn't regress; `tests/bench_compare.bcyr` is the matched-size Rust-vs-
Cyrius comparison (see `docs/benchmarks-rust-v-cyrius.md`). Capture numbers
before claiming a win.

## CI

CI runs `cyrius test` (the `tests/*.tcyr` suites, excluding `tests/hw/`) on
push/PR, plus the non-regression benchmarks. The toolchain version is pinned by
`cyrius = "X.Y.Z"` in `cyrius.cyml` — not hardcoded in CI YAML.
