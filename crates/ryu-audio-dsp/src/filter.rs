use crate::dsp::{InPlace, SampleIo, Split};

/// Biquad coefficients, layout-compatible with Ryujinx's `BiquadFilterParameter2`.
///
/// The filter computes
/// `y[n] = b0*x[n] + b1*x[n-1] + b2*x[n-2] + denominator[0]*y[n-1] + denominator[1]*y[n-2]`.
///
/// **The feedback coefficients are added, not subtracted**: for the textbook transfer function
/// `H(z) = (b0 + b1 z^-1 + b2 z^-2) / (1 + a1 z^-1 + a2 z^-2)` you must store
/// `denominator = [-a1, -a2]`. This matches Ryujinx, which passes the guest's `Denominator`
/// array through unchanged and adds it in `BiquadFilterHelper` (`output * b1` / `State2 * b1`
/// there; note the C# names the numerator `a*` and the denominator `b*`, the opposite of the
/// textbook convention used here). Storing un-negated `a1`/`a2` gives a different, usually
/// unstable, filter.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiquadFilterParameter {
    pub enable: u8,
    pub _reserved: [u8; 3],
    /// Feed-forward coefficients `b0, b1, b2`.
    pub numerator: [f32; 3],
    /// Feedback coefficients **negated**: `[-a1, -a2]`.
    pub denominator: [f32; 2],
}

impl Default for BiquadFilterParameter {
    fn default() -> Self {
        Self {
            enable: 1,
            _reserved: [0; 3],
            numerator: [1.0, 0.0, 0.0],
            denominator: [0.0, 0.0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BiquadFilterState {
    pub state0: f32,
    pub state1: f32,
    pub state2: f32,
    pub state3: f32,
    pub state4: f32,
    pub state5: f32,
    pub state6: f32,
    pub state7: f32,
}

impl BiquadFilterState {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[inline(always)]
fn round_half_to_even(val: f32) -> f32 {
    val.round_ties_even()
}

/// Apply a single biquad filter in Direct Form 2.
/// Matches `BiquadFilterHelper.ProcessBiquadFilter(ref parameter, ref state, outputBuffer, inputBuffer, sampleCount)`.
#[inline]
pub fn process_biquad_filter_df2(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
) {
    biquad_df2_io(parameter, state, &mut Split::new(output, input));
}

/// [`process_biquad_filter_df2`] with `buffer` as both input and output.
#[inline]
pub fn process_biquad_filter_df2_in_place(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    buffer: &mut [f32],
) {
    biquad_df2_io(parameter, state, &mut InPlace(buffer));
}

#[inline(always)]
pub(crate) fn biquad_df2_io<I: SampleIo>(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    io: &mut I,
) {
    let count = io.len();
    let a0 = parameter.numerator[0];
    let a1 = parameter.numerator[1];
    let a2 = parameter.numerator[2];

    let b1 = parameter.denominator[0];
    let b2 = parameter.denominator[1];

    let mut s0 = state.state0;
    let mut s1 = state.state1;

    for i in 0..count {
        let in_samp = io.input(i);
        let out_samp = in_samp * a0 + s0;

        s0 = in_samp * a1 + out_samp * b1 + s1;
        s1 = in_samp * a2 + out_samp * b2;

        *io.output(i) = out_samp;
    }

    state.state0 = s0;
    state.state1 = s1;
}

/// Apply a single biquad filter in Direct Form 1 and mix into output buffer.
/// Matches `BiquadFilterHelper.ProcessBiquadFilterAndMix(ref parameter, ref state, outputBuffer, inputBuffer, sampleCount, volume)`.
#[inline]
pub fn process_biquad_filter_and_mix_df1(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
    volume: f32,
) {
    biquad_and_mix_df1_io(parameter, state, &mut Split::new(output, input), volume);
}

/// [`process_biquad_filter_and_mix_df1`] with `buffer` as both input and output.
#[inline]
pub fn process_biquad_filter_and_mix_df1_in_place(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    buffer: &mut [f32],
    volume: f32,
) {
    biquad_and_mix_df1_io(parameter, state, &mut InPlace(buffer), volume);
}

#[inline(always)]
pub(crate) fn biquad_and_mix_df1_io<I: SampleIo>(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    io: &mut I,
    volume: f32,
) {
    let count = io.len();
    let a0 = parameter.numerator[0];
    let a1 = parameter.numerator[1];
    let a2 = parameter.numerator[2];

    let b1 = parameter.denominator[0];
    let b2 = parameter.denominator[1];

    let mut s0 = state.state0;
    let mut s1 = state.state1;
    let mut s2 = state.state2;
    let mut s3 = state.state3;

    for i in 0..count {
        let in_samp = io.input(i);
        let out_samp = in_samp * a0 + s0 * a1 + s1 * a2 + s2 * b1 + s3 * b2;

        s1 = s0;
        s0 = in_samp;
        s3 = s2;
        s2 = out_samp;

        *io.output(i) += round_half_to_even(out_samp * volume);
    }

    state.state0 = s0;
    state.state1 = s1;
    state.state2 = s2;
    state.state3 = s3;
}

/// Apply a single biquad filter in Direct Form 1 and mix into output buffer with volume ramp.
/// Matches `BiquadFilterHelper.ProcessBiquadFilterAndMixRamp(...)`.
#[inline]
pub fn process_biquad_filter_and_mix_ramp_df1(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
    volume0: f32,
    volume1: f32,
) -> f32 {
    biquad_and_mix_ramp_df1_io(
        parameter,
        state,
        &mut Split::new(output, input),
        volume0,
        volume1,
    )
}

/// [`process_biquad_filter_and_mix_ramp_df1`] with `buffer` as both input and output.
#[inline]
pub fn process_biquad_filter_and_mix_ramp_df1_in_place(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    buffer: &mut [f32],
    volume0: f32,
    volume1: f32,
) -> f32 {
    biquad_and_mix_ramp_df1_io(parameter, state, &mut InPlace(buffer), volume0, volume1)
}

#[inline(always)]
pub(crate) fn biquad_and_mix_ramp_df1_io<I: SampleIo>(
    parameter: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    io: &mut I,
    volume0: f32,
    volume1: f32,
) -> f32 {
    let count = io.len();
    if count == 0 {
        return 0.0;
    }

    let a0 = parameter.numerator[0];
    let a1 = parameter.numerator[1];
    let a2 = parameter.numerator[2];

    let b1 = parameter.denominator[0];
    let b2 = parameter.denominator[1];

    let mut s0 = state.state0;
    let mut s1 = state.state1;
    let mut s2 = state.state2;
    let mut s3 = state.state3;

    let ramp = (volume1 - volume0) / count as f32;
    let mut volume = volume0;
    let mut mix_state = 0.0f32;

    for i in 0..count {
        let in_samp = io.input(i);
        let out_samp = in_samp * a0 + s0 * a1 + s1 * a2 + s2 * b1 + s3 * b2;

        s1 = s0;
        s0 = in_samp;
        s3 = s2;
        s2 = out_samp;

        mix_state = round_half_to_even(out_samp * volume);
        *io.output(i) += mix_state;
        volume += ramp;
    }

    state.state0 = s0;
    state.state1 = s1;
    state.state2 = s2;
    state.state3 = s3;

    mix_state
}

/// Apply multiple biquad filter stages in Direct Form 1 cascaded.
/// Matches `BiquadFilterHelper.ProcessBiquadFilter(parameters, states, outputBuffer, inputBuffer, sampleCount)`.
#[inline]
pub fn process_multi_tap_biquad_filter(
    parameters: &[BiquadFilterParameter],
    states: &mut [BiquadFilterState],
    output: &mut [f32],
    input: &[f32],
) {
    let stage_count = parameters.len().min(states.len());
    let count = output.len().min(input.len());

    for stage_idx in 0..stage_count {
        let param = &parameters[stage_idx];
        let state = &mut states[stage_idx];

        let a0 = param.numerator[0];
        let a1 = param.numerator[1];
        let a2 = param.numerator[2];

        let b1 = param.denominator[0];
        let b2 = param.denominator[1];

        let mut s0 = state.state0;
        let mut s1 = state.state1;
        let mut s2 = state.state2;
        let mut s3 = state.state3;

        for i in 0..count {
            let in_samp = if stage_idx != 0 { output[i] } else { input[i] };
            let out_samp = in_samp * a0 + s0 * a1 + s1 * a2 + s2 * b1 + s3 * b2;

            s1 = s0;
            s0 = in_samp;
            s3 = s2;
            s2 = out_samp;

            output[i] = out_samp;
        }

        state.state0 = s0;
        state.state1 = s1;
        state.state2 = s2;
        state.state3 = s3;
    }
}

/// Apply double biquad filter in Direct Form 1 and mix into output buffer.
/// Matches `BiquadFilterHelper.ProcessDoubleBiquadFilterAndMix(...)`.
#[inline]
pub fn process_double_biquad_filter_and_mix(
    parameter0: &BiquadFilterParameter,
    parameter1: &BiquadFilterParameter,
    state0: &mut BiquadFilterState,
    state1: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
    volume: f32,
) {
    let count = output.len().min(input.len());

    let a00 = parameter0.numerator[0];
    let a10 = parameter0.numerator[1];
    let a20 = parameter0.numerator[2];
    let b10 = parameter0.denominator[0];
    let b20 = parameter0.denominator[1];

    let a01 = parameter1.numerator[0];
    let a11 = parameter1.numerator[1];
    let a21 = parameter1.numerator[2];
    let b11 = parameter1.denominator[0];
    let b21 = parameter1.denominator[1];

    let mut s0_0 = state0.state0;
    let mut s0_1 = state0.state1;
    let mut s0_2 = state0.state2;
    let mut s0_3 = state0.state3;

    let mut s1_0 = state1.state0;
    let mut s1_1 = state1.state1;
    let mut s1_2 = state1.state2;
    let mut s1_3 = state1.state3;

    for i in 0..count {
        let in0 = input[i];
        let out0 = in0 * a00 + s0_0 * a10 + s0_1 * a20 + s0_2 * b10 + s0_3 * b20;

        s0_1 = s0_0;
        s0_0 = in0;
        s0_3 = s0_2;
        s0_2 = out0;

        let in1 = out0;
        let out1 = in1 * a01 + s1_0 * a11 + s1_1 * a21 + s1_2 * b11 + s1_3 * b21;

        s1_1 = s1_0;
        s1_0 = in1;
        s1_3 = s1_2;
        s1_2 = out1;

        output[i] += round_half_to_even(out1 * volume);
    }

    state0.state0 = s0_0;
    state0.state1 = s0_1;
    state0.state2 = s0_2;
    state0.state3 = s0_3;

    state1.state0 = s1_0;
    state1.state1 = s1_1;
    state1.state2 = s1_2;
    state1.state3 = s1_3;
}

/// Apply double biquad filter in Direct Form 1 and mix into output buffer with volume ramp.
/// Matches `BiquadFilterHelper.ProcessDoubleBiquadFilterAndMixRamp(...)`.
#[allow(clippy::too_many_arguments)] // mirrors C# `BiquadFilterHelper` parameter list and backs the C ABI export
#[inline]
pub fn process_double_biquad_filter_and_mix_ramp(
    parameter0: &BiquadFilterParameter,
    parameter1: &BiquadFilterParameter,
    state0: &mut BiquadFilterState,
    state1: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
    volume0: f32,
    volume1: f32,
) -> f32 {
    let count = output.len().min(input.len());
    if count == 0 {
        return 0.0;
    }

    let a00 = parameter0.numerator[0];
    let a10 = parameter0.numerator[1];
    let a20 = parameter0.numerator[2];
    let b10 = parameter0.denominator[0];
    let b20 = parameter0.denominator[1];

    let a01 = parameter1.numerator[0];
    let a11 = parameter1.numerator[1];
    let a21 = parameter1.numerator[2];
    let b11 = parameter1.denominator[0];
    let b21 = parameter1.denominator[1];

    let mut s0_0 = state0.state0;
    let mut s0_1 = state0.state1;
    let mut s0_2 = state0.state2;
    let mut s0_3 = state0.state3;

    let mut s1_0 = state1.state0;
    let mut s1_1 = state1.state1;
    let mut s1_2 = state1.state2;
    let mut s1_3 = state1.state3;

    let ramp = (volume1 - volume0) / count as f32;
    let mut volume = volume0;
    let mut mix_state = 0.0f32;

    for i in 0..count {
        let in0 = input[i];
        let out0 = in0 * a00 + s0_0 * a10 + s0_1 * a20 + s0_2 * b10 + s0_3 * b20;

        s0_1 = s0_0;
        s0_0 = in0;
        s0_3 = s0_2;
        s0_2 = out0;

        let in1 = out0;
        let out1 = in1 * a01 + s1_0 * a11 + s1_1 * a21 + s1_2 * b11 + s1_3 * b21;

        s1_1 = s1_0;
        s1_0 = in1;
        s1_3 = s1_2;
        s1_2 = out1;

        mix_state = round_half_to_even(out1 * volume);
        output[i] += mix_state;
        volume += ramp;
    }

    state0.state0 = s0_0;
    state0.state1 = s0_1;
    state0.state2 = s0_2;
    state0.state3 = s0_3;

    state1.state0 = s1_0;
    state1.state1 = s1_1;
    state1.state2 = s1_2;
    state1.state3 = s1_3;

    mix_state
}
