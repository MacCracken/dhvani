//! x86_64 SIMD kernels — SSE2 (baseline) + AVX2 (runtime-detected).

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn add_buffers(dst: &mut [f32], src: &[f32]) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { add_buffers_avx2(dst, src) };
    } else {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="sse2") function.
        unsafe { add_buffers_sse2(dst, src) };
    }
}

pub fn apply_gain(samples: &mut [f32], gain: f32) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { apply_gain_avx2(samples, gain) };
    } else {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="sse2") function.
        unsafe { apply_gain_sse2(samples, gain) };
    }
}

pub fn clamp(samples: &mut [f32], min: f32, max: f32) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { clamp_avx2(samples, min, max) };
    } else {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="sse2") function.
        unsafe { clamp_sse2(samples, min, max) };
    }
}

pub fn peak_abs(samples: &[f32]) -> f32 {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { peak_abs_avx2(samples) }
    } else {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="sse2") function.
        unsafe { peak_abs_sse2(samples) }
    }
}

pub fn sum_of_squares(samples: &[f32]) -> f64 {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { sum_of_squares_avx2(samples) }
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { sum_of_squares_sse2(samples) }
    }
}

pub fn noise_gate(samples: &mut [f32], threshold: f32) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { noise_gate_avx2(samples, threshold) };
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { noise_gate_sse2(samples, threshold) };
    }
}

pub fn i16_to_f32(src: &[i16], dst: &mut [f32]) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { i16_to_f32_avx2(src, dst) };
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { i16_to_f32_sse2(src, dst) };
    }
}

pub fn f32_to_i16(src: &[f32], dst: &mut [i16]) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { f32_to_i16_avx2(src, dst) };
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { f32_to_i16_sse2(src, dst) };
    }
}

pub fn weighted_sum(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { weighted_sum_avx2(samples, weights) }
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { weighted_sum_sse2(samples, weights) }
    }
}

pub fn i24_to_f32(src: &[i32], dst: &mut [f32]) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { i24_to_f32_avx2(src, dst) };
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { i24_to_f32_sse2(src, dst) };
    }
}

pub fn f32_to_i24(src: &[f32], dst: &mut [i32]) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { f32_to_i24_avx2(src, dst) };
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { f32_to_i24_sse2(src, dst) };
    }
}

pub fn u8_to_f32(src: &[u8], dst: &mut [f32]) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { u8_to_f32_avx2(src, dst) };
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { u8_to_f32_sse2(src, dst) };
    }
}

pub fn f32_to_u8(src: &[f32], dst: &mut [u8]) {
    if is_x86_feature_detected!("avx2") {
        // SAFETY: CPU feature detected above; calling the matching target_feature(enable="avx2") function.
        unsafe { f32_to_u8_avx2(src, dst) };
    } else {
        // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
        unsafe { f32_to_u8_sse2(src, dst) };
    }
}

pub fn biquad_stereo(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    // SAFETY: SSE2 is always available on x86_64; calling the matching target_feature(enable="sse2") function.
    // SSE2 processes 2×f64 which is perfect for stereo (L+R).
    unsafe { biquad_stereo_sse2(samples, coeffs, state) };
}

// ── SSE2 (4 f32 per op) ────────────────────────────────────────────

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn add_buffers_sse2(dst: &mut [f32], src: &[f32]) {
    let len = dst.len().min(src.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm_loadu_ps(dst.as_ptr().add(off));
            let b = _mm_loadu_ps(src.as_ptr().add(off));
            _mm_storeu_ps(dst.as_mut_ptr().add(off), _mm_add_ps(a, b));
        }
    }
    for i in (chunks * 4)..len {
        dst[i] += src[i];
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn apply_gain_sse2(samples: &mut [f32], gain: f32) {
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let g = unsafe { _mm_set1_ps(gain) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            _mm_storeu_ps(samples.as_mut_ptr().add(off), _mm_mul_ps(a, g));
        }
    }
    for i in (chunks * 4)..samples.len() {
        samples[i] *= gain;
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn clamp_sse2(samples: &mut [f32], min_val: f32, max_val: f32) {
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let vmin = unsafe { _mm_set1_ps(min_val) };
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let vmax = unsafe { _mm_set1_ps(max_val) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            let clamped = _mm_min_ps(_mm_max_ps(a, vmin), vmax);
            _mm_storeu_ps(samples.as_mut_ptr().add(off), clamped);
        }
    }
    for i in (chunks * 4)..samples.len() {
        samples[i] = samples[i].clamp(min_val, max_val);
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn peak_abs_sse2(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    // SAFETY: SSE2 intrinsic to create abs mask; no memory access.
    let abs_mask = unsafe { _mm_castsi128_ps(_mm_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
    // SAFETY: SSE2 intrinsic to create zero vector; no memory access.
    let mut vmax = unsafe { _mm_setzero_ps() };

    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            let abs_a = _mm_and_ps(a, abs_mask);
            vmax = _mm_max_ps(vmax, abs_a);
        }
    }

    // SAFETY: Calling target_feature(enable="sse2") helper with register value; no memory access.
    let mut result = unsafe { horizontal_max_sse2(vmax) };
    for i in (chunks * 4)..samples.len() {
        result = result.max(samples[i].abs());
    }
    result
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn horizontal_max_sse2(v: __m128) -> f32 {
    // SAFETY: SSE2 register-only shuffle and max operations; no memory access.
    unsafe {
        let shuf = _mm_shuffle_ps(v, v, 0b_01_00_11_10);
        let max1 = _mm_max_ps(v, shuf);
        let shuf2 = _mm_shuffle_ps(max1, max1, 0b_00_01_00_01);
        let max2 = _mm_max_ps(max1, shuf2);
        _mm_cvtss_f32(max2)
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn sum_of_squares_sse2(samples: &[f32]) -> f64 {
    let mut total = 0.0f64;
    let chunks = samples.len() / 4;
    // SAFETY: SSE2 intrinsic to create zero vector; no memory access.
    let mut acc = unsafe { _mm_setzero_ps() };

    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            acc = _mm_add_ps(acc, _mm_mul_ps(a, a));
        }
        if (i + 1) % 256 == 0 {
            // SAFETY: Calling target_feature(enable="sse2") helper with register value; no memory access.
            total += unsafe { horizontal_sum_f64_sse2(acc) };
            // SAFETY: SSE2 intrinsic to create zero vector; no memory access.
            acc = unsafe { _mm_setzero_ps() };
        }
    }
    // SAFETY: Calling target_feature(enable="sse2") helper with register value; no memory access.
    total += unsafe { horizontal_sum_f64_sse2(acc) };

    for i in (chunks * 4)..samples.len() {
        let s = samples[i] as f64;
        total += s * s;
    }
    total
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn horizontal_sum_f64_sse2(v: __m128) -> f64 {
    // SAFETY: SSE2 register-only conversion and arithmetic operations; no memory access.
    unsafe {
        let lo = _mm_cvtps_pd(v);
        let hi = _mm_cvtps_pd(_mm_movehl_ps(v, v));
        let sum = _mm_add_pd(lo, hi);
        let hi64 = _mm_unpackhi_pd(sum, sum);
        let result = _mm_add_sd(sum, hi64);
        _mm_cvtsd_f64(result)
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn noise_gate_sse2(samples: &mut [f32], threshold: f32) {
    // SAFETY: SSE2 intrinsic to create abs mask; no memory access.
    let abs_mask = unsafe { _mm_castsi128_ps(_mm_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let thresh = unsafe { _mm_set1_ps(threshold) };

    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            let abs_a = _mm_and_ps(a, abs_mask);
            let mask = _mm_cmpge_ps(abs_a, thresh);
            let result = _mm_and_ps(a, mask);
            _mm_storeu_ps(samples.as_mut_ptr().add(off), result);
        }
    }
    for i in (chunks * 4)..samples.len() {
        if samples[i].abs() < threshold {
            samples[i] = 0.0;
        }
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn i16_to_f32_sse2(src: &[i16], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let scale = unsafe { _mm_set1_ps(1.0 / 32768.0) };

    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let s0 = src[off] as i32;
        let s1 = src[off + 1] as i32;
        let s2 = src[off + 2] as i32;
        let s3 = src[off + 3] as i32;
        // SAFETY: SSE2 intrinsics operating on scalars loaded above. Storing to slice with bounds checked by loop range.
        unsafe {
            let ints = _mm_set_epi32(s3, s2, s1, s0);
            let floats = _mm_cvtepi32_ps(ints);
            let scaled = _mm_mul_ps(floats, scale);
            _mm_storeu_ps(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 4)..len {
        dst[i] = src[i] as f32 / 32768.0;
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn f32_to_i16_sse2(src: &[f32], dst: &mut [i16]) {
    let len = src.len().min(dst.len());
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let vmin = unsafe { _mm_set1_ps(-1.0) };
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let vmax = unsafe { _mm_set1_ps(1.0) };
    // SAFETY: SSE2 intrinsic to broadcast a scalar; no memory access.
    let scale = unsafe { _mm_set1_ps(32767.0) };

    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        // Storing to dst with bounds checked by loop range via indexed access.
        unsafe {
            let a = _mm_loadu_ps(src.as_ptr().add(off));
            let clamped = _mm_min_ps(_mm_max_ps(a, vmin), vmax);
            let scaled = _mm_mul_ps(clamped, scale);
            let ints = _mm_cvtps_epi32(scaled);
            let packed = _mm_packs_epi32(ints, ints);
            dst[off] = _mm_extract_epi16(packed, 0) as i16;
            dst[off + 1] = _mm_extract_epi16(packed, 1) as i16;
            dst[off + 2] = _mm_extract_epi16(packed, 2) as i16;
            dst[off + 3] = _mm_extract_epi16(packed, 3) as i16;
        }
    }
    for i in (chunks * 4)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 32767.0) as i16;
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn weighted_sum_sse2(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    let len = samples.len().min(weights.len());
    let chunks = len / 4;
    let mut acc_sum = _mm_setzero_ps();
    let mut acc_wt = _mm_setzero_ps();

    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm_loadu_ps allows unaligned reads.
        unsafe {
            let s = _mm_loadu_ps(samples.as_ptr().add(off));
            let w = _mm_loadu_ps(weights.as_ptr().add(off));
            acc_sum = _mm_add_ps(acc_sum, _mm_mul_ps(s, w));
            acc_wt = _mm_add_ps(acc_wt, w);
        }
    }

    // Horizontal sum both accumulators
    // SAFETY: Calling target_feature(enable="sse2") helper with register value; no memory access.
    let sum = unsafe { horizontal_sum_f32_sse2(acc_sum) };
    // SAFETY: Calling target_feature(enable="sse2") helper with register value; no memory access.
    let wt = unsafe { horizontal_sum_f32_sse2(acc_wt) };

    let mut total_sum = sum;
    let mut total_wt = wt;
    for i in (chunks * 4)..len {
        total_sum += samples[i] * weights[i];
        total_wt += weights[i];
    }
    (total_sum, total_wt)
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn horizontal_sum_f32_sse2(v: __m128) -> f32 {
    // SAFETY: SSE2 register-only shuffle and add operations; no memory access.
    unsafe {
        let shuf = _mm_shuffle_ps(v, v, 0b_01_00_11_10);
        let sum1 = _mm_add_ps(v, shuf);
        let shuf2 = _mm_shuffle_ps(sum1, sum1, 0b_00_01_00_01);
        let sum2 = _mm_add_ps(sum1, shuf2);
        _mm_cvtss_f32(sum2)
    }
}

// ── AVX2 (8 f32 per op) ────────────────────────────────────────────

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn add_buffers_avx2(dst: &mut [f32], src: &[f32]) {
    let len = dst.len().min(src.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm256_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm256_loadu_ps(dst.as_ptr().add(off));
            let b = _mm256_loadu_ps(src.as_ptr().add(off));
            _mm256_storeu_ps(dst.as_mut_ptr().add(off), _mm256_add_ps(a, b));
        }
    }
    for i in (chunks * 8)..len {
        dst[i] += src[i];
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn apply_gain_avx2(samples: &mut [f32], gain: f32) {
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let g = unsafe { _mm256_set1_ps(gain) };
    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm256_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm256_loadu_ps(samples.as_ptr().add(off));
            _mm256_storeu_ps(samples.as_mut_ptr().add(off), _mm256_mul_ps(a, g));
        }
    }
    for i in (chunks * 8)..samples.len() {
        samples[i] *= gain;
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn clamp_avx2(samples: &mut [f32], min_val: f32, max_val: f32) {
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let vmin = unsafe { _mm256_set1_ps(min_val) };
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let vmax = unsafe { _mm256_set1_ps(max_val) };
    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm256_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm256_loadu_ps(samples.as_ptr().add(off));
            let clamped = _mm256_min_ps(_mm256_max_ps(a, vmin), vmax);
            _mm256_storeu_ps(samples.as_mut_ptr().add(off), clamped);
        }
    }
    for i in (chunks * 8)..samples.len() {
        samples[i] = samples[i].clamp(min_val, max_val);
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn sum_of_squares_avx2(samples: &[f32]) -> f64 {
    let mut total = 0.0f64;
    let chunks = samples.len() / 8;
    // SAFETY: AVX2 intrinsic to create zero vector; no memory access.
    let mut acc = unsafe { _mm256_setzero_ps() };

    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        unsafe {
            let a = _mm256_loadu_ps(samples.as_ptr().add(off));
            acc = _mm256_add_ps(acc, _mm256_mul_ps(a, a));
        }
        if (i + 1) % 128 == 0 {
            // SAFETY: Calling target_feature helper with register value; no memory access.
            total += unsafe { horizontal_sum_f64_avx2(acc) };
            // SAFETY: AVX2 intrinsic to create zero vector; no memory access.
            acc = unsafe { _mm256_setzero_ps() };
        }
    }
    // SAFETY: Calling target_feature helper with register value; no memory access.
    total += unsafe { horizontal_sum_f64_avx2(acc) };

    for i in (chunks * 8)..samples.len() {
        let s = samples[i] as f64;
        total += s * s;
    }
    total
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn horizontal_sum_f64_avx2(v: __m256) -> f64 {
    // SAFETY: AVX2/SSE2 register-only operations; no memory access.
    // Split 256 into two 128, sum to f64, then horizontal sum.
    unsafe {
        let hi128 = _mm256_extractf128_ps(v, 1);
        let lo128 = _mm256_castps256_ps128(v);
        let sum128 = _mm_add_ps(lo128, hi128);
        horizontal_sum_f64_sse2(sum128)
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn noise_gate_avx2(samples: &mut [f32], threshold: f32) {
    // SAFETY: AVX2 intrinsic to create abs mask; no memory access.
    let abs_mask = unsafe { _mm256_castsi256_ps(_mm256_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let thresh = unsafe { _mm256_set1_ps(threshold) };

    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        // Storing to slice with bounds checked by loop range. _mm256_storeu_ps allows unaligned writes.
        unsafe {
            let a = _mm256_loadu_ps(samples.as_ptr().add(off));
            let abs_a = _mm256_and_ps(a, abs_mask);
            let mask = _mm256_cmp_ps(abs_a, thresh, _CMP_GE_OQ);
            let result = _mm256_and_ps(a, mask);
            _mm256_storeu_ps(samples.as_mut_ptr().add(off), result);
        }
    }
    for i in (chunks * 8)..samples.len() {
        if samples[i].abs() < threshold {
            samples[i] = 0.0;
        }
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn i16_to_f32_avx2(src: &[i16], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let scale = unsafe { _mm256_set1_ps(1.0 / 32768.0) };

    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading 8 i16s from slice with bounds checked by loop range.
        // _mm_loadu_si128 allows unaligned reads. _mm256_cvtepi16_epi32 widens to 32-bit.
        // Storing to slice with bounds checked by loop range.
        unsafe {
            let ints_16 = _mm_loadu_si128(src.as_ptr().add(off) as *const __m128i);
            let ints_32 = _mm256_cvtepi16_epi32(ints_16);
            let floats = _mm256_cvtepi32_ps(ints_32);
            let scaled = _mm256_mul_ps(floats, scale);
            _mm256_storeu_ps(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 8)..len {
        dst[i] = src[i] as f32 / 32768.0;
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn f32_to_i16_avx2(src: &[f32], dst: &mut [i16]) {
    let len = src.len().min(dst.len());
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let vmin = unsafe { _mm256_set1_ps(-1.0) };
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let vmax = unsafe { _mm256_set1_ps(1.0) };
    // SAFETY: AVX2 intrinsic to broadcast a scalar; no memory access.
    let scale = unsafe { _mm256_set1_ps(32767.0) };

    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        // _mm256_cvtps_epi32 converts to i32. _mm256_extracti128_si256 splits halves.
        // _mm_packs_epi32 saturating packs to i16. Storing to slice with bounds checked.
        unsafe {
            let a = _mm256_loadu_ps(src.as_ptr().add(off));
            let clamped = _mm256_min_ps(_mm256_max_ps(a, vmin), vmax);
            let scaled = _mm256_mul_ps(clamped, scale);
            let ints = _mm256_cvtps_epi32(scaled);
            let lo = _mm256_castsi256_si128(ints);
            let hi = _mm256_extracti128_si256(ints, 1);
            let packed = _mm_packs_epi32(lo, hi);
            // _mm_packs_epi32 packs [lo0 lo1 lo2 lo3 | hi0 hi1 hi2 hi3]
            // which gives [lo0 lo1 lo2 lo3 hi0 hi1 hi2 hi3] in order
            dst[off] = _mm_extract_epi16(packed, 0) as i16;
            dst[off + 1] = _mm_extract_epi16(packed, 1) as i16;
            dst[off + 2] = _mm_extract_epi16(packed, 2) as i16;
            dst[off + 3] = _mm_extract_epi16(packed, 3) as i16;
            dst[off + 4] = _mm_extract_epi16(packed, 4) as i16;
            dst[off + 5] = _mm_extract_epi16(packed, 5) as i16;
            dst[off + 6] = _mm_extract_epi16(packed, 6) as i16;
            dst[off + 7] = _mm_extract_epi16(packed, 7) as i16;
        }
    }
    for i in (chunks * 8)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 32767.0) as i16;
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn weighted_sum_avx2(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    let len = samples.len().min(weights.len());
    let chunks = len / 8;
    // SAFETY: AVX2 intrinsic to create zero vector; no memory access.
    let mut acc_sum = unsafe { _mm256_setzero_ps() };
    // SAFETY: AVX2 intrinsic to create zero vector; no memory access.
    let mut acc_wt = unsafe { _mm256_setzero_ps() };

    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        unsafe {
            let s = _mm256_loadu_ps(samples.as_ptr().add(off));
            let w = _mm256_loadu_ps(weights.as_ptr().add(off));
            acc_sum = _mm256_add_ps(acc_sum, _mm256_mul_ps(s, w));
            acc_wt = _mm256_add_ps(acc_wt, w);
        }
    }

    // Horizontal sum both accumulators: reduce 256→128→scalar
    // SAFETY: AVX2/SSE2 register-only extract, add, and shuffle operations; no memory access.
    let (sum, wt) = unsafe {
        let sum_hi = _mm256_extractf128_ps(acc_sum, 1);
        let sum_lo = _mm256_castps256_ps128(acc_sum);
        let sum128 = _mm_add_ps(sum_lo, sum_hi);
        let s = horizontal_sum_f32_sse2(sum128);

        let wt_hi = _mm256_extractf128_ps(acc_wt, 1);
        let wt_lo = _mm256_castps256_ps128(acc_wt);
        let wt128 = _mm_add_ps(wt_lo, wt_hi);
        let w = horizontal_sum_f32_sse2(wt128);
        (s, w)
    };

    let mut total_sum = sum;
    let mut total_wt = wt;
    for i in (chunks * 8)..len {
        total_sum += samples[i] * weights[i];
        total_wt += weights[i];
    }
    (total_sum, total_wt)
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn peak_abs_avx2(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    // SAFETY: AVX2 intrinsic to create abs mask; no memory access.
    let abs_mask = unsafe { _mm256_castsi256_ps(_mm256_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
    // SAFETY: AVX2 intrinsic to create zero vector; no memory access.
    let mut vmax = unsafe { _mm256_setzero_ps() };

    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range. _mm256_loadu_ps allows unaligned reads.
        unsafe {
            let a = _mm256_loadu_ps(samples.as_ptr().add(off));
            let abs_a = _mm256_and_ps(a, abs_mask);
            vmax = _mm256_max_ps(vmax, abs_a);
        }
    }

    // SAFETY: AVX2/SSE2 register-only extract and max operations; then calling SSE2 helper.
    let mut result = unsafe {
        let hi128 = _mm256_extractf128_ps(vmax, 1);
        let lo128 = _mm256_castps256_ps128(vmax);
        let max128 = _mm_max_ps(lo128, hi128);
        horizontal_max_sse2(max128)
    };

    for i in (chunks * 8)..samples.len() {
        result = result.max(samples[i].abs());
    }
    result
}

// ── i24 conversion (SSE2 + AVX2) ──────────────────────────────────

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn i24_to_f32_sse2(src: &[i32], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: SSE2 intrinsics to broadcast scalars; no memory access.
    let shift8 = unsafe { _mm_set1_epi32(8) };
    let scale = unsafe { _mm_set1_ps(1.0 / 8388608.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let raw = _mm_loadu_si128(src.as_ptr().add(off) as *const __m128i);
            // Sign-extend: (val << 8) >> 8
            let shifted = _mm_srai_epi32(_mm_slli_epi32(raw, 8), 8);
            let floats = _mm_cvtepi32_ps(shifted);
            let scaled = _mm_mul_ps(floats, scale);
            _mm_storeu_ps(dst.as_mut_ptr().add(off), scaled);
        }
    }
    let _ = shift8; // suppress unused warning
    for i in (chunks * 4)..len {
        let extended = (src[i] << 8) >> 8;
        dst[i] = extended as f32 / 8388608.0;
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn f32_to_i24_sse2(src: &[f32], dst: &mut [i32]) {
    let len = src.len().min(dst.len());
    // SAFETY: SSE2 intrinsics to broadcast scalars; no memory access.
    let vmin = unsafe { _mm_set1_ps(-1.0) };
    let vmax = unsafe { _mm_set1_ps(1.0) };
    let scale = unsafe { _mm_set1_ps(8388607.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let a = _mm_loadu_ps(src.as_ptr().add(off));
            let clamped = _mm_min_ps(_mm_max_ps(a, vmin), vmax);
            let scaled = _mm_mul_ps(clamped, scale);
            let ints = _mm_cvtps_epi32(scaled);
            _mm_storeu_si128(dst.as_mut_ptr().add(off) as *mut __m128i, ints);
        }
    }
    for i in (chunks * 4)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 8388607.0) as i32;
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn i24_to_f32_avx2(src: &[i32], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: AVX2 intrinsic to broadcast scalar; no memory access.
    let scale = unsafe { _mm256_set1_ps(1.0 / 8388608.0) };
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let raw = _mm256_loadu_si256(src.as_ptr().add(off) as *const __m256i);
            let shifted = _mm256_srai_epi32(_mm256_slli_epi32(raw, 8), 8);
            let floats = _mm256_cvtepi32_ps(shifted);
            let scaled = _mm256_mul_ps(floats, scale);
            _mm256_storeu_ps(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 8)..len {
        let extended = (src[i] << 8) >> 8;
        dst[i] = extended as f32 / 8388608.0;
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn f32_to_i24_avx2(src: &[f32], dst: &mut [i32]) {
    let len = src.len().min(dst.len());
    // SAFETY: AVX2 intrinsics to broadcast scalars; no memory access.
    let vmin = unsafe { _mm256_set1_ps(-1.0) };
    let vmax = unsafe { _mm256_set1_ps(1.0) };
    let scale = unsafe { _mm256_set1_ps(8388607.0) };
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let a = _mm256_loadu_ps(src.as_ptr().add(off));
            let clamped = _mm256_min_ps(_mm256_max_ps(a, vmin), vmax);
            let scaled = _mm256_mul_ps(clamped, scale);
            let ints = _mm256_cvtps_epi32(scaled);
            _mm256_storeu_si256(dst.as_mut_ptr().add(off) as *mut __m256i, ints);
        }
    }
    for i in (chunks * 8)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 8388607.0) as i32;
    }
}

// ── u8 conversion (SSE2 + AVX2) ───────────────────────────────────

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn u8_to_f32_sse2(src: &[u8], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: SSE2 intrinsics to broadcast scalars; no memory access.
    let bias = unsafe { _mm_set1_ps(128.0) };
    let inv_scale = unsafe { _mm_set1_ps(1.0 / 128.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // Load 4 u8, widen to i32, convert to f32
        // SAFETY: Indexed access within bounds checked by loop range.
        unsafe {
            let ints = _mm_set_epi32(
                src[off + 3] as i32,
                src[off + 2] as i32,
                src[off + 1] as i32,
                src[off] as i32,
            );
            let floats = _mm_cvtepi32_ps(ints);
            let centered = _mm_sub_ps(floats, bias);
            let scaled = _mm_mul_ps(centered, inv_scale);
            _mm_storeu_ps(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 4)..len {
        dst[i] = (f32::from(src[i]) - 128.0) / 128.0;
    }
}

// SAFETY: Caller verifies SSE2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "sse2")]
unsafe fn f32_to_u8_sse2(src: &[f32], dst: &mut [u8]) {
    let len = src.len().min(dst.len());
    // SAFETY: SSE2 intrinsics to broadcast scalars; no memory access.
    let vmin = unsafe { _mm_set1_ps(-1.0) };
    let vmax = unsafe { _mm_set1_ps(1.0) };
    let scale = unsafe { _mm_set1_ps(128.0) };
    let bias = unsafe { _mm_set1_ps(128.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let a = _mm_loadu_ps(src.as_ptr().add(off));
            let clamped = _mm_min_ps(_mm_max_ps(a, vmin), vmax);
            let scaled = _mm_add_ps(_mm_mul_ps(clamped, scale), bias);
            let ints = _mm_cvtps_epi32(scaled);
            // Extract 4 i32 values and store as u8
            dst[off] = _mm_extract_epi16(ints, 0) as u8;
            dst[off + 1] = _mm_extract_epi16(ints, 2) as u8;
            dst[off + 2] = _mm_extract_epi16(ints, 4) as u8;
            dst[off + 3] = _mm_extract_epi16(ints, 6) as u8;
        }
    }
    for i in (chunks * 4)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = ((clamped * 128.0) + 128.0).clamp(0.0, 255.0) as u8;
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn u8_to_f32_avx2(src: &[u8], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: AVX2 intrinsics to broadcast scalars; no memory access.
    let bias = unsafe { _mm256_set1_ps(128.0) };
    let inv_scale = unsafe { _mm256_set1_ps(1.0 / 128.0) };
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading 8 u8s from slice with bounds checked by loop range.
        // _mm_loadl_epi64 loads 8 bytes. _mm256_cvtepu8_epi32 zero-extends to 8×i32.
        unsafe {
            let bytes = _mm_loadl_epi64(src.as_ptr().add(off) as *const __m128i);
            let ints = _mm256_cvtepu8_epi32(bytes);
            let floats = _mm256_cvtepi32_ps(ints);
            let centered = _mm256_sub_ps(floats, bias);
            let scaled = _mm256_mul_ps(centered, inv_scale);
            _mm256_storeu_ps(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 8)..len {
        dst[i] = (f32::from(src[i]) - 128.0) / 128.0;
    }
}

// SAFETY: Caller verifies AVX2 support via is_x86_feature_detected before calling.
#[target_feature(enable = "avx2")]
unsafe fn f32_to_u8_avx2(src: &[f32], dst: &mut [u8]) {
    let len = src.len().min(dst.len());
    // SAFETY: AVX2 intrinsics to broadcast scalars; no memory access.
    let vmin = unsafe { _mm256_set1_ps(-1.0) };
    let vmax = unsafe { _mm256_set1_ps(1.0) };
    let scale = unsafe { _mm256_set1_ps(128.0) };
    let bias = unsafe { _mm256_set1_ps(128.0) };
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let a = _mm256_loadu_ps(src.as_ptr().add(off));
            let clamped = _mm256_min_ps(_mm256_max_ps(a, vmin), vmax);
            let scaled = _mm256_add_ps(_mm256_mul_ps(clamped, scale), bias);
            let ints = _mm256_cvtps_epi32(scaled);
            // Extract via 128-bit halves
            let lo = _mm256_castsi256_si128(ints);
            let hi = _mm256_extracti128_si256(ints, 1);
            // Pack i32→i16→u8 would be ideal but we extract directly for simplicity
            dst[off] = _mm_extract_epi32(lo, 0) as u8;
            dst[off + 1] = _mm_extract_epi32(lo, 1) as u8;
            dst[off + 2] = _mm_extract_epi32(lo, 2) as u8;
            dst[off + 3] = _mm_extract_epi32(lo, 3) as u8;
            dst[off + 4] = _mm_extract_epi32(hi, 0) as u8;
            dst[off + 5] = _mm_extract_epi32(hi, 1) as u8;
            dst[off + 6] = _mm_extract_epi32(hi, 2) as u8;
            dst[off + 7] = _mm_extract_epi32(hi, 3) as u8;
        }
    }
    for i in (chunks * 8)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = ((clamped * 128.0) + 128.0).clamp(0.0, 255.0) as u8;
    }
}

// ── Stereo biquad SSE2 (2×f64 cross-channel) ──────────────────────

/// Process stereo interleaved samples through biquad using 2×f64 SSE2 registers.
/// coeffs = [b0, b1, b2, a1, a2], state = [z1_L, z2_L, z1_R, z2_R].
// SAFETY: SSE2 is always available on x86_64.
#[target_feature(enable = "sse2")]
unsafe fn biquad_stereo_sse2(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    let [b0, b1, b2, a1, a2] = *coeffs;
    // SAFETY: SSE2 intrinsics to broadcast f64 scalars; no memory access.
    let vb0 = unsafe { _mm_set1_pd(b0) };
    let vb1 = unsafe { _mm_set1_pd(b1) };
    let vb2 = unsafe { _mm_set1_pd(b2) };
    let va1 = unsafe { _mm_set1_pd(a1) };
    let va2 = unsafe { _mm_set1_pd(a2) };

    // z1 = [z1_L, z1_R], z2 = [z2_L, z2_R]
    // SAFETY: Loading from state array with known length 4.
    let mut vz1 = unsafe { _mm_set_pd(state[2], state[0]) };
    let mut vz2 = unsafe { _mm_set_pd(state[3], state[1]) };

    let frames = samples.len() / 2;
    for f in 0..frames {
        let idx = f * 2;
        // SAFETY: Indexed access within bounds (frames * 2 <= samples.len()).
        unsafe {
            // Load stereo pair, convert to f64
            let in_lr = _mm_set_pd(samples[idx + 1] as f64, samples[idx] as f64);
            // out = b0 * in + z1
            let out = _mm_add_pd(_mm_mul_pd(vb0, in_lr), vz1);
            // z1 = b1 * in - a1 * out + z2
            vz1 = _mm_add_pd(
                _mm_sub_pd(_mm_mul_pd(vb1, in_lr), _mm_mul_pd(va1, out)),
                vz2,
            );
            // z2 = b2 * in - a2 * out
            vz2 = _mm_sub_pd(_mm_mul_pd(vb2, in_lr), _mm_mul_pd(va2, out));
            // Store back as f32
            samples[idx] = _mm_cvtsd_f64(out) as f32;
            samples[idx + 1] = _mm_cvtsd_f64(_mm_unpackhi_pd(out, out)) as f32;
        }
    }

    // Write state back
    // SAFETY: SSE2 register-only extract operations; storing to known-length array.
    unsafe {
        state[0] = _mm_cvtsd_f64(vz1);
        state[1] = _mm_cvtsd_f64(vz2);
        state[2] = _mm_cvtsd_f64(_mm_unpackhi_pd(vz1, vz1));
        state[3] = _mm_cvtsd_f64(_mm_unpackhi_pd(vz2, vz2));
    }
}
