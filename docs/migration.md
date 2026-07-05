# Migration Guide

## Moving off the Rust version → Cyrius port (2.x)

dhvani is now the **Cyrius** audio engine — the completed port of the former Rust
library (which shipped through 1.1.0). The Rust source is frozen at `rust-old/`
as the parity oracle. Current version **2.1.2**. This guide covers migrating a
consumer from the Rust crate to the Cyrius bundle.

### Build / dependency model

There is no Cargo, no crates.io, no `[dependencies]`/features. Use the **cyrius**
toolchain:

```sh
cyrius deps                             # resolve/vendor deps into lib/
cyrius build src/main.cyr build/dhvani  # compile
cyrius test tests/<mod>.tcyr            # run a test suite (explicit path)
cyrius distlib                          # (re)build dist/dhvani.cyr consumer bundle
```

A consumer links `dist/dhvani.cyr` plus the sibling bundles for the features it
uses, **included in dependency order** — not wired as auto-resolved `[deps]`
(that mis-orders cross-bundle types and overflows the compiler's LEXID cap).

### Language idioms (Rust → Cyrius)

Consumer code changes shape, not just names:

- **f32 → f64** throughout — sample buffers and DSP math are f64.
- **enums → integer `var` codes** — e.g. `SampleFormat` variants become integer
  codes.
- **`Result`/`Option` → sentinel returns** — a negative error code, `NaN`, or `0`
  in place of `Err`/`None`; there is no `?`, no `panic`, no unwinding.
- **closures + generic traits → fn-ptr** — graph nodes take a process fn-ptr.
- **tuples → out-params**.
- **No serde** — integer codes and explicit accessors instead of derive.

```cyr
# Cyrius port
var ch = dhvani_buffer_channels(buf)
var sr = dhvani_buffer_sample_rate(buf)
var spec = dhvani_fft_spectrum(buf, 4096)   # sentinel return on error, no Result
```

### Device I/O: PipeWire → vani (ALSA)

The Rust version did device I/O via **PipeWire** (`capture/pw`, an unsafe-FFI
backend). The Cyrius port is **FFI-free** and does **not** use PipeWire. Device
I/O is now via **vani** ([github.com/MacCracken/vani](https://github.com/MacCracken/vani))
— raw `/dev/snd` ALSA PCM via ioctls, no libpipewire, no libasound.
See [ADR 004](decisions/004-pipewire-feature-gated.md).

dhvani's `playback` module (`src/playback.cyr`) bridges `AudioBuffer` ↔ vani PCM
(S16/S24/S32): blocking play/record, a lock-free/alloc-free RT ring **player**
(`dhvani_player_*`) and **recorder** (`dhvani_recorder_*`), plus device
enumeration and default-device open (`src/device.cyr`,
`dhvani_devices_list` / `dhvani_playback_open_default` / `dhvani_capture_open_default`).

- `enumerate_devices()` → `dhvani_devices_list()`
- PipeWire capture → `dhvani_capture_open_default()` + `dhvani_capture_read()`,
  or the RT recorder ring.

### Format coverage

`SampleFormat` covers i16/i24/i32/f32/f64/u8. The PCM device bridge handles the
little-endian **S16/S24/S32** interleaved formats.

## Blocked / non-goals (post-port)

These Rust features are intentionally not in the current Cyrius port:

- **`ffi`** — deliberate non-goal (Cyrius-native consumers, free-less bump
  allocator breaks the C ownership model).
- **`simd/x86` + `simd/aarch64` intrinsics** — scalar-f64 only pending a Cyrius
  SIMD story (the source of the throughput gap).
- **g2p** (← shabda) and **voice_synth/bhava_bridge** (← bhava) — dep-blocked;
  shabda/bhava are still Rust.

Consumers (shruti, jalwa, aethersafta, kiran) migrate up the stack post-port.
