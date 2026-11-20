# ADR 007: the multiband compressor is LR4 crossovers in a tree, one compressor per band

## Status
**Accepted** (2026-11-20, 2.3.4). The user picked the multiband compressor as the first
"Advanced DSP" backlog item. rust-old has no multiband compressor, so there is no oracle
to match. This ADR is the reference that `src/multiband.cyr` and `tests/multiband.tcyr`
hold to.

## Context

The roadmap's post-2.0.0 backlog lists "multiband compressor" first under Advanced DSP.
dhvani already has, ported and hardened:
- an RBJ biquad (`src/biquad.cyr`) with a cross-channel stereo fast path;
- a feed-forward compressor (`src/compressor.cyr`) with a stereo-linked peak detector, a
  soft knee, makeup gain and a dry/wet mix.

A multiband compressor needs three things on top of those: a crossover that splits the
signal into bands, a compressor per band, and a sum. The crossover decides most of the
behaviour, in particular what the processor does to a signal when no band compresses.

## Decision

1. **Crossover: Linkwitz-Riley 4th order (LR4).** Each LR4 lowpass or highpass is two
   cascaded Butterworth biquads (Q = 1/√2) at the crossover frequency.
   - An LR4 lowpass plus its highpass is a 2nd-order allpass. The magnitude is flat, so
     uncompressed bands sum back to the input's spectrum. The two outputs are in phase
     at the crossover, each −6 dB there, and fall at 24 dB/octave.
   - The RBJ lowpass, highpass and allpass at one w0 and Q are bilinear transforms of
     the analog Butterworth sections. That makes the identity exact in the digital
     filters too, to rounding, not only approximate.
2. **Topology: a tree, with allpass compensation.**
   - Band j is the LR4 lowpass at f[j] of what is left above f[j−1]. The highpass carries
     the rest on to the next crossover, and the last band is the final highpass.
   - Band k then runs through the allpass of every crossover above its upper edge
     (f[k+1..N−2]). Those are the phase shifts the bands above it picked up when they
     were split there. That is (N−1)(N−2)/2 allpasses: 0 for 2 bands, 1 for 3, 3 for 4.
   - With every band uncompressed, the output is the input through the allpasses at
     f[0..N−2] in series. The magnitude is flat, there is no latency, and the phase is
     not linear. `tests/multiband.tcyr` checks both the identity (≤ 1e-9 against the
     allpass chain, 2 to 8 bands) and the level of a sine (within 1e-6 at 50 Hz–12 kHz).
3. **Per band: the existing `DhCompressor`, unchanged.** Each band's detector sees only
   its band, and the band count is 2 to 8. Every band starts on
   `dhvani_compparams_default()`, as a lone compressor does. `set_band_params` validates
   and copies the params like `dhvani_comp_set_params`.
4. **Real-time contract: fixed internal chunks.**
   - `new()` allocates every filter, compressor and scratch buffer. `process()` walks the
     host block in chunks of `DH_MBC_CHUNK` (256) frames through that scratch, so it
     allocates nothing for any host block size.
   - Chunking never changes the output: every stage is a per-frame recurrence whose state
     carries across chunks. The suite checks this bit for bit against host blocks of 1,
     7, 255, 256, 257 and 1000 frames.
   - `set_crossovers`, `set_band_params`, `set_sample_rate` and `reset` allocate nothing.
     The filters are redesigned in place and keep their state, as a biquad `set_params`
     does.
   - A per-call scratch sized to the block was the alternative. It would grow, and so
     allocate, whenever a host sends a larger block.
5. **Bad input (ADR 005's rule).**
   - A non-finite input sample (NaN, ±Inf) enters the crossover as 0, so it never
     reaches a filter state. The output is bit for bit the output for 0 at that sample.
   - A finite input large enough to overflow a filter or the band sum gives 0 at those
     samples. The biquads clear a non-finite state at the end of the chunk.
6. **Contracts the other processors share.**
   - The buffer's channel count must equal the instance's, or `process` returns
     `DHVANI_ERR_INVALID_CHANNELS` and leaves the buffer untouched. The biquad passes
     extra channels through instead, but here they would also skip the band sum.
   - It walks `min(frames, storage / channels)` frames.
   - Bypassed, it leaves the buffer untouched. A bypassed band passes its split signal
     uncompressed.
   - Crossovers must be ascending and inside the biquad design's own clamp, [1 Hz,
     0.499 × rate], so the frequency reported is the frequency designed. `set_sample_rate`
     re-checks them against the new rate.

## Consequences

- An uncompressed multiband compressor is not a wire: it is an allpass. A host that
  wants bit-exact passthrough uses `dhvani_mbcomp_set_bypass`.
- Cost is about one compressor per band plus four biquads per crossover. In the bench, a
  3-band stereo 512-frame block takes 216 µs against 42 µs for one compressor (2.3% of a
  core in real time at 44.1 kHz, `BENCHMARKS.md`).
- Not done, and demand-gated like the rest of the backlog:
  - a linear-phase (FIR) crossover, which would add latency;
  - a sidechain input or band solo/mute for metering UIs;
  - a graph-node wrapper;
  - per-band lookahead.
