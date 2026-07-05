# Real-Time Safety Guide

## Overview

Audio processing runs in real-time (RT) threads where allocation, locking, and syscalls cause audio glitches. This guide documents which dhvani operations are RT-safe and which are not.

Cyrius runs a **free-less bump allocator**: an allocation is never reclaimed, so a per-sample or per-block `alloc()` in a render loop leaks unboundedly and eventually exhausts the heap. "Not RT-safe" here therefore means *worse* than in the Rust original — the hot path must allocate **zero bytes per sample/block**, reusing scratch owned by the processor struct. The type names below use the `dhvani_*` bundle surface (`dist/dhvani.cyr`).

## RT-Safe Types (no alloc, no lock, no syscall in hot path)

| Type | Module | Notes |
|------|--------|-------|
| `AudioBuffer` | `buffer` | Pre-allocated. `apply_gain()`, `clamp()`, `peak()`, `rms()` are allocation-free |
| `AudioClock` | `clock` | Pure arithmetic — `advance()`, `position_ms()`, `beat_position()` |
| `BiquadFilter` | `dsp::biquad` | `process()` and `process_sample()` — no allocation, fixed state |
| `SvfFilter` | `dsp::svf` | `process()` — allocation-free, safe for modulation |
| `Compressor` | `dsp::compressor` | `process()` — fixed state per channel |
| `EnvelopeLimiter` | `dsp::limiter` | `process()` — fixed state |
| `DelayLine` | `dsp::delay` | `process()` — pre-allocated ring buffer |
| `Reverb` | `dsp::reverb` | `process()` — pre-allocated delay lines |
| `StereoPanner` | `dsp::pan` | `process()` — stateless |
| `Envelope` | `dsp::envelope` | `tick()` — pure arithmetic |
| `Lfo` | `dsp::lfo` | `tick()` — pure arithmetic |
| `Oscillator` | `dsp::oscillator` | `sample()` — pure arithmetic |
| `GainSmoother` | `dsp::gain_smoother` | `next()` — pure arithmetic |
| `GraphProcessor` | `graph` | `process()` / `process_parallel()` — plan swap is `try_lock` (non-blocking) |
| `LevelMeter` | `meter` | `process()` — fixed state |
| `Player` | `playback` | `dhvani_player_push()` / `dhvani_player_drain()` — packs into a reusable scratch, writes to a lock-free vani ring, zero bytes/block |
| `Recorder` | `playback` | `dhvani_recorder_fill()` — reads the device into a lock-free vani ring, zero bytes/block on the fill side |

## Non-RT Types (may allocate or block)

| Type | Module | When it allocates |
|------|--------|-------------------|
| `dhvani_buffer_from_interleaved()` | `buffer` | Constructor allocates a `vec` |
| `dhvani_buffer_silence()` | `buffer` | Constructor allocates |
| `dhvani_recorder_read()` | `playback` | Decodes the ring into a **new** AudioBuffer per call — the read side allocates; drain into pre-sized storage off the RT path |
| `mix()` | `buffer` | Creates new output buffer |
| `resample_linear()` / `resample_sinc()` | `buffer::resample` | Allocates output buffer |
| `Graph::compile()` | `graph` | Builds `ExecutionPlan` with `HashMap` allocation |
| `GraphSwapHandle::swap()` | `graph` | Takes `Mutex` lock (blocks briefly) |
| `ParametricEq::new()` | `dsp::eq` | Allocates band vector |
| `spectrum_fft()` | `analysis::fft` | Allocates work buffers |
| `measure_r128()` | `analysis::loudness` | Allocates K-weighted copy |
| `RoutingMatrix::apply()` | `dsp::routing` | Allocates output buffer |
| `AutomationLane::add()` | `dsp::automation` | May reallocate breakpoint vector |

## RT-Thread Pattern

The device sink is a vani ALSA PCM (see `production.md`); the RT loop feeds it
through a lock-free ring **Player** whose scratch and ring are pre-allocated
off the hot path:

```cyr
# Setup (off the RT path): pre-allocate the ring + reusable scratch once.
# ring_bytes = ring capacity, max_frames = largest block, 2 channels.
var player = dhvani_player_new(65536, 1024, 2);   # 16-bit PCM by default
var dev = dhvani_playback_open_default_fmt(48000, 2, 16);   # 0 if no device

# RT loop: push producer blocks, then drain to the device.
# dhvani_player_push packs into the reusable scratch and writes the vani ring —
# zero bytes allocated per block. It returns bytes written (< block when the
# ring is full), so pace / retry the remainder next cycle.
while (running) {
    var block = produce_block();            # a pre-sized AudioBuffer you own
    dhvani_player_push(player, block);
    dhvani_player_drain(player, dev);       # vani_play_from_ring, no alloc
}
```

For a graph render, compile the plan off the RT path (`compile()` allocates)
and hand it over with the non-blocking swap handle; the RT `process()` is
allocation-free and returns a reference to the processor's internal buffer.

## Guidelines

1. **Pre-allocate everything** before entering the RT loop
2. **Never call** `dhvani_buffer_from_interleaved()`, `dhvani_buffer_silence()`, `mix()`, `resample_*()`, or `dhvani_recorder_read()` from the RT thread — they allocate, and the free-less allocator never reclaims it
3. **`AutomationLane::render_fast()`** is RT-safe if the lane is pre-built
4. **Parameter changes** (`set_frequency()`, `set_params()`, etc.) are RT-safe — they update coefficients inline
5. **Graph plan swaps** use `try_lock()` — the RT thread never blocks
6. **`SvfFilter`** is preferred over `BiquadFilter` for modulated parameters (no coefficient discontinuities)
