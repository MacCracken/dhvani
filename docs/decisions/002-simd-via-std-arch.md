# ADR 002: SIMD via std::arch Intrinsics

## Status
Accepted

## Context
Rust's portable SIMD (`std::simd`) is not stabilized as of Rust 1.93. Options: `std::arch` intrinsics, third-party crates (`wide`, `packed_simd`), or wait for stabilization.

## Decision
Use **`std::arch` intrinsics** with `#[target_feature]` annotations and runtime AVX2 detection via `is_x86_feature_detected!`.

## Rationale
- `std::arch` is stable since Rust 1.27 — no nightly dependency
- Full control over instruction selection (SSE2, AVX2, NEON)
- Runtime detection allows single binary with best-available path
- No external dependency (zero supply chain risk)
- SSE2 is baseline on x86_64 (always available); NEON is baseline on aarch64

## Consequences
- Platform-specific code in `src/simd/x86.rs` and `src/simd/aarch64.rs`
- Scalar fallback for unsupported platforms
- `unsafe` blocks required — all annotated with `// SAFETY:` comments
- `simd` feature flag (default on) controls dispatch
