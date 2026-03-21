//! aarch64 SIMD kernels — NEON (baseline, 4 f32 per op).

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

pub fn add_buffers(dst: &mut [f32], src: &[f32]) {
    unsafe { add_buffers_neon(dst, src) };
}

pub fn apply_gain(samples: &mut [f32], gain: f32) {
    unsafe { apply_gain_neon(samples, gain) };
}

pub fn clamp(samples: &mut [f32], min: f32, max: f32) {
    unsafe { clamp_neon(samples, min, max) };
}

pub fn peak_abs(samples: &[f32]) -> f32 {
    unsafe { peak_abs_neon(samples) }
}

pub fn sum_of_squares(samples: &[f32]) -> f64 {
    unsafe { sum_of_squares_neon(samples) }
}

pub fn noise_gate(samples: &mut [f32], threshold: f32) {
    unsafe { noise_gate_neon(samples, threshold) };
}

pub fn i16_to_f32(src: &[i16], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        dst[i] = src[i] as f32 / 32768.0;
    }
}

pub fn f32_to_i16(src: &[f32], dst: &mut [i16]) {
    let len = src.len().min(dst.len());
    for i in 0..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 32767.0) as i16;
    }
}

pub fn weighted_sum(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    unsafe { weighted_sum_neon(samples, weights) }
}

// ── NEON implementations ────────────────────────────────────────────

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn add_buffers_neon(dst: &mut [f32], src: &[f32]) {
    let len = dst.len().min(src.len());
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = vld1q_f32(dst.as_ptr().add(off));
            let b = vld1q_f32(src.as_ptr().add(off));
            vst1q_f32(dst.as_mut_ptr().add(off), vaddq_f32(a, b));
        }
    }
    for i in (chunks * 4)..len {
        dst[i] += src[i];
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn apply_gain_neon(samples: &mut [f32], gain: f32) {
    let g = unsafe { vdupq_n_f32(gain) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = vld1q_f32(samples.as_ptr().add(off));
            vst1q_f32(samples.as_mut_ptr().add(off), vmulq_f32(a, g));
        }
    }
    for i in (chunks * 4)..samples.len() {
        samples[i] *= gain;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn clamp_neon(samples: &mut [f32], min_val: f32, max_val: f32) {
    let vmin = unsafe { vdupq_n_f32(min_val) };
    let vmax = unsafe { vdupq_n_f32(max_val) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = vld1q_f32(samples.as_ptr().add(off));
            let clamped = vminq_f32(vmaxq_f32(a, vmin), vmax);
            vst1q_f32(samples.as_mut_ptr().add(off), clamped);
        }
    }
    for i in (chunks * 4)..samples.len() {
        samples[i] = samples[i].clamp(min_val, max_val);
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn peak_abs_neon(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut vmax = unsafe { vdupq_n_f32(0.0) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = vld1q_f32(samples.as_ptr().add(off));
            let abs_a = vabsq_f32(a);
            vmax = vmaxq_f32(vmax, abs_a);
        }
    }
    let mut result = unsafe { vmaxvq_f32(vmax) };
    for i in (chunks * 4)..samples.len() {
        result = result.max(samples[i].abs());
    }
    result
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn sum_of_squares_neon(samples: &[f32]) -> f64 {
    let mut total = 0.0f64;
    let mut acc = unsafe { vdupq_n_f32(0.0) };
    let chunks = samples.len() / 4;

    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = vld1q_f32(samples.as_ptr().add(off));
            acc = vmlaq_f32(acc, a, a);
        }
        if (i + 1) % 256 == 0 {
            total += unsafe { vaddvq_f32(acc) } as f64;
            acc = unsafe { vdupq_n_f32(0.0) };
        }
    }
    total += unsafe { vaddvq_f32(acc) } as f64;

    for i in (chunks * 4)..samples.len() {
        let s = samples[i] as f64;
        total += s * s;
    }
    total
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn noise_gate_neon(samples: &mut [f32], threshold: f32) {
    let thresh = unsafe { vdupq_n_f32(threshold) };
    let zero = unsafe { vdupq_n_f32(0.0) };
    let chunks = samples.len() / 4;

    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let a = vld1q_f32(samples.as_ptr().add(off));
            let abs_a = vabsq_f32(a);
            let mask = vcgeq_f32(abs_a, thresh);
            let result = vbslq_f32(mask, a, zero);
            vst1q_f32(samples.as_mut_ptr().add(off), result);
        }
    }
    for i in (chunks * 4)..samples.len() {
        if samples[i].abs() < threshold {
            samples[i] = 0.0;
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn weighted_sum_neon(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    let len = samples.len().min(weights.len());
    let chunks = len / 4;
    let mut acc_sum = unsafe { vdupq_n_f32(0.0) };
    let mut acc_wt = unsafe { vdupq_n_f32(0.0) };

    for i in 0..chunks {
        let off = i * 4;
        unsafe {
            let s = vld1q_f32(samples.as_ptr().add(off));
            let w = vld1q_f32(weights.as_ptr().add(off));
            acc_sum = vmlaq_f32(acc_sum, s, w);
            acc_wt = vaddq_f32(acc_wt, w);
        }
    }

    let mut total_sum = unsafe { vaddvq_f32(acc_sum) };
    let mut total_wt = unsafe { vaddvq_f32(acc_wt) };
    for i in (chunks * 4)..len {
        total_sum += samples[i] * weights[i];
        total_wt += weights[i];
    }
    (total_sum, total_wt)
}
