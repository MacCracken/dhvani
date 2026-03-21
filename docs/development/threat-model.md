# Threat Model

## Attack surface

Nada is an audio processing library. Its attack surface is limited to:

1. **Audio data** — untrusted f32 sample buffers from external sources
2. **MIDI data** — untrusted event sequences (note numbers, velocities, CC values)
3. **PipeWire IPC** — inter-process communication with the PipeWire daemon
4. **FFI boundary** — C callers passing raw pointers

## Trust assumptions

| Boundary | Trust level | Rationale |
|----------|-------------|-----------|
| Audio samples (f32) | Untrusted | May contain NaN, Inf, out-of-range values |
| MIDI events | Untrusted | May have invalid note numbers, velocities |
| PipeWire daemon | Trusted | System service, same user |
| FFI pointers | Untrusted | C caller may pass null or invalid pointers |
| Cargo dependencies | Audited | cargo-audit + cargo-deny in CI |

## Mitigations

### Audio data
| Threat | Mitigation |
|--------|-----------|
| NaN/Inf in samples | DSP effects check `is_finite()`, replace with 0.0 |
| Extreme amplitudes | `clamp()` and `hard_limiter()` bound output |
| Buffer length mismatch | `mix()` validates channels/rate before processing |
| Zero channels/rate | `AudioBuffer::from_interleaved()` returns `Err` |

### SIMD operations
| Threat | Mitigation |
|--------|-----------|
| Buffer overflow | Remainder loops handle non-aligned tails |
| Unaligned memory | `_mm_loadu_ps` / `vld1q_f32` (unaligned loads) |
| Wrong CPU features | Runtime `is_x86_feature_detected!` for AVX2; SSE2/NEON are baseline |

### FFI
| Threat | Mitigation |
|--------|-----------|
| Null pointer | All `nada_buffer_*` functions check for null, return 0/null |
| Use after free | Opaque handle API — caller must follow ownership rules |
| Invalid parameters | `nada_buffer_from_interleaved` validates channels > 0, rate > 0 |

### PipeWire
| Threat | Mitigation |
|--------|-----------|
| Daemon unavailable | `enumerate_devices()` returns empty Vec, not panic |
| Malformed audio data | Byte-to-f32 conversion with bounds check |
| Channel disconnection | `try_recv()` returns None; `recv()` returns Err |

## Unsafe code inventory

| Location | Purpose | Safety comment |
|----------|---------|----------------|
| `src/simd/x86.rs` | SSE2/AVX2 intrinsics | `#[target_feature]` + unaligned loads |
| `src/simd/aarch64.rs` | NEON intrinsics | `#[target_feature]` + unaligned loads |
| `src/ffi.rs` | C FFI exports | Null checks on all inputs |
| `src/meter/mod.rs` | `unsafe impl Sync` for MeterBank | Only contains AtomicU32 |
