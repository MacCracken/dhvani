//! Analyze audio: FFT, EBU R128, dynamics, chromagram, onset detection.

use dhvani::analysis::{
    analyze_dynamics, chromagram, compute_stft, detect_onsets, measure_r128, spectrum_fft,
};
use dhvani::buffer::AudioBuffer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sr = 44100u32;

    // Generate a 440Hz A note, 2 seconds
    let samples: Vec<f32> = (0..sr as usize * 2)
        .map(|i| 0.7 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / sr as f32).sin())
        .collect();
    let buf = AudioBuffer::from_interleaved(samples, 1, sr).unwrap();

    // FFT spectrum
    let spec = spectrum_fft(&buf, 4096)?;
    println!(
        "FFT: {} bins, resolution={:.1} Hz",
        spec.bin_count(),
        spec.freq_resolution()
    );
    println!(
        "  Dominant: {:.1} Hz",
        spec.dominant_frequency().unwrap_or(0.0)
    );

    // Dynamics
    let dyn_ = analyze_dynamics(&buf);
    println!("Dynamics:");
    println!(
        "  Peak: {:.2} dB, True peak: {:.2} dB",
        dyn_.max_peak_db(),
        dyn_.max_true_peak_db()
    );
    println!(
        "  RMS: {:.2} dB, Crest factor: {:.1} dB",
        dyn_.rms_db().first().unwrap_or(&f32::NEG_INFINITY),
        dyn_.mean_crest_factor_db()
    );
    println!("  Dynamic range: {:.1} dB", dyn_.dynamic_range_db());

    // EBU R128
    let r128 = measure_r128(&buf)?;
    println!("EBU R128:");
    println!("  Integrated: {:.1} LUFS", r128.integrated_lufs());
    println!("  Short-term: {:.1} LUFS", r128.short_term_lufs());
    println!("  LRA: {:.1} LU", r128.range_lu());

    // Chromagram
    let chroma = chromagram(&buf, 8192)?;
    println!(
        "Chromagram: dominant pitch class = {}",
        chroma.dominant_name()
    );

    // STFT spectrogram
    let sg = compute_stft(&buf, 2048, 512)?;
    println!("STFT: {} frames x {} bins", sg.num_frames(), sg.num_bins);

    // Onset detection
    let onsets = detect_onsets(&buf, 2048, 512, 0.3)?;
    println!("Onsets: {} detected", onsets.count());

    Ok(())
}
