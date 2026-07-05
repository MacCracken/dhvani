# ADR 004: PipeWire as Feature-Gated Optional Backend

## Status
**Superseded** (2026-07, Cyrius port) — replaced by **vani** (ALSA PCM). See below.

## Superseded by vani (2.1.x)

The Cyrius port does not use PipeWire. The Rust `capture/pw` backend is a
PipeWire/`spa` unsafe-FFI client, and the Cyrius port is FFI-free (a free-less
bump allocator that also breaks the C ownership model), so `capture/pw` was
platform-blocked and never ported — deferred here "until Cyrius has an
audio-device story."

That story now exists: **[vani](https://github.com/MacCracken/vani)** — dh(vani)'s
sibling — owns raw `/dev/snd` PCM playback/capture via ioctls, with **no
libpipewire, no libasound, no FFI**. dhvani's `playback` module (`src/playback.cyr`,
2.1.x) bridges the `AudioBuffer` to vani's interleaved PCM (S16/S24/S32), with a
blocking path, a lock-free/alloc-free RT ring player and recorder, and device
enumeration + default-device open (`src/device.cyr`). So:

- Device I/O is **first-party via vani/ALSA**, not an FFI system dependency.
- PipeWire is no longer a dhvani concern. A PipeWire backend could return later as
  an *additional* vani-level transport, but it is not on the dhvani roadmap.
- `capture/mod` + `capture/record` (config + WAV/ring recorder) ported as-is; the
  `pipewire` Cargo feature is gone (there are no Cargo features in Cyrius).

The historical Rust-era decision is kept below as a record.

---

## Historical (Rust 1.x)

### Context
Audio I/O requires platform-specific backends. PipeWire is the modern Linux audio
standard but adds a system dependency (`libpipewire-0.3-dev`).

### Decision
PipeWire support was behind the `pipewire` Cargo feature flag (not default). The
`capture` module's types (`AudioDevice`, `CaptureConfig`, `CaptureEvent`) were
always available; only `PwCapture`, `PwOutput`, and `enumerate_devices()` required
the feature.

### Rationale
- Core audio math (buffers, DSP, analysis, MIDI) works everywhere — no system deps
- macOS and Windows users were not forced to install PipeWire headers
- CI could test without PipeWire on macOS (`--no-default-features`)
- Feature-gated code compiled to nothing when disabled

### Consequences
- Linux users had to install `libpipewire-0.3-dev` to use capture features
- Cross-platform audio I/O required additional backends (CoreAudio, WASAPI)
