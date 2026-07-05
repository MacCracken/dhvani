#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::midi::{MidiClip, NoteEvent};
use dhvani::midi::voice::{VoiceManager, VoiceStealMode};
use dhvani::midi::translate;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 { return; }

    let mut clip = MidiClip::new("fuzz", 0, 1_000_000);
    for chunk in data.chunks(5) {
        if chunk.len() < 5 { continue; }
        let pos = (chunk[0] as u64) * 1000;
        let dur = (chunk[1] as u64).max(1) * 100;
        let note = chunk[2] % 128;
        let vel = chunk[3] % 128;
        let ch = chunk[4] % 16;
        clip.add_note(pos, dur, note, vel, ch);
    }

    let _ = clip.notes_at(5000);
    let _ = clip.events_in_range(0, 50000);
    clip.transpose(5);
    clip.quantize(1000);

    // Voice manager
    let mut mgr = VoiceManager::new(8, VoiceStealMode::Oldest);
    for chunk in data.chunks(3) {
        if chunk.len() < 3 { continue; }
        let note = chunk[0] % 128;
        let vel = chunk[1] % 128;
        mgr.note_on(note, vel, 0);
        if chunk[2] % 3 == 0 {
            mgr.note_off(note, 0);
        }
    }

    // Translation roundtrip
    for &v in &data[..data.len().min(128)] {
        let v16 = translate::velocity_7_to_16(v % 128);
        let _ = translate::velocity_16_to_7(v16);
    }
});
