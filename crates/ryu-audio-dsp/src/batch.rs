use crate::dsp;
use crate::filter::{self, BiquadFilterParameter, BiquadFilterState};

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DspOpcode {
    Invalid = 0,
    ClearBuffers = 1,
    ClearBuffer = 2,
    CopyBuffer = 3,
    Mix = 4,
    MixRamp = 5,
    Volume = 6,
    VolumeRamp = 7,
    BiquadFilter = 8,
    BiquadFilterAndMix = 9,
    BiquadFilterAndMixRamp = 10,
}

impl DspOpcode {
    pub fn from_u32(val: u32) -> Self {
        match val {
            1 => DspOpcode::ClearBuffers,
            2 => DspOpcode::ClearBuffer,
            3 => DspOpcode::CopyBuffer,
            4 => DspOpcode::Mix,
            5 => DspOpcode::MixRamp,
            6 => DspOpcode::Volume,
            7 => DspOpcode::VolumeRamp,
            8 => DspOpcode::BiquadFilter,
            9 => DspOpcode::BiquadFilterAndMix,
            10 => DspOpcode::BiquadFilterAndMixRamp,
            _ => DspOpcode::Invalid,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct DspCommandEntry {
    pub opcode: u32,
    pub input_buffer_index: u16,
    pub output_buffer_index: u16,
    pub volume0: f32,
    pub volume1: f32,
    pub filter_param: BiquadFilterParameter,
    pub state_index: u32,       // Index into states array, or u32::MAX
    pub last_sample_index: u32, // Index into last_samples array, or u32::MAX
}

impl Default for DspCommandEntry {
    fn default() -> Self {
        Self {
            opcode: DspOpcode::Invalid as u32,
            input_buffer_index: 0,
            output_buffer_index: 0,
            volume0: 0.0,
            volume1: 0.0,
            filter_param: BiquadFilterParameter::default(),
            state_index: u32::MAX,
            last_sample_index: u32::MAX,
        }
    }
}

/// Executes a batch of DSP commands across the 5ms quantum mix buffers.
/// Returns the number of successfully executed commands, or a negative error code.
pub fn process_command_batch(
    mix_buffers: &mut [f32],
    buffer_count: usize,
    sample_count: usize,
    commands: &[DspCommandEntry],
    states: &mut [BiquadFilterState],
    last_samples: &mut [f32],
) -> Result<usize, i32> {
    let required_floats = buffer_count.saturating_mul(sample_count);
    if mix_buffers.len() < required_floats {
        return Err(-2); // Invalid argument: buffer too small
    }

    let mut executed_count = 0usize;
    let base_ptr = mix_buffers.as_mut_ptr();

    for cmd in commands {
        let opcode = DspOpcode::from_u32(cmd.opcode);
        if opcode == DspOpcode::Invalid {
            continue;
        }

        let in_idx = cmd.input_buffer_index as usize;
        let out_idx = cmd.output_buffer_index as usize;

        match opcode {
            DspOpcode::ClearBuffers => {
                mix_buffers[..required_floats].fill(0.0);
            }
            DspOpcode::ClearBuffer => {
                if out_idx < buffer_count {
                    let offset = out_idx * sample_count;
                    mix_buffers[offset..offset + sample_count].fill(0.0);
                }
            }
            DspOpcode::CopyBuffer => {
                if in_idx < buffer_count && out_idx < buffer_count && in_idx != out_idx {
                    unsafe {
                        let src_ptr = base_ptr.add(in_idx * sample_count);
                        let dst_ptr = base_ptr.add(out_idx * sample_count);
                        std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, sample_count);
                    }
                }
            }
            DspOpcode::Mix => {
                if in_idx < buffer_count && out_idx < buffer_count {
                    unsafe {
                        let src_slice = std::slice::from_raw_parts(
                            base_ptr.add(in_idx * sample_count),
                            sample_count,
                        );
                        let dst_slice = std::slice::from_raw_parts_mut(
                            base_ptr.add(out_idx * sample_count),
                            sample_count,
                        );
                        dsp::mix(dst_slice, src_slice, cmd.volume0);
                    }
                }
            }
            DspOpcode::MixRamp => {
                if in_idx < buffer_count && out_idx < buffer_count {
                    let last_s = unsafe {
                        let src_slice = std::slice::from_raw_parts(
                            base_ptr.add(in_idx * sample_count),
                            sample_count,
                        );
                        let dst_slice = std::slice::from_raw_parts_mut(
                            base_ptr.add(out_idx * sample_count),
                            sample_count,
                        );
                        dsp::mix_ramp(dst_slice, src_slice, cmd.volume0, cmd.volume1)
                    };
                    let ls_idx = cmd.last_sample_index as usize;
                    if ls_idx < last_samples.len() {
                        last_samples[ls_idx] = last_s;
                    }
                }
            }
            DspOpcode::Volume => {
                if in_idx < buffer_count && out_idx < buffer_count {
                    unsafe {
                        let src_slice = std::slice::from_raw_parts(
                            base_ptr.add(in_idx * sample_count),
                            sample_count,
                        );
                        let dst_slice = std::slice::from_raw_parts_mut(
                            base_ptr.add(out_idx * sample_count),
                            sample_count,
                        );
                        dsp::volume(dst_slice, src_slice, cmd.volume0);
                    }
                }
            }
            DspOpcode::VolumeRamp => {
                if in_idx < buffer_count && out_idx < buffer_count {
                    unsafe {
                        let src_slice = std::slice::from_raw_parts(
                            base_ptr.add(in_idx * sample_count),
                            sample_count,
                        );
                        let dst_slice = std::slice::from_raw_parts_mut(
                            base_ptr.add(out_idx * sample_count),
                            sample_count,
                        );
                        dsp::volume_ramp(dst_slice, src_slice, cmd.volume0, cmd.volume1);
                    }
                }
            }
            DspOpcode::BiquadFilter => {
                if in_idx < buffer_count && out_idx < buffer_count {
                    let st_idx = cmd.state_index as usize;
                    if st_idx < states.len() {
                        unsafe {
                            let src_slice = std::slice::from_raw_parts(
                                base_ptr.add(in_idx * sample_count),
                                sample_count,
                            );
                            let dst_slice = std::slice::from_raw_parts_mut(
                                base_ptr.add(out_idx * sample_count),
                                sample_count,
                            );
                            filter::process_biquad_filter_df2(
                                &cmd.filter_param,
                                &mut states[st_idx],
                                dst_slice,
                                src_slice,
                            );
                        }
                    }
                }
            }
            DspOpcode::BiquadFilterAndMix => {
                if in_idx < buffer_count && out_idx < buffer_count {
                    let st_idx = cmd.state_index as usize;
                    if st_idx < states.len() {
                        unsafe {
                            let src_slice = std::slice::from_raw_parts(
                                base_ptr.add(in_idx * sample_count),
                                sample_count,
                            );
                            let dst_slice = std::slice::from_raw_parts_mut(
                                base_ptr.add(out_idx * sample_count),
                                sample_count,
                            );
                            filter::process_biquad_filter_and_mix_df1(
                                &cmd.filter_param,
                                &mut states[st_idx],
                                dst_slice,
                                src_slice,
                                cmd.volume0,
                            );
                        }
                    }
                }
            }
            DspOpcode::BiquadFilterAndMixRamp => {
                if in_idx < buffer_count && out_idx < buffer_count {
                    let st_idx = cmd.state_index as usize;
                    if st_idx < states.len() {
                        let last_s = unsafe {
                            let src_slice = std::slice::from_raw_parts(
                                base_ptr.add(in_idx * sample_count),
                                sample_count,
                            );
                            let dst_slice = std::slice::from_raw_parts_mut(
                                base_ptr.add(out_idx * sample_count),
                                sample_count,
                            );
                            filter::process_biquad_filter_and_mix_ramp_df1(
                                &cmd.filter_param,
                                &mut states[st_idx],
                                dst_slice,
                                src_slice,
                                cmd.volume0,
                                cmd.volume1,
                            )
                        };
                        let ls_idx = cmd.last_sample_index as usize;
                        if ls_idx < last_samples.len() {
                            last_samples[ls_idx] = last_s;
                        }
                    }
                }
            }
            DspOpcode::Invalid => {}
        }

        executed_count += 1;
    }

    Ok(executed_count)
}
