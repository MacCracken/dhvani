# ADR 006: abaco is dhvani's math base; hisab stays as a sibling dependency

## Status
**Accepted** (2026-11-19). This records a decision taken at Wave B of the port (2026-07),
which until now lived only in `cyrius.cyml` and `port-audit.md`.

## Context

Rust dhvani took its DSP math from `abaco::dsp`. Every audio sibling dhvani wraps (naad,
svara, goonj and the rest) depends on **hisab** instead, so dhvani links hisab whenever a
synthesis feature is on. `port-audit.md` asked for a choice before the port went far:
- keep dhvani's own math on abaco, or standardize on hisab;
- record the choice as an ADR;
- prune the loser from the manifest.

The 2.0.0 criteria carried that as an open box. The choice was made at Wave B (keep abaco),
but no ADR was written.

What dhvani's own code (`src/`) calls today, from a symbol diff against the vendored bundles:

| Library | Symbols | Where |
|---|---|---|
| abaco 2.8.0 (8.5k lines) | `amplitude_to_db`, `db_to_amplitude`, `db_gain_factor`, `angular_frequency`, `time_constant`, `constant_power_pan`, `poly_blep`, `f64_round_half_away` | the DSP modules, pan, oscillator, rounding conversions |
| hisab 3.5.0 (80.6k lines) | `hvec3_new` | the acoustics bridge: goonj's source and listener positions |

The two libraries define no function name in common. hisab has none of abaco's dB, pan or
polyBLEP helpers under any name dhvani could call.

## Decision

1. **abaco is dhvani's math base.** The eight functions above come from abaco and nowhere
   else. A new DSP helper that abaco provides is taken from abaco, not re-derived locally.
2. **hisab is not pruned.**
   - It is not a second math base. It is a dependency of the sibling engines: goonj's
     geometry (positions, directions, its vector helpers) is built on its `HVec3`, and
     naad, svara and goonj pin it.
   - dhvani's own use is one call, `hvec3_new`, to hand goonj a position.
   - Removing it would break the acoustics feature and every synthesis-stack build.
3. **Core consumers need only abaco.** `dist/dhvani.cyr` externalizes both. `tests/bundle.tcyr`
   links `lib/abaco.cyr` and the bundle alone. hisab enters only with the synthesis
   features, through the siblings that need it.
4. **abaco wins name clashes.** Every bundle shares one namespace. When another bundle
   defines one of abaco's names, the other bundle renames. For example, naad 2.3.4 renamed
   its `power_spectrum` to `naad_power_spectrum` so dhvani could take abaco 2.8.0.
5. **Not from abaco:**
   - **Transcendentals and rounding primitives.** `f64_sin`, `f64_cos`, `f64_sqrt`,
     `f64_exp` and `f64_round` are cycc builtins, not abaco functions: no bundle or
     stdlib leaf defines them. `f64_round` rounds ties to even, so the Rust `round`
     semantics come from abaco's `f64_round_half_away` (`port-audit.md`).
   - **The three MIDI note constants.** abaco keeps them internal (`DSP_A4_FREQ` and its
     two siblings), so dhvani defines them locally in `src/voice.cyr`.

## Consequences

- **Positive:**
  - The ecosystem's DSP math has one implementation. When abaco 2.3.2 fixed its dB constants
    (a wrong `ln(10)`, about 0.04% per dB), dhvani took the fix by bumping a tag. A local
    copy would have needed its own fix, found its own way.
  - A core consumer links 8.5k lines of math, not 89k.
- **Negative:**
  - With synthesis features on, two math libraries are linked. hisab's surface is reachable
    only through the siblings and `hvec3_new`, and the rest is DCE-pruned.
  - abaco's dist also carries json, http and net helpers dhvani never calls. They resolve
    as unreachable and are pruned, which leaves benign warnings in the build.
- **Neutral:**
  - The 2.0.0 criterion closes as "decision recorded; hisab kept", not "loser pruned". The
    premise that one library would lose did not hold: they are not alternatives for dhvani.

## Alternatives considered

- **Standardize on hisab.** Rejected. hisab has none of the eight helpers. Moving would mean
  porting them into hisab, or into dhvani, for no change in output. Core consumers would
  also take a math dependency about 10× larger.
- **Copy the eight helpers into dhvani and drop abaco.** Rejected. It would remove a
  dependency but fork the ecosystem's math. The dB-constant fix above shows the cost: every
  copy needs every fix.
