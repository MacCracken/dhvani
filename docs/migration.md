# Migration Guide

## v0.20.3 (current)

This is the initial release. No migration needed.

## Planned breaking changes for v0.21.3

The following changes are planned and will require code updates:

### AudioBuffer field encapsulation

Fields will become private. Update direct field access to use accessors:

```rust
// Before (v0.20.3)
let ch = buf.channels;
let sr = buf.sample_rate;
let data = &buf.samples;

// After (v0.21.3)
let ch = buf.channels();
let sr = buf.sample_rate();
let data = buf.samples();
```

### Deprecated API removal

The following deprecated items will be removed:

- `EqBand` struct — use `EqBandConfig` with `ParametricEq`
- `apply_eq_band()` function — use `ParametricEq::process()`
- `compress()` free function — use `Compressor` struct

### anyhow dependency removal

`NadaError::Other(anyhow::Error)` will be replaced with `NadaError::Other(Box<dyn std::error::Error + Send + Sync>)`. If you match on this variant, update the pattern.
