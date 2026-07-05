//! Creature vocal synthesis example — generate animal and creature vocalizations.

use dhvani::creature::*;

fn main() {
    let sr = 44100;

    // ── Wolf howl ──────────────────────────────────────────────────
    let wolf = CreatureVoice::new(Species::Wolf);
    let buf = render_vocalization(&wolf, &Vocalization::Howl, sr, 1.5).unwrap();
    println!(
        "Wolf howl:     {} frames ({:.2}s), peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.frames() as f64 / sr as f64,
        buf.peak(),
        buf.rms()
    );

    // ── Cat purr ───────────────────────────────────────────────────
    let cat = CreatureVoice::new(Species::Cat);
    let buf = render_vocalization(&cat, &Vocalization::Purr, sr, 1.0).unwrap();
    println!(
        "Cat purr:      {} frames ({:.2}s), peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.frames() as f64 / sr as f64,
        buf.peak(),
        buf.rms()
    );

    // ── Dragon roar with threat intent ─────────────────────────────
    let dragon = CreatureVoice::new(Species::Dragon).with_size(2.0);
    let buf =
        render_vocalization_with_intent(&dragon, &Vocalization::Roar, CallIntent::Threat, sr, 1.0)
            .unwrap();
    println!(
        "Dragon threat: {} frames ({:.2}s), peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.frames() as f64 / sr as f64,
        buf.peak(),
        buf.rms()
    );

    // ── Songbird alarm chirp ───────────────────────────────────────
    let bird = CreatureVoice::new(Species::Songbird);
    let buf =
        render_vocalization_with_intent(&bird, &Vocalization::Chirp, CallIntent::Alarm, sr, 0.5)
            .unwrap();
    println!(
        "Bird alarm:    {} frames ({:.2}s), peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.frames() as f64 / sr as f64,
        buf.peak(),
        buf.rms()
    );

    // ── Snake hiss ─────────────────────────────────────────────────
    let snake = CreatureVoice::new(Species::Snake);
    let buf = render_vocalization(&snake, &Vocalization::Hiss, sr, 0.8).unwrap();
    println!(
        "Snake hiss:    {} frames ({:.2}s), peak {:.3}, rms {:.3}",
        buf.frames(),
        buf.frames() as f64 / sr as f64,
        buf.peak(),
        buf.rms()
    );

    println!("\nAll creature vocalizations rendered successfully.");
}
