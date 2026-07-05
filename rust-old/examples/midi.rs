//! MIDI: clips, voice management, translation, routing.

use dhvani::midi::routing::{CcMapping, MidiRoute, VelocityCurve};
use dhvani::midi::translate::{note_event_to_v2, velocity_7_to_16};
use dhvani::midi::voice::{VoiceManager, VoiceStealMode};
use dhvani::midi::{MidiClip, NoteEvent};

fn main() {
    // Create a MIDI clip
    let mut clip = MidiClip::new("melody", 0, 44100 * 4);
    clip.add_note(0, 22050, 60, 100, 0); // C4
    clip.add_note(22050, 22050, 64, 90, 0); // E4
    clip.add_note(44100, 22050, 67, 85, 0); // G4
    clip.add_note(66150, 22050, 72, 95, 0); // C5

    println!("Clip '{}': {} notes", clip.name(), clip.notes().len());

    // Query active notes at different positions
    for pos in [0, 11025, 22050, 44100] {
        let active = clip.notes_at(pos);
        let names: Vec<u8> = active.iter().map(|n| n.note()).collect();
        println!("  At frame {}: notes {:?}", pos, names);
    }

    // Range query (binary search)
    let range = clip.events_in_range(0, 44100);
    println!("  Notes in first second: {}", range.len());

    // Transpose up a major third
    clip.transpose(4);
    println!(
        "  After +4 semitones: first note = {}",
        clip.notes()[0].note()
    );

    // MIDI 1.0 → 2.0 translation
    let note = NoteEvent::new(0, 1000, 60, 100, 0);
    let v2 = note_event_to_v2(&note);
    println!(
        "\nMIDI 2.0: velocity 7-bit {} → 16-bit {}",
        note.velocity(),
        v2.velocity
    );
    println!("  velocity_7_to_16(127) = {}", velocity_7_to_16(127));

    // Voice management
    let mut voices = VoiceManager::new(8, VoiceStealMode::Oldest);
    for note in [60u8, 64, 67, 72] {
        if let Some(slot) = voices.note_on(note, 100, 0) {
            let freq = voices.voice(slot).unwrap().frequency();
            println!("  Voice {}: note={}, freq={:.1} Hz", slot, note, freq);
        }
    }
    println!("  Active voices: {}", voices.active_count());

    // Routing with velocity curve
    let route = MidiRoute::new(Some(0), VelocityCurve::Soft, (48, 84));
    let event = NoteEvent::new(0, 1000, 60, 127, 0);
    if let Some(filtered) = route.filter_event(&event) {
        println!(
            "\nRouting: velocity {} → {} (Soft curve)",
            event.velocity(),
            filtered.velocity()
        );
    }

    // CC mapping
    let mapping = CcMapping::new(1, 0, 20.0, 20000.0); // mod wheel → filter cutoff
    println!("CC mapping: CC1=64 → {:.0} Hz", mapping.map_value(64));
}
