//! Mix multiple audio sources, apply DSP, and analyze the result.

use nada::analysis;
use nada::buffer::{AudioBuffer, mix};
use nada::dsp::{self, Compressor, CompressorParams, ParametricEq, EqBandConfig, BandType};

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
    println!("Mixed: {} frames, peak={:.3}", mixed.frames, mixed.peak());

    // EQ: cut low rumble, boost presence
    let mut eq = ParametricEq::new(vec![
        EqBandConfig { band_type: BandType::HighPass, freq_hz: 60.0, gain_db: 0.0, q: 0.707, enabled: true },
        EqBandConfig { band_type: BandType::Peaking, freq_hz: 3000.0, gain_db: 2.0, q: 1.5, enabled: true },
    ], sr, 1);
    eq.process(&mut mixed);

    // Compress
    let mut comp = Compressor::new(CompressorParams {
        threshold_db: -12.0, ratio: 3.0, attack_ms: 10.0, release_ms: 80.0,
        makeup_gain_db: 2.0, knee_db: 6.0,
    }, sr);
    comp.process(&mut mixed);

    // Normalize
    dsp::normalize(&mut mixed, 0.95);

    // Analyze
    let lufs = analysis::loudness_lufs(&mixed);
    let spec = analysis::spectrum_fft(&mixed, 4096);
    println!("Output: peak={:.3}, LUFS={:.1}", mixed.peak(), lufs);
    println!("Dominant freq: {:.1} Hz", spec.dominant_frequency().unwrap_or(0.0));
}
