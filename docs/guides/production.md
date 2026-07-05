# Production Deployment Guide

## Build and packaging

There are no Cargo features. Build the engine with the cyrius toolchain:

```sh
cyrius deps                              # resolve/vendor deps into lib/
cyrius build src/main.cyr build/dhvani    # compile
cyrius distlib                           # (re)build dist/dhvani.cyr consumer bundle
```

`dist/dhvani.cyr` is the flat consumer surface. It **externalizes** abaco and
the synthesis siblings (naad/svara/…): a consumer includes the sibling bundles
it actually uses, in dependency order, alongside `dist/dhvani.cyr`; unused
feature references DCE-prune, so linking only what you need keeps the binary
minimal. The `playback` bridge IS in the bundle, but references vani through
functions only, so it DCE-prunes when you don't link vani — the core stays
device-agnostic. The `device` enumeration module is separate (not bundled). Link
the vani/yukti stack when you want device I/O (see below). Do not edit
`dist/dhvani.cyr`; regenerate it with `cyrius distlib`.

## SIMD

There is no SIMD in the Cyrius port — every kernel is scalar (Cyrius has no SIMD
intrinsics yet; see `simd.md`). This is the source of the Rust-vs-Cyrius
throughput gap. No CPU flags, no runtime feature detection, nothing to toggle.
Capture hot-path `.bcyr` numbers (`cyrius bench`) before shipping to catch
regressions.

## Real-time audio guidelines

### RT-safe (no allocation, no locks)
- `dhvani_buffer_apply_gain()`, `dhvani_buffer_clamp()`, `dhvani_buffer_peak()`, `dhvani_buffer_rms()`
- All DSP `process()` paths (each reuses pre-allocated per-instance state)
- `dhvani_meter_peak_store()` / `dhvani_meter_peak_load()` — atomic operations
- The graph `process()` — uses `try_lock()`, never blocks
- The RT ring `dhvani_player_push()` / `dhvani_player_drain()` and `dhvani_recorder_fill()`

### Non-RT types (may allocate or block)
- `Graph::compile()` — topological sort allocates
- `dhvani_buffer_from_interleaved()` — allocates a `vec`
- `dhvani_recorder_read()` — decodes the ring into a new AudioBuffer per call
- `dhvani_devices_list()` — enumerates endpoints via yukti (udevadm + sysfs)

Under the free-less bump allocator these never reclaim their memory, so keep
them off the render loop entirely — allocate up front, reuse the RT ring
(`Player`/`Recorder`) on the hot path.

### Thread safety
- DSP effect structs hold per-instance mutable state — give each RT thread its own
- Use the graph swap handle to pass a compiled plan from the UI thread to the RT thread (non-blocking `try_lock`)
- Metering uses atomic peak stores (`dhvani_meter_peak_store` / `_load`) across threads

## Buffer sizes

Typical configurations:
- **Low latency** (music production): 64-256 frames at 44100/48000 Hz
- **Standard** (media playback): 512-1024 frames
- **High throughput** (offline processing): 4096+ frames

Larger buffers amortize per-buffer overhead but increase latency.

## Device I/O (vani / ALSA)

The Rust dhvani did device I/O via PipeWire (an FFI backend). The Cyrius port
supersedes it (see `docs/decisions/004-pipewire-feature-gated.md`): device I/O
is now **FFI-free** via **vani** (dh(vani)'s sibling), which talks to raw
`/dev/snd` ALSA PCM through ioctls — no libpipewire, no libasound, no PipeWire
daemon. No `-dev` package on the build host, no runtime daemon.

Link the vani stack alongside `dist/dhvani.cyr` when you need device I/O; the
`device` module additionally pulls yukti for enumeration. The runtime
requirement is just permission to open `/dev/snd/*` (typically the `audio`
group). Blocking and RT-ring paths:

```cyr
# Open the default playback endpoint, 48 kHz stereo 16-bit (0 if none)
var dev = dhvani_playback_open_default_fmt(48000, 2, 16);

# Blocking write of one AudioBuffer
dhvani_playback_write(dev, buf);

# ...or the lock-free RT ring (see rt-safety.md)
var player = dhvani_player_new(65536, 1024, 2);
dhvani_player_push(player, buf);
dhvani_player_drain(player, dev);
```

Enumerate endpoints without hardcoding card/device numbers via
`dhvani_devices_list()` and the `dhvani_device_*` accessors.

## Memory usage

- Each `AudioBuffer` of 1 second stereo at 48kHz uses ~384 KB (48000 * 2 * 4 bytes)
- DSP effects pre-allocate internal state at construction time
- Reverb delay lines: ~140 KB at 44100 Hz (4 combs + 2 allpasses, stereo)
- MeterBank: 8 bytes per slot (two AtomicU32)
