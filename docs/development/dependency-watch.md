# Dependency Watch

Direct dependencies to monitor for updates, CVEs, and breaking changes.

## Core Dependencies

| Crate | Version | Role | Notes |
|-------|---------|------|-------|
| **abaco** | 1.1.0 | DSP math (amplitude/dB, poly_blep, panning, filters) | AGNOS crate — coordinate upgrades with consumers |
| **serde** | 1 | Serialization for all public types | Stable, low risk |
| **thiserror** | 2 | Error derive macros | MSRV 1.89 compatible |
| **tracing** | 0.1 | Structured logging | Stable facade, low risk |

## Optional Dependencies

| Crate | Version | Feature | Notes |
|-------|---------|---------|-------|
| **naad** | 1.0.0 | `synthesis` | AGNOS synthesis crate — coordinate with svara |
| **svara** | 1.1.0 | `voice` | AGNOS voice synthesis — depends on naad |
| **goonj** | 1.1.0 | `acoustics` | AGNOS acoustics — depends on hisab |
| **prani** | 1.1.0 | `creature` | AGNOS creature vocals |
| **garjan** | 1.0.0 | `environment` | AGNOS environmental sounds — depends on naad |
| **ghurni** | 1.0.0 | `mechanical` | AGNOS mechanical sounds — depends on naad |
| **nidhi** | 1.1.0 | `sampler` | AGNOS sample playback |
| **shabda** | 1.0.0 | `g2p` | AGNOS grapheme-to-phoneme |
| **rayon** | 1 | `parallel` | Stable, well-maintained |
| **pipewire** | 0.9 | `pipewire` | Rust bindings — track upstream PipeWire releases |

## Dev Dependencies

| Crate | Version | Notes |
|-------|---------|-------|
| **criterion** | 0.8 | Benchmarking — html_reports feature |
| **proptest** | 1 | Property-based testing |
| **serde_json** | 1 | Test serialization roundtrips |

## Security Monitoring

- `cargo audit` — run in CI, check weekly
- `cargo deny check` — advisories, bans, licenses, sources
- AGNOS crates are maintained in-house — prioritize coordinated upgrades

## Upgrade Policy

- **Patch versions**: Apply immediately if CI passes
- **Minor versions**: Review changelog, test, apply within a week
- **Major versions**: Plan migration, update CHANGELOG with breaking section
- **AGNOS crates**: Coordinate across the stack (abaco → naad/svara/goonj → dhvani → shruti/jalwa)
