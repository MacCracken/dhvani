# ADR 002: SIMD via std::arch Intrinsics

## Status
**Superseded** (2026-07, Cyrius port). The `std::arch` decision is Rust-only. From 2.0.0
to 2.3.1 the port shipped **scalar kernels only**. Since 2.3.2 the accelerated arms are
back, on Cyrius's packed verbs. The historical Rust-era decision is kept below as a
record.

## Cyrius port

`std::arch` intrinsics, `#[target_feature]` and `is_x86_feature_detected!` are
Rust-specific, so this decision does not carry over as written. Its *shape* does.

- **2.0.0 – 2.3.1: blocked.** Cyrius had no vector facility, so only the scalar fallback
  path was ported. The SSE2/AVX2 (`x86.rs`) and NEON (`aarch64.rs`) arms were dropped as
  platform-blocked. That was the Rust-vs-Cyrius throughput gap.
- **2.3.2: restored on the compiler's flat-array packed verbs.** `f64v_*` gives 2 f64 lanes
  (SSE2 on x86, NEON on aarch64, per-lane loops on cx) and `f64v256_*` gives 4 (AVX2). They
  play the part `std::arch` played:
  - **runtime detection** is the stdlib's `simd_has_avx2()` CPUID probe, cached, in place of
    `is_x86_feature_detected!("avx2")`;
  - **the scalar fallback** is kept beside each accelerated kernel as `<k>_scalar`, as in the
    oracle, and is the parity oracle;
  - **no `unsafe`**, since the verbs are builtins. Their one hazard is that they have no
    scalar tail. Every call goes through row helpers that finish an odd last element in
    scalar code (see `docs/guides/simd.md`).
- There are still no Cargo features, so the `simd` flag is gone. The accelerated path is
  the only build, and its oracle is the `_scalar` form.

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
