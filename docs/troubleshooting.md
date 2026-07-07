# Troubleshooting

## Build errors

### Toolchain pin drift
```
error: cyrius toolchain 6.4.x does not match pinned 6.4.12
```
The toolchain is pinned in `cyrius.cyml [package].cyrius` (currently `6.4.12`) —
that manifest is the source of truth, not a CI YAML or an env override. Install
or select the pinned toolchain rather than editing the pin:
```bash
cyrius --version
```

### Benign warnings (safe to ignore)
The following warnings are expected and do **not** indicate a broken build:
- **Duplicate `ERR_INVALID`** — the same sentinel error code is defined by more
  than one included bundle; the flat namespace reports the shadow. Harmless.
- **Undefined `bayan_*` / `map_*` / `http_*` symbols DCE-pruned** — abaco's and
  the siblings' json/http/net helpers are unreachable from dhvani's code and get
  dead-code-eliminated. The "undefined, pruned" note is expected.
- **Toolchain pin drift** — see above; a mismatch note against the `cyrius.cyml`
  pin is informational.

### LEXID cap overflow
```
error: identifier table overflow (16384 LEXID cap)
```
Linking **all 10 siblings + `dist/dhvani.cyr` in one compilation unit** overflows
the compiler's 16384-entry LEXID identifier table (force-including `bayan` is the
usual trigger). This is the unrealistic "every feature at once" case — link only
the sibling bundles for the features a consumer actually uses (per-feature linking
is well under the cap). Include siblings **in dependency order**, not as `[deps]`.

### Concurrent `cyrius` calls corrupt `cyrius.lock`
Every `cyrius build/test/deps` re-resolves deps and races on `cyrius.lock`.
Serialize toolchain calls behind a file lock:
```bash
flock <scratch>/dhvani-build.lock cyrius test tests/buffer.tcyr
```

## Runtime issues

### Device I/O: `dhvani_devices_list()` returns empty
Device I/O runs over **vani** (raw `/dev/snd` ALSA PCM via ioctls — no PipeWire,
no libasound, no FFI). Enumeration is via yukti (`src/device.cyr`).
- Verify the sound subsystem exposes PCM nodes: `ls /dev/snd/`
- Check your user is in the `audio` group: `groups`
- List cards directly: `cat /proc/asound/cards`

### Capture produces silence
- Check the target card/device index matches an active capture device
- Verify the PCM format matches (dhvani bridges S16/S24/S32 little-endian PCM)
- Confirm the default-capture open picked a real device
  (`dhvani_capture_open_default`)

### DSP output contains NaN or Infinity
- Check input samples for NaN/Inf before processing
- Ensure compressor ratio > 1.0
- Check that sample rates are non-zero
- The `dhvani_dsp_hard_limiter()` function clamps output — use as a safety net

### FFT returns all zeros
- Input may be silence — check the buffer peak is nonzero
- Window size must be a power of 2 for `dhvani_fft_spectrum()` (it rounds down
  automatically)
- For very short buffers, use `dhvani_analysis_spectrum_dft()` which handles any
  size

## Performance issues

### Audio glitches/dropouts
- Increase the ring/buffer size on the RT player/recorder
- Avoid allocation in the audio path — the free-less bump allocator leaks any
  per-block allocation
- Use the RT ring **player**/**recorder** (`dhvani_player_*` / `dhvani_recorder_*`)
  for real-time-safe I/O
- Check CPU usage with `htop` during processing

### Benchmarks slower than expected
- Run the hot-path benches: `cyrius test tests/hotpath.bcyr`
- Close other CPU-intensive applications
- Check thermal throttling: `sensors`
- Note the port is **scalar-f64 only** (no SIMD intrinsics yet) — see
  [`performance.md`](performance.md) and
  [`benchmarks-rust-v-cyrius.md`](benchmarks-rust-v-cyrius.md) for the expected
  Rust-vs-Cyrius throughput gap.
