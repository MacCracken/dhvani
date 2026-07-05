//! Reverb — Schroeder/Freeverb algorithm with comb and allpass filters.
//!
//! Topology: 4 parallel comb filters summed, then cascaded through 2 allpass filters.
//! Delay line lengths are tuned for 44100 Hz and scaled proportionally for other rates.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;

/// Base comb filter delay lengths at 44100 Hz (Freeverb standard).
const COMB_DELAYS: [usize; 4] = [1557, 1617, 1491, 1422];
/// Base allpass filter delay lengths at 44100 Hz.
const ALLPASS_DELAYS: [usize; 2] = [225, 556];
/// Stereo offset applied to right channel comb delays.
const STEREO_OFFSET: usize = 23;
/// Allpass feedback coefficient.
const ALLPASS_FEEDBACK: f32 = 0.5;

/// Reverb parameters.
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct ReverbParams {
    /// Room size (0.0–1.0). Controls feedback amount.
    pub room_size: f32,
    /// Damping (0.0–1.0). Low-pass filtering in comb feedback path.
    pub damping: f32,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    pub mix: f32,
}

impl ReverbParams {
    /// Create reverb parameters with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set room size (0.0--1.0).
    #[inline]
    pub fn with_room_size(mut self, size: f32) -> Self {
        self.room_size = size;
        self
    }

    /// Set damping (0.0--1.0).
    #[inline]
    pub fn with_damping(mut self, damping: f32) -> Self {
        self.damping = damping;
        self
    }

    /// Set dry/wet mix (0.0--1.0).
    #[inline]
    pub fn with_mix(mut self, mix: f32) -> Self {
        self.mix = mix;
        self
    }

    /// Validate parameters.
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(0.0..=1.0).contains(&self.room_size) {
            return Err("room_size must be 0.0–1.0");
        }
        if !(0.0..=1.0).contains(&self.damping) {
            return Err("damping must be 0.0–1.0");
        }
        if !(0.0..=1.0).contains(&self.mix) {
            return Err("mix must be 0.0–1.0");
        }
        Ok(())
    }
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            room_size: 0.5,
            damping: 0.5,
            mix: 0.3,
        }
    }
}

/// Comb filter with damped feedback.
#[derive(Debug, Clone)]
pub(crate) struct CombFilter {
    buffer: Vec<f32>,
    write_pos: usize,
    feedback: f32,
    damp: f32,
    damp_state: f32,
}

impl CombFilter {
    fn new(delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; delay_samples.max(1)],
            write_pos: 0,
            feedback: 0.5,
            damp: 0.5,
            damp_state: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.write_pos];
        // One-pole low-pass damping on feedback
        self.damp_state = output * (1.0 - self.damp) + self.damp_state * self.damp;
        self.buffer[self.write_pos] = input + self.damp_state * self.feedback;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        output
    }

    fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback;
    }

    fn set_damp(&mut self, damp: f32) {
        self.damp = damp;
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
        self.damp_state = 0.0;
    }
}

/// Allpass filter.
#[derive(Debug, Clone)]
pub(crate) struct AllpassFilter {
    buffer: Vec<f32>,
    write_pos: usize,
}

impl AllpassFilter {
    fn new(delay_samples: usize) -> Self {
        Self {
            buffer: vec![0.0; delay_samples.max(1)],
            write_pos: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.write_pos];
        self.buffer[self.write_pos] = input + buffered * ALLPASS_FEEDBACK;
        self.write_pos = (self.write_pos + 1) % self.buffer.len();
        -input + buffered
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.write_pos = 0;
    }
}

/// Scale a base delay length from 44100 Hz to the target sample rate.
fn scale_delay(base: usize, sample_rate: u32) -> usize {
    ((base as f64 * sample_rate as f64 / 44100.0).round() as usize).max(1)
}

/// Schroeder/Freeverb reverb processor.
#[must_use]
#[derive(Debug, Clone)]
pub struct Reverb {
    /// Left channel comb filters.
    combs_l: Vec<CombFilter>,
    /// Right channel comb filters (with stereo offset).
    combs_r: Vec<CombFilter>,
    /// Left channel allpass filters.
    allpasses_l: Vec<AllpassFilter>,
    /// Right channel allpass filters.
    allpasses_r: Vec<AllpassFilter>,
    params: ReverbParams,
    bypassed: bool,
}

impl Reverb {
    /// Create a new reverb processor. Returns an error if parameters are invalid.
    pub fn new(params: ReverbParams, sample_rate: u32) -> crate::Result<Self> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "ReverbParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        let combs_l: Vec<CombFilter> = COMB_DELAYS
            .iter()
            .map(|&d| CombFilter::new(scale_delay(d, sample_rate)))
            .collect();
        let combs_r: Vec<CombFilter> = COMB_DELAYS
            .iter()
            .map(|&d| CombFilter::new(scale_delay(d + STEREO_OFFSET, sample_rate)))
            .collect();
        let allpasses_l: Vec<AllpassFilter> = ALLPASS_DELAYS
            .iter()
            .map(|&d| AllpassFilter::new(scale_delay(d, sample_rate)))
            .collect();
        let allpasses_r: Vec<AllpassFilter> = ALLPASS_DELAYS
            .iter()
            .map(|&d| AllpassFilter::new(scale_delay(d + STEREO_OFFSET, sample_rate)))
            .collect();

        tracing::debug!(
            sample_rate,
            room_size = params.room_size,
            damping = params.damping,
            "Reverb: created"
        );
        let mut reverb = Self {
            combs_l,
            combs_r,
            allpasses_l,
            allpasses_r,
            params: params.clone(),
            bypassed: false,
        };
        reverb.update_params(&params);
        Ok(reverb)
    }

    /// Process an audio buffer in-place.
    ///
    /// Mono buffers are processed through the left channel path only.
    /// Stereo and multichannel buffers use L/R decorrelated paths.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.bypassed {
            return;
        }
        let ch = buf.channels as usize;
        let dry = 1.0 - self.params.mix;
        let wet = self.params.mix;

        for frame in 0..buf.frames {
            if ch == 1 {
                // Mono: process through left path
                let input = buf.samples[frame];
                let reverb_out = self.process_mono(input);
                buf.samples[frame] = input * dry + reverb_out * wet;
                if !buf.samples[frame].is_finite() {
                    buf.samples[frame] = 0.0;
                }
            } else {
                // Stereo or multichannel: use L/R paths
                let l_idx = frame * ch;
                let r_idx = frame * ch + 1;
                let input_l = buf.samples[l_idx];
                let input_r = buf.samples[r_idx];

                let (rev_l, rev_r) = self.process_stereo(input_l, input_r);
                buf.samples[l_idx] = input_l * dry + rev_l * wet;
                buf.samples[r_idx] = input_r * dry + rev_r * wet;
                if !buf.samples[l_idx].is_finite() {
                    buf.samples[l_idx] = 0.0;
                }
                if !buf.samples[r_idx].is_finite() {
                    buf.samples[r_idx] = 0.0;
                }

                // Additional channels pass through unchanged
            }
        }
    }

    fn process_mono(&mut self, input: f32) -> f32 {
        let mut comb_sum = 0.0f32;
        for comb in &mut self.combs_l {
            comb_sum += comb.process(input);
        }
        comb_sum *= 0.25; // Scale by 1/num_combs

        let mut out = comb_sum;
        for ap in &mut self.allpasses_l {
            out = ap.process(out);
        }
        out
    }

    fn process_stereo(&mut self, input_l: f32, input_r: f32) -> (f32, f32) {
        let mut comb_sum_l = 0.0f32;
        let mut comb_sum_r = 0.0f32;

        for comb in &mut self.combs_l {
            comb_sum_l += comb.process(input_l);
        }
        for comb in &mut self.combs_r {
            comb_sum_r += comb.process(input_r);
        }

        comb_sum_l *= 0.25;
        comb_sum_r *= 0.25;

        let mut out_l = comb_sum_l;
        let mut out_r = comb_sum_r;
        for ap in &mut self.allpasses_l {
            out_l = ap.process(out_l);
        }
        for ap in &mut self.allpasses_r {
            out_r = ap.process(out_r);
        }

        (out_l, out_r)
    }

    fn update_params(&mut self, params: &ReverbParams) {
        let feedback = params.room_size.clamp(0.0, 1.0) * 0.9 + 0.05;
        let damp = params.damping.clamp(0.0, 1.0);

        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.set_feedback(feedback);
            comb.set_damp(damp);
        }
    }

    /// Set whether this reverb is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this reverb is currently bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Update reverb parameters. Returns an error if parameters are invalid.
    pub fn set_params(&mut self, params: ReverbParams) -> crate::Result<()> {
        params
            .validate()
            .map_err(|reason| crate::NadaError::InvalidParameter {
                name: "ReverbParams".into(),
                value: String::new(),
                reason: reason.into(),
            })?;
        self.update_params(&params);
        self.params = params;
        Ok(())
    }

    /// Rebuild the reverb for a new sample rate.
    ///
    /// This reallocates internal buffers — call during session changes, not per-buffer.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        tracing::debug!(sample_rate, "Reverb: sample rate updated");
        // Rebuild with current params at new rate (ignore Result — params already validated)
        let was_bypassed = self.bypassed;
        if let Ok(new) = Self::new(self.params.clone(), sample_rate) {
            *self = new;
        }
        self.bypassed = was_bypassed;
    }

    /// Reset all internal state.
    pub fn reset(&mut self) {
        for comb in self.combs_l.iter_mut().chain(self.combs_r.iter_mut()) {
            comb.reset();
        }
        for ap in self
            .allpasses_l
            .iter_mut()
            .chain(self.allpasses_r.iter_mut())
        {
            ap.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_in_silence_out() {
        let mut reverb = Reverb::new(ReverbParams::default(), 44100).unwrap();
        let mut buf = AudioBuffer::silence(2, 4096, 44100);
        reverb.process(&mut buf);
        assert!(buf.peak() < f32::EPSILON);
    }

    #[test]
    fn impulse_produces_tail() {
        let mut reverb = Reverb::new(
            ReverbParams {
                room_size: 0.8,
                damping: 0.3,
                mix: 1.0,
            },
            44100,
        )
        .unwrap();

        // Impulse at frame 0
        let mut samples = vec![0.0f32; 44100 * 2]; // 1 second stereo
        samples[0] = 1.0; // L
        samples[1] = 1.0; // R
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();

        reverb.process(&mut buf);

        // Check that there's reverb tail energy after the first few thousand samples
        let tail_rms: f32 = {
            let tail = &buf.samples[4000..];
            let sum_sq: f64 = tail.iter().map(|s| (*s as f64) * (*s as f64)).sum();
            (sum_sq / tail.len() as f64).sqrt() as f32
        };
        assert!(
            tail_rms > 0.001,
            "Reverb should produce a tail, got RMS={tail_rms}"
        );
    }

    #[test]
    fn zero_room_size_minimal_reverb() {
        let mut reverb = Reverb::new(
            ReverbParams {
                room_size: 0.0,
                damping: 1.0,
                mix: 1.0,
            },
            44100,
        )
        .unwrap();

        let mut samples = vec![0.0f32; 44100 * 2];
        samples[0] = 1.0;
        samples[1] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        reverb.process(&mut buf);

        // With room_size=0 and damping=1, reverb tail should decay quickly
        let late_tail_rms: f32 = {
            let tail = &buf.samples[20000..];
            let sum_sq: f64 = tail.iter().map(|s| (*s as f64) * (*s as f64)).sum();
            (sum_sq / tail.len() as f64).sqrt() as f32
        };
        assert!(
            late_tail_rms < 0.01,
            "Minimal reverb should have short tail"
        );
    }

    #[test]
    fn mono_processing() {
        let mut reverb = Reverb::new(ReverbParams::default(), 44100).unwrap();
        let mut samples = vec![0.0f32; 4096];
        samples[0] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        reverb.process(&mut buf);
        assert!(buf.samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn reset_clears_tail() {
        let mut reverb = Reverb::new(
            ReverbParams {
                room_size: 0.9,
                damping: 0.2,
                mix: 1.0,
            },
            44100,
        )
        .unwrap();

        // Feed an impulse
        let mut samples = vec![0.0f32; 2048];
        samples[0] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        reverb.process(&mut buf);

        reverb.reset();

        // After reset, processing silence should produce silence
        let mut silence = AudioBuffer::silence(1, 1024, 44100);
        reverb.process(&mut silence);
        assert!(silence.peak() < f32::EPSILON);
    }

    #[test]
    fn different_sample_rates() {
        // Should not panic at various sample rates
        for sr in [22050, 44100, 48000, 96000] {
            let mut reverb = Reverb::new(ReverbParams::default(), sr).unwrap();
            let mut buf = AudioBuffer::silence(2, 1024, sr);
            buf.samples[0] = 1.0;
            buf.samples[1] = 1.0;
            reverb.process(&mut buf);
            assert!(buf.samples.iter().all(|s| s.is_finite()));
        }
    }
}
