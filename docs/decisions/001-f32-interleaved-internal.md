# ADR 001: f32 Interleaved as Internal Format

## Status
Accepted

## Context
Audio engines must choose an internal sample format. Options include f32, f64, i16, i32, and various layouts (interleaved vs planar).

## Decision
All internal processing uses **f32 interleaved**. Format conversion (i16, i32, planar) happens at boundaries.

## Rationale
- **f32** is the standard for real-time audio DSP — sufficient precision for mixing/effects, native to SIMD (4x f32 per SSE2 register)
- **Interleaved** simplifies buffer management — one contiguous allocation per buffer, no per-channel pointer tracking
- **f64** used only for coefficient computation (biquad) and accumulation (RMS, FFT) where precision matters
- Conversion functions (`i16_to_f32`, `interleaved_to_planar`) handle boundary cases explicitly

## Consequences
- All DSP effects assume f32 interleaved input — no format negotiation needed
- Users must convert at input/output boundaries (the `buffer::convert` module provides this)
- SIMD kernels only need f32 variants
