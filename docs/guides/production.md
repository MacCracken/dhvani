# Production Deployment Guide

## Feature selection

For minimal binary size and no system dependencies:
```toml
nada = { version = "0.20", default-features = false }
```

For full Linux audio support:
```toml
nada = { version = "0.20", features = ["full"] }
```

## SIMD

SIMD is enabled by default (`simd` feature). On x86_64, SSE2 is always used; AVX2 is detected at runtime. No special CPU flags needed.

To verify SIMD is active, check benchmark results — SIMD-accelerated operations (mix, gain, clamp, peak) should show 3-8x speedup vs `--no-default-features`.

## Real-time audio guidelines

### RT-safe types (no allocation, no locks)
- `AudioBuffer::apply_gain()`, `.clamp()`, `.peak()`, `.rms()`
- All DSP `process()` methods (except `DeEsser` which reuses a pre-allocated buffer)
- `PeakMeter::store()` / `.load()` — atomic operations
- `GraphProcessor::process()` — uses `try_lock()`, never blocks

### Non-RT types (may allocate or block)
- `Graph::compile()` — topological sort allocates
- `RecordManager::finish()` — joins background thread
- `AudioBuffer::from_interleaved()` — allocates Vec
- `enumerate_devices()` — PipeWire round-trip

### Thread safety
- DSP effect structs (`Compressor`, `Reverb`, `EQ`, etc.) are `Send` but NOT `Sync`
- Use `GraphSwapHandle` to pass plans from UI thread to RT thread
- Use `SharedMeterBank` (Arc) for metering across threads

## Buffer sizes

Typical configurations:
- **Low latency** (music production): 64-256 frames at 44100/48000 Hz
- **Standard** (media playback): 512-1024 frames
- **High throughput** (offline processing): 4096+ frames

Larger buffers amortize per-buffer overhead but increase latency.

## PipeWire capture

Requires `libpipewire-0.3-dev` on the build host. At runtime, PipeWire daemon must be running.

```bash
# Ubuntu/Debian
sudo apt-get install libpipewire-0.3-dev

# Arch
sudo pacman -S pipewire
```

## Memory usage

- Each `AudioBuffer` of 1 second stereo at 48kHz uses ~384 KB (48000 * 2 * 4 bytes)
- DSP effects pre-allocate internal state at construction time
- Reverb delay lines: ~140 KB at 44100 Hz (4 combs + 2 allpasses, stereo)
- MeterBank: 8 bytes per slot (two AtomicU32)
