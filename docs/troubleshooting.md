# Troubleshooting

## Build errors

### `pipewire` feature fails to compile
```
error: could not find system library 'libpipewire-0.3'
```
Install PipeWire development headers:
```bash
# Ubuntu/Debian
sudo apt-get install libpipewire-0.3-dev

# Arch
sudo pacman -S pipewire

# Fedora
sudo dnf install pipewire-devel
```

Or build without PipeWire:
```bash
cargo build --no-default-features
```

### MSRV errors
Nada requires Rust 1.89+. Check your toolchain:
```bash
rustc --version
rustup update stable
```

## Runtime issues

### PipeWire: `enumerate_devices()` returns empty
- Verify PipeWire is running: `systemctl --user status pipewire`
- Check for audio nodes: `pw-cli list-objects | grep Audio`
- Ensure `libpipewire-0.3.so` is loadable: `ldd target/debug/nada | grep pipewire`

### PipeWire: capture produces silence
- Check the target device ID matches an active source
- Verify the audio format matches (nada uses F32LE)
- Monitor PipeWire graph: `pw-top`

### DSP output contains NaN or Infinity
- Check input samples for NaN/Inf before processing
- Ensure compressor ratio > 1.0
- Check that sample rates are non-zero
- The `hard_limiter()` function clamps output — use as a safety net

### FFT returns all zeros
- Input may be silence — check `buf.peak() > 0`
- Window size must be a power of 2 for `spectrum_fft()` (it rounds down automatically)
- For very short buffers, use `spectrum_dft()` which handles any size

### SIMD not active
- Verify the `simd` feature is enabled (it's default)
- On x86_64, SSE2 is always used; AVX2 requires CPU support
- Check with: `cargo test simd` — SIMD tests should pass

## Performance issues

### Audio glitches/dropouts
- Increase buffer size (`CaptureConfig::buffer_frames`)
- Avoid allocation in the audio callback
- Use `GraphProcessor` instead of manual DSP chains
- Check CPU usage with `htop` during processing

### Benchmarks slower than expected
- Ensure running in release mode: `cargo bench`
- Close other CPU-intensive applications
- Check thermal throttling: `sensors`
- AVX2 benchmarks require CPU support — check `lscpu | grep avx2`
