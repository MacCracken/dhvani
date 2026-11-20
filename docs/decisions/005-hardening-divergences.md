# ADR 005: Hardening divergences from the oracle

## Status
**Accepted** (2026-11-18, dhvani 2.3.3). Extended 2026-11-19 with the MIDI 1.0 → 2.0
upscaling section (unreleased; its tests are named in the CHANGELOG `[Unreleased]`
section).

## Context

The port's correctness bar is "matches what `rust-old` did" (CLAUDE.md), and a divergence
needs an ADR. The 2.3.3 P(-1) sweep found places where matching the oracle literally is
wrong for this port. They fall into four groups:

1. **Where the oracle panics, the port aborted the process.** A Rust `usize` cannot be
   negative, a `[f32; 12]` cannot be short, and an out-of-range index panics and unwinds.
   The flattened i64 port has none of those guarantees. Its `vec_get` calls `exit(1)`, and a
   raw `load64` off a bad handle is a wild access. Defining the behaviour is not a
   divergence from anything observable, but it is a choice, so it is recorded.
2. **Where the oracle lets one bad sample win forever.** `docs/development/threat-model.md`
   treats incoming samples as untrusted and expects NaN. In the oracle, one NaN or Inf
   written into a filter's state, a delay or reverb ring, or a meter's running sum stays
   there until `reset()`. One bad sample silences the effect, or a whole channel. The port
   now keeps it from persisting.
3. **Where the oracle is simply wrong** and the fix is small and unambiguous.
4. **Where Rust's allocator made a temporary free.** Under the free-less bump allocator a
   per-call temporary is a leak. Those fixes change no output, so they are not listed here
   (see the CHANGELOG).

## Decision

dhvani defines the behaviour below and keeps it. Every row has a regression test, named in
the CHANGELOG `[2.3.3]` section.

### Non-finite input never persists in state

| Where | Oracle | dhvani |
|---|---|---|
| biquad (and so EQ, de-esser, graphic EQ, loudness K-weighting) | NaN state forever | a non-finite z1/z2 is cleared at the end of the block it arrived in; that block's own outputs may be NaN |
| svf | same | same rule: a non-finite state is cleared at the end of its block |
| delay / modulated delay rings | NaN recirculates and zeroes the output every lap | a non-finite value is stored into the ring as 0 |
| reverb comb / allpass rings, comb damping state | NaN mutes the channel until `reset()` | stored as 0; one NaN costs one output sample |
| compressor / limiter detector | +Inf latches `env_db` at +Inf, so gain is never applied again | a non-finite sample is kept out of the detector and comes out as 0 |
| level meter RMS / LUFS / peak | one NaN makes RMS NaN forever; +Inf pins peak and LUFS | non-finite samples are skipped (they count as silence in the sums, and the frame still counts) |

### Validation rejects NaN

- **Where the oracle used `contains`.** Every `validate()` now goes through
  `dh_in_range` (error.cyr), the NaN-rejecting form of `(lo..=hi).contains(&x)`. Where the
  oracle wrote `contains`, this restores parity: envelope sustain, gain-smoother
  attack/release, moddelay feedback/mix, reverb, and the meter. The port's negated
  `lt || gt` form had let NaN through.
- **Where the oracle used bare `<` / `>`, which accept NaN**, NaN is now rejected too
  (a divergence):
  - compressor ratio, attack, release and knee;
  - limiter ceiling, release and knee;
  - ADSR attack, decay and release;
  - moddelay base, depth and rate;
  - LFO rate, depth and sample rate;
  - oscillator sample rate.

### Delay times

- **Negative, -Inf and NaN delay times become 0 samples**, as the oracle's saturating
  `as usize` makes them. The port's truncating cast had made them negative, and -Inf
  aborted the process. A huge time is capped at 2^62 samples.
- **`moddelay.set_params` with a longer base + depth grows the ring**, keeping every
  sample at its age. The oracle does not resize: its usize subtraction panics in debug
  builds and wraps in release, and the port aborted.
- **Reads are clamped to ring − 1 samples.** This changes output only after an outgrown
  parameter change, or at rates under about 2 kHz.
- **A ring that cannot be allocated** (more than the vec cap) passes its channel through
  instead of aborting.

### Small values

- **reverb:** |x| < 1e-30 is flushed to signed zero on ring and damping-state stores. The
  oracle's comb could sit in a permanent subnormal cycle, which is a lasting CPU cost on
  x86 once the input is silent. Normal audio never produces values that small.

### Defined behaviour where the oracle panics

- **Shapes.**
  - Negative channel counts, sample rates, frame counts, hop sizes, band indices, meter
    indices and channel indices are refused (0, an error code, or a pass-through).
  - `silence()` refuses a `channels * frames` past the vec cap.
  - A buffer header that claims more frames than its vec holds is clamped to the storage
    (stft, dynamics, flux, reverb, de-esser, noise reduction, convolution, waveform, pan,
    fades) or refused (resample).
- **Rates.**
  - Both resample paths refuse a source or target rate that is `<= 0` or above
    `DH_MAX_SAMPLE_RATE`. The oracle checks only `target == 0`.
  - A biquad or svf whose sample rate is too low for the `[1 Hz, 0.499·sr]` clamp
    designs unity (pass-through) coefficients. The oracle's `f32::clamp` panics there.
- **Channel counts.**
  - A buffer with more channels than a filter or delay has state for: the extra channels
    pass through untouched.
  - A 0-channel buffer is a no-op (reverb, de-esser, noise reduction, convolution).
- **Lengths.**
  - `chromagram_from_array` needs exactly 12 values.
  - `simd_biquad_stereo` refuses fewer than 5 coefficients or 4 state slots.
  - dither fills the shorter of its two vecs.
- **FFT sizes.** FFT windows past `DH_MAX_FFT` (2^24) are refused by `fft_spectrum`,
  `chroma`, `key`, `stft` and `stftproc_new`. `dh_next_pow2` answers 0 past 2^62, where
  it looped forever.
- **Graph.**
  - Swapping in a null plan, or swapping through a null handle, returns an error and
    keeps the running plan.
  - Plan accessors answer 0 for a null plan.
  - A bypassed node copies the shorter of its input and output lengths.
- **Bridges.**
  - Every sibling-engine bridge refuses a handle `<= 0` before calling upstream.
  - The handle-returning bridges answer 0 on failure, so one null-handle rule covers
    environment, mechanical, acoustics, creature, voice and g2p.
- **Device I/O.**
  - Bit depths outside {16, 24, 32} are refused everywhere, where they used to be
    treated as 32.
  - The player and recorder move whole frames only, never a fragment.
  - A capture read decodes only the frames the kernel delivered.

### Oracle bugs fixed

- **MIDI 2.0 → 1.0 note-on:**
  - A velocity that scales down to 0 becomes 1. In MIDI 1.0 velocity 0 means note-off,
    and the MIDI 2.0 translation rule forbids turning a note-on into one.
  - This includes a 2.0 velocity of 0, which the oracle's own doc comment calls a
    note-off. In the MIDI 2.0 protocol it is a note-on.
- **`MidiClip::merge`** extends the duration to the latest note *end* of the other clip,
  not to the end of its last-starting note.
- **`events_in_range`** returns the true overlap set. The oracle's `partition_point` over
  note end times is not monotone when durations vary, so it skips long notes that start
  early. (The port has always scanned; this is now documented and tested.)
- **`merge(c, c)`**, which Rust forbids at compile time, copies the clip's notes first.
- **Unknown curve codes.** An unknown velocity-curve code acts as Linear, and translate
  inputs are masked to the Rust type width. Rust's types cannot represent either case.
- **`acoustics_convolve_multiband`** convolves the trailing `frames % block` frames. The
  oracle returned them raw.
- **Graphic EQ settings handle.** Gains edited through the live `dhvani_geq_settings`
  handle are applied at the next `process`. The oracle's `settings()` is immutable, so
  this is an extension.
- **Graph:**
  - `compile` copies the node table, so the graph stays usable afterwards. The node
    objects are still shared.
  - An edge to a node that was never added always fails `compile`. The oracle let one
    through when an edge to a missing node and an edge from one appeared together.
  - Total latency is fixed at compile time.
- **Level meter:** `dhvani_meter_level_new` takes an integer sample rate, like every
  other constructor, and refuses a window under one frame or a rate past 768 kHz.

### Convolution streams with one block of latency (user decision, 2026-11-18)

- **The oracle's timing.** It writes a block's output only when that block completes,
  and only into the call that completed it. With zero latency this is correct only when
  every call ends on a block boundary. Otherwise some frames come back raw: with
  256-frame calls against the 512-frame block, every other call was dry.
- **`dhvani_conv_process` now streams one block behind.** Output frame t is
  `dry * x[t - B] + mix * wet[t - B]`. The dry path is delayed with the wet one so the
  two stay aligned, and the first block after construction or `reset()` is silence.
  - Every frame is processed for any host block size, and how the input is split into
    calls does not change a bit.
  - For calls that do end on block boundaries, the output is the oracle's, bit for bit,
    one block later.
- **The latency is reported** by `dhvani_conv_latency_frames` (`block_size`, or 0 for an
  empty IR), for the graph's latency compensation.
- **`dhvani_conv_render`** is the aligned, offline form. `acoustics_convolve_multiband`
  uses it.

### MIDI 1.0 → 2.0 upscaling is min-center-max (user decision, 2026-11-19, after 2.3.3)

- **The oracle's rule.** `velocity_7_to_16`, `cc_7_to_32` and `pitch_bend_14_to_32` used
  plain bit replication (`v << 9 | v << 2 | v >> 5` for velocity), which its doc comment
  calls "MIDI 2.0 spec scaling". It is not the spec's rule, and it moves every centre value:
  - velocity 64 went to `0x8102`, not `0x8000`;
  - CC 64 went to `0x81020408`, not `0x80000000`;
  - pitch bend 8192 went to `0x80020008`, not `PITCH_BEND_CENTER` (`0x80000000`). A MIDI
    1.0 bend at rest became a slight detune, and a consumer's `== PITCH_BEND_CENTER`
    test for "no bend" never matched.
- **dhvani now uses min-center-max**, the up-scaling rule in the MIDI 2.0 UMP
  specification (M2-104-UM), as its reference algorithm (`dh_miditx_scale_up`):
  - at or below the centre, a plain shift, so 0 and the centre land exactly on 0 and the
    2.0 centre;
  - above the centre, the low source bits are repeated downward, so the maximum lands
    exactly on the 2.0 maximum.
- **What changes.** Every up-scaled value except 0 and the maximum. Down-scaling is the
  spec's plain shift and is unchanged, every value still round-trips exactly, and the
  up-scalers stay strictly increasing over the whole domain.
- **Out-of-range input.** A 7-bit input of 128–255 (a valid Rust `u8`, outside MIDI 1.0)
  saturates to 127, and a 14-bit input above 16383 to 16383. The oracle let them wrap in
  `u16` / `u32`. Inputs are still masked to the Rust type width first.
- **Tests.** The oracle's 12 tests still pass unchanged: they check only round trips,
  near-maximum thresholds and the centre round trip. `upscale_is_min_center_max` pins
  the centres, extremes and hand-computed vectors either side of the centre.
  `upscale_monotone_over_the_whole_domain` covers every 7- and 14-bit value.

### Not changed (parity kept, documented)
- **clamp / noise_gate** pass NaN through, as the oracle's scalar and NEON paths do. Its
  x86 SIMD path did not, and it was inconsistent across platforms.

## Consequences

- **Positive:**
  - No public entry point found by the sweep aborts the process or makes a wild access
    on bad input.
  - A single bad sample cannot silence an effect for the rest of a session.
  - The fixed oracle bugs no longer reach consumers.
- **Negative:**
  - These rows are now dhvani's to keep. A future re-port or a consumer comparing
    byte-for-byte against the Rust crate must expect them.
  - The non-finite guards cost a compare per sample in a few loops, or per block in the
    filters.
- **Neutral:**
  - Convolution consumers now see one block (512 frames, about 11.6 ms at 44.1 kHz) of
    latency, and must compensate for it like any other latent node.
  - MIDI 1.0 → 2.0 up-scaled values differ from the Rust crate's (min-center-max). A
    consumer that stored the oracle's 2.0 values sees new bits for the same 1.0 input.

## Alternatives considered

- **Literal parity everywhere.** Rejected. The threat model already commits the library
  to surviving NaN input, and an abort is not "what Rust did" in any useful sense.
- **Sanitizing every sample on input.** Rejected. That costs a compare per sample on every
  path, including the clean majority, and it hides the bad samples from a caller who
  wants to see them. Guarding only the state that persists is cheaper and leaves the bad
  sample's own output visible.
