# Getting started with dhvani

## Build

```sh
cyrius deps                              # resolve dependencies
cyrius build src/main.cyr build/dhvani    # compile
cyrius test                              # run tests/*.tcyr
```

## Layout

- `src/main.cyr` — entry point. Top-level `var r = main(); syscall(SYS_EXIT, r);`.
- `tests/` — test suite (`.tcyr` files, auto-discovered by `cyrius test`).
- `rust-old/` — original Rust source preserved for parity checks. Do not modify; it's the reference oracle.

## Adding a feature

1. Edit `src/main.cyr` (or add a new module and `include` it).
2. Cross-check parity against `rust-old/`.
3. Add a test case to `tests/dhvani.tcyr`.
4. Run `cyrius test`.
5. Bump `VERSION` and add a CHANGELOG entry before tagging.

## Usage example

The dist bundle (`dist/dhvani.cyr`, rebuilt with `cyrius distlib`) exposes a
flat `dhvani_*` surface. A minimal buffer + DSP pass:

```cyr
# 1 second of stereo silence at 48 kHz
var buf = dhvani_buffer_silence(2, 48000, 48000);

# apply -6 dB of gain, then clamp to [-1, 1]
dhvani_buffer_apply_gain(buf, 0.5);
dhvani_buffer_clamp(buf);

# read back levels
var peak = dhvani_buffer_peak(buf);
var rms  = dhvani_buffer_rms(buf);
```

Wrapping an existing interleaved f64 block (out-param style, no `Result`):

```cyr
var buf = dhvani_buffer_from_interleaved(samples, 2, 48000);
if (buf == 0) { return -1; }   # 0 sentinel on invalid channels/rate/length
```

See [`../adr/template.md`](../adr/template.md) when a non-trivial design choice deserves an ADR.
