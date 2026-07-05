#![no_main]
use libfuzzer_sys::fuzz_target;
use dhvani::buffer::AudioBuffer;
use dhvani::buffer::convert;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 { return; }

    // i16 roundtrip
    let i16_samples: Vec<i16> = data.chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect();
    if !i16_samples.is_empty() {
        let f32s = convert::i16_to_f32(&i16_samples);
        let back = convert::f32_to_i16(&f32s);
        assert_eq!(back.len(), i16_samples.len());
    }

    // Interleaved/planar roundtrip
    let f32_samples: Vec<f32> = data.chunks_exact(4)
        .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        .filter(|f| f.is_finite())
        .collect();
    if f32_samples.len() >= 4 {
        if let Ok(buf) = AudioBuffer::from_interleaved(f32_samples, 2, 44100) {
            let planes = convert::interleaved_to_planar(&buf);
            let back = convert::planar_to_interleaved(&planes, 44100);
            assert!(back.is_ok());
        }
    }
});
