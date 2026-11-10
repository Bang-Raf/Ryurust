use crate::mixer::*;
use std::slice;

pub const RYU_AUDIO_MIXER_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_audio_mixer_version() -> u32 {
    RYU_AUDIO_MIXER_VERSION
}

#[no_mangle]
pub unsafe extern "C" fn ryu_audio_mix(
    output: *mut f32,
    input: *const f32,
    count: usize,
    volume: f32,
) -> i32 {
    if output.is_null() || input.is_null() || count == 0 {
        return -1;
    }

    let out_slice = slice::from_raw_parts_mut(output, count);
    let in_slice = slice::from_raw_parts(input, count);

    mix_buffer(out_slice, in_slice, volume);
    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_audio_mix_ramp(
    output: *mut f32,
    input: *const f32,
    count: usize,
    volume0: f32,
    volume1: f32,
    out_last_sample: *mut f32,
) -> i32 {
    if output.is_null() || input.is_null() || count == 0 {
        return -1;
    }

    let out_slice = slice::from_raw_parts_mut(output, count);
    let in_slice = slice::from_raw_parts(input, count);

    let last = mix_ramp(out_slice, in_slice, volume0, volume1);

    if !out_last_sample.is_null() {
        *out_last_sample = last;
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_audio_mix_batch(
    output: *mut f32,
    inputs: *const *const f32,
    volumes: *const f32,
    voice_count: usize,
    sample_count: usize,
) -> i32 {
    if output.is_null() || inputs.is_null() || volumes.is_null() || sample_count == 0 {
        return -1;
    }

    let out_slice = slice::from_raw_parts_mut(output, sample_count);
    let in_ptrs = slice::from_raw_parts(inputs, voice_count);
    let vol_slice = slice::from_raw_parts(volumes, voice_count);

    for v in 0..voice_count {
        if !in_ptrs[v].is_null() {
            let in_slice = slice::from_raw_parts(in_ptrs[v], sample_count);
            mix_buffer(out_slice, in_slice, vol_slice[v]);
        }
    }

    0
}

#[no_mangle]
pub unsafe extern "C" fn ryu_audio_mix_matrix(
    outputs: *const *mut f32,
    output_channels: usize,
    input: *const f32,
    gains: *const f32,
    sample_count: usize,
) -> i32 {
    if outputs.is_null() || input.is_null() || gains.is_null() || sample_count == 0 {
        return -1;
    }

    let out_ptrs = slice::from_raw_parts(outputs, output_channels);
    let in_slice = slice::from_raw_parts(input, sample_count);
    let gain_slice = slice::from_raw_parts(gains, output_channels);

    for c in 0..output_channels {
        if !out_ptrs[c].is_null() {
            let out_slice = slice::from_raw_parts_mut(out_ptrs[c], sample_count);
            mix_buffer(out_slice, in_slice, gain_slice[c]);
        }
    }

    0
}
