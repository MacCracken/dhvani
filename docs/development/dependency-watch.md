# Dependency Watch

Direct dependencies to monitor for updates, CVEs, and breaking changes.

dhvani is a Cyrius engine: `[deps.abaco]` is the one auto-resolved bundle;
the synthesis siblings + device-I/O bundles are **vendored into `lib/`** (committed)
and `include`d in dependency order, not wired as `[deps]` (see
[`port-audit.md`](port-audit.md) Wave F). "Version" below is the tag/bundle version
tracked, not a crates.io release.

## Core Dependencies

| Bundle | Version | Role | Notes |
|--------|---------|------|-------|
| **abaco** | 2.3.2 | DSP math (amplitude/dB, poly_blep, panning, filters) | AGNOS bundle — the one `[deps]`-wired dep; coordinate upgrades with consumers |

## Synthesis-Stack Siblings (vendored in `lib/`, included in dep order)

| Bundle | Version | Layer | Notes |
|--------|---------|-------|-------|
| **naad** | 2.1.1 | `synthesis` | AGNOS synthesis — coordinate with svara |
| **svara** | 3.0.1 | `voice` | AGNOS voice synthesis — depends on naad |
| **goonj** | 2.0.0 | `acoustics` | AGNOS acoustics — depends on hisab |
| **prani** | 2.0.1 | `creature` | AGNOS creature vocals |
| **garjan** | 2.0.0 | `environment` | AGNOS environmental sounds — depends on naad |
| **ghurni** | 2.0.0 | `mechanical` | AGNOS mechanical sounds — depends on naad |
| **nidhi** | 2.0.0 | `sampler` | AGNOS sample playback — needs shravan (WAV codec) |
| **sakshi** | 2.4.4 | (support) | Logging used across the stack |
| **hisab** | 2.6.7 | (support) | HVec3 math for goonj |
| **shravan** | 2.x | (support) | WAV codec, needed by nidhi |

Include order (dependency order):
`sakshi → hisab → goonj → naad → shravan → svara → ghurni → garjan → prani`.

## Device-I/O Bundles (2.1.x, vendored in `lib/`)

| Bundle | Role | Notes |
|--------|------|-------|
| **vani** | ALSA PCM via raw `/dev/snd` ioctls | dh(vani)'s sibling — no libpipewire/libasound, no FFI; drives `playback` (S16/S24/S32) |
| **yukti** | PCM device discovery | backs `device` enumeration / default-open |

## Blocked (not yet ported to Cyrius — still Rust)

| Bundle | Feature | Notes |
|--------|---------|-------|
| **shabda** | `g2p` | grapheme-to-phoneme — port shabda first |
| **bhava** | `bhava-voice` | emotion/personality engine — port bhava first |

## Security Monitoring

- AGNOS bundles are maintained in-house — prioritize coordinated upgrades.
- Vendored `lib/` bundles are committed and auditable — re-vendor on upgrade and
  review the diff; `cyrius.lock` pins the resolved `[deps]` set (abaco).
- No third-party crate supply chain: dhvani is Cyrius-native, no Cargo/crates.io.

## Upgrade Policy

- **Patch versions**: Apply immediately if `cyrius test` passes.
- **Minor versions**: Review changelog, test, apply within a week.
- **Major versions**: Plan migration, update CHANGELOG with breaking section.
- **AGNOS bundles**: Coordinate across the stack (abaco → naad/svara/goonj → dhvani → shruti/jalwa).
