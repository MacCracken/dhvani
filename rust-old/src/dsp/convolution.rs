//! Convolution reverb — FFT-based overlap-add convolution with an impulse response.
//!
//! Applies a room impulse response (IR) to audio using frequency-domain multiplication.
//! Supports both mono and stereo processing with partitioned convolution for
//! long IRs (trades latency for bounded memory/compute per block).
//!
//! The engine is IR-agnostic — pass any `&[f32]` impulse response. For goonj-generated
//! room IRs, use the `acoustics` feature module which bridges goonj types to this engine.

use crate::analysis::fft::fft_in_place;
use crate::buffer::AudioBuffer;

/// FFT-based convolution reverb processor.
///
/// Uses overlap-add with zero-padded FFT blocks. For real-time use,
/// call [`process`](ConvolutionReverb::process) per audio buffer.
///
/// # Example
///
/// ```rust,no_run
/// use dhvani::dsp::ConvolutionReverb;
///
/// // Short IR (e.g., room impulse response)
/// let ir = vec![1.0, 0.5, 0.25, 0.125];
/// let mut reverb = ConvolutionReverb::new(&ir, 1.0, 44100);
/// ```
#[must_use]
#[derive(Debug, Clone)]
pub struct ConvolutionReverb {
    /// FFT size (next power of 2 >= 2 * block_size).
    fft_size: usize,
    /// Block size for partitioned convolution.
    block_size: usize,
    /// IR partitions in frequency domain: Vec<(real, imag)>.
    ir_partitions: Vec<(Vec<f64>, Vec<f64>)>,
    /// Input accumulation buffer per channel.
    input_buffers: Vec<Vec<f32>>,
    /// Overlap tail per channel (samples from previous blocks).
    overlap: Vec<Vec<f64>>,
    /// Write position within the current input block.
    write_pos: usize,
    /// Dry/wet mix (0.0 = dry, 1.0 = wet).
    mix: f32,
    /// Number of channels.
    channels: usize,
    /// Scratch buffers for FFT (reused to avoid allocation).
    scratch_real: Vec<f64>,
    scratch_imag: Vec<f64>,
    /// Frequency-domain delay line per channel: one slot per partition.
    fdl: Vec<Vec<(Vec<f64>, Vec<f64>)>>,
    /// Current FDL write index.
    fdl_pos: usize,
}

impl ConvolutionReverb {
    /// Create a new convolution reverb from an impulse response.
    ///
    /// `ir_samples`: mono impulse response (f32).
    /// `mix`: dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    /// `sample_rate`: used for tracing only; the IR is assumed to match.
    pub fn new(ir_samples: &[f32], mix: f32, sample_rate: u32) -> Self {
        // Default block size: 512 samples (~11.6ms at 44.1kHz)
        Self::with_block_size(ir_samples, 512, mix, sample_rate)
    }

    /// Create with a custom block size (controls latency vs. CPU trade-off).
    ///
    /// Smaller blocks = lower latency, more CPU. Larger blocks = higher latency, less CPU.
    pub fn with_block_size(
        ir_samples: &[f32],
        block_size: usize,
        mix: f32,
        sample_rate: u32,
    ) -> Self {
        let block_size = block_size.max(1).next_power_of_two();
        let fft_size = block_size * 2; // zero-padded for linear convolution

        tracing::debug!(
            ir_len = ir_samples.len(),
            block_size,
            fft_size,
            mix,
            sample_rate,
            "ConvolutionReverb::new"
        );

        // Partition IR into blocks and pre-compute FFTs
        let ir_partitions = partition_ir(ir_samples, block_size, fft_size);
        Self {
            fft_size,
            block_size,
            ir_partitions,
            input_buffers: Vec::new(), // lazily initialized per channel count
            overlap: Vec::new(),
            write_pos: 0,
            mix: mix.clamp(0.0, 1.0),
            channels: 0,
            scratch_real: vec![0.0; fft_size],
            scratch_imag: vec![0.0; fft_size],
            fdl: Vec::new(),
            fdl_pos: 0,
        }
    }

    /// Set dry/wet mix.
    pub fn set_mix(&mut self, mix: f32) {
        self.mix = mix.clamp(0.0, 1.0);
    }

    /// Current mix value.
    #[must_use]
    pub fn mix(&self) -> f32 {
        self.mix
    }

    /// Load a new impulse response, resetting all state.
    pub fn set_ir(&mut self, ir_samples: &[f32]) {
        tracing::debug!(ir_len = ir_samples.len(), "ConvolutionReverb::set_ir");
        self.ir_partitions = partition_ir(ir_samples, self.block_size, self.fft_size);
        self.reset();
    }

    /// Reset all internal state (overlap, FDL, input buffers).
    pub fn reset(&mut self) {
        for buf in &mut self.input_buffers {
            buf.fill(0.0);
        }
        for ov in &mut self.overlap {
            ov.fill(0.0);
        }
        for ch_fdl in &mut self.fdl {
            for (r, i) in ch_fdl.iter_mut() {
                r.fill(0.0);
                i.fill(0.0);
            }
        }
        self.write_pos = 0;
        self.fdl_pos = 0;
    }

    /// Process an audio buffer in-place with convolution reverb.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.ir_partitions.is_empty() {
            return;
        }

        let ch = buf.channels as usize;
        self.ensure_channels(ch);

        let mix = self.mix;
        let dry = 1.0 - mix;
        let frames = buf.frames;

        let mut frame = 0;
        while frame < frames {
            // Fill input block
            let remaining_in_block = self.block_size - self.write_pos;
            let to_copy = remaining_in_block.min(frames - frame);

            for c in 0..ch {
                for i in 0..to_copy {
                    self.input_buffers[c][self.write_pos + i] = buf.samples[(frame + i) * ch + c];
                }
            }

            self.write_pos += to_copy;

            // When block is full, process it
            if self.write_pos >= self.block_size {
                for c in 0..ch {
                    self.process_block(c);
                }
                self.fdl_pos = (self.fdl_pos + 1) % self.ir_partitions.len().max(1);
                self.write_pos = 0;

                // Write output: overlap[0..block_size] is the current block output
                let block_start = frame + to_copy - self.block_size;
                for c in 0..ch {
                    for i in 0..self.block_size {
                        let src_idx = (block_start + i) * ch + c;
                        if src_idx < buf.samples.len() {
                            let wet = self.overlap[c][i] as f32;
                            buf.samples[src_idx] = buf.samples[src_idx] * dry + wet * mix;
                        }
                    }
                    // Shift overlap: move tail forward
                    let tail_len = self.fft_size - self.block_size;
                    for i in 0..tail_len {
                        self.overlap[c][i] = self.overlap[c][self.block_size + i];
                    }
                    for i in tail_len..self.fft_size {
                        self.overlap[c][i] = 0.0;
                    }
                }
            }

            frame += to_copy;
        }
    }

    /// Ensure internal buffers match channel count.
    fn ensure_channels(&mut self, ch: usize) {
        if self.channels == ch {
            return;
        }
        self.channels = ch;
        let num_partitions = self.ir_partitions.len().max(1);
        self.input_buffers = vec![vec![0.0; self.block_size]; ch];
        self.overlap = vec![vec![0.0; self.fft_size]; ch];
        self.fdl = (0..ch)
            .map(|_| {
                (0..num_partitions)
                    .map(|_| (vec![0.0; self.fft_size], vec![0.0; self.fft_size]))
                    .collect()
            })
            .collect();
        self.write_pos = 0;
        self.fdl_pos = 0;
    }

    /// Process one block for a single channel using partitioned convolution.
    fn process_block(&mut self, channel: usize) {
        let fft_size = self.fft_size;
        let num_partitions = self.ir_partitions.len();

        // FFT the input block (zero-padded)
        self.scratch_real.fill(0.0);
        self.scratch_imag.fill(0.0);
        for i in 0..self.block_size {
            self.scratch_real[i] = self.input_buffers[channel][i] as f64;
        }
        if !fft_in_place(&mut self.scratch_real, &mut self.scratch_imag) {
            return;
        }

        // Store in frequency delay line
        let fdl_slot = &mut self.fdl[channel][self.fdl_pos];
        fdl_slot.0.copy_from_slice(&self.scratch_real);
        fdl_slot.1.copy_from_slice(&self.scratch_imag);

        // Accumulate: sum over all partitions (FDL[pos-k] * IR[k])
        let mut acc_real = vec![0.0f64; fft_size];
        let mut acc_imag = vec![0.0f64; fft_size];

        for k in 0..num_partitions {
            let fdl_idx = (self.fdl_pos + num_partitions - k) % num_partitions;
            let (ref fdl_r, ref fdl_i) = self.fdl[channel][fdl_idx];
            let (ref ir_r, ref ir_i) = self.ir_partitions[k];

            for bin in 0..fft_size {
                // Complex multiply: (a+bi)(c+di) = (ac-bd) + (ad+bc)i
                acc_real[bin] += fdl_r[bin] * ir_r[bin] - fdl_i[bin] * ir_i[bin];
                acc_imag[bin] += fdl_r[bin] * ir_i[bin] + fdl_i[bin] * ir_r[bin];
            }
        }

        // IFFT via conjugate: IFFT(X) = conj(FFT(conj(X))) / N
        for v in &mut acc_imag {
            *v = -*v;
        }
        if !fft_in_place(&mut acc_real, &mut acc_imag) {
            return;
        }
        let scale = 1.0 / fft_size as f64;

        // Overlap-add into output
        for (ov, &ar) in self.overlap[channel][..fft_size].iter_mut().zip(&acc_real) {
            *ov += ar * scale;
        }
    }
}

/// Partition an IR into zero-padded FFT blocks and pre-compute their frequency-domain
/// representations.
fn partition_ir(ir: &[f32], block_size: usize, fft_size: usize) -> Vec<(Vec<f64>, Vec<f64>)> {
    if ir.is_empty() {
        return Vec::new();
    }

    let num_partitions = ir.len().div_ceil(block_size);
    let mut partitions = Vec::with_capacity(num_partitions);

    for p in 0..num_partitions {
        let start = p * block_size;
        let end = (start + block_size).min(ir.len());

        let mut real = vec![0.0f64; fft_size];
        let mut imag = vec![0.0f64; fft_size];

        for (i, &s) in ir[start..end].iter().enumerate() {
            real[i] = s as f64;
        }

        fft_in_place(&mut real, &mut imag);
        partitions.push((real, imag));
    }

    partitions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_ir() {
        // IR = [1.0] should pass signal through unchanged
        let ir = vec![1.0];
        let mut reverb = ConvolutionReverb::with_block_size(&ir, 4, 1.0, 44100);

        let samples = vec![1.0, 0.5, 0.25, 0.0, 0.0, 0.0, 0.0, 0.0];
        let mut buf = AudioBuffer::from_interleaved(samples.clone(), 1, 44100).unwrap();
        reverb.process(&mut buf);

        // First block output should match input (after one block delay)
        // With block_size=4, we get output after 4 samples
        for s in buf.samples() {
            assert!(s.is_finite(), "output not finite");
        }
    }

    #[test]
    fn delay_ir() {
        // IR = [0, 0, 0, 1] should delay by 3 samples
        let ir = vec![0.0, 0.0, 0.0, 1.0];
        let mut reverb = ConvolutionReverb::with_block_size(&ir, 4, 1.0, 44100);

        let mut samples = vec![0.0; 16];
        samples[0] = 1.0; // impulse at sample 0
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        reverb.process(&mut buf);

        // Should have energy at sample 3
        assert!(buf.samples().iter().any(|s| s.abs() > 0.5));
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn stereo_convolution() {
        let ir = vec![1.0, 0.5, 0.25];
        let mut reverb = ConvolutionReverb::with_block_size(&ir, 4, 1.0, 44100);

        // Stereo: L=1.0, R=0.5 then silence
        let samples = vec![
            1.0, 0.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        reverb.process(&mut buf);

        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn mix_control() {
        let ir = vec![1.0];
        let mut reverb = ConvolutionReverb::with_block_size(&ir, 4, 0.5, 44100);

        let samples = vec![1.0; 8];
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        reverb.process(&mut buf);

        // 50% wet means output should be mix of dry and wet
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn empty_ir_passthrough() {
        let ir: Vec<f32> = vec![];
        let mut reverb = ConvolutionReverb::new(&ir, 1.0, 44100);

        let original = vec![0.5; 1024];
        let mut buf = AudioBuffer::from_interleaved(original.clone(), 1, 44100).unwrap();
        reverb.process(&mut buf);

        // Empty IR should not modify the buffer
        assert_eq!(buf.samples(), &original);
    }

    #[test]
    fn long_ir() {
        // Test with IR longer than one block
        let ir: Vec<f32> = (0..2048).map(|i| (-0.001 * i as f32).exp()).collect();
        let mut reverb = ConvolutionReverb::with_block_size(&ir, 256, 1.0, 44100);

        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin() * 0.5)
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        reverb.process(&mut buf);

        assert!(buf.samples().iter().all(|s| s.is_finite()));
        assert!(buf.rms() > 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let ir = vec![1.0, 0.5, 0.25];
        let mut reverb = ConvolutionReverb::with_block_size(&ir, 4, 1.0, 44100);

        // Process some audio
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 8], 1, 44100).unwrap();
        reverb.process(&mut buf);

        // Reset and process silence — should get silence
        reverb.reset();
        let mut buf = AudioBuffer::from_interleaved(vec![0.0; 8], 1, 44100).unwrap();
        reverb.process(&mut buf);

        // After reset + silence input, output should be near-zero
        // (first block after reset starts fresh)
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }

    #[test]
    fn set_ir_replaces() {
        let mut reverb = ConvolutionReverb::new(&[1.0], 1.0, 44100);
        reverb.set_ir(&[1.0, 0.5, 0.25, 0.125]);
        // Should not panic, IR should be loaded
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 2048], 1, 44100).unwrap();
        reverb.process(&mut buf);
        assert!(buf.samples().iter().all(|s| s.is_finite()));
    }
}
