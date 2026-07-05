//! Synthesis example — generate audio using naad synthesis engines.

use dhvani::synthesis::*;

fn main() {
    // ── Subtractive synth ──────────────────────────────────────────
    let mut synth = SubtractiveSynth::new(Waveform::Saw, 440.0, 2000.0, 0.7, 44100.0).unwrap();
    synth.note_on();
    let buf = render_to_buffer(|| synth.next_sample(), 2, 44100, 44100);
    println!(
        "Subtractive: {} frames, peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.peak(),
        buf.rms()
    );

    // ── FM synth ───────────────────────────────────────────────────
    let mut fm = FmSynth::new(4, 44100.0).unwrap();
    fm.set_algorithm(FmAlgorithm::Serial4);
    fm.operator_mut(0).unwrap().set_frequency(440.0);
    fm.operator_mut(0).unwrap().set_level(1.0);
    fm.operator_mut(1).unwrap().set_frequency(880.0);
    fm.operator_mut(1).unwrap().set_level(0.3);
    fm.note_on();
    let buf = render_to_buffer(|| fm.next_sample(), 2, 44100, 44100);
    println!(
        "FM:          {} frames, peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.peak(),
        buf.rms()
    );

    // ── Karplus-Strong ─────────────────────────────────────────────
    let mut ks = KarplusStrong::new(330.0, 0.995, 0.5, 44100.0).unwrap();
    ks.pluck();
    let buf = render_to_buffer(|| ks.next_sample(), 2, 44100, 44100);
    println!(
        "Karplus:     {} frames, peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.peak(),
        buf.rms()
    );

    // ── Drum pattern ───────────────────────────────────────────────
    let mut kick = KickDrum::new(150.0, 40.0, 200.0, 0.8, 44100.0).unwrap();
    kick.trigger();
    let kick_buf = render_to_buffer(|| kick.next_sample(), 1, 11025, 44100);
    println!(
        "Kick:        {} frames, peak {:.3}",
        kick_buf.frames(),
        kick_buf.peak()
    );

    // ── Wavetable with morphing ────────────────────────────────────
    let saw_table = Wavetable::from_harmonics(32, &[], 2048).unwrap();
    let mut wt_osc = WavetableOscillator::new(saw_table, 440.0, 44100.0).unwrap();
    let buf = render_to_buffer(|| wt_osc.next_sample(), 2, 22050, 44100);
    println!(
        "Wavetable:   {} frames, peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.peak(),
        buf.rms()
    );

    println!("\nAll synthesis engines rendered successfully.");
}
