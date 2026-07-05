# FFI Usage Guide

## Status: FFI is a non-goal in the Cyrius port

The Rust dhvani shipped a C-compatible FFI (`src/ffi.rs`) with opaque handles
and create/free lifecycle management, for calling from C, Python, and other
languages. **The Cyrius port does not provide a C-ABI.** FFI is a deliberate
post-port non-goal, for two reasons:

- **No C boundary in Cyrius.** There is no `extern "C"` / `#[no_mangle]` /
  raw-pointer handle story; the consumers (shruti, jalwa, aethersafta, kiran)
  are all Cyrius-native and link the in-language bundle directly.
- **The free-less bump allocator breaks `*_free`.** The Rust FFI's
  `nada_buffer_free` / `nada_free_string` contract has no meaning under a
  bump allocator that never frees, so the create/free handle model can't be
  ported as-is.

See `docs/development/roadmap.md` (deferred layers) — if a C boundary is ever
needed it would be re-architected as an in-language handle table, not the old
`extern "C"` surface.

## Consuming dhvani from Cyrius (the FFI replacement)

There is no shared library to `dlopen`. A Cyrius consumer links the dist bundle
`dist/dhvani.cyr` (rebuild with `cyrius distlib`) and calls the flat `dhvani_*`
surface directly — the same operations the Rust FFI wrapped, minus the handle
lifecycle:

```cyr
# Create 1 second of stereo silence at 44.1 kHz (0 sentinel on bad args —
# no NULL handle, no free)
var buf = dhvani_buffer_silence(2, 44100, 44100);

# Apply gain, clamp, read level — all in place, alloc-free
dhvani_buffer_apply_gain(buf, 0.5);
dhvani_buffer_clamp(buf);
var rms = dhvani_buffer_rms(buf);

# Access the raw sample vec + its length
var samples = dhvani_buffer_samples(buf);
var len = dhvani_buffer_total_samples(buf);
```

Notes for anyone porting off the old C API:

- Constructors return a **0 sentinel** on invalid parameters (zero channels,
  zero/oversized sample rate, mis-sized interleaved length) instead of a `NULL`
  handle — check `== 0`.
- There is **no `_free`**: the bump allocator reclaims nothing during a render,
  so buffers are not individually released. Size the workload accordingly (see
  `rt-safety.md`).
- Values that were error-typed in Rust come back as sentinels (negative code /
  NaN / 0), not `Result` — there is no unwinding.

## Cross-language (C / Python) consumers

Not supported. Wrap dhvani from another Cyrius unit; there is no `.so`/`.dylib`
to bind via `gcc -ldhvani` or `ctypes.CDLL`.
