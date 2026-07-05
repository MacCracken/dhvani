# ADR 002: SIMD via std::arch Intrinsics

## Status
**Superseded / blocked** (2026-07, Cyrius port) — the `std::arch` decision is
Rust-only; the Cyrius port ships **scalar kernels only**. See below. The
historical Rust-era decision is kept below as a record.

## Superseded / blocked (Cyrius port)

`std::arch` intrinsics, `#[target_feature]`, and `is_x86_feature_detected!` are
Rust-specific and have no Cyrius equivalent, so this decision does not carry over.
The Cyrius port ports **only the scalar fallback path** — the SSE2/AVX2 (`x86.rs`)
and NEON (`aarch64.rs`) arms are dropped as **platform-blocked** until Cyrius has
a SIMD story. The scalar kernels live in `src/simd.cyr` (f64 throughout,
alloc-free/in-place). This is the source of the known Rust-vs-Cyrius throughput
gap; a future Cyrius SIMD facility could restore the vectorized path. There are no
Cargo features in Cyrius, so the `simd` feature flag is gone (scalar is the only
path).

---

## Historical (Rust 1.x)

### Context
Rust's portable SIMD (`std::simd`) is not stabilized as of Rust 1.93. Options: `std::arch` intrinsics, third-party crates (`wide`, `packed_simd`), or wait for stabilization.

### Decision
Use **`std::arch` intrinsics** with `#[target_feature]` annotations and runtime AVX2 detection via `is_x86_feature_detected!`.

### Rationale
- `std::arch` is stable since Rust 1.27 — no nightly dependency
- Full control over instruction selection (SSE2, AVX2, NEON)
- Runtime detection allows single binary with best-available path
- No external dependency (zero supply chain risk)
- SSE2 is baseline on x86_64 (always available); NEON is baseline on aarch64

### Consequences
- Platform-specific code in `src/simd/x86.rs` and `src/simd/aarch64.rs`
- Scalar fallback for unsupported platforms
- `unsafe` blocks required — all annotated with `// SAFETY:` comments
- `simd` feature flag (default on) controls dispatch
