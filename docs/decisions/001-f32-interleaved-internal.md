# ADR 001: f32 Interleaved as Internal Format

## Status
**Superseded** (2026-07, Cyrius port) — internal format widened to **f64** (still interleaved). See below. The historical Rust-era decision is kept below as a record.

## Superseded by f64 (Cyrius port)

The Cyrius port widened the internal sample format from **f32 to f64**. The
hisab/ganita math the port builds on is f64-only, so the widening was forced —
and it improves precision. `AudioBuffer.samples` is now a `vec` of f64 (was
`Vec<f32>`), and the scalar DSP/conversion kernels (`src/simd.cyr`) operate on
f64 samples throughout (`f64_add`/`f64_mul`/… over the 8-byte slots). The
**interleaved** layout decision below is unchanged — buffers remain one
contiguous interleaved allocation, with planar conversion at boundaries. The
`buffer::convert` boundary functions (`i16_to_f32`, `interleaved_to_planar`, …)
were ported as-is; they now target f64 internally.

---

## Historical (Rust 1.x)

### Context
Audio engines must choose an internal sample format. Options include f32, f64, i16, i32, and various layouts (interleaved vs planar).

### Decision
All internal processing uses **f32 interleaved**. Format conversion (i16, i32, planar) happens at boundaries.

### Rationale
- **f32** is the standard for real-time audio DSP — sufficient precision for mixing/effects, native to SIMD (4x f32 per SSE2 register)
- **Interleaved** simplifies buffer management — one contiguous allocation per buffer, no per-channel pointer tracking
- **f64** used only for coefficient computation (biquad) and accumulation (RMS, FFT) where precision matters
- Conversion functions (`i16_to_f32`, `interleaved_to_planar`) handle boundary cases explicitly

### Consequences
- All DSP effects assume f32 interleaved input — no format negotiation needed
- Users must convert at input/output boundaries (the `buffer::convert` module provides this)
- SIMD kernels only need f32 variants
