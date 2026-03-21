# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.20.x | Yes |
| < 0.20 | No |

## Reporting a Vulnerability

If you discover a security vulnerability in nada:

1. **Do not** open a public GitHub issue
2. Use [GitHub Security Advisories](https://github.com/MacCracken/nada/security/advisories/new) to report privately
3. Include: description, steps to reproduce, impact assessment
4. **Response SLA**: acknowledgement within 72 hours, fix within 14 days for critical issues
5. We will coordinate disclosure timing with you

## Scope

Nada is an audio processing library. Security-relevant areas include:

| Area | Risk | Mitigation |
|------|------|-----------|
| SIMD kernels | Buffer overflow in remainder loops | Unaligned loads, scalar tail handling |
| `unsafe` code | Memory safety in FFI and intrinsics | All blocks have `// SAFETY:` comments |
| Integer overflow | Frame count / sample rate calculations | Validated at construction time |
| NaN/Inf propagation | Garbage output from malformed input | DSP effects check `is_finite()`, clamp output |
| PipeWire capture | Device access, buffer handling | Feature-gated, daemon assumed trusted |
| FFI boundary | Null/invalid pointer from C callers | All functions check for null |

## Security Practices

- All `unsafe` blocks annotated with `// SAFETY:` explaining the invariants
- CI runs `cargo audit` (dependency vulnerability scanning)
- CI runs `cargo deny` (license and supply chain validation)
- CI runs `cargo vet` (supply chain verification)
- Fuzz targets test mix, resample, and DSP chains with random input
- Property-based testing with `proptest` for edge case discovery
- No panicking paths in production code (assertions replaced with graceful returns)

## Threat Model

See [docs/development/threat-model.md](docs/development/threat-model.md) for the full threat model including attack surface analysis and mitigations table.
