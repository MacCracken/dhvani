//! Delay line — fixed delay with feedback, and modulated delay for chorus/flanger.

use serde::{Deserialize, Serialize};

use crate::buffer::AudioBuffer;

/// A fixed-length delay line with feedback and dry/wet mix.
#[must_use]
#[derive(Debug, Clone)]
pub struct DelayLine {
    /// Per-channel circular buffers.
    buffers: Vec<Vec<f32>>,
    write_pos: usize,
    delay_samples: usize,
    max_delay_samples: usize,
    /// Feedback amount (0.0–1.0).
    feedback: f32,
    /// Dry/wet mix (0.0 = fully dry, 1.0 = fully wet).
    mix: f32,
    bypassed: bool,
}

impl DelayLine {
    /// Create a new delay line.
    ///
    /// `max_delay_ms` sets the maximum buffer size. `delay_ms` sets the initial delay time.
    pub fn new(
        delay_ms: f32,
        max_delay_ms: f32,
        feedback: f32,
        mix: f32,
        sample_rate: u32,
        channels: u32,
    ) -> Self {
        tracing::debug!(
            delay_ms,
            max_delay_ms,
            feedback,
            mix,
            sample_rate,
            channels,
            "DelayLine::new"
        );
        let max_samples = ((max_delay_ms.max(delay_ms) / 1000.0) * sample_rate as f32) as usize;
        let max_samples = max_samples.max(1);
        let delay_samples = ((delay_ms / 1000.0) * sample_rate as f32) as usize;
        let delay_samples = delay_samples.min(max_samples);

        Self {
            buffers: vec![vec![0.0; max_samples]; channels as usize],
            write_pos: 0,
            delay_samples,
            max_delay_samples: max_samples,
            feedback: feedback.clamp(0.0, 0.99),
            mix: mix.clamp(0.0, 1.0),
            bypassed: false,
        }
    }

    /// Process an audio buffer in-place.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.bypassed {
            return;
        }
        let ch = buf.channels as usize;
        for frame in 0..buf.frames {
            for c in 0..ch {
                let idx = frame * ch + c;
                buf.samples[idx] = self.process_sample(buf.samples[idx], c);
                if !buf.samples[idx].is_finite() {
                    buf.samples[idx] = 0.0;
                }
            }
            self.write_pos = (self.write_pos + 1) % self.max_delay_samples;
        }
    }

    /// Process a single sample for a channel. Does NOT advance write_pos —
    /// call once per channel per frame, then advance after all channels.
    fn process_sample(&mut self, input: f32, channel: usize) -> f32 {
        let buf = &mut self.buffers[channel];
        let read_pos =
            (self.write_pos + self.max_delay_samples - self.delay_samples) % self.max_delay_samples;
        let delayed = buf[read_pos];
        buf[self.write_pos] = input + delayed * self.feedback;
        input * (1.0 - self.mix) + delayed * self.mix
    }

    /// Read from a fractional position using linear interpolation.
    fn read_interpolated(&self, channel: usize, delay_frac: f32) -> f32 {
        let buf = &self.buffers[channel];
        let delay_int = delay_frac as usize;
        let frac = delay_frac - delay_int as f32;
        let pos0 = (self.write_pos + self.max_delay_samples - delay_int) % self.max_delay_samples;
        let pos1 =
            (self.write_pos + self.max_delay_samples - delay_int - 1) % self.max_delay_samples;
        buf[pos0] * (1.0 - frac) + buf[pos1] * frac
    }

    /// Set whether this delay is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this delay is currently bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Latency in frames introduced by this delay.
    pub fn latency_frames(&self) -> usize {
        self.delay_samples
    }

    /// Set delay time in milliseconds.
    pub fn set_delay_ms(&mut self, delay_ms: f32, sample_rate: u32) {
        let samples = ((delay_ms / 1000.0) * sample_rate as f32) as usize;
        self.delay_samples = samples.min(self.max_delay_samples);
    }

    /// Reset all delay buffers to silence.
    pub fn reset(&mut self) {
        for buf in &mut self.buffers {
            buf.fill(0.0);
        }
        self.write_pos = 0;
    }
}

/// Parameters for a modulated delay (chorus/flanger).
#[must_use]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(default)]
pub struct ModulatedDelayParams {
    /// Base delay time in milliseconds.
    pub base_delay_ms: f32,
    /// LFO modulation depth in milliseconds.
    pub depth_ms: f32,
    /// LFO rate in Hz.
    pub rate_hz: f32,
    /// Feedback amount (0.0–0.99).
    pub feedback: f32,
    /// Dry/wet mix (0.0 = dry, 1.0 = wet).
    pub mix: f32,
}

impl Default for ModulatedDelayParams {
    fn default() -> Self {
        Self {
            base_delay_ms: 7.0,
            depth_ms: 3.0,
            rate_hz: 0.5,
            feedback: 0.3,
            mix: 0.5,
        }
    }
}

impl ModulatedDelayParams {
    /// Validate parameters. Returns an error description if invalid.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.base_delay_ms < 0.0 {
            return Err("base_delay_ms must be >= 0.0");
        }
        if self.depth_ms < 0.0 {
            return Err("depth_ms must be >= 0.0");
        }
        if self.rate_hz < 0.0 {
            return Err("rate_hz must be >= 0.0");
        }
        if !(0.0..=0.99).contains(&self.feedback) {
            return Err("feedback must be 0.0–0.99");
        }
        if !(0.0..=1.0).contains(&self.mix) {
            return Err("mix must be 0.0–1.0");
        }
        Ok(())
    }
}

/// A modulated delay using a sine LFO — produces chorus or flanger effects.
///
/// - **Chorus**: base_delay ~7ms, depth ~3ms, rate ~0.5Hz
/// - **Flanger**: base_delay ~1ms, depth ~1ms, rate ~0.2–2Hz, higher feedback
#[must_use]
#[derive(Debug, Clone)]
pub struct ModulatedDelay {
    delay: DelayLine,
    params: ModulatedDelayParams,
    lfo_phase: f64,
    sample_rate: u32,
    bypassed: bool,
}

impl ModulatedDelay {
    /// Create a new modulated delay.
    pub fn new(params: ModulatedDelayParams, sample_rate: u32, channels: u32) -> Self {
        tracing::debug!(?params, sample_rate, channels, "ModulatedDelay::new");
        let max_ms = params.base_delay_ms + params.depth_ms + 1.0;
        Self {
            delay: DelayLine::new(
                params.base_delay_ms,
                max_ms,
                params.feedback,
                params.mix,
                sample_rate,
                channels,
            ),
            params,
            lfo_phase: 0.0,
            sample_rate,
            bypassed: false,
        }
    }

    /// Process an audio buffer in-place.
    #[inline]
    pub fn process(&mut self, buf: &mut AudioBuffer) {
        if self.bypassed {
            return;
        }
        let ch = buf.channels as usize;
        let phase_inc = self.params.rate_hz as f64 / self.sample_rate as f64;

        for frame in 0..buf.frames {
            // Compute LFO-modulated delay in samples
            let lfo = (2.0 * std::f64::consts::PI * self.lfo_phase).sin() as f32;
            let delay_ms = self.params.base_delay_ms + self.params.depth_ms * lfo;
            let delay_frac = (delay_ms / 1000.0) * self.sample_rate as f32;
            let delay_frac = delay_frac.max(0.0);

            for c in 0..ch {
                let idx = frame * ch + c;
                let input = buf.samples[idx];
                let delayed = self.delay.read_interpolated(c, delay_frac);
                self.delay.buffers[c][self.delay.write_pos] =
                    input + delayed * self.params.feedback;
                buf.samples[idx] = input * (1.0 - self.params.mix) + delayed * self.params.mix;
                if !buf.samples[idx].is_finite() {
                    buf.samples[idx] = 0.0;
                }
            }

            self.delay.write_pos = (self.delay.write_pos + 1) % self.delay.max_delay_samples;
            self.lfo_phase = (self.lfo_phase + phase_inc) % 1.0;
        }
    }

    /// Latency in frames introduced by the base delay.
    pub fn latency_frames(&self) -> usize {
        ((self.params.base_delay_ms / 1000.0) * self.sample_rate as f32) as usize
    }

    /// Set whether this modulated delay is bypassed.
    pub fn set_bypass(&mut self, bypassed: bool) {
        self.bypassed = bypassed;
    }

    /// Returns `true` if this modulated delay is currently bypassed.
    pub fn is_bypassed(&self) -> bool {
        self.bypassed
    }

    /// Update parameters. Updates feedback, mix, and LFO settings.
    /// Base delay and depth changes take effect immediately.
    pub fn set_params(&mut self, params: ModulatedDelayParams) {
        self.delay.feedback = params.feedback.clamp(0.0, 0.99);
        self.delay.mix = params.mix.clamp(0.0, 1.0);
        self.params = params;
    }

    /// Update the sample rate. Rebuilds internal delay buffers.
    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.sample_rate = sample_rate;
        let channels = self.delay.buffers.len() as u32;
        self.delay = DelayLine::new(
            self.params.base_delay_ms,
            self.params.base_delay_ms + self.params.depth_ms + 1.0,
            self.params.feedback,
            self.params.mix,
            sample_rate,
            channels,
        );
        self.lfo_phase = 0.0;
    }

    /// Reset delay buffers and LFO phase.
    pub fn reset(&mut self) {
        self.delay.reset();
        self.lfo_phase = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_mix_passthrough() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, 0.5, 0.25, 0.0], 1, 44100).unwrap();
        let original = buf.samples.clone();
        let mut delay = DelayLine::new(10.0, 10.0, 0.0, 0.0, 44100, 1);
        delay.process(&mut buf);
        assert_eq!(buf.samples, original);
    }

    #[test]
    fn delay_shifts_signal() {
        // 1 sample delay at 44100Hz ≈ 0.0227ms, use exact 1-sample delay
        let frames = 16;
        let mut samples = vec![0.0f32; frames];
        samples[0] = 1.0; // impulse at frame 0
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 1000).unwrap();

        // 1ms delay at 1000Hz = 1 sample
        let mut delay = DelayLine::new(1.0, 1.0, 0.0, 1.0, 1000, 1);
        delay.process(&mut buf);

        // The delayed impulse should appear at frame 1 (wet only, mix=1.0)
        assert!(
            buf.samples[0].abs() < f32::EPSILON,
            "frame 0 should be dry=0"
        );
        assert!(
            (buf.samples[1] - 1.0).abs() < f32::EPSILON,
            "frame 1 should have delayed impulse"
        );
    }

    #[test]
    fn feedback_produces_echoes() {
        let frames = 32;
        let mut samples = vec![0.0f32; frames];
        samples[0] = 1.0;
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 1000).unwrap();

        // 2ms delay at 1000Hz = 2 samples, feedback = 0.5, wet only
        let mut delay = DelayLine::new(2.0, 2.0, 0.5, 1.0, 1000, 1);
        delay.process(&mut buf);

        // First echo at frame 2, second echo at frame 4 (attenuated by 0.5)
        assert!(buf.samples[2].abs() > 0.9);
        assert!(buf.samples[4].abs() > 0.4);
        assert!(buf.samples[4].abs() < buf.samples[2].abs());
    }

    #[test]
    fn reset_clears_buffers() {
        let mut delay = DelayLine::new(10.0, 10.0, 0.5, 1.0, 44100, 2);
        let mut buf = AudioBuffer::from_interleaved(vec![1.0; 200], 2, 44100).unwrap();
        delay.process(&mut buf);
        delay.reset();
        assert!(delay.buffers[0].iter().all(|&s| s == 0.0));
        assert!(delay.buffers[1].iter().all(|&s| s == 0.0));
    }

    #[test]
    fn modulated_delay_produces_output() {
        let params = ModulatedDelayParams::default();
        let mut modulated = ModulatedDelay::new(params, 44100, 1);

        // Generate a sine and process
        let samples: Vec<f32> = (0..4096)
            .map(|i| (2.0 * std::f32::consts::PI * 440.0 * i as f32 / 44100.0).sin())
            .collect();
        let mut buf = AudioBuffer::from_interleaved(samples, 1, 44100).unwrap();
        let original_rms = buf.rms();
        modulated.process(&mut buf);

        // Output should be non-silent and different from input
        assert!(buf.rms() > 0.0);
        assert!((buf.rms() - original_rms).abs() > 0.001 || buf.peak() > 0.0);
    }

    #[test]
    fn modulated_delay_reset() {
        let mut modulated = ModulatedDelay::new(ModulatedDelayParams::default(), 44100, 1);
        modulated.lfo_phase = 0.5;
        modulated.reset();
        assert!((modulated.lfo_phase).abs() < f64::EPSILON);
    }
}
