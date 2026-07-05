//! aarch64 SIMD kernels — NEON (baseline, 4 f32 per op).

#[cfg(target_arch = "aarch64")]
use std::arch::aarch64::*;

pub fn add_buffers(dst: &mut [f32], src: &[f32]) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { add_buffers_neon(dst, src) };
}

pub fn apply_gain(samples: &mut [f32], gain: f32) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { apply_gain_neon(samples, gain) };
}

pub fn clamp(samples: &mut [f32], min: f32, max: f32) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { clamp_neon(samples, min, max) };
}

pub fn peak_abs(samples: &[f32]) -> f32 {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { peak_abs_neon(samples) }
}

pub fn sum_of_squares(samples: &[f32]) -> f64 {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { sum_of_squares_neon(samples) }
}

pub fn noise_gate(samples: &mut [f32], threshold: f32) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { noise_gate_neon(samples, threshold) };
}

pub fn i16_to_f32(src: &[i16], dst: &mut [f32]) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { i16_to_f32_neon(src, dst) };
}

pub fn f32_to_i16(src: &[f32], dst: &mut [i16]) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { f32_to_i16_neon(src, dst) };
}

pub fn i24_to_f32(src: &[i32], dst: &mut [f32]) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { i24_to_f32_neon(src, dst) };
}

pub fn f32_to_i24(src: &[f32], dst: &mut [i32]) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { f32_to_i24_neon(src, dst) };
}

pub fn u8_to_f32(src: &[u8], dst: &mut [f32]) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { u8_to_f32_neon(src, dst) };
}

pub fn f32_to_u8(src: &[f32], dst: &mut [u8]) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { f32_to_u8_neon(src, dst) };
}

pub fn biquad_stereo(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    // NEON f64 SIMD (2×f64) processes stereo L+R simultaneously.
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
    unsafe { biquad_stereo_neon(samples, coeffs, state) };
}

pub fn weighted_sum(samples: &[f32], weights: &[f32]) -> (f32, f32) {
    // SAFETY: NEON is always available on aarch64; calling the matching target_feature(enable="neon") function.
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
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        // Storing to slice with bounds checked by loop range.
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
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let g = unsafe { vdupq_n_f32(gain) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        // Storing to slice with bounds checked by loop range.
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
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let vmin = unsafe { vdupq_n_f32(min_val) };
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let vmax = unsafe { vdupq_n_f32(max_val) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        // Storing to slice with bounds checked by loop range.
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
    // SAFETY: NEON intrinsic to create zero vector; no memory access. NEON is always available on aarch64.
    let mut vmax = unsafe { vdupq_n_f32(0.0) };
    let chunks = samples.len() / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        unsafe {
            let a = vld1q_f32(samples.as_ptr().add(off));
            let abs_a = vabsq_f32(a);
            vmax = vmaxq_f32(vmax, abs_a);
        }
    }
    // SAFETY: NEON intrinsic to reduce vector to scalar max; no memory access. NEON is always available on aarch64.
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
    // SAFETY: NEON intrinsic to create zero vector; no memory access. NEON is always available on aarch64.
    let mut acc = unsafe { vdupq_n_f32(0.0) };
    let chunks = samples.len() / 4;

    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        unsafe {
            let a = vld1q_f32(samples.as_ptr().add(off));
            acc = vmlaq_f32(acc, a, a);
        }
        if (i + 1) % 256 == 0 {
            // SAFETY: NEON intrinsic to reduce vector to scalar sum; no memory access.
            total += unsafe { vaddvq_f32(acc) } as f64;
            // SAFETY: NEON intrinsic to create zero vector; no memory access.
            acc = unsafe { vdupq_n_f32(0.0) };
        }
    }
    // SAFETY: NEON intrinsic to reduce vector to scalar sum; no memory access.
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
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let thresh = unsafe { vdupq_n_f32(threshold) };
    // SAFETY: NEON intrinsic to create zero vector; no memory access. NEON is always available on aarch64.
    let zero = unsafe { vdupq_n_f32(0.0) };
    let chunks = samples.len() / 4;

    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        // Storing to slice with bounds checked by loop range.
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
    // SAFETY: NEON intrinsic to create zero vector; no memory access. NEON is always available on aarch64.
    let mut acc_sum = unsafe { vdupq_n_f32(0.0) };
    // SAFETY: NEON intrinsic to create zero vector; no memory access. NEON is always available on aarch64.
    let mut acc_wt = unsafe { vdupq_n_f32(0.0) };

    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        unsafe {
            let s = vld1q_f32(samples.as_ptr().add(off));
            let w = vld1q_f32(weights.as_ptr().add(off));
            acc_sum = vmlaq_f32(acc_sum, s, w);
            acc_wt = vaddq_f32(acc_wt, w);
        }
    }

    // SAFETY: NEON intrinsic to reduce vector to scalar sum; no memory access.
    let mut total_sum = unsafe { vaddvq_f32(acc_sum) };
    // SAFETY: NEON intrinsic to reduce vector to scalar sum; no memory access.
    let mut total_wt = unsafe { vaddvq_f32(acc_wt) };
    for i in (chunks * 4)..len {
        total_sum += samples[i] * weights[i];
        total_wt += weights[i];
    }
    (total_sum, total_wt)
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn i16_to_f32_neon(src: &[i16], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let scale = unsafe { vdupq_n_f32(1.0 / 32768.0) };

    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading 4 i16s from slice with bounds checked by loop range.
        // vld1_s16 loads 4 × i16. vmovl_s16 widens to 4 × i32. vcvtq_f32_s32 converts to 4 × f32.
        // Storing to slice with bounds checked by loop range.
        unsafe {
            let ints_16 = vld1_s16(src.as_ptr().add(off));
            let ints_32 = vmovl_s16(ints_16);
            let floats = vcvtq_f32_s32(ints_32);
            let scaled = vmulq_f32(floats, scale);
            vst1q_f32(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 4)..len {
        dst[i] = src[i] as f32 / 32768.0;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn f32_to_i16_neon(src: &[f32], dst: &mut [i16]) {
    let len = src.len().min(dst.len());
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let vmin = unsafe { vdupq_n_f32(-1.0) };
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let vmax = unsafe { vdupq_n_f32(1.0) };
    // SAFETY: NEON intrinsic to broadcast a scalar; no memory access. NEON is always available on aarch64.
    let scale = unsafe { vdupq_n_f32(32767.0) };

    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range. NEON is always available on aarch64.
        // vminq/vmaxq clamps. vmulq scales. vcvtq_s32_f32 converts to i32. vqmovn_s32 saturating narrows to i16.
        // Storing to slice with bounds checked by loop range.
        unsafe {
            let a = vld1q_f32(src.as_ptr().add(off));
            let clamped = vminq_f32(vmaxq_f32(a, vmin), vmax);
            let scaled = vmulq_f32(clamped, scale);
            let ints = vcvtq_s32_f32(scaled);
            let narrow = vqmovn_s32(ints);
            vst1_s16(dst.as_mut_ptr().add(off), narrow);
        }
    }
    for i in (chunks * 4)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 32767.0) as i16;
    }
}

// ── i24 conversion (NEON) ──────────────────────────────────────────

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn i24_to_f32_neon(src: &[i32], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: NEON intrinsic to broadcast scalar; no memory access. NEON is always available on aarch64.
    let scale = unsafe { vdupq_n_f32(1.0 / 8388608.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range.
        // vshlq/vshrq for sign-extension. vcvtq_f32_s32 for int→float.
        unsafe {
            let raw = vld1q_s32(src.as_ptr().add(off));
            // Sign-extend: (val << 8) >> 8
            let shifted = vshrq_n_s32(vshlq_n_s32(raw, 8), 8);
            let floats = vcvtq_f32_s32(shifted);
            let scaled = vmulq_f32(floats, scale);
            vst1q_f32(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 4)..len {
        let extended = (src[i] << 8) >> 8;
        dst[i] = extended as f32 / 8388608.0;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn f32_to_i24_neon(src: &[f32], dst: &mut [i32]) {
    let len = src.len().min(dst.len());
    // SAFETY: NEON intrinsics to broadcast scalars; no memory access. NEON is always available on aarch64.
    let vmin = unsafe { vdupq_n_f32(-1.0) };
    let vmax = unsafe { vdupq_n_f32(1.0) };
    let scale = unsafe { vdupq_n_f32(8388607.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let a = vld1q_f32(src.as_ptr().add(off));
            let clamped = vminq_f32(vmaxq_f32(a, vmin), vmax);
            let scaled = vmulq_f32(clamped, scale);
            let ints = vcvtq_s32_f32(scaled);
            vst1q_s32(dst.as_mut_ptr().add(off), ints);
        }
    }
    for i in (chunks * 4)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = (clamped * 8388607.0) as i32;
    }
}

// ── u8 conversion (NEON) ───────────────────────────────────────────

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn u8_to_f32_neon(src: &[u8], dst: &mut [f32]) {
    let len = src.len().min(dst.len());
    // SAFETY: NEON intrinsics to broadcast scalars; no memory access. NEON is always available on aarch64.
    let bias = unsafe { vdupq_n_f32(128.0) };
    let inv_scale = unsafe { vdupq_n_f32(1.0 / 128.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Indexed access within bounds checked by loop range.
        // Manual widening: u8→u32→f32.
        unsafe {
            let u32s = vld1q_u32(
                [
                    src[off] as u32,
                    src[off + 1] as u32,
                    src[off + 2] as u32,
                    src[off + 3] as u32,
                ]
                .as_ptr(),
            );
            let floats = vcvtq_f32_u32(u32s);
            let centered = vsubq_f32(floats, bias);
            let scaled = vmulq_f32(centered, inv_scale);
            vst1q_f32(dst.as_mut_ptr().add(off), scaled);
        }
    }
    for i in (chunks * 4)..len {
        dst[i] = (f32::from(src[i]) - 128.0) / 128.0;
    }
}

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn f32_to_u8_neon(src: &[f32], dst: &mut [u8]) {
    let len = src.len().min(dst.len());
    // SAFETY: NEON intrinsics to broadcast scalars; no memory access. NEON is always available on aarch64.
    let vmin = unsafe { vdupq_n_f32(-1.0) };
    let vmax = unsafe { vdupq_n_f32(1.0) };
    let scale = unsafe { vdupq_n_f32(128.0) };
    let bias = unsafe { vdupq_n_f32(128.0) };
    let chunks = len / 4;
    for i in 0..chunks {
        let off = i * 4;
        // SAFETY: Loading from slice with bounds checked by loop range.
        unsafe {
            let a = vld1q_f32(src.as_ptr().add(off));
            let clamped = vminq_f32(vmaxq_f32(a, vmin), vmax);
            let scaled = vaddq_f32(vmulq_f32(clamped, scale), bias);
            let ints = vcvtq_u32_f32(scaled);
            // Extract 4 values
            dst[off] = vgetq_lane_u32(ints, 0) as u8;
            dst[off + 1] = vgetq_lane_u32(ints, 1) as u8;
            dst[off + 2] = vgetq_lane_u32(ints, 2) as u8;
            dst[off + 3] = vgetq_lane_u32(ints, 3) as u8;
        }
    }
    for i in (chunks * 4)..len {
        let clamped = src[i].clamp(-1.0, 1.0);
        dst[i] = ((clamped * 128.0) + 128.0).clamp(0.0, 255.0) as u8;
    }
}

// ── Stereo biquad NEON (2×f64 cross-channel) ──────────────────────

#[cfg(target_arch = "aarch64")]
#[target_feature(enable = "neon")]
unsafe fn biquad_stereo_neon(samples: &mut [f32], coeffs: &[f64; 5], state: &mut [f64; 4]) {
    let [b0, b1, b2, a1, a2] = *coeffs;
    // SAFETY: NEON f64 intrinsics. NEON is always available on aarch64 (f64 SIMD via ASIMD).
    let vb0 = unsafe { vdupq_n_f64(b0) };
    let vb1 = unsafe { vdupq_n_f64(b1) };
    let vb2 = unsafe { vdupq_n_f64(b2) };
    let va1 = unsafe { vdupq_n_f64(a1) };
    let va2 = unsafe { vdupq_n_f64(a2) };

    // SAFETY: vld1q_f64 loads 2×f64 from a valid stack-allocated array pointer.
    // z1 = [z1_L, z1_R], z2 = [z2_L, z2_R]
    let mut vz1 = unsafe { vld1q_f64([state[0], state[2]].as_ptr()) };
    let mut vz2 = unsafe { vld1q_f64([state[1], state[3]].as_ptr()) };

    let frames = samples.len() / 2;
    for f in 0..frames {
        let idx = f * 2;
        // SAFETY: idx+1 < samples.len() because frames = len/2 and idx = f*2 where f < frames.
        // All NEON f64 intrinsics operate on valid registers loaded above.
        unsafe {
            let in_lr = vld1q_f64([samples[idx] as f64, samples[idx + 1] as f64].as_ptr());
            // out = b0 * in + z1
            let out = vaddq_f64(vmulq_f64(vb0, in_lr), vz1);
            // z1 = b1 * in - a1 * out + z2
            vz1 = vaddq_f64(vsubq_f64(vmulq_f64(vb1, in_lr), vmulq_f64(va1, out)), vz2);
            // z2 = b2 * in - a2 * out
            vz2 = vsubq_f64(vmulq_f64(vb2, in_lr), vmulq_f64(va2, out));
            // Store back
            samples[idx] = vgetq_lane_f64(out, 0) as f32;
            samples[idx + 1] = vgetq_lane_f64(out, 1) as f32;
        }
    }

    // SAFETY: vgetq_lane_f64 extracts f64 from valid NEON registers; state has 4 elements.
    unsafe {
        state[0] = vgetq_lane_f64(vz1, 0);
        state[1] = vgetq_lane_f64(vz2, 0);
        state[2] = vgetq_lane_f64(vz1, 1);
        state[3] = vgetq_lane_f64(vz2, 1);
    }
}
