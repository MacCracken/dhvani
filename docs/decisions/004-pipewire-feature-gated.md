# ADR 004: PipeWire as Feature-Gated Optional Backend

## Status
Accepted

## Context
Audio I/O requires platform-specific backends. PipeWire is the modern Linux audio standard but adds a system dependency (`libpipewire-0.3-dev`).

## Decision
PipeWire support is behind the `pipewire` Cargo feature flag (not default). The `capture` module's types (`AudioDevice`, `CaptureConfig`, `CaptureEvent`) are always available; only `PwCapture`, `PwOutput`, and `enumerate_devices()` require the feature.

## Rationale
- Core audio math (buffers, DSP, analysis, MIDI) works everywhere — no system dependencies
- macOS and Windows users are not forced to install PipeWire headers
- CI can test without PipeWire on macOS (`--no-default-features`)
- Feature-gated code compiles to nothing when disabled — zero binary size impact

## Consequences
- Users on Linux must install `libpipewire-0.3-dev` to use capture features
- Cross-platform audio I/O requires additional backends (CoreAudio, WASAPI — post-v1.0)
- `CaptureConfig` and `OutputConfig` are always available for configuration, even without the backend
