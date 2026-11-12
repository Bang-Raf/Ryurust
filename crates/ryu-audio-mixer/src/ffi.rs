use crate::mixer::*;
use std::slice;

pub const RYU_AUDIO_MIXER_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_audio_mixer_version() -> u32 {
    RYU_AUDIO_MIXER_VERSION
}

/// # Safety
/// `output` must be valid for reads and writes and `input` for reads of `count` floats, and
/// the two ranges must not overlap.
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

/// # Safety
/// `output` must be valid for reads and writes and `input` for reads of `count` floats, and
/// the two ranges must not overlap. `out_last_sample` must be null or valid for a write of
/// one float.
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

/// # Safety
/// `output` must be valid for reads and writes of `sample_count` floats. `inputs` and
/// `volumes` must be valid for reads of `voice_count` elements; each entry of `inputs` must be
/// null (voice skipped) or valid for reads of `sample_count` floats not overlapping `output`.
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

/// # Safety
/// `input` must be valid for reads of `sample_count` floats. `outputs` and `gains` must be
/// valid for reads of `output_channels` elements; each entry of `outputs` must be null
/// (channel skipped) or valid for reads and writes of `sample_count` floats, overlapping
/// neither `input` nor another output channel.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr::{null, null_mut};

    #[test]
    fn version_export_returns_constant() {
        assert_eq!(ryu_audio_mixer_version(), RYU_AUDIO_MIXER_VERSION);
        assert_eq!(RYU_AUDIO_MIXER_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        // SAFETY: every call passes null for the required pointers, which each function
        // rejects before touching memory.
        unsafe {
            assert_eq!(ryu_audio_mix(null_mut(), null(), 4, 1.0), -1);
            assert_eq!(
                ryu_audio_mix_ramp(null_mut(), null(), 4, 0.0, 1.0, null_mut()),
                -1
            );
            assert_eq!(ryu_audio_mix_batch(null_mut(), null(), null(), 2, 4), -1);
            assert_eq!(ryu_audio_mix_matrix(null(), 2, null(), null(), 4), -1);
        }
    }

    #[test]
    fn null_voice_and_channel_entries_are_skipped() {
        let input = [1.0f32; 4];
        let mut output = [0.0f32; 4];
        let inputs: [*const f32; 2] = [null(), input.as_ptr()];
        let outputs: [*mut f32; 2] = [null_mut(), output.as_mut_ptr()];
        let gains = [1.0f32; 2];
        // SAFETY: the pointer tables and `gains` hold 2 entries, and every non-null entry
        // points at 4 floats that do not overlap the other side of the mix.
        unsafe {
            assert_eq!(
                ryu_audio_mix_batch(output.as_mut_ptr(), inputs.as_ptr(), gains.as_ptr(), 2, 4),
                0
            );
            assert_eq!(output, [1.0; 4]);
            assert_eq!(
                ryu_audio_mix_matrix(outputs.as_ptr(), 2, input.as_ptr(), gains.as_ptr(), 4),
                0
            );
            assert_eq!(output, [2.0; 4]);
        }
    }
}
