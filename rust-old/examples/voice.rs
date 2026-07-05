//! Voice synthesis example — generate speech from phoneme sequences.

use dhvani::voice_synth::*;

fn main() {
    let sr = 44100;

    // ── Single phoneme ─────────────────────────────────────────────
    let voice = VoiceProfile::new_male();
    let buf = render_phoneme(&Phoneme::VowelA, &voice, sr, 0.5).unwrap();
    println!(
        "Vowel /a/:   {} frames ({:.2}s), peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.frames() as f64 / sr as f64,
        buf.peak(),
        buf.rms()
    );

    // ── Phoneme sequence: "ani" ────────────────────────────────────
    let female = VoiceProfile::new_female();
    let mut seq = PhonemeSequence::new();
    seq.push(PhonemeEvent::new(Phoneme::VowelA, 0.2, Stress::Primary));
    seq.push(PhonemeEvent::new(Phoneme::NasalN, 0.1, Stress::Unstressed));
    seq.push(PhonemeEvent::new(Phoneme::VowelI, 0.2, Stress::Unstressed));

    let buf = render_sequence(&seq, &female, sr).unwrap();
    println!(
        "Sequence:    {} frames ({:.2}s), peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.frames() as f64 / sr as f64,
        buf.peak(),
        buf.rms()
    );

    // ── Different voice profiles ───────────────────────────────────
    let child = VoiceProfile::new_child();
    let buf = render_phoneme(&Phoneme::VowelO, &child, sr, 0.3).unwrap();
    println!(
        "Child /o/:   {} frames, peak {:.3}, f0={:.0}Hz",
        buf.frames(),
        buf.peak(),
        child.base_f0
    );

    // ── Breathy voice ──────────────────────────────────────────────
    let breathy = VoiceProfile::new_male().with_breathiness(0.7);
    let buf = render_phoneme(&Phoneme::VowelE, &breathy, sr, 0.3).unwrap();
    println!(
        "Breathy /e/: {} frames, peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.peak(),
        buf.rms()
    );

    println!("\nAll voice synthesis rendered successfully.");
}
