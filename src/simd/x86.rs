//! x86_64 SIMD kernels — SSE2 (baseline) + AVX2 (runtime-detected).

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

pub fn add_buffers(dst: &mut [f32], src: &[f32]) {
    if is_x86_feature_detected!("avx2") {
        unsafe { add_buffers_avx2(dst, src) };
    } else {
        unsafe { add_buffers_sse2(dst, src) };
    }
}

pub fn apply_gain(samples: &mut [f32], gain: f32) {
    if is_x86_feature_detected!("avx2") {
        unsafe { apply_gain_avx2(samples, gain) };
    } else {
        unsafe { apply_gain_sse2(samples, gain) };
    }
}

pub fn clamp(samples: &mut [f32], min: f32, max: f32) {
    if is_x86_feature_detected!("avx2") {
        unsafe { clamp_avx2(samples, min, max) };
    } else {
        unsafe { clamp_sse2(samples, min, max) };
    }
}

pub fn peak_abs(samples: &[f32]) -> f32 {
    if is_x86_feature_detected!("avx2") {
        unsafe { peak_abs_avx2(samples) }
    } else {
        unsafe { peak_abs_sse2(samples) }
    }
}

pub fn sum_of_squares(samples: &[f32]) -> f64 {
    unsafe { sum_of_squares_sse2(samples) }
}

pub fn noise_gate(samples: &mut [f32], threshold: f32) {
    unsafe { noise_gate_sse2(samples, threshold) };
}

pub fn i16_to_f32(src: &[i16], dst: &mut [f32]) {
    unsafe { i16_to_f32_sse2(src, dst) };
}

pub fn f32_to_i16(src: &[f32], dst: &mut [i16]) {
    unsafe { f32_to_i16_sse2(src, dst) };
}

pub fn weighted_sum(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    unsafe { weighted_sum_sse2(samples, weights) }
}

// ── SSE2 (4 f32 per op) ────────────────────────────────────────────

#[target_feature(enable = "sse2")]
unsafe fn add_buffers_sse2(dst: &mut [f32], src: &[f32]) {
    let len = dst.len().min(src.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
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

#[target_feature(enable = "sse2")]
unsafe fn apply_gain_sse2(samples: &mut [f32], gain: f32) {
    let g = unsafe { _mm_set1_ps(gain) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            _mm_storeu_ps(samples.as_mut_ptr().add(off), _mm_mul_ps(a, g));
        }
    }
    for i in (chunks * 4)..samples.len() {
        samples[i] *= gain;
    }
}

#[target_feature(enable = "sse2")]
unsafe fn clamp_sse2(samples: &mut [f32], min_val: f32, max_val: f32) {
    let vmin = unsafe { _mm_set1_ps(min_val) };
    let vmax = unsafe { _mm_set1_ps(max_val) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
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

#[target_feature(enable = "sse2")]
unsafe fn peak_abs_sse2(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let abs_mask = unsafe { _mm_castsi128_ps(_mm_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
    let mut vmax = unsafe { _mm_setzero_ps() };

    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            let abs_a = _mm_and_ps(a, abs_mask);
            vmax = _mm_max_ps(vmax, abs_a);
        }
    }

    let mut result = unsafe { horizontal_max_sse2(vmax) };
    for i in (chunks * 4)..samples.len() {
        result = result.max(samples[i].abs());
    }
    result
}

#[target_feature(enable = "sse2")]
unsafe fn horizontal_max_sse2(v: __m128) -> f32 {
    unsafe {
        let shuf = _mm_shuffle_ps(v, v, 0b_01_00_11_10);
        let max1 = _mm_max_ps(v, shuf);
        let shuf2 = _mm_shuffle_ps(max1, max1, 0b_00_01_00_01);
        let max2 = _mm_max_ps(max1, shuf2);
        _mm_cvtss_f32(max2)
    }
}

#[target_feature(enable = "sse2")]
unsafe fn sum_of_squares_sse2(samples: &[f32]) -> f64 {
    let mut total = 0.0f64;
    let chunks = samples.len() / 4;
    let mut acc = unsafe { _mm_setzero_ps() };

    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = _mm_loadu_ps(samples.as_ptr().add(off));
            acc = _mm_add_ps(acc, _mm_mul_ps(a, a));
        }
        if (i + 1) % 256 == 0 {
            total += unsafe { horizontal_sum_f64_sse2(acc) };
            acc = unsafe { _mm_setzero_ps() };
        }
    }
    total += unsafe { horizontal_sum_f64_sse2(acc) };

    for i in (chunks * 4)..samples.len() {
        let s = samples[i] as f64;
        total += s * s;
    }
    total
}

#[target_feature(enable = "sse2")]
unsafe fn horizontal_sum_f64_sse2(v: __m128) -> f64 {
    unsafe {
        let lo = _mm_cvtps_pd(v);
        let hi = _mm_cvtps_pd(_mm_movehl_ps(v, v));
        let sum = _mm_add_pd(lo, hi);
        let hi64 = _mm_unpackhi_pd(sum, sum);
        let result = _mm_add_sd(sum, hi64);
        _mm_cvtsd_f64(result)
    }
}

#[target_feature(enable = "sse2")]
unsafe fn noise_gate_sse2(samples: &mut [f32], threshold: f32) {
    let abs_mask = unsafe { _mm_castsi128_ps(_mm_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
    let thresh = unsafe { _mm_set1_ps(threshold) };

    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
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

#[target_feature(enable = "sse2")]
unsafe fn i16_to_f32_sse2(src: &[i16], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    let scale = unsafe { _mm_set1_ps(1.0 / 32768.0) };

    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        let s0 = src[off] as i32;
        let s1 = src[off + 1] as i32;
        let s2 = src[off + 2] as i32;
        let s3 = src[off + 3] as i32;
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

#[target_feature(enable = "sse2")]
unsafe fn f32_to_i16_sse2(src: &[f32], dst: &mut [i16]) {
    let len = src.len().min(dst.len());
    let vmin = unsafe { _mm_set1_ps(-1.0) };
    let vmax = unsafe { _mm_set1_ps(1.0) };
    let scale = unsafe { _mm_set1_ps(32767.0) };

    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
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

#[target_feature(enable = "sse2")]
unsafe fn weighted_sum_sse2(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    let len = samples.len().min(weights.len());
    let chunks = len / 4;
    let mut acc_sum = _mm_setzero_ps();
    let mut acc_wt = _mm_setzero_ps();

    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let s = _mm_loadu_ps(samples.as_ptr().add(off));
            let w = _mm_loadu_ps(weights.as_ptr().add(off));
            acc_sum = _mm_add_ps(acc_sum, _mm_mul_ps(s, w));
            acc_wt = _mm_add_ps(acc_wt, w);
        }
    }

    // Horizontal sum both accumulators
    let sum = unsafe { horizontal_sum_f32_sse2(acc_sum) };
    let wt = unsafe { horizontal_sum_f32_sse2(acc_wt) };

    let mut total_sum = sum;
    let mut total_wt = wt;
    for i in (chunks * 4)..len {
        total_sum += samples[i] * weights[i];
        total_wt += weights[i];
    }
    (total_sum, total_wt)
}

#[target_feature(enable = "sse2")]
unsafe fn horizontal_sum_f32_sse2(v: __m128) -> f32 {
    unsafe {
        let shuf = _mm_shuffle_ps(v, v, 0b_01_00_11_10);
        let sum1 = _mm_add_ps(v, shuf);
        let shuf2 = _mm_shuffle_ps(sum1, sum1, 0b_00_01_00_01);
        let sum2 = _mm_add_ps(sum1, shuf2);
        _mm_cvtss_f32(sum2)
    }
}

// ── AVX2 (8 f32 per op) ────────────────────────────────────────────

#[target_feature(enable = "avx2")]
unsafe fn add_buffers_avx2(dst: &mut [f32], src: &[f32]) {
    let len = dst.len().min(src.len());
    let chunks = len / 8;
    for i in 0..chunks {
        let off = i * 8;
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

#[target_feature(enable = "avx2")]
unsafe fn apply_gain_avx2(samples: &mut [f32], gain: f32) {
    let g = unsafe { _mm256_set1_ps(gain) };
    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let off = i * 8;
        unsafe {
            let a = _mm256_loadu_ps(samples.as_ptr().add(off));
            _mm256_storeu_ps(samples.as_mut_ptr().add(off), _mm256_mul_ps(a, g));
        }
    }
    for i in (chunks * 8)..samples.len() {
        samples[i] *= gain;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn clamp_avx2(samples: &mut [f32], min_val: f32, max_val: f32) {
    let vmin = unsafe { _mm256_set1_ps(min_val) };
    let vmax = unsafe { _mm256_set1_ps(max_val) };
    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let off = i * 8;
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

#[target_feature(enable = "avx2")]
unsafe fn peak_abs_avx2(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let abs_mask = unsafe { _mm256_castsi256_ps(_mm256_set1_epi32(0x7FFF_FFFF_u32 as i32)) };
    let mut vmax = unsafe { _mm256_setzero_ps() };

    let chunks = samples.len() / 8;
    for i in 0..chunks {
        let off = i * 8;
        unsafe {
            let a = _mm256_loadu_ps(samples.as_ptr().add(off));
            let abs_a = _mm256_and_ps(a, abs_mask);
            vmax = _mm256_max_ps(vmax, abs_a);
        }
    }

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
