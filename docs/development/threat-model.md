# Threat Model

## Attack surface

Dhvani is a Cyrius audio-processing engine. Its attack surface is limited to:

1. **Audio data** — untrusted f64 sample buffers from external sources
2. **MIDI data** — untrusted event sequences (note numbers, velocities, CC values)
3. **ALSA device I/O** — PCM exchange with the kernel sound layer (`/dev/snd`) via vani

There is **no FFI boundary**: dhvani is Cyrius-native with no C-ABI exports and no
C callers. Device I/O uses vani's raw ioctl ALSA backend, not libpipewire/libasound.

## Trust assumptions

| Boundary | Trust level | Rationale |
|----------|-------------|-----------|
| Audio samples (f64) | Untrusted | May contain NaN, Inf, out-of-range values |
| MIDI events | Untrusted | May have invalid note numbers, velocities |
| Kernel sound layer (`/dev/snd`) | Trusted | System device, same user, accessed via vani ioctls |
| Vendored/`[deps]` bundles | Audited | AGNOS bundles maintained in-house; `lib/` committed, `cyrius.lock` pins abaco |

## Mitigations

### Audio data
| Threat | Mitigation |
|--------|-----------|
| NaN/Inf in samples | DSP effects check finiteness (`dhvani_is_finite`), sanitize to 0.0 |
| Extreme amplitudes | `dhvani_dsp_hard_limiter(buf, ceiling)` bounds output |
| Buffer length mismatch | `dhvani_buffer_mix` validates channels/rate before processing |
| Zero channels/rate | `dhvani_buffer_from_interleaved` returns 0 (sentinel), not a handle |

### Numerics / memory model
| Threat | Mitigation |
|--------|-----------|
| Buffer overrun | Explicit length checks; block loops handle non-aligned tails |
| Hot-path allocation | Free-less bump allocator ⇒ process loops are alloc-free; RT player/recorder reuse struct-owned scratch (0 bytes/block) |
| No SIMD widening bugs | Scalar-only DSP for now (no x86/aarch64 intrinsics ported — deliberate non-goal pending a Cyrius SIMD story) |

### Device I/O (ALSA via vani)
| Threat | Mitigation |
|--------|-----------|
| Device unavailable | `dhvani_devices_list` (over yukti) returns an empty list, no panic |
| Malformed PCM data | Byte ↔ f64 conversion (S16/S24/S32) with bounds checks |
| RT ring under/overrun | Lock-free/alloc-free ring; `dhvani_player_*` / `dhvani_recorder_*` over vani's ring drain, sentinel returns (no unwinding) |

## Error model

Cyrius has **no panic/unwinding** and no `Result`/`Option`: fallible calls return
sentinels — negative error codes, NaN, or `0`/null handles — that callers must
check. There is no exception path an attacker can trigger to abort a render.

## Unsafe / sensitive surface inventory

| Location | Purpose | Safety note |
|----------|---------|-------------|
| `src/simd.cyr` | SIMD shim | Scalar fallback only — no intrinsics/unsafe paths ported |
| `src/playback.cyr` | vani PCM bridge (play/record + RT ring) | Alloc-free struct-owned scratch; sentinel returns |
| `src/device.cyr` | Device enumeration / default-open over yukti | Empty-list on discovery failure |
| `lib/vani.cyr` (vendored) | Raw `/dev/snd` ALSA ioctls | Kernel PCM boundary; committed + auditable |
