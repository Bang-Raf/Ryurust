use crate::batch::{process_command_batch, DspCommandEntry};
use crate::dsp;
use crate::filter::{
    process_biquad_filter_and_mix_df1, process_biquad_filter_and_mix_df1_in_place,
    process_biquad_filter_and_mix_ramp_df1, process_biquad_filter_and_mix_ramp_df1_in_place,
    process_biquad_filter_df2, process_biquad_filter_df2_in_place,
    process_double_biquad_filter_and_mix, process_double_biquad_filter_and_mix_ramp,
    process_multi_tap_biquad_filter, BiquadFilterParameter, BiquadFilterState,
};
use std::slice;

pub const DSP_STATUS_OK: i32 = 0;
pub const DSP_STATUS_NULL_POINTER: i32 = -1;
pub const DSP_STATUS_INVALID_ARGUMENT: i32 = -2;
pub const DSP_STATUS_OUT_OF_BOUNDS: i32 = -3;

// Aliasing contract for the `output` / `input` pairs below: the two ranges must either be
// disjoint or start at the same address (in-place processing, which is handled with a single
// `&mut [f32]`). Partially overlapping ranges are not supported.

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_clear_buffer(buffer: *mut f32, count: usize) -> i32 {
    if buffer.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    if count == 0 {
        return DSP_STATUS_OK;
    }
    let s = slice::from_raw_parts_mut(buffer, count);
    dsp::clear_buffer(s);
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_clear_buffers(buffers: *mut f32, total_floats: usize) -> i32 {
    if buffers.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    if total_floats == 0 {
        return DSP_STATUS_OK;
    }
    let s = slice::from_raw_parts_mut(buffers, total_floats);
    dsp::clear_buffer(s);
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_copy_buffer(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
) -> i32 {
    if output.is_null() || input.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    // SAFETY: caller guarantees `n` valid floats behind each pointer; `ptr::copy` (memmove)
    // is used because `output == input` is allowed.
    std::ptr::copy(input, output, n);
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_mix(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    volume: f32,
) -> i32 {
    if output.is_null() || input.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid, writable floats at `output`; input aliases it
        // exactly, so only one (mutable) slice is created.
        let buf = slice::from_raw_parts_mut(output, n);
        dsp::mix_in_place(buf, volume);
    } else {
        // SAFETY: caller guarantees `n` valid floats behind each pointer and that the two
        // ranges are disjoint when the pointers differ.
        let dst = slice::from_raw_parts_mut(output, n);
        let src = slice::from_raw_parts(input, n);
        dsp::mix(dst, src, volume);
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_mix_ramp(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    volume0: f32,
    volume1: f32,
    last_sample_out: *mut f32,
) -> i32 {
    if output.is_null() || input.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        if !last_sample_out.is_null() {
            *last_sample_out = 0.0;
        }
        return DSP_STATUS_OK;
    }
    let last = if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid, writable floats at `output`; input aliases it
        // exactly, so only one (mutable) slice is created.
        let buf = slice::from_raw_parts_mut(output, n);
        dsp::mix_ramp_in_place(buf, volume0, volume1)
    } else {
        // SAFETY: caller guarantees `n` valid floats behind each pointer and that the two
        // ranges are disjoint when the pointers differ.
        let dst = slice::from_raw_parts_mut(output, n);
        let src = slice::from_raw_parts(input, n);
        dsp::mix_ramp(dst, src, volume0, volume1)
    };
    if !last_sample_out.is_null() {
        *last_sample_out = last;
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_volume(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    volume: f32,
) -> i32 {
    if output.is_null() || input.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid, writable floats at `output`; input aliases it
        // exactly, so only one (mutable) slice is created.
        let buf = slice::from_raw_parts_mut(output, n);
        dsp::volume_in_place(buf, volume);
    } else {
        // SAFETY: caller guarantees `n` valid floats behind each pointer and that the two
        // ranges are disjoint when the pointers differ.
        let dst = slice::from_raw_parts_mut(output, n);
        let src = slice::from_raw_parts(input, n);
        dsp::volume(dst, src, volume);
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_volume_ramp(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    volume0: f32,
    volume1: f32,
) -> i32 {
    if output.is_null() || input.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid, writable floats at `output`; input aliases it
        // exactly, so only one (mutable) slice is created.
        let buf = slice::from_raw_parts_mut(output, n);
        dsp::volume_ramp_in_place(buf, volume0, volume1);
    } else {
        // SAFETY: caller guarantees `n` valid floats behind each pointer and that the two
        // ranges are disjoint when the pointers differ.
        let dst = slice::from_raw_parts_mut(output, n);
        let src = slice::from_raw_parts(input, n);
        dsp::volume_ramp(dst, src, volume0, volume1);
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_biquad_filter(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    param: *const BiquadFilterParameter,
    state: *mut BiquadFilterState,
) -> i32 {
    if output.is_null() || input.is_null() || param.is_null() || state.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid, writable floats at `output`; input aliases it
        // exactly, so only one (mutable) slice is created.
        let buf = slice::from_raw_parts_mut(output, n);
        process_biquad_filter_df2_in_place(&*param, &mut *state, buf);
    } else {
        // SAFETY: caller guarantees `n` valid floats behind each pointer and that the two
        // ranges are disjoint when the pointers differ.
        let dst = slice::from_raw_parts_mut(output, n);
        let src = slice::from_raw_parts(input, n);
        process_biquad_filter_df2(&*param, &mut *state, dst, src);
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_biquad_filter_and_mix(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    param: *const BiquadFilterParameter,
    state: *mut BiquadFilterState,
    volume: f32,
) -> i32 {
    if output.is_null() || input.is_null() || param.is_null() || state.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid, writable floats at `output`; input aliases it
        // exactly, so only one (mutable) slice is created.
        let buf = slice::from_raw_parts_mut(output, n);
        process_biquad_filter_and_mix_df1_in_place(&*param, &mut *state, buf, volume);
    } else {
        // SAFETY: caller guarantees `n` valid floats behind each pointer and that the two
        // ranges are disjoint when the pointers differ.
        let dst = slice::from_raw_parts_mut(output, n);
        let src = slice::from_raw_parts(input, n);
        process_biquad_filter_and_mix_df1(&*param, &mut *state, dst, src, volume);
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_biquad_filter_and_mix_ramp(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    param: *const BiquadFilterParameter,
    state: *mut BiquadFilterState,
    volume0: f32,
    volume1: f32,
    last_sample_out: *mut f32,
) -> i32 {
    if output.is_null() || input.is_null() || param.is_null() || state.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        if !last_sample_out.is_null() {
            *last_sample_out = 0.0;
        }
        return DSP_STATUS_OK;
    }
    let last = if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid, writable floats at `output`; input aliases it
        // exactly, so only one (mutable) slice is created.
        let buf = slice::from_raw_parts_mut(output, n);
        process_biquad_filter_and_mix_ramp_df1_in_place(&*param, &mut *state, buf, volume0, volume1)
    } else {
        // SAFETY: caller guarantees `n` valid floats behind each pointer and that the two
        // ranges are disjoint when the pointers differ.
        let dst = slice::from_raw_parts_mut(output, n);
        let src = slice::from_raw_parts(input, n);
        process_biquad_filter_and_mix_ramp_df1(&*param, &mut *state, dst, src, volume0, volume1)
    };
    if !last_sample_out.is_null() {
        *last_sample_out = last;
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_double_biquad_filter_and_mix(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    param0: *const BiquadFilterParameter,
    param1: *const BiquadFilterParameter,
    state0: *mut BiquadFilterState,
    state1: *mut BiquadFilterState,
    volume: f32,
) -> i32 {
    if output.is_null()
        || input.is_null()
        || param0.is_null()
        || param1.is_null()
        || state0.is_null()
        || state1.is_null()
    {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    // In-place calls (`output == input`) take a copy of the input first so that no `&[f32]`
    // aliases the `&mut [f32]`; these cascades are rare enough that the copy is acceptable.
    let input_copy;
    let src: &[f32] = if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid floats at `input`; the shared slice is dropped
        // (copied out) before the mutable slice over the same memory is created.
        input_copy = slice::from_raw_parts(input, n).to_vec();
        &input_copy
    } else {
        // SAFETY: caller guarantees `n` valid floats at `input`, disjoint from `output`.
        slice::from_raw_parts(input, n)
    };
    // SAFETY: caller guarantees `n` valid, writable floats at `output`.
    let dst = slice::from_raw_parts_mut(output, n);
    process_double_biquad_filter_and_mix(
        &*param0,
        &*param1,
        &mut *state0,
        &mut *state1,
        dst,
        src,
        volume,
    );
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_double_biquad_filter_and_mix_ramp(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    param0: *const BiquadFilterParameter,
    param1: *const BiquadFilterParameter,
    state0: *mut BiquadFilterState,
    state1: *mut BiquadFilterState,
    volume0: f32,
    volume1: f32,
    last_sample_out: *mut f32,
) -> i32 {
    if output.is_null()
        || input.is_null()
        || param0.is_null()
        || param1.is_null()
        || state0.is_null()
        || state1.is_null()
    {
        return DSP_STATUS_NULL_POINTER;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        if !last_sample_out.is_null() {
            *last_sample_out = 0.0;
        }
        return DSP_STATUS_OK;
    }
    // In-place calls (`output == input`) take a copy of the input first so that no `&[f32]`
    // aliases the `&mut [f32]`; these cascades are rare enough that the copy is acceptable.
    let input_copy;
    let src: &[f32] = if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid floats at `input`; the shared slice is dropped
        // (copied out) before the mutable slice over the same memory is created.
        input_copy = slice::from_raw_parts(input, n).to_vec();
        &input_copy
    } else {
        // SAFETY: caller guarantees `n` valid floats at `input`, disjoint from `output`.
        slice::from_raw_parts(input, n)
    };
    // SAFETY: caller guarantees `n` valid, writable floats at `output`.
    let dst = slice::from_raw_parts_mut(output, n);
    let last = process_double_biquad_filter_and_mix_ramp(
        &*param0,
        &*param1,
        &mut *state0,
        &mut *state1,
        dst,
        src,
        volume0,
        volume1,
    );
    if !last_sample_out.is_null() {
        *last_sample_out = last;
    }
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_multi_tap_biquad_filter(
    output: *mut f32,
    output_len: usize,
    input: *const f32,
    input_len: usize,
    params: *const BiquadFilterParameter,
    states: *mut BiquadFilterState,
    stage_count: usize,
) -> i32 {
    if output.is_null() || input.is_null() || params.is_null() || states.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    if stage_count == 0 {
        return DSP_STATUS_OK;
    }
    let n = output_len.min(input_len);
    if n == 0 {
        return DSP_STATUS_OK;
    }
    // In-place calls (`output == input`) take a copy of the input first so that no `&[f32]`
    // aliases the `&mut [f32]`; these cascades are rare enough that the copy is acceptable.
    let input_copy;
    let src: &[f32] = if std::ptr::eq(output.cast_const(), input) {
        // SAFETY: caller guarantees `n` valid floats at `input`; the shared slice is dropped
        // (copied out) before the mutable slice over the same memory is created.
        input_copy = slice::from_raw_parts(input, n).to_vec();
        &input_copy
    } else {
        // SAFETY: caller guarantees `n` valid floats at `input`, disjoint from `output`.
        slice::from_raw_parts(input, n)
    };
    // SAFETY: caller guarantees `n` valid, writable floats at `output`.
    let dst = slice::from_raw_parts_mut(output, n);
    let p_slice = slice::from_raw_parts(params, stage_count);
    let s_slice = slice::from_raw_parts_mut(states, stage_count);
    process_multi_tap_biquad_filter(p_slice, s_slice, dst, src);
    DSP_STATUS_OK
}

#[no_mangle]
pub unsafe extern "C" fn ryu_dsp_process_command_batch(
    buffers: *mut f32,
    buffer_count: usize,
    sample_count: usize,
    commands: *const DspCommandEntry,
    command_count: usize,
    states: *mut BiquadFilterState,
    state_count: usize,
    last_samples: *mut f32,
    last_samples_count: usize,
) -> i32 {
    if buffers.is_null() || commands.is_null() {
        return DSP_STATUS_NULL_POINTER;
    }
    if buffer_count == 0 || sample_count == 0 || command_count == 0 {
        return DSP_STATUS_OK;
    }

    let total_floats = buffer_count.saturating_mul(sample_count);
    let mix_slice = slice::from_raw_parts_mut(buffers, total_floats);
    let cmd_slice = slice::from_raw_parts(commands, command_count);

    let states_slice = if !states.is_null() && state_count > 0 {
        slice::from_raw_parts_mut(states, state_count)
    } else {
        &mut []
    };

    let ls_slice = if !last_samples.is_null() && last_samples_count > 0 {
        slice::from_raw_parts_mut(last_samples, last_samples_count)
    } else {
        &mut []
    };

    match process_command_batch(
        mix_slice,
        buffer_count,
        sample_count,
        cmd_slice,
        states_slice,
        ls_slice,
    ) {
        Ok(count) => count as i32,
        Err(err) => err,
    }
}
