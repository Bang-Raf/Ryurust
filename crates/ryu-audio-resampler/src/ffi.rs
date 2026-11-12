//! C-ABI Foreign Function Interface for Audio Resampling and Hardware ADPCM Decoding.

use crate::adpcm::{
    decode_adpcm, get_adpcm_data_size, get_nibbles_from_sample_count, AdpcmLoopContext,
};
use crate::resampler::{resample, ResampleQuality};

pub const AUDIO_STATUS_SUCCESS: i32 = 0;
pub const AUDIO_STATUS_NULL_POINTER: i32 = -1;
pub const AUDIO_STATUS_INVALID_ARGUMENT: i32 = -2;

pub const RYU_AUDIO_RESAMPLER_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_audio_resampler_version() -> u32 {
    RYU_AUDIO_RESAMPLER_VERSION
}

/// # Safety
/// This function takes no pointers and has no requirements; it is `unsafe` only because it
/// was originally exported that way.
#[no_mangle]
pub unsafe extern "C" fn ryu_adpcm_get_nibbles_from_sample_count(sample_count: usize) -> usize {
    get_nibbles_from_sample_count(sample_count)
}

/// # Safety
/// This function takes no pointers and has no requirements; it is `unsafe` only because it
/// was originally exported that way.
#[no_mangle]
pub unsafe extern "C" fn ryu_adpcm_get_data_size(sample_count: usize) -> usize {
    get_adpcm_data_size(sample_count)
}

/// # Safety
/// `output` must be valid for writes of `output_len` floats and `input` for reads of
/// `input_len` samples, without overlapping. `fraction_ptr` must point to a valid float that
/// is read and written.
#[no_mangle]
pub unsafe extern "C" fn ryu_audio_resample(
    output: *mut f32,
    output_len: usize,
    input: *const i16,
    input_len: usize,
    ratio: f32,
    fraction_ptr: *mut f32,
    sample_count: i32,
    quality: u8,
) -> i32 {
    if output.is_null() || input.is_null() || fraction_ptr.is_null() {
        return AUDIO_STATUS_NULL_POINTER;
    }
    if sample_count <= 0 || ratio <= 0.0 {
        return AUDIO_STATUS_INVALID_ARGUMENT;
    }
    let count = sample_count as usize;
    if output_len < count {
        return AUDIO_STATUS_INVALID_ARGUMENT;
    }

    let out_slice = std::slice::from_raw_parts_mut(output, output_len);
    let in_slice = std::slice::from_raw_parts(input, input_len);

    let mut fraction = *fraction_ptr;
    resample(
        out_slice,
        in_slice,
        ratio,
        &mut fraction,
        count,
        ResampleQuality::from(quality),
    );
    *fraction_ptr = fraction;

    AUDIO_STATUS_SUCCESS
}

/// # Safety
/// `output` must be valid for writes of `output_len` samples, `input` for reads of `input_len`
/// bytes and `coefficients` for reads of `coeff_len` values, with `output` overlapping neither
/// of the others. `loop_context` must point to a valid `AdpcmLoopContext` not accessed by
/// anything else during the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_adpcm_decode(
    output: *mut i16,
    output_len: usize,
    input: *const u8,
    input_len: usize,
    start_sample_offset: i32,
    end_sample_offset: i32,
    offset: i32,
    count: i32,
    coefficients: *const i16,
    coeff_len: usize,
    loop_context: *mut AdpcmLoopContext,
) -> i32 {
    if output.is_null() || input.is_null() || coefficients.is_null() || loop_context.is_null() {
        return AUDIO_STATUS_NULL_POINTER;
    }
    if count <= 0 || output_len == 0 || input_len == 0 {
        return 0;
    }

    let out_slice = std::slice::from_raw_parts_mut(output, output_len);
    let in_slice = std::slice::from_raw_parts(input, input_len);
    let coeff_slice = std::slice::from_raw_parts(coefficients, coeff_len);

    decode_adpcm(
        out_slice,
        in_slice,
        start_sample_offset,
        end_sample_offset,
        offset,
        count,
        coeff_slice,
        &mut *loop_context,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};

    #[test]
    fn version_export_returns_constant() {
        assert_eq!(ryu_audio_resampler_version(), RYU_AUDIO_RESAMPLER_VERSION);
        assert_eq!(RYU_AUDIO_RESAMPLER_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        // SAFETY: every call passes null for the required pointers, which each function
        // rejects before touching memory.
        unsafe {
            assert_eq!(
                ryu_audio_resample(null_mut(), 4, null(), 4, 1.0, null_mut(), 4, 0),
                AUDIO_STATUS_NULL_POINTER
            );
            assert_eq!(
                ryu_adpcm_decode(null_mut(), 4, null(), 4, 0, 0, 0, 4, null(), 16, null_mut()),
                AUDIO_STATUS_NULL_POINTER
            );
        }
    }
}
