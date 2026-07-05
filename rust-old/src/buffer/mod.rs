//! Audio buffer types — unified sample buffers with format awareness.
//!
//! Supports interleaved and planar layouts, multiple sample formats,
//! and zero-copy views for read-only processing.

pub mod convert;
pub mod dither;
pub mod ops;
pub mod resample;

use serde::{Deserialize, Serialize};

use crate::NadaError;

/// Sample format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SampleFormat {
    /// 32-bit float (-1.0 to 1.0, standard for processing)
    F32,
    /// 16-bit signed integer (-32768 to 32767, CD quality)
    I16,
    /// 32-bit signed integer (high-resolution audio)
    I32,
    /// 24-bit signed integer (professional audio)
    I24,
    /// 64-bit float (high-precision processing)
    F64,
    /// Unsigned 8-bit (legacy formats)
    U8,
}

impl SampleFormat {
    /// Bytes per sample.
    #[inline]
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            Self::F32 => 4,
            Self::I16 => 2,
            Self::I32 => 4,
            Self::I24 => 3,
            Self::F64 => 8,
            Self::U8 => 1,
        }
    }
}

impl std::fmt::Display for SampleFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::F32 => write!(f, "f32"),
            Self::I16 => write!(f, "i16"),
            Self::I32 => write!(f, "i32"),
            Self::I24 => write!(f, "i24"),
            Self::F64 => write!(f, "f64"),
            Self::U8 => write!(f, "u8"),
        }
    }
}

/// Buffer layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Layout {
    /// Samples interleaved: [L0, R0, L1, R1, ...]
    Interleaved,
    /// Samples planar: [L0, L1, ...] [R0, R1, ...]
    Planar,
}

/// An audio buffer holding sample data in a known format.
///
/// Use accessor methods (`samples()`, `channels()`, etc.) to read fields.
/// Use `samples_mut()` for in-place processing.
#[must_use]
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    /// Raw sample data (f32 internally, converted on input/output).
    pub(crate) samples: Vec<f32>,
    /// Number of channels.
    pub(crate) channels: u32,
    /// Sample rate in Hz.
    pub(crate) sample_rate: u32,
    /// Number of frames (samples per channel).
    pub(crate) frames: usize,
}

// Accessor methods — use these instead of direct field access.
// Fields will become private in v0.21.3.
impl AudioBuffer {
    /// Immutable reference to the raw sample data.
    #[inline]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Mutable reference to the raw sample data.
    #[inline]
    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    /// Number of channels.
    #[inline]
    pub fn channels(&self) -> u32 {
        self.channels
    }

    /// Sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Number of frames (samples per channel).
    #[inline]
    pub fn frames(&self) -> usize {
        self.frames
    }
}

impl AudioBuffer {
    /// Create a new buffer from f32 interleaved samples.
    pub fn from_interleaved(
        samples: Vec<f32>,
        channels: u32,
        sample_rate: u32,
    ) -> Result<Self, NadaError> {
        if channels == 0 {
            tracing::warn!(channels, "AudioBuffer: invalid channel count");
            return Err(NadaError::InvalidChannels(0));
        }
        if sample_rate == 0 || sample_rate > 768000 {
            tracing::warn!(sample_rate, "AudioBuffer: invalid sample rate");
            return Err(NadaError::InvalidSampleRate(sample_rate));
        }
        let ch = channels as usize;
        if !samples.is_empty() && !samples.len().is_multiple_of(ch) {
            tracing::warn!(
                len = samples.len(),
                channels,
                "AudioBuffer: sample count not divisible by channels"
            );
            return Err(NadaError::LengthMismatch {
                expected: (samples.len() / ch) * ch,
                actual: samples.len(),
            });
        }
        let frames = samples.len() / ch;
        Ok(Self {
            samples,
            channels,
            sample_rate,
            frames,
        })
    }

    /// Create a silent buffer with the given dimensions.
    ///
    /// # Panics
    ///
    /// Debug-asserts that `channels > 0` and `sample_rate > 0`. In release mode,
    /// zero values are accepted to avoid panics on the RT path.
    pub fn silence(channels: u32, frames: usize, sample_rate: u32) -> Self {
        debug_assert!(channels > 0, "AudioBuffer::silence: channels must be > 0");
        debug_assert!(
            sample_rate > 0,
            "AudioBuffer::silence: sample_rate must be > 0"
        );
        Self {
            samples: vec![0.0; channels as usize * frames],
            channels,
            sample_rate,
            frames,
        }
    }

    /// Duration of this buffer in seconds.
    #[inline]
    pub fn duration_secs(&self) -> f64 {
        self.frames as f64 / self.sample_rate as f64
    }

    /// Total number of samples (frames * channels).
    #[inline]
    pub fn total_samples(&self) -> usize {
        self.samples.len()
    }

    /// Peak amplitude across all channels.
    #[inline]
    pub fn peak(&self) -> f32 {
        #[cfg(feature = "simd")]
        {
            crate::simd::peak_abs(&self.samples)
        }
        #[cfg(not(feature = "simd"))]
        {
            self.samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
        }
    }

    /// RMS (root mean square) level.
    #[inline]
    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        #[cfg(feature = "simd")]
        {
            let sum_sq = crate::simd::sum_of_squares(&self.samples);
            (sum_sq / self.samples.len() as f64).sqrt() as f32
        }
        #[cfg(not(feature = "simd"))]
        {
            let sum_sq: f64 = self.samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
            (sum_sq / self.samples.len() as f64).sqrt() as f32
        }
    }

    /// Apply gain (multiply all samples by factor).
    pub fn apply_gain(&mut self, gain: f32) {
        #[cfg(feature = "simd")]
        {
            crate::simd::apply_gain(&mut self.samples, gain);
        }
        #[cfg(not(feature = "simd"))]
        {
            for s in &mut self.samples {
                *s *= gain;
            }
        }
    }

    /// Clamp all samples to [-1.0, 1.0].
    pub fn clamp(&mut self) {
        #[cfg(feature = "simd")]
        {
            crate::simd::clamp(&mut self.samples, -1.0, 1.0);
        }
        #[cfg(not(feature = "simd"))]
        {
            for s in &mut self.samples {
                *s = s.clamp(-1.0, 1.0);
            }
        }
    }
}

/// Mix multiple buffers together (summing). All must have same channels and sample rate.
pub fn mix(buffers: &[&AudioBuffer]) -> Result<AudioBuffer, NadaError> {
    if buffers.is_empty() {
        return Ok(AudioBuffer::silence(2, 0, 44100));
    }

    let channels = buffers[0].channels;
    let sample_rate = buffers[0].sample_rate;
    let max_frames = buffers.iter().map(|b| b.frames).max().unwrap_or(0);

    for b in buffers {
        if b.channels != channels {
            return Err(NadaError::FormatMismatch {
                expected: format!("{} ch", channels),
                actual: format!("{} ch", b.channels),
            });
        }
        if b.sample_rate != sample_rate {
            return Err(NadaError::InvalidSampleRate(b.sample_rate));
        }
    }

    let total = max_frames * channels as usize;
    let mut mixed = vec![0.0f32; total];

    for buf in buffers {
        #[cfg(feature = "simd")]
        {
            crate::simd::add_buffers(&mut mixed, &buf.samples);
        }
        #[cfg(not(feature = "simd"))]
        {
            for (i, s) in buf.samples.iter().enumerate() {
                if i < total {
                    mixed[i] += s;
                }
            }
        }
    }

    Ok(AudioBuffer {
        samples: mixed,
        channels,
        sample_rate,
        frames: max_frames,
    })
}

/// Resample a buffer to a target sample rate using linear interpolation.
pub fn resample_linear(buf: &AudioBuffer, target_rate: u32) -> Result<AudioBuffer, NadaError> {
    if target_rate == 0 || target_rate > 768000 {
        return Err(NadaError::InvalidSampleRate(target_rate));
    }
    if target_rate == buf.sample_rate {
        return Ok(buf.clone());
    }
    if buf.frames == 0 {
        return Ok(AudioBuffer::silence(buf.channels, 0, target_rate));
    }

    let ratio = target_rate as f64 / buf.sample_rate as f64;
    let new_frames = (buf.frames as f64 * ratio).ceil() as usize;
    let ch = buf.channels as usize;
    let mut out = vec![0.0f32; new_frames * ch];

    for frame in 0..new_frames {
        let src_pos = frame as f64 / ratio;
        let src_frame = src_pos.floor() as usize;
        let frac = (src_pos - src_frame as f64) as f32;

        for c in 0..ch {
            let i0 = src_frame * ch + c;
            let i1 = ((src_frame + 1).min(buf.frames - 1)) * ch + c;
            let s0 = buf.samples.get(i0).copied().unwrap_or(0.0);
            let s1 = buf.samples.get(i1).copied().unwrap_or(0.0);
            out[frame * ch + c] = s0 + frac * (s1 - s0);
        }
    }

    Ok(AudioBuffer {
        samples: out,
        channels: buf.channels,
        sample_rate: target_rate,
        frames: new_frames,
    })
}

/// A read-only view into an audio buffer without copying sample data.
///
/// Borrows the sample slice from an existing [`AudioBuffer`], avoiding allocation
/// for analysis and metering paths that don't need to modify audio.
#[must_use]
#[derive(Debug)]
pub struct AudioBufferRef<'a> {
    samples: &'a [f32],
    channels: u32,
    sample_rate: u32,
    frames: usize,
}

impl<'a> AudioBufferRef<'a> {
    /// Create a read-only view of an existing buffer.
    pub fn from_buffer(buf: &'a AudioBuffer) -> Self {
        Self {
            samples: &buf.samples,
            channels: buf.channels,
            sample_rate: buf.sample_rate,
            frames: buf.frames,
        }
    }

    /// Create from a raw slice.
    pub fn from_slice(samples: &'a [f32], channels: u32, sample_rate: u32) -> Self {
        let frames = if channels > 0 {
            samples.len() / channels as usize
        } else {
            0
        };
        Self {
            samples,
            channels,
            sample_rate,
            frames,
        }
    }

    /// Immutable sample data.
    #[inline]
    pub fn samples(&self) -> &[f32] {
        self.samples
    }

    /// Number of channels.
    #[inline]
    pub fn channels(&self) -> u32 {
        self.channels
    }

    /// Sample rate in Hz.
    #[inline]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Number of frames.
    #[inline]
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// Peak amplitude.
    #[inline]
    pub fn peak(&self) -> f32 {
        self.samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
    }

    /// RMS level.
    #[inline]
    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f64 = self.samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
        (sum_sq / self.samples.len() as f64).sqrt() as f32
    }
}

/// A reusable pool of audio buffers to reduce allocation pressure.
///
/// Effects and graph nodes borrow buffers from the pool instead of
/// allocating fresh ones each cycle.
#[must_use]
#[derive(Debug)]
pub struct BufferPool {
    buffers: Vec<AudioBuffer>,
    channels: u32,
    frames: usize,
    sample_rate: u32,
}

impl BufferPool {
    /// Create a pool pre-loaded with `capacity` silent buffers.
    pub fn new(capacity: usize, channels: u32, frames: usize, sample_rate: u32) -> Self {
        let buffers = (0..capacity)
            .map(|_| AudioBuffer::silence(channels, frames, sample_rate))
            .collect();
        Self {
            buffers,
            channels,
            frames,
            sample_rate,
        }
    }

    /// Take a buffer from the pool. If the pool is empty, allocates a new one.
    pub fn acquire(&mut self) -> AudioBuffer {
        self.buffers
            .pop()
            .unwrap_or_else(|| AudioBuffer::silence(self.channels, self.frames, self.sample_rate))
    }

    /// Return a buffer to the pool for reuse. Silences it before storing.
    pub fn release(&mut self, mut buf: AudioBuffer) {
        buf.samples.fill(0.0);
        self.buffers.push(buf);
    }

    /// Number of available buffers in the pool.
    #[inline]
    pub fn available(&self) -> usize {
        self.buffers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_from_interleaved() {
        let samples = vec![0.5, -0.5, 0.3, -0.3];
        let buf = AudioBuffer::from_interleaved(samples, 2, 44100).unwrap();
        assert_eq!(buf.channels, 2);
        assert_eq!(buf.frames, 2);
        assert_eq!(buf.sample_rate, 44100);
    }

    #[test]
    fn buffer_silence() {
        let buf = AudioBuffer::silence(2, 1024, 48000);
        assert_eq!(buf.total_samples(), 2048);
        assert_eq!(buf.peak(), 0.0);
        assert_eq!(buf.rms(), 0.0);
    }

    #[test]
    fn buffer_duration() {
        let buf = AudioBuffer::silence(2, 44100, 44100);
        assert!((buf.duration_secs() - 1.0).abs() < 0.001);
    }

    #[test]
    fn buffer_gain() {
        let mut buf = AudioBuffer::from_interleaved(vec![1.0, -1.0], 1, 44100).unwrap();
        buf.apply_gain(0.5);
        assert!((buf.samples[0] - 0.5).abs() < f32::EPSILON);
        assert!((buf.samples[1] + 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn buffer_clamp() {
        let mut buf = AudioBuffer::from_interleaved(vec![2.0, -2.0], 1, 44100).unwrap();
        buf.clamp();
        assert_eq!(buf.samples[0], 1.0);
        assert_eq!(buf.samples[1], -1.0);
    }

    #[test]
    fn buffer_peak() {
        let buf = AudioBuffer::from_interleaved(vec![0.3, -0.7, 0.5, 0.1], 2, 44100).unwrap();
        assert!((buf.peak() - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn buffer_zero_channels_rejected() {
        let result = AudioBuffer::from_interleaved(vec![0.0], 0, 44100);
        assert!(result.is_err());
    }

    #[test]
    fn mix_two_buffers() {
        let a = AudioBuffer::from_interleaved(vec![0.5, 0.5], 1, 44100).unwrap();
        let b = AudioBuffer::from_interleaved(vec![0.3, 0.3], 1, 44100).unwrap();
        let mixed = mix(&[&a, &b]).unwrap();
        assert!((mixed.samples[0] - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn mix_channel_mismatch() {
        let a = AudioBuffer::silence(1, 100, 44100);
        let b = AudioBuffer::silence(2, 100, 44100);
        assert!(mix(&[&a, &b]).is_err());
    }

    #[test]
    fn resample_same_rate() {
        let buf = AudioBuffer::silence(2, 1000, 44100);
        let out = resample_linear(&buf, 44100).unwrap();
        assert_eq!(out.frames, 1000);
    }

    #[test]
    fn resample_double_rate() {
        let buf = AudioBuffer::from_interleaved(vec![1.0, 0.0, 1.0, 0.0], 1, 44100).unwrap();
        let out = resample_linear(&buf, 88200).unwrap();
        assert!(out.frames >= 7); // ~2x frames
        assert_eq!(out.sample_rate, 88200);
    }

    #[test]
    fn sample_format_bytes() {
        assert_eq!(SampleFormat::F32.bytes_per_sample(), 4);
        assert_eq!(SampleFormat::I16.bytes_per_sample(), 2);
        assert_eq!(SampleFormat::I32.bytes_per_sample(), 4);
    }

    #[test]
    fn sample_format_display() {
        assert_eq!(SampleFormat::F32.to_string(), "f32");
        assert_eq!(SampleFormat::I16.to_string(), "i16");
    }
}
