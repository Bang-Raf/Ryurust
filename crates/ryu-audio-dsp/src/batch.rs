use crate::dsp::{self, InPlace, Split};
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

/// The input/output view of the mix buffers for one command.
enum CommandBuffers<'a> {
    /// `input_buffer_index != output_buffer_index`: two disjoint slices.
    Split(Split<'a>),
    /// `input_buffer_index == output_buffer_index`: one slice that is read and written.
    InPlace(InPlace<'a>),
}

/// Borrows buffer `in_idx` (read) and `out_idx` (written) out of `mix_buffers` without
/// aliasing a `&[f32]` with a `&mut [f32]`. Both indices must be `< buffer_count`, with
/// `mix_buffers.len() >= buffer_count * sample_count`.
fn command_buffers(
    mix_buffers: &mut [f32],
    sample_count: usize,
    in_idx: usize,
    out_idx: usize,
) -> CommandBuffers<'_> {
    let in_off = in_idx * sample_count;
    let out_off = out_idx * sample_count;
    if in_idx == out_idx {
        return CommandBuffers::InPlace(InPlace(&mut mix_buffers[out_off..out_off + sample_count]));
    }
    if in_idx < out_idx {
        let (lo, hi) = mix_buffers.split_at_mut(out_off);
        CommandBuffers::Split(Split::new(
            &mut hi[..sample_count],
            &lo[in_off..in_off + sample_count],
        ))
    } else {
        let (lo, hi) = mix_buffers.split_at_mut(in_off);
        CommandBuffers::Split(Split::new(
            &mut lo[out_off..out_off + sample_count],
            &hi[..sample_count],
        ))
    }
}

/// Runs `$body` with `$io` bound to the concrete `SampleIo` of `$bufs`, so each kernel is
/// monomorphised for the split and the in-place case (no per-sample branching).
macro_rules! with_io {
    ($bufs:expr, |$io:ident| $body:expr) => {
        match $bufs {
            CommandBuffers::Split(mut split) => {
                let $io = &mut split;
                $body
            }
            CommandBuffers::InPlace(mut in_place) => {
                let $io = &mut in_place;
                $body
            }
        }
    };
}

/// Executes a batch of DSP commands across the 5ms quantum mix buffers.
/// Returns the number of successfully executed commands, or a negative error code.
///
/// Commands whose input and output buffer index are equal are processed in place, with the
/// same per-sample semantics as Ryujinx's C# commands running on aliased spans.
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

    for cmd in commands {
        let opcode = DspOpcode::from_u32(cmd.opcode);
        if opcode == DspOpcode::Invalid {
            continue;
        }

        let in_idx = cmd.input_buffer_index as usize;
        let out_idx = cmd.output_buffer_index as usize;
        let indices_valid = in_idx < buffer_count && out_idx < buffer_count;

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
                if indices_valid && in_idx != out_idx {
                    let src = in_idx * sample_count;
                    let dst = out_idx * sample_count;
                    mix_buffers.copy_within(src..src + sample_count, dst);
                }
            }
            DspOpcode::Mix => {
                if indices_valid {
                    let bufs = command_buffers(mix_buffers, sample_count, in_idx, out_idx);
                    with_io!(bufs, |io| dsp::mix_io(io, cmd.volume0));
                }
            }
            DspOpcode::MixRamp => {
                if indices_valid {
                    let bufs = command_buffers(mix_buffers, sample_count, in_idx, out_idx);
                    let last_s =
                        with_io!(bufs, |io| dsp::mix_ramp_io(io, cmd.volume0, cmd.volume1));
                    let ls_idx = cmd.last_sample_index as usize;
                    if ls_idx < last_samples.len() {
                        last_samples[ls_idx] = last_s;
                    }
                }
            }
            DspOpcode::Volume => {
                if indices_valid {
                    let bufs = command_buffers(mix_buffers, sample_count, in_idx, out_idx);
                    with_io!(bufs, |io| dsp::volume_io(io, cmd.volume0));
                }
            }
            DspOpcode::VolumeRamp => {
                if indices_valid {
                    let bufs = command_buffers(mix_buffers, sample_count, in_idx, out_idx);
                    with_io!(bufs, |io| dsp::volume_ramp_io(io, cmd.volume0, cmd.volume1));
                }
            }
            DspOpcode::BiquadFilter => {
                if indices_valid {
                    if let Some(state) = states.get_mut(cmd.state_index as usize) {
                        let bufs = command_buffers(mix_buffers, sample_count, in_idx, out_idx);
                        with_io!(bufs, |io| filter::biquad_df2_io(
                            &cmd.filter_param,
                            state,
                            io
                        ));
                    }
                }
            }
            DspOpcode::BiquadFilterAndMix => {
                if indices_valid {
                    if let Some(state) = states.get_mut(cmd.state_index as usize) {
                        let bufs = command_buffers(mix_buffers, sample_count, in_idx, out_idx);
                        with_io!(bufs, |io| filter::biquad_and_mix_df1_io(
                            &cmd.filter_param,
                            state,
                            io,
                            cmd.volume0
                        ));
                    }
                }
            }
            DspOpcode::BiquadFilterAndMixRamp => {
                if indices_valid {
                    if let Some(state) = states.get_mut(cmd.state_index as usize) {
                        let bufs = command_buffers(mix_buffers, sample_count, in_idx, out_idx);
                        let last_s = with_io!(bufs, |io| filter::biquad_and_mix_ramp_df1_io(
                            &cmd.filter_param,
                            state,
                            io,
                            cmd.volume0,
                            cmd.volume1
                        ));
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
