# Security Policy

## Supported versions

| Version | Supported |
|---------|-----------|
| 0.22.x | Yes |
| < 0.22 | No |

## Reporting a vulnerability

If you discover a security vulnerability in nada, please report it responsibly:

1. **Do not** open a public GitHub issue
2. Email security concerns to the maintainers via the repository contact
3. Include a description of the vulnerability and steps to reproduce
4. Allow reasonable time for a fix before public disclosure

## Scope

Nada is an audio processing library. Security-relevant areas include:

- **Buffer overflows** in SIMD kernel remainder loops
- **Unsafe code** in `src/simd/` (x86 and aarch64 intrinsics)
- **Integer overflow** in sample rate / frame count calculations
- **Denial of service** via pathological inputs (extreme buffer sizes, zero denominators)
- **PipeWire capture** — device access, buffer handling (when `pipewire` feature is enabled)

## Practices

- All `unsafe` blocks have `// SAFETY:` comments
- SIMD kernels have scalar remainder loops for non-aligned tails
- CI runs `cargo audit` for dependency vulnerability scanning
- CI runs `cargo deny` for license and supply chain validation
- Property-based testing with `proptest` for edge case discovery
