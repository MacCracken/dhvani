# Dhvani Architecture

> Core audio engine — buffers, DSP, resampling, mixing, analysis, and device I/O.
>
> **Name**: Dhvani (ध्वनि, Sanskrit) — sound, resonance.
> Cyrius port of a 23,695-line Rust library (frozen at `rust-old/` as the parity oracle). Consumed as a standalone, reusable bundle.

---

## Design Principles

1. **f64 internally** — all processing in 64-bit float; format conversion at I/O boundaries only (i16/i24/i32/f32/f64/u8)
2. **Sample-accurate** — clock, mixing, and DSP operate at sample granularity
3. **Zero-allocation hot path** — mixing and DSP reuse buffers; the free-less bump allocator makes per-frame alloc a leak
4. **Scalar kernels** — mixing, resampling, gain run through portable scalar loops (SIMD arms are platform-blocked, see below)
5. **FFI-free device I/O** — Linux audio capture/output via vani (raw `/dev/snd` ALSA PCM over ioctls, no libpipewire/libasound)

---

## Module Structure

Every module is a flat `src/<name>.cyr` unit in one bundle namespace (all
symbols `dhvani_`/`DHVANI_`/`DH_`-prefixed). Modules never `include` each
other — the entry file (`src/main.cyr`) orders them in dependency order.
Grouped below by the original Rust layer:

```
src/
├── main.cyr            Entry: orders all modules in dependency order
├── error.cyr          DHVANI_ERR_* codes (was NadaError enum), f64 tolerances/sentinels
│   buffer layer
├── buffer.cyr         AudioBuffer, SampleFormat, Layout, mix(), BufferPool
├── convert.cyr        i16/i24/i32/f32/f64/u8, interleaved/planar, mono/stereo, 5.1 downmix
├── resample.cyr       Linear + sinc resampling (Blackman-Harris window)
├── dither.cyr         Dither for bit-depth reduction
│   dsp layer
├── dsp.cyr            noise_gate, hard_limiter, normalize, dB conversions
├── biquad.cyr         BiquadFilter (8 types, Bristow-Johnson cookbook)
├── svf.cyr            State-variable filter
├── eq.cyr            ParametricEq (N-band cascade)
├── graphic_eq.cyr     GraphicEq
├── compressor.cyr     Compressor (envelope follower, soft knee, makeup gain)
├── limiter.cyr        EnvelopeLimiter (brick-wall)
├── reverb.cyr         Reverb (Schroeder/Freeverb, 4 combs + 2 allpasses)
├── delay.cyr          DelayLine + ModulatedDelay (chorus/flanger)
├── deesser.cyr        DeEsser (biquad sidechain)
├── envelope.cyr       ADSR envelope generation
├── oscillator.cyr     PolyBLEP synthesis (sine, saw, square, triangle, noise)
├── lfo.cyr           LFO (6 shapes, sample-and-hold, tempo sync)
├── pan.cyr           StereoPanner (constant-power law)
├── gain_smoother.cyr  Gain smoothing
├── noise_reduction.cyr  Spectral noise reduction (STFT gating)
│   analysis layer
├── analysis.cyr       Spectrum type, spectrum_dft, loudness_lufs, is_silent
├── fft.cyr           Radix-2 Cooley-Tukey FFT
├── loudness.cyr       EBU R128 (K-weighting, gating, LRA)
├── dynamics.cyr       True peak (4x), crest factor, dynamic range
├── chroma.cyr         Chromagram (12 pitch classes)
├── key.cyr           Key detection
├── onset.cyr          Onset detection (spectral flux)
├── beat.cyr          Beat tracking
├── zcr.cyr           Zero-crossing rate
├── stft.cyr           STFT spectrograms
├── waveform.cyr       Downsampled min/max for UI visualization
│   clock / automation
├── clock.cyr          AudioClock (position, tempo, beats, PTS, seek)
├── automation.cyr     Parameter automation
├── ops.cyr           Buffer/parameter ops
│   midi layer
├── midi.cyr           NoteEvent, ControlChange, MidiEvent, MidiClip
├── midi_v2.cyr        MIDI 2.0 / UMP types
├── voice.cyr          VoiceManager (16-voice pool, 4 steal modes)
├── midi_routing.cyr   VelocityCurve, MidiRoute, CcMapping
├── routing.cyr        Routing helpers
├── translate.cyr      MIDI 1.0 ↔ 2.0 conversion
│   graph / meter
├── graph.cyr          AudioNode, Graph, ExecutionPlan, GraphProcessor
├── meter.cyr          PeakMeter, MeterBank (lock-free)
│   device I/O layer  (vani/ALSA, no PipeWire)
├── capture.cyr        CaptureConfig, OutputConfig, AudioDevice descriptors
├── record.cyr         RecordManager, LoopRecordManager (ring-buffer)
├── playback.cyr       AudioBuffer ↔ vani PCM (S16/S24/S32): blocking + RT ring player/recorder
├── device.cyr         Device enumeration + default-device open (via yukti)
│   compute
├── simd.cyr           Scalar audio kernels (SSE2/AVX2/NEON arms platform-blocked, dropped)
│   feature layers (wrap sibling bundles; DCE-prune when unused)
├── synthesis.cyr      Synth engines via naad (subtractive, FM, additive, wavetable, granular, drum, vocoder)
├── voice_synth.cyr    Voice synthesis via svara (glottal, formant, phoneme, prosody, vocal tract) — bhava_bridge blocked (bhava still Rust)
├── creature.cyr       Animal/creature vocals via prani
├── environment.cyr    Nature/environmental sounds via garjan
├── mechanical.cyr     Mechanical sounds via ghurni
├── sampler.cyr        Sample playback via nidhi
├── convolution.cyr    Convolution
├── acoustics.cyr      Room acoustics via goonj (IR, convolution, FDN, ambisonics, presets)
└── (g2p via shabda — blocked, shabda still Rust)

tests/                 tests/*.tcyr (one per module) + proptest.tcyr;
                       tests/hw/ = local-only hardware I/O; *.bcyr = benchmarks
```

### Distribution

`cyrius distlib` concatenates every ported module in dependency order into
`dist/dhvani.cyr` — the single-file bundle a consumer links (do not hand-edit
it; rebuild with `cyrius distlib`). The sibling engines (abaco, naad, svara,
prani, garjan, ghurni, goonj, nidhi, sakshi, hisab, shravan, vani, yukti) are
**vendored into `lib/`** and `include`d in dependency order — not wired as
auto-resolved `[deps]`. A consumer supplies only the sibling bundles for the
feature layers it uses (synthesis, voice, sampler, acoustics, device I/O);
unused feature references DCE-prune. `device.cyr`/`playback.cyr` sit outside the
core bundle so device-agnostic consumers stay free of the yukti/vani stack.

---

## Pipeline

```
Input (file, capture, synthesis, MIDI)
    │
    ▼
AudioBuffer (f64 interleaved, channels, sample_rate)
    │
    ├──▶ DSP chain (EQ → compress → gate → limit → reverb → delay)
    │
    ├──▶ Analysis (FFT spectrum, R128 loudness, dynamics, chromagram, onsets)
    │
    ├──▶ Audio graph (topological execution, double-buffered plan swap)
    │
    ├──▶ Mix (sum multiple sources with gain)
    │
    ├──▶ Resample (linear + sinc, 44.1k ↔ 48k ↔ 96k)
    │
    ├──▶ Meter (lock-free peak/RMS via atomics)
    │
    ▼
Output (encode via shravan, play via vani/ALSA device I/O, sync via clock PTS)
```

---

## Key Types

### AudioBuffer
Core sample buffer. Holds f64 interleaved samples with channel count, sample rate, and frame count. Provides peak/RMS/gain/clamp operations. Fallible constructors return a valid handle or `0` (null) on error.

### AudioClock
Sample-accurate transport. Tracks position in samples, converts to seconds/ms/beats/PTS. Tempo-aware for DAW integration. Generates PTS timestamps for A/V sync with aethersafta.

### Spectrum
FFT magnitude analysis. Provides frequency bins, dominant frequency detection, and per-bin access. Radix-2 Cooley-Tukey FFT (O(n log n)) for production use; simple DFT available for small windows.

---

## Consumers

| Project | Usage |
|---------|-------|
| **shruti** | DAW — all audio math (mix, DSP, analysis, transport, synthesis) |
| **jalwa** | Media player — playback EQ, spectrum visualizer, resampling, normalization |
| **aethersafta** | Compositor — ALSA device capture (via vani), audio mixing for streams |
| **kiran** | Game engine — game audio, spatial sound, creature/environment synthesis |
