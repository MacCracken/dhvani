# Real-Time Safety Guide

## Overview

Audio processing runs in real-time (RT) threads where allocation, locking, and syscalls cause audio glitches. This guide documents which dhvani types are RT-safe and which are not.

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

## Non-RT Types (may allocate or block)

| Type | Module | When it allocates |
|------|--------|-------------------|
| `AudioBuffer::from_interleaved()` | `buffer` | Constructor allocates `Vec<f32>` |
| `AudioBuffer::silence()` | `buffer` | Constructor allocates |
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

```rust
// Non-RT thread: build and compile graph
let mut graph = Graph::new();
graph.add_node(src_id, Box::new(source));
graph.add_node(fx_id, Box::new(effect));
graph.connect(src_id, fx_id);
let plan = graph.compile().unwrap(); // allocates

// Swap handle — pass to RT thread
let handle = processor.swap_handle();
handle.swap(plan); // takes lock briefly

// RT thread: process() is allocation-free
loop {
    if let Some(output) = processor.process() {
        // output is a reference to internal buffer — zero-copy
        send_to_hardware(output);
    }
}
```

## Guidelines

1. **Pre-allocate everything** before entering the RT loop
2. **Never call** `from_interleaved()`, `silence()`, `mix()`, or `resample_*()` from the RT thread
3. **`AutomationLane::render_fast()`** is RT-safe if the lane is pre-built
4. **Parameter changes** (`set_frequency()`, `set_params()`, etc.) are RT-safe — they update coefficients inline
5. **Graph plan swaps** use `try_lock()` — the RT thread never blocks
6. **`SvfFilter`** is preferred over `BiquadFilter` for modulated parameters (no coefficient discontinuities)
