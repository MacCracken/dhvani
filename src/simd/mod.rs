//! SIMD-accelerated audio processing kernels.
//!
//! Provides platform-specific SIMD implementations for hot-path audio operations.
//! Falls back to scalar code when SIMD is not available or the `simd` feature is disabled.
//!
//! - **x86_64**: SSE2 (baseline, 4 f32/op) + AVX2 (runtime-detected, 8 f32/op)
//! - **aarch64**: NEON (baseline, 4 f32/op)

#[cfg(target_arch = "x86_64")]
#[allow(unused_unsafe, clippy::needless_range_loop)]
mod x86;

#[cfg(target_arch = "aarch64")]
mod aarch64;

// ── Platform dispatch ───────────────────────────────────────────────

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn add_buffers(dst: &mut [f32], src: &[f32]) {
    x86::add_buffers(dst, src)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn add_buffers(dst: &mut [f32], src: &[f32]) {
    aarch64::add_buffers(dst, src)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn add_buffers(dst: &mut [f32], src: &[f32]) {
    add_buffers_scalar(dst, src)
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn apply_gain(samples: &mut [f32], gain: f32) {
    x86::apply_gain(samples, gain)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn apply_gain(samples: &mut [f32], gain: f32) {
    aarch64::apply_gain(samples, gain)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn apply_gain(samples: &mut [f32], gain: f32) {
    apply_gain_scalar(samples, gain)
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn clamp(samples: &mut [f32], min: f32, max: f32) {
    x86::clamp(samples, min, max)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn clamp(samples: &mut [f32], min: f32, max: f32) {
    aarch64::clamp(samples, min, max)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn clamp(samples: &mut [f32], min: f32, max: f32) {
    clamp_scalar(samples, min, max)
}

#[cfg(target_arch = "x86_64")]
#[must_use]
#[inline]
pub fn peak_abs(samples: &[f32]) -> f32 {
    x86::peak_abs(samples)
}
#[cfg(target_arch = "aarch64")]
#[must_use]
#[inline]
pub fn peak_abs(samples: &[f32]) -> f32 {
    aarch64::peak_abs(samples)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[must_use]
#[inline]
pub fn peak_abs(samples: &[f32]) -> f32 {
    peak_abs_scalar(samples)
}

#[cfg(target_arch = "x86_64")]
#[must_use]
#[inline]
pub fn sum_of_squares(samples: &[f32]) -> f64 {
    x86::sum_of_squares(samples)
}
#[cfg(target_arch = "aarch64")]
#[must_use]
#[inline]
pub fn sum_of_squares(samples: &[f32]) -> f64 {
    aarch64::sum_of_squares(samples)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[must_use]
#[inline]
pub fn sum_of_squares(samples: &[f32]) -> f64 {
    sum_of_squares_scalar(samples)
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn noise_gate(samples: &mut [f32], threshold: f32) {
    x86::noise_gate(samples, threshold)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn noise_gate(samples: &mut [f32], threshold: f32) {
    aarch64::noise_gate(samples, threshold)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn noise_gate(samples: &mut [f32], threshold: f32) {
    noise_gate_scalar(samples, threshold)
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn i16_to_f32(src: &[i16], dst: &mut [f32]) {
    x86::i16_to_f32(src, dst)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn i16_to_f32(src: &[i16], dst: &mut [f32]) {
    aarch64::i16_to_f32(src, dst)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn i16_to_f32(src: &[i16], dst: &mut [f32]) {
    i16_to_f32_scalar(src, dst)
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn f32_to_i16(src: &[f32], dst: &mut [i16]) {
    x86::f32_to_i16(src, dst)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn f32_to_i16(src: &[f32], dst: &mut [i16]) {
    aarch64::f32_to_i16(src, dst)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn f32_to_i16(src: &[f32], dst: &mut [i16]) {
    f32_to_i16_scalar(src, dst)
}

/// Weighted dot product: sum(samples[i] * weights[i]) for pre-computed sinc kernels.
/// Returns (weighted_sum, weight_sum) for normalization.
#[cfg(target_arch = "x86_64")]
#[must_use]
#[inline]
pub fn weighted_sum(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    x86::weighted_sum(samples, weights)
}
#[cfg(target_arch = "aarch64")]
#[must_use]
#[inline]
pub fn weighted_sum(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    aarch64::weighted_sum(samples, weights)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[must_use]
#[inline]
pub fn weighted_sum(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    weighted_sum_scalar(samples, weights)
}

// ── 24-bit conversion ──────────────────────────────────────────────

/// Convert i24 (stored as i32, lower 24 bits) to f32.
/// Sign-extends from 24 bits then scales to [-1.0, 1.0).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn i24_to_f32(src: &[i32], dst: &mut [f32]) {
    x86::i24_to_f32(src, dst)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn i24_to_f32(src: &[i32], dst: &mut [f32]) {
    aarch64::i24_to_f32(src, dst)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn i24_to_f32(src: &[i32], dst: &mut [f32]) {
    i24_to_f32_scalar(src, dst)
}

/// Convert f32 to i24 (stored as i32, clamped to 24-bit range).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn f32_to_i24(src: &[f32], dst: &mut [i32]) {
    x86::f32_to_i24(src, dst)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn f32_to_i24(src: &[f32], dst: &mut [i32]) {
    aarch64::f32_to_i24(src, dst)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn f32_to_i24(src: &[f32], dst: &mut [i32]) {
    f32_to_i24_scalar(src, dst)
}

// ── u8 conversion ──────────────────────────────────────────────────

/// Convert unsigned 8-bit PCM to f32. u8 [0,255] → f32 [-1.0, 1.0).
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn u8_to_f32(src: &[u8], dst: &mut [f32]) {
    x86::u8_to_f32(src, dst)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn u8_to_f32(src: &[u8], dst: &mut [f32]) {
    aarch64::u8_to_f32(src, dst)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn u8_to_f32(src: &[u8], dst: &mut [f32]) {
    u8_to_f32_scalar(src, dst)
}

/// Convert f32 to unsigned 8-bit PCM. f32 [-1.0, 1.0] → u8 [0, 255].
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn f32_to_u8(src: &[f32], dst: &mut [u8]) {
    x86::f32_to_u8(src, dst)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn f32_to_u8(src: &[f32], dst: &mut [u8]) {
    aarch64::f32_to_u8(src, dst)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn f32_to_u8(src: &[f32], dst: &mut [u8]) {
    f32_to_u8_scalar(src, dst)
}

// ── Stereo biquad (2×f64 cross-channel) ────────────────────────────

/// Process stereo interleaved samples through a biquad filter using SIMD.
///
/// Processes L and R channels simultaneously using 2×f64 SIMD registers.
/// `coeffs` = [b0, b1, b2, a1, a2], `state` = [z1_L, z2_L, z1_R, z2_R].
#[cfg(target_arch = "x86_64")]
#[inline]
pub fn biquad_stereo(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    x86::biquad_stereo(samples, coeffs, state)
}
#[cfg(target_arch = "aarch64")]
#[inline]
pub fn biquad_stereo(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    aarch64::biquad_stereo(samples, coeffs, state)
}
#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
#[inline]
pub fn biquad_stereo(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    biquad_stereo_scalar(samples, coeffs, state)
}

// ── Scalar fallbacks ────────────────────────────────────────────────

#[allow(dead_code)]
fn add_buffers_scalar(dst: &mut [f32], src: &[f32]) {
    let len = dst.len().min(src.len());
    for i in 0..len {
        dst[i] += src[i];
    }
}

#[allow(dead_code)]
fn apply_gain_scalar(samples: &mut [f32], gain: f32) {
    for s in samples.iter_mut() {
        *s *= gain;
    }
}

#[allow(dead_code)]
fn clamp_scalar(samples: &mut [f32], min: f32, max: f32) {
    for s in samples.iter_mut() {
        *s = s.clamp(min, max);
    }
}

#[allow(dead_code)]
fn peak_abs_scalar(samples: &[f32]) -> f32 {
    samples.iter().map(|s| s.abs()).fold(0.0f32, f32::max)
}

#[allow(dead_code)]
fn sum_of_squares_scalar(samples: &[f32]) -> f64 {
    samples.iter().map(|s| (*s as f64) * (*s as f64)).sum()
}

#[allow(dead_code)]
fn noise_gate_scalar(samples: &mut [f32], threshold: f32) {
    for s in samples.iter_mut() {
        if s.abs() < threshold {
            *s = 0.0;
        }
    }
}

#[allow(dead_code)]
fn i16_to_f32_scalar(src: &[i16], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = src[i] as f32 / 32768.0;
    }
}

#[allow(dead_code)]
fn weighted_sum_scalar(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    let len = samples.len().min(weights.len());
    let mut sum = 0.0f32;
    let mut weight_sum = 0.0f32;
    for i in 0..len {
        sum += samples[i] * weights[i];
        weight_sum += weights[i];
    }
    (sum, weight_sum)
}

#[allow(dead_code)]
fn f32_to_i16_scalar(src: &[f32], dst: &mut [i16]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = (src[i].clamp(-1.0, 1.0) * 32767.0) as i16;
    }
}

#[allow(dead_code)]
fn i24_to_f32_scalar(src: &[i32], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        let extended = (src[i] << 8) >> 8;
        dst[i] = extended as f32 / 8388608.0;
    }
}

#[allow(dead_code)]
fn f32_to_i24_scalar(src: &[f32], dst: &mut [i32]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 8388607.0) as i32;
    }
}

#[allow(dead_code)]
fn u8_to_f32_scalar(src: &[u8], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = (f32::from(src[i]) - 128.0) / 128.0;
    }
}

#[allow(dead_code)]
fn f32_to_u8_scalar(src: &[f32], dst: &mut [u8]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = ((clamped * 128.0) + 128.0).clamp(0.0, 255.0) as u8;
    }
}

#[allow(dead_code)]
fn biquad_stereo_scalar(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    let [b0, b1, b2, a1, a2] = *coeffs;
    let frames = samples.len() / 2;
    for f in 0..frames {
        let idx = f * 2;
        // Left channel
        let in_l = samples[idx] as f64;
        let out_l = b0 * in_l + state[0];
        state[0] = b1 * in_l - a1 * out_l + state[1];
        state[1] = b2 * in_l - a2 * out_l;
        samples[idx] = out_l as f32;
        // Right channel
        let in_r = samples[idx + 1] as f64;
        let out_r = b0 * in_r + state[2];
        state[2] = b1 * in_r - a1 * out_r + state[3];
        state[3] = b2 * in_r - a2 * out_r;
        samples[idx + 1] = out_r as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_buffers_basic() {
        let mut dst = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0];
        let src = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0];
        add_buffers(&mut dst, &src);
        assert_eq!(
            dst,
            vec![11.0, 22.0, 33.0, 44.0, 55.0, 66.0, 77.0, 88.0, 99.0]
        );
    }

    #[test]
    fn add_buffers_mismatched_len() {
        let mut dst = vec![1.0, 2.0, 3.0];
        let src = vec![10.0, 20.0];
        add_buffers(&mut dst, &src);
        assert_eq!(dst, vec![11.0, 22.0, 3.0]);
    }

    #[test]
    fn apply_gain_basic() {
        let mut samples = vec![0.5, -0.5, 1.0, -1.0, 0.25];
        apply_gain(&mut samples, 2.0);
        assert_eq!(samples, vec![1.0, -1.0, 2.0, -2.0, 0.5]);
    }

    #[test]
    fn clamp_basic() {
        let mut samples = vec![2.0, -2.0, 0.5, -0.5, 1.5, -1.5, 0.0, 0.99, -0.99];
        clamp(&mut samples, -1.0, 1.0);
        assert_eq!(
            samples,
            vec![1.0, -1.0, 0.5, -0.5, 1.0, -1.0, 0.0, 0.99, -0.99]
        );
    }

    #[test]
    fn peak_abs_basic() {
        let samples = vec![0.3, -0.7, 0.5, 0.1, -0.2, 0.6, -0.4, 0.0, 0.69];
        assert!((peak_abs(&samples) - 0.7).abs() < f32::EPSILON);
    }

    #[test]
    fn peak_abs_empty() {
        assert_eq!(peak_abs(&[]), 0.0);
    }

    #[test]
    fn sum_of_squares_basic() {
        let samples = vec![1.0, 2.0, 3.0];
        let result = sum_of_squares(&samples);
        assert!((result - 14.0).abs() < 1e-6);
    }

    #[test]
    fn noise_gate_basic() {
        let mut samples = vec![0.01, -0.01, 0.5, -0.5, 0.001, 0.8];
        noise_gate(&mut samples, 0.1);
        assert_eq!(samples[0], 0.0);
        assert_eq!(samples[1], 0.0);
        assert!((samples[2] - 0.5).abs() < f32::EPSILON);
        assert!((samples[3] + 0.5).abs() < f32::EPSILON);
        assert_eq!(samples[4], 0.0);
        assert!((samples[5] - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn i16_f32_roundtrip() {
        let src_i16: Vec<i16> = vec![0, 16384, -16384, 32767, -32768];
        let mut f32_buf = vec![0.0f32; 5];
        i16_to_f32(&src_i16, &mut f32_buf);
        let mut back_i16 = vec![0i16; 5];
        f32_to_i16(&f32_buf, &mut back_i16);
        for (a, b) in src_i16.iter().zip(back_i16.iter()) {
            assert!((*a as i32 - *b as i32).abs() <= 1, "{a} != {b}");
        }
    }

    #[test]
    fn weighted_sum_basic() {
        let samples = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let weights = vec![0.5, 0.5, 0.5, 0.5, 0.5];
        let (sum, wt) = weighted_sum(&samples, &weights);
        assert!((sum - 7.5).abs() < 1e-4, "weighted_sum={sum}");
        assert!((wt - 2.5).abs() < 1e-4, "weight_sum={wt}");
    }

    #[test]
    fn weighted_sum_unequal_weights() {
        let samples = vec![1.0, 2.0, 3.0];
        let weights = vec![1.0, 0.0, 0.5];
        let (sum, wt) = weighted_sum(&samples, &weights);
        assert!((sum - 2.5).abs() < 1e-4);
        assert!((wt - 1.5).abs() < 1e-4);
    }

    /// Verify SIMD paths match scalar fallbacks for all kernels.
    #[test]
    fn simd_scalar_parity() {
        let data: Vec<f32> = (0..1025).map(|i| (i as f32 * 0.01).sin() * 0.9).collect();

        // peak_abs
        let simd_peak = peak_abs(&data);
        let scalar_peak = super::peak_abs_scalar(&data);
        assert!(
            (simd_peak - scalar_peak).abs() < 1e-6,
            "peak_abs: simd={simd_peak} scalar={scalar_peak}"
        );

        // sum_of_squares
        let simd_sos = sum_of_squares(&data);
        let scalar_sos = super::sum_of_squares_scalar(&data);
        assert!(
            (simd_sos - scalar_sos).abs() < 1e-3,
            "sum_of_squares: simd={simd_sos} scalar={scalar_sos}"
        );

        // add_buffers
        let mut simd_dst = vec![1.0f32; data.len()];
        let mut scalar_dst = simd_dst.clone();
        add_buffers(&mut simd_dst, &data);
        super::add_buffers_scalar(&mut scalar_dst, &data);
        assert_eq!(simd_dst, scalar_dst, "add_buffers mismatch");

        // apply_gain
        let mut simd_gain = data.clone();
        let mut scalar_gain = data.clone();
        apply_gain(&mut simd_gain, 0.75);
        super::apply_gain_scalar(&mut scalar_gain, 0.75);
        for (i, (s, sc)) in simd_gain.iter().zip(scalar_gain.iter()).enumerate() {
            assert!(
                (s - sc).abs() < 1e-6,
                "apply_gain[{i}]: simd={s} scalar={sc}"
            );
        }

        // clamp
        let mut simd_clamp: Vec<f32> = data.iter().map(|s| s * 2.0).collect();
        let mut scalar_clamp = simd_clamp.clone();
        clamp(&mut simd_clamp, -0.5, 0.5);
        super::clamp_scalar(&mut scalar_clamp, -0.5, 0.5);
        assert_eq!(simd_clamp, scalar_clamp, "clamp mismatch");

        // noise_gate
        let mut simd_gate = data.clone();
        let mut scalar_gate = data.clone();
        noise_gate(&mut simd_gate, 0.3);
        super::noise_gate_scalar(&mut scalar_gate, 0.3);
        assert_eq!(simd_gate, scalar_gate, "noise_gate mismatch");

        // i16_to_f32 / f32_to_i16 roundtrip parity
        let i16_data: Vec<i16> = (0..1025).map(|i| ((i * 31) % 65536) as i16).collect();
        let mut simd_f32 = vec![0.0f32; i16_data.len()];
        let mut scalar_f32 = vec![0.0f32; i16_data.len()];
        i16_to_f32(&i16_data, &mut simd_f32);
        super::i16_to_f32_scalar(&i16_data, &mut scalar_f32);
        for (i, (s, sc)) in simd_f32.iter().zip(scalar_f32.iter()).enumerate() {
            assert!(
                (s - sc).abs() < 1e-6,
                "i16_to_f32[{i}]: simd={s} scalar={sc}"
            );
        }

        let mut simd_i16 = vec![0i16; simd_f32.len()];
        let mut scalar_i16 = vec![0i16; scalar_f32.len()];
        f32_to_i16(&simd_f32, &mut simd_i16);
        super::f32_to_i16_scalar(&scalar_f32, &mut scalar_i16);
        // Allow ±1 difference: SIMD uses round-to-nearest, scalar uses truncation
        for (i, (s, sc)) in simd_i16.iter().zip(scalar_i16.iter()).enumerate() {
            assert!(
                (*s as i32 - *sc as i32).abs() <= 1,
                "f32_to_i16[{i}]: simd={s} scalar={sc}"
            );
        }

        // weighted_sum
        let weights: Vec<f32> = (0..data.len()).map(|i| (i as f32 * 0.003).cos()).collect();
        let (simd_ws, simd_wt) = weighted_sum(&data, &weights);
        let (scalar_ws, scalar_wt) = super::weighted_sum_scalar(&data, &weights);
        assert!(
            (simd_ws - scalar_ws).abs() < 1e-2,
            "weighted_sum: simd={simd_ws} scalar={scalar_ws}"
        );
        assert!(
            (simd_wt - scalar_wt).abs() < 1e-2,
            "weight_sum: simd={simd_wt} scalar={scalar_wt}"
        );
    }

    #[test]
    fn i24_f32_roundtrip() {
        let src_i24: Vec<i32> = vec![0, 4194304, -4194304, 8388607, -8388608];
        let mut f32_buf = vec![0.0f32; 5];
        i24_to_f32(&src_i24, &mut f32_buf);
        let mut back_i24 = vec![0i32; 5];
        f32_to_i24(&f32_buf, &mut back_i24);
        for (a, b) in src_i24.iter().zip(back_i24.iter()) {
            assert!((*a - *b).abs() <= 1, "i24 roundtrip: {a} != {b}");
        }
    }

    #[test]
    fn u8_f32_roundtrip() {
        let src_u8: Vec<u8> = vec![0, 64, 128, 192, 255];
        let mut f32_buf = vec![0.0f32; 5];
        u8_to_f32(&src_u8, &mut f32_buf);
        // 128 should map to ~0.0
        assert!(f32_buf[2].abs() < 0.01, "u8 128 → {}", f32_buf[2]);
        // 0 should map to ~-1.0
        assert!((f32_buf[0] + 1.0).abs() < 0.01, "u8 0 → {}", f32_buf[0]);
        let mut back_u8 = vec![0u8; 5];
        f32_to_u8(&f32_buf, &mut back_u8);
        for (a, b) in src_u8.iter().zip(back_u8.iter()) {
            assert!(
                (*a as i16 - *b as i16).abs() <= 1,
                "u8 roundtrip: {a} != {b}"
            );
        }
    }

    #[test]
    fn i24_parity() {
        let data: Vec<i32> = (0..1025)
            .map(|i| ((i * 7919) % 16777216) - 8388608)
            .collect();
        let mut simd_dst = vec![0.0f32; data.len()];
        let mut scalar_dst = vec![0.0f32; data.len()];
        i24_to_f32(&data, &mut simd_dst);
        super::i24_to_f32_scalar(&data, &mut scalar_dst);
        for (i, (s, sc)) in simd_dst.iter().zip(scalar_dst.iter()).enumerate() {
            assert!(
                (s - sc).abs() < 1e-6,
                "i24_to_f32[{i}]: simd={s} scalar={sc}"
            );
        }

        let mut simd_back = vec![0i32; simd_dst.len()];
        let mut scalar_back = vec![0i32; scalar_dst.len()];
        f32_to_i24(&simd_dst, &mut simd_back);
        super::f32_to_i24_scalar(&scalar_dst, &mut scalar_back);
        for (i, (s, sc)) in simd_back.iter().zip(scalar_back.iter()).enumerate() {
            assert!(
                (*s - *sc).abs() <= 1,
                "f32_to_i24[{i}]: simd={s} scalar={sc}"
            );
        }
    }

    #[test]
    fn u8_parity() {
        let data: Vec<u8> = (0..=255).collect();
        let mut simd_dst = vec![0.0f32; data.len()];
        let mut scalar_dst = vec![0.0f32; data.len()];
        u8_to_f32(&data, &mut simd_dst);
        super::u8_to_f32_scalar(&data, &mut scalar_dst);
        for (i, (s, sc)) in simd_dst.iter().zip(scalar_dst.iter()).enumerate() {
            assert!(
                (s - sc).abs() < 1e-6,
                "u8_to_f32[{i}]: simd={s} scalar={sc}"
            );
        }

        let mut simd_back = vec![0u8; simd_dst.len()];
        let mut scalar_back = vec![0u8; scalar_dst.len()];
        f32_to_u8(&simd_dst, &mut simd_back);
        super::f32_to_u8_scalar(&scalar_dst, &mut scalar_back);
        for (i, (s, sc)) in simd_back.iter().zip(scalar_back.iter()).enumerate() {
            assert!(
                (*s as i16 - *sc as i16).abs() <= 1,
                "f32_to_u8[{i}]: simd={s} scalar={sc}"
            );
        }
    }

    #[test]
    fn biquad_stereo_parity() {
        // Lowpass biquad coefficients (pre-computed for 1kHz, Q=0.707, sr=44100)
        let coeffs: [f64; 5] = [
            0.004836739652368523, // b0
            0.009673479304737046, // b1
            0.004836739652368523, // b2
            -1.9029109205028356,  // a1
            0.9222578791123097,   // a2
        ];

        // Generate test stereo signal
        let mut simd_samples: Vec<f32> = (0..2048)
            .map(|i| {
                let t = (i / 2) as f32 / 44100.0;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.8
            })
            .collect();
        let mut scalar_samples = simd_samples.clone();

        let mut simd_state = [0.0f64; 4];
        let mut scalar_state = [0.0f64; 4];

        biquad_stereo(&mut simd_samples, &coeffs, &mut simd_state);
        super::biquad_stereo_scalar(&mut scalar_samples, &coeffs, &mut scalar_state);

        for (i, (s, sc)) in simd_samples.iter().zip(scalar_samples.iter()).enumerate() {
            assert!(
                (s - sc).abs() < 1e-5,
                "biquad_stereo[{i}]: simd={s} scalar={sc}"
            );
        }
        for (i, (s, sc)) in simd_state.iter().zip(scalar_state.iter()).enumerate() {
            assert!(
                (s - sc).abs() < 1e-10,
                "biquad_state[{i}]: simd={s} scalar={sc}"
            );
        }
    }

    #[test]
    fn various_buffer_sizes() {
        for size in [0, 1, 3, 4, 7, 8, 15, 16, 17] {
            let mut dst = vec![1.0f32; size];
            let src = vec![2.0f32; size];
            add_buffers(&mut dst, &src);
            assert!(
                dst.iter().all(|&s| (s - 3.0).abs() < f32::EPSILON),
                "add size={size}"
            );

            let mut samples = vec![0.5f32; size];
            apply_gain(&mut samples, 2.0);
            assert!(
                samples.iter().all(|&s| (s - 1.0).abs() < f32::EPSILON),
                "gain size={size}"
            );

            let mut samples = vec![2.0f32; size];
            clamp(&mut samples, -1.0, 1.0);
            assert!(
                samples.iter().all(|&s| (s - 1.0).abs() < f32::EPSILON),
                "clamp size={size}"
            );
        }
    }
}
