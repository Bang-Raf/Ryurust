//! C-ABI Foreign Function Interface for ryu-audio-fx.

use crate::biquad::{
    process_biquad_filter, process_biquad_filter_and_mix, process_biquad_filter_and_mix_ramp,
    process_double_biquad_filter_and_mix, process_double_biquad_filter_and_mix_ramp,
    process_multi_biquad_filter, BiquadFilterParameter, BiquadFilterState,
};
use std::slice;

pub const RYU_AUDIO_FX_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_audio_fx_version() -> u32 {
    RYU_AUDIO_FX_VERSION
}

/// # Safety
/// `input` must be valid for reads and `output` for writes of `sample_count` floats, and the
/// two ranges must not overlap.
/// `param` and `state` must point to valid values; `state` must not be accessed by anything
/// else during the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_biquad_process(
    param: *const BiquadFilterParameter,
    state: *mut BiquadFilterState,
    output: *mut f32,
    input: *const f32,
    sample_count: usize,
) -> i32 {
    if param.is_null() || state.is_null() || output.is_null() || input.is_null() {
        return -1;
    }
    if sample_count == 0 {
        return 0;
    }

    let param_ref = &*param;
    let state_ref = &mut *state;
    let in_slice = slice::from_raw_parts(input, sample_count);
    let out_slice = slice::from_raw_parts_mut(output, sample_count);

    process_biquad_filter(param_ref, state_ref, out_slice, in_slice, sample_count);
    0
}

/// # Safety
/// `input` must be valid for reads and `output` for writes of `sample_count` floats, and the
/// two ranges must not overlap.
/// `param` and `state` must point to valid values; `state` must not be accessed by anything
/// else during the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_biquad_process_and_mix(
    param: *const BiquadFilterParameter,
    state: *mut BiquadFilterState,
    output: *mut f32,
    input: *const f32,
    sample_count: usize,
    volume: f32,
) -> i32 {
    if param.is_null() || state.is_null() || output.is_null() || input.is_null() {
        return -1;
    }
    if sample_count == 0 {
        return 0;
    }

    let param_ref = &*param;
    let state_ref = &mut *state;
    let in_slice = slice::from_raw_parts(input, sample_count);
    let out_slice = slice::from_raw_parts_mut(output, sample_count);

    process_biquad_filter_and_mix(
        param_ref,
        state_ref,
        out_slice,
        in_slice,
        sample_count,
        volume,
    );
    0
}

/// # Safety
/// `input` must be valid for reads and `output` for writes of `sample_count` floats, and the
/// two ranges must not overlap.
/// `param` and `state` must point to valid values; `state` must not be accessed by anything
/// else during the call.
/// `out_last_sample` must be null or valid for a write of one float.
#[no_mangle]
pub unsafe extern "C" fn ryu_biquad_process_and_mix_ramp(
    param: *const BiquadFilterParameter,
    state: *mut BiquadFilterState,
    output: *mut f32,
    input: *const f32,
    sample_count: usize,
    volume: f32,
    ramp: f32,
    out_last_sample: *mut f32,
) -> i32 {
    if param.is_null() || state.is_null() || output.is_null() || input.is_null() {
        return -1;
    }
    if sample_count == 0 {
        return 0;
    }

    let param_ref = &*param;
    let state_ref = &mut *state;
    let in_slice = slice::from_raw_parts(input, sample_count);
    let out_slice = slice::from_raw_parts_mut(output, sample_count);

    let last_sample = process_biquad_filter_and_mix_ramp(
        param_ref,
        state_ref,
        out_slice,
        in_slice,
        sample_count,
        volume,
        ramp,
    );

    if !out_last_sample.is_null() {
        *out_last_sample = last_sample;
    }

    0
}

/// # Safety
/// `input` must be valid for reads and `output` for writes of `sample_count` floats, and the
/// two ranges must not overlap.
/// `params` must be valid for reads and `states` for reads and writes of `stage_count`
/// elements; `states` must not be accessed by anything else during the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_biquad_process_multi(
    params: *const BiquadFilterParameter,
    states: *mut BiquadFilterState,
    stage_count: usize,
    output: *mut f32,
    input: *const f32,
    sample_count: usize,
) -> i32 {
    if params.is_null() || states.is_null() || output.is_null() || input.is_null() {
        return -1;
    }
    if stage_count == 0 || sample_count == 0 {
        return 0;
    }

    let params_slice = slice::from_raw_parts(params, stage_count);
    let states_slice = slice::from_raw_parts_mut(states, stage_count);
    let in_slice = slice::from_raw_parts(input, sample_count);
    let out_slice = slice::from_raw_parts_mut(output, sample_count);

    process_multi_biquad_filter(
        params_slice,
        states_slice,
        out_slice,
        in_slice,
        sample_count,
    );
    0
}

/// # Safety
/// `input` must be valid for reads and `output` for writes of `sample_count` floats, and the
/// two ranges must not overlap.
/// `param0`/`param1` and `state0`/`state1` must point to valid values; the two states must be
/// distinct and not accessed by anything else during the call.
#[no_mangle]
pub unsafe extern "C" fn ryu_biquad_process_double_and_mix(
    param0: *const BiquadFilterParameter,
    param1: *const BiquadFilterParameter,
    state0: *mut BiquadFilterState,
    state1: *mut BiquadFilterState,
    output: *mut f32,
    input: *const f32,
    sample_count: usize,
    volume: f32,
) -> i32 {
    if param0.is_null()
        || param1.is_null()
        || state0.is_null()
        || state1.is_null()
        || output.is_null()
        || input.is_null()
    {
        return -1;
    }
    if sample_count == 0 {
        return 0;
    }

    let p0 = &*param0;
    let p1 = &*param1;
    let s0 = &mut *state0;
    let s1 = &mut *state1;
    let in_slice = slice::from_raw_parts(input, sample_count);
    let out_slice = slice::from_raw_parts_mut(output, sample_count);

    process_double_biquad_filter_and_mix(p0, p1, s0, s1, out_slice, in_slice, sample_count, volume);
    0
}

/// # Safety
/// `input` must be valid for reads and `output` for writes of `sample_count` floats, and the
/// two ranges must not overlap.
/// `param0`/`param1` and `state0`/`state1` must point to valid values; the two states must be
/// distinct and not accessed by anything else during the call.
/// `out_last_sample` must be null or valid for a write of one float.
#[no_mangle]
pub unsafe extern "C" fn ryu_biquad_process_double_and_mix_ramp(
    param0: *const BiquadFilterParameter,
    param1: *const BiquadFilterParameter,
    state0: *mut BiquadFilterState,
    state1: *mut BiquadFilterState,
    output: *mut f32,
    input: *const f32,
    sample_count: usize,
    volume: f32,
    ramp: f32,
    out_last_sample: *mut f32,
) -> i32 {
    if param0.is_null()
        || param1.is_null()
        || state0.is_null()
        || state1.is_null()
        || output.is_null()
        || input.is_null()
    {
        return -1;
    }
    if sample_count == 0 {
        return 0;
    }

    let p0 = &*param0;
    let p1 = &*param1;
    let s0 = &mut *state0;
    let s1 = &mut *state1;
    let in_slice = slice::from_raw_parts(input, sample_count);
    let out_slice = slice::from_raw_parts_mut(output, sample_count);

    let last_sample = process_double_biquad_filter_and_mix_ramp(
        p0,
        p1,
        s0,
        s1,
        out_slice,
        in_slice,
        sample_count,
        volume,
        ramp,
    );

    if !out_last_sample.is_null() {
        *out_last_sample = last_sample;
    }

    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};

    #[test]
    fn version_export_returns_constant() {
        assert_eq!(ryu_audio_fx_version(), RYU_AUDIO_FX_VERSION);
        assert_eq!(RYU_AUDIO_FX_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        // SAFETY: every call passes null for the required pointers, which each function
        // rejects before touching memory.
        unsafe {
            assert_eq!(
                ryu_biquad_process(null(), null_mut(), null_mut(), null(), 4),
                -1
            );
            assert_eq!(
                ryu_biquad_process_and_mix(null(), null_mut(), null_mut(), null(), 4, 1.0),
                -1
            );
            assert_eq!(
                ryu_biquad_process_and_mix_ramp(
                    null(),
                    null_mut(),
                    null_mut(),
                    null(),
                    4,
                    1.0,
                    0.0,
                    null_mut()
                ),
                -1
            );
            assert_eq!(
                ryu_biquad_process_multi(null(), null_mut(), 2, null_mut(), null(), 4),
                -1
            );
            assert_eq!(
                ryu_biquad_process_double_and_mix(
                    null(),
                    null(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                    4,
                    1.0
                ),
                -1
            );
            assert_eq!(
                ryu_biquad_process_double_and_mix_ramp(
                    null(),
                    null(),
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                    4,
                    1.0,
                    0.0,
                    null_mut()
                ),
                -1
            );
        }
    }
}
