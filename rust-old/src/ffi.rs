//! C-compatible FFI for key dhvani types.
//!
//! Provides an opaque handle API for creating and processing audio buffers
//! from C, Python (via ctypes), or other FFI consumers.
//!
//! # Memory model
//!
//! - `nada_buffer_create` allocates — caller must call `nada_buffer_free`
//! - `nada_buffer_json` returns a C string — caller must call `nada_free_string`
//! - All functions validate input pointers before dereferencing

use std::ffi::CString;
use std::os::raw::c_char;

use crate::buffer::AudioBuffer;

/// Opaque handle to an AudioBuffer.
pub struct NadaBuffer(AudioBuffer);

/// Create a silent audio buffer.
///
/// # Safety
/// Caller must free the returned pointer with `nada_buffer_free`.
#[unsafe(no_mangle)]
pub extern "C" fn nada_buffer_silence(
    channels: u32,
    frames: usize,
    sample_rate: u32,
) -> *mut NadaBuffer {
    if channels == 0 || sample_rate == 0 {
        tracing::warn!(
            channels,
            sample_rate,
            "nada_buffer_silence: returning null due to invalid params"
        );
        return std::ptr::null_mut();
    }
    let buf = AudioBuffer::silence(channels, frames, sample_rate);
    Box::into_raw(Box::new(NadaBuffer(buf)))
}

/// Create a buffer from interleaved f32 samples.
///
/// # Safety
/// - `samples` must point to at least `len` valid f32 values
/// - Caller must free the returned pointer with `nada_buffer_free`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_from_interleaved(
    samples: *const f32,
    len: usize,
    channels: u32,
    sample_rate: u32,
) -> *mut NadaBuffer {
    if samples.is_null() || len == 0 || channels == 0 || sample_rate == 0 {
        tracing::warn!(
            samples_null = samples.is_null(),
            len,
            channels,
            sample_rate,
            "nada_buffer_from_interleaved: returning null due to invalid params"
        );
        return std::ptr::null_mut();
    }
    // SAFETY: Caller guarantees samples points to len valid f32 values.
    let slice = unsafe { std::slice::from_raw_parts(samples, len) };
    match AudioBuffer::from_interleaved(slice.to_vec(), channels, sample_rate) {
        Ok(buf) => Box::into_raw(Box::new(NadaBuffer(buf))),
        Err(e) => {
            tracing::warn!(
                error = %e,
                "nada_buffer_from_interleaved: returning null due to construction error"
            );
            std::ptr::null_mut()
        }
    }
}

/// Free a buffer.
///
/// # Safety
/// `ptr` must be a valid pointer returned by a `nada_buffer_*` function, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_free(ptr: *mut NadaBuffer) {
    if !ptr.is_null() {
        // SAFETY: Caller guarantees ptr was allocated by us.
        drop(unsafe { Box::from_raw(ptr) });
    }
}

/// Get the number of frames in a buffer.
///
/// # Safety
/// `ptr` must be a valid NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_frames(ptr: *const NadaBuffer) -> usize {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: Caller guarantees valid pointer.
    unsafe { (*ptr).0.frames }
}

/// Get the number of channels.
///
/// # Safety
/// `ptr` must be a valid NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_channels(ptr: *const NadaBuffer) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
    unsafe { (*ptr).0.channels }
}

/// Get the sample rate.
///
/// # Safety
/// `ptr` must be a valid NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_sample_rate(ptr: *const NadaBuffer) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
    unsafe { (*ptr).0.sample_rate }
}

/// Get the peak amplitude.
///
/// # Safety
/// `ptr` must be a valid NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_peak(ptr: *const NadaBuffer) -> f32 {
    if ptr.is_null() {
        return 0.0;
    }
    // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
    unsafe { (*ptr).0.peak() }
}

/// Get the RMS level.
///
/// # Safety
/// `ptr` must be a valid NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_rms(ptr: *const NadaBuffer) -> f32 {
    if ptr.is_null() {
        return 0.0;
    }
    // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
    unsafe { (*ptr).0.rms() }
}

/// Apply gain to a buffer.
///
/// # Safety
/// `ptr` must be a valid mutable NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_apply_gain(ptr: *mut NadaBuffer, gain: f32) {
    if !ptr.is_null() {
        // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
        unsafe { (*ptr).0.apply_gain(gain) };
    }
}

/// Clamp buffer samples to [-1.0, 1.0].
///
/// # Safety
/// `ptr` must be a valid mutable NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_clamp(ptr: *mut NadaBuffer) {
    if !ptr.is_null() {
        // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
        unsafe { (*ptr).0.clamp() };
    }
}

/// Apply noise gate.
///
/// # Safety
/// `ptr` must be a valid mutable NadaBuffer pointer.
#[cfg(feature = "dsp")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_noise_gate(ptr: *mut NadaBuffer, threshold: f32) {
    if !ptr.is_null() {
        // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
        crate::dsp::noise_gate(unsafe { &mut (*ptr).0 }, threshold);
    }
}

/// Apply hard limiter.
///
/// # Safety
/// `ptr` must be a valid mutable NadaBuffer pointer.
#[cfg(feature = "dsp")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_hard_limiter(ptr: *mut NadaBuffer, ceiling: f32) {
    if !ptr.is_null() {
        // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
        crate::dsp::hard_limiter(unsafe { &mut (*ptr).0 }, ceiling);
    }
}

/// Get a pointer to the raw sample data (read-only).
///
/// # Safety
/// `ptr` must be a valid NadaBuffer pointer. The returned pointer is valid
/// only as long as the buffer is not freed or mutated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_samples(ptr: *const NadaBuffer) -> *const f32 {
    if ptr.is_null() {
        return std::ptr::null();
    }
    // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
    unsafe { (*ptr).0.samples.as_ptr() }
}

/// Get the total number of samples (frames * channels).
///
/// # Safety
/// `ptr` must be a valid NadaBuffer pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_buffer_total_samples(ptr: *const NadaBuffer) -> usize {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: Caller guarantees ptr is non-null and was allocated by this library.
    unsafe { (*ptr).0.total_samples() }
}

/// Free a C string returned by dhvani.
///
/// # Safety
/// `ptr` must be a valid C string returned by a `nada_*` function, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn nada_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        // SAFETY: Caller guarantees ptr was allocated by CString.
        drop(unsafe { CString::from_raw(ptr) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_silence_and_free() {
        let buf = nada_buffer_silence(2, 1024, 48000);
        assert!(!buf.is_null());
        // SAFETY: buf was just created by nada_buffer_silence and is non-null.
        unsafe {
            assert_eq!(nada_buffer_frames(buf), 1024);
            assert_eq!(nada_buffer_channels(buf), 2);
            assert_eq!(nada_buffer_sample_rate(buf), 48000);
            assert_eq!(nada_buffer_peak(buf), 0.0);
            nada_buffer_free(buf);
        }
    }

    #[test]
    fn ffi_from_interleaved() {
        let samples = [0.5f32, -0.5, 0.3, -0.3];
        // SAFETY: samples.as_ptr() points to samples.len() valid f32 values.
        let buf =
            unsafe { nada_buffer_from_interleaved(samples.as_ptr(), samples.len(), 2, 44100) };
        assert!(!buf.is_null());
        // SAFETY: buf was just created above and is non-null.
        unsafe {
            assert_eq!(nada_buffer_frames(buf), 2);
            assert!(nada_buffer_peak(buf) > 0.4);
            nada_buffer_free(buf);
        }
    }

    #[test]
    fn ffi_null_safety() {
        // SAFETY: Testing null-pointer handling; all FFI functions are null-safe.
        unsafe {
            assert_eq!(nada_buffer_frames(std::ptr::null()), 0);
            assert_eq!(nada_buffer_channels(std::ptr::null()), 0);
            assert_eq!(nada_buffer_peak(std::ptr::null()), 0.0);
            nada_buffer_free(std::ptr::null_mut());
            nada_free_string(std::ptr::null_mut());
        }
    }

    #[test]
    fn ffi_apply_gain() {
        let buf = nada_buffer_silence(1, 4, 44100);
        // Write some data
        // SAFETY: Pointers created by this library's FFI constructors with valid data.
        unsafe {
            let samples = [0.5f32, -0.5, 0.25, -0.25];
            let buf2 = nada_buffer_from_interleaved(samples.as_ptr(), 4, 1, 44100);
            nada_buffer_apply_gain(buf2, 2.0);
            assert!((nada_buffer_peak(buf2) - 1.0).abs() < f32::EPSILON);
            nada_buffer_free(buf2);
            nada_buffer_free(buf);
        }
    }

    #[test]
    fn ffi_invalid_params_return_null() {
        // SAFETY: Testing invalid parameter handling; functions are designed to return null.
        unsafe {
            assert!(nada_buffer_from_interleaved(std::ptr::null(), 0, 2, 44100).is_null());
            assert!(nada_buffer_from_interleaved(std::ptr::null(), 10, 0, 44100).is_null());
        }
    }

    #[test]
    fn ffi_rms() {
        let samples = vec![0.5f32; 100];
        // SAFETY: samples.as_ptr() points to samples.len() valid f32 values.
        let buf =
            unsafe { nada_buffer_from_interleaved(samples.as_ptr(), samples.len(), 1, 44100) };
        assert!(!buf.is_null());
        // SAFETY: buf was just created above and is non-null.
        unsafe {
            let rms = nada_buffer_rms(buf);
            assert!((rms - 0.5).abs() < 0.01);
            assert_eq!(nada_buffer_rms(std::ptr::null()), 0.0);
            nada_buffer_free(buf);
        }
    }

    #[test]
    fn ffi_clamp() {
        let samples = [2.0f32, -2.0, 0.5];
        // SAFETY: samples.as_ptr() points to samples.len() valid f32 values.
        let buf =
            unsafe { nada_buffer_from_interleaved(samples.as_ptr(), samples.len(), 1, 44100) };
        // SAFETY: buf was just created above and is non-null.
        unsafe {
            nada_buffer_clamp(buf);
            assert!((nada_buffer_peak(buf) - 1.0).abs() < f32::EPSILON);
            nada_buffer_clamp(std::ptr::null_mut()); // null safety
            nada_buffer_free(buf);
        }
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn ffi_noise_gate() {
        let samples = [0.01f32, 0.5, 0.001, 0.8];
        // SAFETY: samples.as_ptr() points to samples.len() valid f32 values.
        let buf =
            unsafe { nada_buffer_from_interleaved(samples.as_ptr(), samples.len(), 1, 44100) };
        // SAFETY: buf was just created above and is non-null.
        unsafe {
            nada_buffer_noise_gate(buf, 0.1);
            nada_buffer_noise_gate(std::ptr::null_mut(), 0.1); // null safety
            nada_buffer_free(buf);
        }
    }

    #[cfg(feature = "dsp")]
    #[test]
    fn ffi_hard_limiter() {
        let samples = [2.0f32, -2.0, 0.5];
        // SAFETY: samples.as_ptr() points to samples.len() valid f32 values.
        let buf =
            unsafe { nada_buffer_from_interleaved(samples.as_ptr(), samples.len(), 1, 44100) };
        // SAFETY: buf was just created above and is non-null.
        unsafe {
            nada_buffer_hard_limiter(buf, 1.0);
            assert!((nada_buffer_peak(buf) - 1.0).abs() < f32::EPSILON);
            nada_buffer_hard_limiter(std::ptr::null_mut(), 1.0); // null safety
            nada_buffer_free(buf);
        }
    }

    #[test]
    fn ffi_samples_ptr() {
        let samples = [0.5f32, -0.5];
        // SAFETY: samples.as_ptr() points to samples.len() valid f32 values.
        let buf =
            unsafe { nada_buffer_from_interleaved(samples.as_ptr(), samples.len(), 1, 44100) };
        // SAFETY: buf was just created above and is non-null.
        unsafe {
            let ptr = nada_buffer_samples(buf);
            assert!(!ptr.is_null());
            assert_eq!(nada_buffer_total_samples(buf), 2);
            assert_eq!(nada_buffer_samples(std::ptr::null()), std::ptr::null());
            assert_eq!(nada_buffer_total_samples(std::ptr::null()), 0);
            nada_buffer_free(buf);
        }
    }

    #[test]
    fn ffi_sample_rate_null() {
        // SAFETY: Testing null-pointer handling; function is null-safe.
        unsafe {
            assert_eq!(nada_buffer_sample_rate(std::ptr::null()), 0);
        }
    }

    #[test]
    fn ffi_apply_gain_null() {
        // SAFETY: Testing null-pointer handling; function is null-safe.
        unsafe {
            nada_buffer_apply_gain(std::ptr::null_mut(), 2.0); // should not panic
        }
    }

    #[test]
    fn ffi_zero_sample_rate_returns_null() {
        let samples = [0.5f32; 4];
        // SAFETY: samples.as_ptr() points to valid data; testing that sample_rate=0 returns null.
        unsafe {
            let buf = nada_buffer_from_interleaved(samples.as_ptr(), samples.len(), 1, 0);
            assert!(buf.is_null());
        }
    }
}
