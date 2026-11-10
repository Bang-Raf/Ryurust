//! C-ABI Foreign Function Interface for Audio Resampling and Hardware ADPCM Decoding.

use crate::adpcm::{
    decode_adpcm, get_adpcm_data_size, get_nibbles_from_sample_count, AdpcmLoopContext,
};
use crate::resampler::{resample, ResampleQuality};

pub const AUDIO_STATUS_SUCCESS: i32 = 0;
pub const AUDIO_STATUS_NULL_POINTER: i32 = -1;
pub const AUDIO_STATUS_INVALID_ARGUMENT: i32 = -2;

#[no_mangle]
pub unsafe extern "C" fn ryu_adpcm_get_nibbles_from_sample_count(sample_count: usize) -> usize {
    get_nibbles_from_sample_count(sample_count)
}

#[no_mangle]
pub unsafe extern "C" fn ryu_adpcm_get_data_size(sample_count: usize) -> usize {
    get_adpcm_data_size(sample_count)
}

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
