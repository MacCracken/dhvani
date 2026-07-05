#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::AudioBuffer;
use dhvani::analysis;

fuzz_target!(|data: &[u8]| {
    if data.len() < 8 { return; }

    let samples: Vec<f32> = data.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|f| f.is_finite())
        .collect();
    if samples.len() < 4 { return; }

    if let Ok(buf) = AudioBuffer::from_interleaved(samples, 1, 44100) {
        let _ = analysis::spectrum_fft(&buf, 256);
        let _ = analysis::analyze_dynamics(&buf);
        let _ = analysis::loudness_lufs(&buf);
        let _ = analysis::is_silent(&buf, -60.0);
        let _ = analysis::chromagram(&buf, 256);
        if buf.frames >= 512 {
            let _ = analysis::detect_onsets(&buf, 256, 128, 0.3);
        }
    }
});
