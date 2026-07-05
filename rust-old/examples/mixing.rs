//! Mix multiple audio sources, apply DSP, and analyze the result.

use dhvani::analysis;
use dhvani::buffer::{AudioBuffer, mix};
use dhvani::dsp::{self, BandType, Compressor, CompressorParams, EqBandConfig, ParametricEq};

fn main() {
    // Generate two sine wave sources
    let sr = 44100u32;
    let vocals: Vec<f32> = (0..sr as usize)
        .map(|i| 0.6 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
        .collect();
    let drums: Vec<f32> = (0..sr as usize)
        .map(|i| 0.4 * (2.0 * std::f32::consts::PI * 100.0 * i as f32 / sr as f32).sin())
        .collect();

    let vocal_buf = AudioBuffer::from_interleaved(vocals, 1, sr).unwrap();
    let drum_buf = AudioBuffer::from_interleaved(drums, 1, sr).unwrap();

    // Mix
    let mut mixed = mix(&[&vocal_buf, &drum_buf]).unwrap();
    println!("Mixed: {} frames, peak={:.3}", mixed.frames(), mixed.peak());

    // EQ: cut low rumble, boost presence
    let mut eq = ParametricEq::new(
        vec![
            EqBandConfig::new(BandType::HighPass, 60.0, 0.0, 0.707, true),
            EqBandConfig::new(BandType::Peaking, 3000.0, 2.0, 1.5, true),
        ],
        sr,
        1,
    );
    eq.process(&mut mixed);

    // Compress
    let mut comp = Compressor::new(
        CompressorParams::new()
            .with_threshold(-12.0)
            .with_ratio(3.0)
            .with_attack(10.0)
            .with_release(80.0)
            .with_makeup_gain(2.0)
            .with_knee(6.0),
        sr,
    )
    .unwrap();
    comp.process(&mut mixed);

    // Normalize
    dsp::normalize(&mut mixed, 0.95);

    // Analyze
    let lufs = analysis::loudness_lufs(&mixed);
    let spec = analysis::spectrum_fft(&mixed, 4096).unwrap();
    println!("Output: peak={:.3}, LUFS={:.1}", mixed.peak(), lufs);
    println!(
        "Dominant freq: {:.1} Hz",
        spec.dominant_frequency().unwrap_or(0.0)
    );
}
