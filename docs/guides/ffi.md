# FFI Usage Guide

## Overview

Dhvani provides a C-compatible FFI (`src/ffi.rs`) for integration with C, Python, and other languages. The API uses opaque handles with create/free lifecycle management.

## Building the Shared Library

```bash
# Build as cdylib
cargo build --release --lib
# Output: target/release/libdhvani.so (Linux) / libdhvani.dylib (macOS)
```

Add to `Cargo.toml` if not already present:
```toml
[lib]
crate-type = ["lib", "cdylib"]
```

## C API

### Buffer Lifecycle

```c
#include <stdint.h>

// Opaque handle
typedef struct NadaBuffer NadaBuffer;

// Create a silent buffer
NadaBuffer* nada_buffer_silence(uint32_t channels, size_t frames, uint32_t sample_rate);

// Create from interleaved f32 samples
NadaBuffer* nada_buffer_from_interleaved(
    const float* samples, size_t len, uint32_t channels, uint32_t sample_rate
);

// Free a buffer (must call for every created buffer)
void nada_buffer_free(NadaBuffer* buf);

// Access raw sample pointer (read-only)
const float* nada_buffer_samples(const NadaBuffer* buf, size_t* out_len);

// Get sample rate
uint32_t nada_buffer_sample_rate(const NadaBuffer* buf);
```

### DSP Operations

```c
// Apply gain to all samples
void nada_buffer_apply_gain(NadaBuffer* buf, float gain);

// Clamp all samples to [-1.0, 1.0]
void nada_buffer_clamp(NadaBuffer* buf);

// Get RMS level
float nada_buffer_rms(const NadaBuffer* buf);

// Hard limiter
void nada_buffer_hard_limiter(NadaBuffer* buf, float ceiling);

// Noise gate
void nada_buffer_noise_gate(NadaBuffer* buf, float threshold);
```

### Memory String

```c
// Get JSON representation (caller must free with nada_free_string)
char* nada_buffer_json(const NadaBuffer* buf);
void nada_free_string(char* s);
```

## C Example

```c
#include <stdio.h>

int main() {
    // Create 1 second of silence, stereo, 44.1kHz
    NadaBuffer* buf = nada_buffer_silence(2, 44100, 44100);
    if (!buf) return 1;

    // Apply gain
    nada_buffer_apply_gain(buf, 0.5);

    // Read samples
    size_t len;
    const float* samples = nada_buffer_samples(buf, &len);
    printf("Buffer has %zu samples\n", len);

    // Cleanup
    nada_buffer_free(buf);
    return 0;
}
```

Compile:
```bash
gcc -o example example.c -L target/release -ldhvani
```

## Python (ctypes) Example

```python
import ctypes

lib = ctypes.CDLL("target/release/libdhvani.so")

# Define return types
lib.nada_buffer_silence.restype = ctypes.c_void_p
lib.nada_buffer_rms.restype = ctypes.c_float
lib.nada_buffer_samples.restype = ctypes.POINTER(ctypes.c_float)

# Create buffer
buf = lib.nada_buffer_silence(2, 44100, 44100)

# Apply gain
lib.nada_buffer_apply_gain(buf, ctypes.c_float(0.8))

# Get RMS
rms = lib.nada_buffer_rms(buf)
print(f"RMS: {rms}")

# Cleanup
lib.nada_buffer_free(buf)
```

## Safety Notes

- All FFI functions validate input pointers before dereferencing
- `NULL` pointers are returned on invalid parameters (zero channels, zero sample rate)
- Every `nada_buffer_*` create function requires a corresponding `nada_buffer_free`
- Every `nada_buffer_json` call requires a `nada_free_string`
- The `samples` pointer from `nada_buffer_samples` is valid only while the buffer lives
