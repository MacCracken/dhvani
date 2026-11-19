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

Since 2.3.2 the buffer kernels run on Cyrius's packed verbs: 2-lane `f64v_*`
everywhere, and 4-lane `f64v256_*` on x86 CPUs that report AVX2. The choice is made
at runtime from CPUID, so there is nothing to toggle (`simd.md`). Capture hot-path
`.bcyr` numbers (`cyrius bench`) before shipping to catch regressions.

## Real-time audio guidelines

### RT-safe (no allocation, no locks)
- `dhvani_buffer_apply_gain()`, `dhvani_buffer_clamp()`, `dhvani_buffer_peak()`, `dhvani_buffer_rms()`
- All DSP `process()` paths (each reuses pre-allocated per-instance state), and since
  2.3.3 their parameter setters (`set_params`, `set_band`, …), which rewrite the existing
  coefficients in place
- `dhvani_meter_peak_store()`, and the allocation-free readers `dhvani_meter_peak_left/right()`,
  `dhvani_meter_bank_left/right()` and the `*_into` forms (2.3.3). `dhvani_meter_peak_load()` /
  `bank_load()` / `read_all()` return a fresh vec, so poll with the readers instead.
- The graph `process()`, and a plan swap, which since 2.3.3 reuses the processor's buffers
  (a plan larger than any before allocates once)
- The RT ring `dhvani_player_push()` / `dhvani_player_drain()`, `dhvani_recorder_fill()`,
  and `dhvani_recorder_read()` (2.3.3: it decodes into a buffer the recorder owns and
  reuses, valid until the next read)
- The MIDI `_into` queries and transforms (`dhvani_midi_clip_events_in_range_into`,
  `dhvani_midiroute_filter_event_into`, `dhvani_miditx_*_into`, 2.3.3)

### Non-RT types (may allocate or block)
- `Graph::compile()`: the topological sort and the node-table snapshot allocate
- `dhvani_buffer_from_interleaved()` / `silence()` / `clone()` / `mix()` / `resample_*()`
- The allocating forms of the MIDI clip queries and translators (use `_into`)
- `dhvani_acoustics_convolve_multiband()`: builds a convolver per call (offline only;
  keep one from `dhvani_acoustics_convolution_from_multiband()` for per-block use)
- `dhvani_devices_list()`: enumerates endpoints via yukti (/dev/snd, /proc/asound, sysfs)

Under the free-less bump allocator these never reclaim their memory, so keep
them off the render loop entirely — allocate up front, reuse the RT ring
(`Player`/`Recorder`) on the hot path.

### Thread safety
- DSP effect structs hold per-instance mutable state: give each RT thread its own
- Pass a compiled plan to the render thread through the graph swap handle. The port has
  no locks or atomics (the oracle's `try_lock` / atomic loads became plain fields), so
  swap between cycles on the thread that calls `process()`
- `dhvani_playback_write` and the capture reads share one grow-only scratch per
  direction: one thread per direction

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
