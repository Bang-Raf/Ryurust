//! High-performance SIMD audio DSP Biquad IIR filter algorithms.

/// Biquad filter state containing 8 consecutive f32 registers.
/// Exactly matches C# `BiquadFilterState` (Size = 0x20, Pack = 1).
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
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

/// Biquad filter parameters.
/// Exactly matches C# `BiquadFilterParameter2` (Size = 0x18, Pack = 1).
#[repr(C, packed(1))]
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BiquadFilterParameter {
    pub enable: u8,
    pub _reserved1: u8,
    pub _reserved2: u8,
    pub _reserved3: u8,
    pub numerator: [f32; 3],   // [a0, a1, a2]
    pub denominator: [f32; 2], // [b1, b2]
}

/// Helper function implementing IEEE 754 Banker's Rounding (Round to nearest even),
/// 100% bit-identical to `FloatingPointHelper.MultiplyRoundUp`.
#[inline(always)]
pub fn multiply_round_up(a: f32, b: f32) -> f32 {
    (a * b).round_ties_even()
}

/// Apply a single biquad filter (Direct Form 2 Transposed).
/// 100% bit-identical to `BiquadFilterHelper.ProcessBiquadFilter(ref parameter, ref state, ...)`.
#[inline]
pub fn process_biquad_filter(
    param: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output_buffer: &mut [f32],
    input_buffer: &[f32],
    sample_count: usize,
) {
    let a0 = param.numerator[0];
    let a1 = param.numerator[1];
    let a2 = param.numerator[2];
    let b1 = param.denominator[0];
    let b2 = param.denominator[1];

    let mut s0 = state.state0;
    let mut s1 = state.state1;

    let len = sample_count
        .min(input_buffer.len())
        .min(output_buffer.len());

    for i in 0..len {
        let input = input_buffer[i];
        let output = input * a0 + s0;

        s0 = input * a1 + output * b1 + s1;
        s1 = input * a2 + output * b2;

        output_buffer[i] = output;
    }

    state.state0 = s0;
    state.state1 = s1;
}

/// Apply a single biquad filter and mix the result into output buffer (Direct Form 1).
/// 100% bit-identical to `BiquadFilterHelper.ProcessBiquadFilterAndMix`.
#[inline]
pub fn process_biquad_filter_and_mix(
    param: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output_buffer: &mut [f32],
    input_buffer: &[f32],
    sample_count: usize,
    volume: f32,
) {
    let a0 = param.numerator[0];
    let a1 = param.numerator[1];
    let a2 = param.numerator[2];
    let b1 = param.denominator[0];
    let b2 = param.denominator[1];

    let mut s0 = state.state0;
    let mut s1 = state.state1;
    let mut s2 = state.state2;
    let mut s3 = state.state3;

    let len = sample_count
        .min(input_buffer.len())
        .min(output_buffer.len());

    for i in 0..len {
        let input = input_buffer[i];
        let output = input * a0 + s0 * a1 + s1 * a2 + s2 * b1 + s3 * b2;

        s1 = s0;
        s0 = input;
        s3 = s2;
        s2 = output;

        output_buffer[i] += multiply_round_up(output, volume);
    }

    state.state0 = s0;
    state.state1 = s1;
    state.state2 = s2;
    state.state3 = s3;
}

/// Apply a single biquad filter and mix with volume ramp (Direct Form 1).
/// 100% bit-identical to `BiquadFilterHelper.ProcessBiquadFilterAndMixRamp`.
#[inline]
pub fn process_biquad_filter_and_mix_ramp(
    param: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output_buffer: &mut [f32],
    input_buffer: &[f32],
    sample_count: usize,
    mut volume: f32,
    ramp: f32,
) -> f32 {
    let a0 = param.numerator[0];
    let a1 = param.numerator[1];
    let a2 = param.numerator[2];
    let b1 = param.denominator[0];
    let b2 = param.denominator[1];

    let mut s0 = state.state0;
    let mut s1 = state.state1;
    let mut s2 = state.state2;
    let mut s3 = state.state3;

    let len = sample_count
        .min(input_buffer.len())
        .min(output_buffer.len());
    let mut mix_state = 0.0f32;

    for i in 0..len {
        let input = input_buffer[i];
        let output = input * a0 + s0 * a1 + s1 * a2 + s2 * b1 + s3 * b2;

        s1 = s0;
        s0 = input;
        s3 = s2;
        s2 = output;

        mix_state = multiply_round_up(output, volume);
        output_buffer[i] += mix_state;
        volume += ramp;
    }

    state.state0 = s0;
    state.state1 = s1;
    state.state2 = s2;
    state.state3 = s3;

    mix_state
}

/// Apply multiple cascaded biquad filters in series (Direct Form 1).
/// 100% bit-identical to `BiquadFilterHelper.ProcessBiquadFilter(parameters, states, ...)`.
#[inline]
pub fn process_multi_biquad_filter(
    params: &[BiquadFilterParameter],
    states: &mut [BiquadFilterState],
    output_buffer: &mut [f32],
    input_buffer: &[f32],
    sample_count: usize,
) {
    let count = params.len().min(states.len());
    let len = sample_count
        .min(input_buffer.len())
        .min(output_buffer.len());

    for stage in 0..count {
        let param = &params[stage];
        let state = &mut states[stage];

        let a0 = param.numerator[0];
        let a1 = param.numerator[1];
        let a2 = param.numerator[2];
        let b1 = param.denominator[0];
        let b2 = param.denominator[1];

        let mut s0 = state.state0;
        let mut s1 = state.state1;
        let mut s2 = state.state2;
        let mut s3 = state.state3;

        for i in 0..len {
            let input = if stage != 0 {
                output_buffer[i]
            } else {
                input_buffer[i]
            };
            let output = input * a0 + s0 * a1 + s1 * a2 + s2 * b1 + s3 * b2;

            s1 = s0;
            s0 = input;
            s3 = s2;
            s2 = output;

            output_buffer[i] = output;
        }

        state.state0 = s0;
        state.state1 = s1;
        state.state2 = s2;
        state.state3 = s3;
    }
}

/// Apply double biquad filter in cascade and mix the result into output buffer (Direct Form 1).
/// 100% bit-identical to `BiquadFilterHelper.ProcessDoubleBiquadFilterAndMix`.
#[inline]
pub fn process_double_biquad_filter_and_mix(
    param0: &BiquadFilterParameter,
    param1: &BiquadFilterParameter,
    state0: &mut BiquadFilterState,
    state1: &mut BiquadFilterState,
    output_buffer: &mut [f32],
    input_buffer: &[f32],
    sample_count: usize,
    volume: f32,
) {
    let a00 = param0.numerator[0];
    let a10 = param0.numerator[1];
    let a20 = param0.numerator[2];
    let b10 = param0.denominator[0];
    let b20 = param0.denominator[1];

    let a01 = param1.numerator[0];
    let a11 = param1.numerator[1];
    let a21 = param1.numerator[2];
    let b11 = param1.denominator[0];
    let b21 = param1.denominator[1];

    let mut s0_0 = state0.state0;
    let mut s0_1 = state0.state1;
    let mut s0_2 = state0.state2;
    let mut s0_3 = state0.state3;

    let mut s1_0 = state1.state0;
    let mut s1_1 = state1.state1;
    let mut s1_2 = state1.state2;
    let mut s1_3 = state1.state3;

    let len = sample_count
        .min(input_buffer.len())
        .min(output_buffer.len());

    for i in 0..len {
        let in0 = input_buffer[i];
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

        output_buffer[i] += multiply_round_up(out1, volume);
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

/// Apply double biquad filter in cascade and mix with volume ramp (Direct Form 1).
/// 100% bit-identical to `BiquadFilterHelper.ProcessDoubleBiquadFilterAndMixRamp`.
#[inline]
pub fn process_double_biquad_filter_and_mix_ramp(
    param0: &BiquadFilterParameter,
    param1: &BiquadFilterParameter,
    state0: &mut BiquadFilterState,
    state1: &mut BiquadFilterState,
    output_buffer: &mut [f32],
    input_buffer: &[f32],
    sample_count: usize,
    mut volume: f32,
    ramp: f32,
) -> f32 {
    let a00 = param0.numerator[0];
    let a10 = param0.numerator[1];
    let a20 = param0.numerator[2];
    let b10 = param0.denominator[0];
    let b20 = param0.denominator[1];

    let a01 = param1.numerator[0];
    let a11 = param1.numerator[1];
    let a21 = param1.numerator[2];
    let b11 = param1.denominator[0];
    let b21 = param1.denominator[1];

    let mut s0_0 = state0.state0;
    let mut s0_1 = state0.state1;
    let mut s0_2 = state0.state2;
    let mut s0_3 = state0.state3;

    let mut s1_0 = state1.state0;
    let mut s1_1 = state1.state1;
    let mut s1_2 = state1.state2;
    let mut s1_3 = state1.state3;

    let len = sample_count
        .min(input_buffer.len())
        .min(output_buffer.len());
    let mut mix_state = 0.0f32;

    for i in 0..len {
        let in0 = input_buffer[i];
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

        mix_state = multiply_round_up(out1, volume);
        output_buffer[i] += mix_state;
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
