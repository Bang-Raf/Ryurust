//! C-ABI Foreign Function Interface for ryu-audio-fx.

use crate::biquad::{
    process_biquad_filter, process_biquad_filter_and_mix, process_biquad_filter_and_mix_ramp,
    process_double_biquad_filter_and_mix, process_double_biquad_filter_and_mix_ramp,
    process_multi_biquad_filter, BiquadFilterParameter, BiquadFilterState,
};
use std::slice;

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
