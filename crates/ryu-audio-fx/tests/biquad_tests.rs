use ryu_audio_fx::ffi::*;
use ryu_audio_fx::*;
use std::mem::size_of;

// ----------------------------------------------------------------------------
// Reference C# Implementations (Exact bit-for-bit ported from BiquadFilterHelper.cs)
// ----------------------------------------------------------------------------

#[inline(always)]
fn ref_round_up(a: f32) -> f32 {
    a.round_ties_even()
}

#[inline(always)]
fn ref_multiply_round_up(a: f32, b: f32) -> f32 {
    ref_round_up(a * b)
}

fn ref_process_biquad_filter(
    param: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
    sample_count: usize,
) {
    let a0 = param.numerator[0];
    let a1 = param.numerator[1];
    let a2 = param.numerator[2];
    let b1 = param.denominator[0];
    let b2 = param.denominator[1];

    for i in 0..sample_count {
        let in_val = input[i];
        let out_val = in_val * a0 + state.state0;

        state.state0 = in_val * a1 + out_val * b1 + state.state1;
        state.state1 = in_val * a2 + out_val * b2;

        output[i] = out_val;
    }
}

fn ref_process_biquad_filter_and_mix(
    param: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
    sample_count: usize,
    volume: f32,
) {
    let a0 = param.numerator[0];
    let a1 = param.numerator[1];
    let a2 = param.numerator[2];
    let b1 = param.denominator[0];
    let b2 = param.denominator[1];

    for i in 0..sample_count {
        let in_val = input[i];
        let out_val = in_val * a0
            + state.state0 * a1
            + state.state1 * a2
            + state.state2 * b1
            + state.state3 * b2;

        state.state1 = state.state0;
        state.state0 = in_val;
        state.state3 = state.state2;
        state.state2 = out_val;

        output[i] += ref_multiply_round_up(out_val, volume);
    }
}

fn ref_process_biquad_filter_and_mix_ramp(
    param: &BiquadFilterParameter,
    state: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
    sample_count: usize,
    mut volume: f32,
    ramp: f32,
) -> f32 {
    let a0 = param.numerator[0];
    let a1 = param.numerator[1];
    let a2 = param.numerator[2];
    let b1 = param.denominator[0];
    let b2 = param.denominator[1];

    let mut mix_state = 0.0f32;

    for i in 0..sample_count {
        let in_val = input[i];
        let out_val = in_val * a0
            + state.state0 * a1
            + state.state1 * a2
            + state.state2 * b1
            + state.state3 * b2;

        state.state1 = state.state0;
        state.state0 = in_val;
        state.state3 = state.state2;
        state.state2 = out_val;

        mix_state = ref_multiply_round_up(out_val, volume);
        output[i] += mix_state;
        volume += ramp;
    }

    mix_state
}

fn ref_process_multi_biquad_filter(
    params: &[BiquadFilterParameter],
    states: &mut [BiquadFilterState],
    output: &mut [f32],
    input: &[f32],
    sample_count: usize,
) {
    for stage in 0..params.len() {
        let param = &params[stage];
        let state = &mut states[stage];

        let a0 = param.numerator[0];
        let a1 = param.numerator[1];
        let a2 = param.numerator[2];
        let b1 = param.denominator[0];
        let b2 = param.denominator[1];

        for i in 0..sample_count {
            let in_val = if stage != 0 { output[i] } else { input[i] };
            let out_val = in_val * a0
                + state.state0 * a1
                + state.state1 * a2
                + state.state2 * b1
                + state.state3 * b2;

            state.state1 = state.state0;
            state.state0 = in_val;
            state.state3 = state.state2;
            state.state2 = out_val;

            output[i] = out_val;
        }
    }
}

fn ref_process_double_biquad_filter_and_mix(
    param0: &BiquadFilterParameter,
    param1: &BiquadFilterParameter,
    state0: &mut BiquadFilterState,
    state1: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
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

    for i in 0..sample_count {
        let in0 = input[i];
        let out0 = in0 * a00
            + state0.state0 * a10
            + state0.state1 * a20
            + state0.state2 * b10
            + state0.state3 * b20;

        state0.state1 = state0.state0;
        state0.state0 = in0;
        state0.state3 = state0.state2;
        state0.state2 = out0;

        let in1 = out0;
        let out1 = in1 * a01
            + state1.state0 * a11
            + state1.state1 * a21
            + state1.state2 * b11
            + state1.state3 * b21;

        state1.state1 = state1.state0;
        state1.state0 = in1;
        state1.state3 = state1.state2;
        state1.state2 = out1;

        output[i] += ref_multiply_round_up(out1, volume);
    }
}

fn ref_process_double_biquad_filter_and_mix_ramp(
    param0: &BiquadFilterParameter,
    param1: &BiquadFilterParameter,
    state0: &mut BiquadFilterState,
    state1: &mut BiquadFilterState,
    output: &mut [f32],
    input: &[f32],
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

    let mut mix_state = 0.0f32;

    for i in 0..sample_count {
        let in0 = input[i];
        let out0 = in0 * a00
            + state0.state0 * a10
            + state0.state1 * a20
            + state0.state2 * b10
            + state0.state3 * b20;

        state0.state1 = state0.state0;
        state0.state0 = in0;
        state0.state3 = state0.state2;
        state0.state2 = out0;

        let in1 = out0;
        let out1 = in1 * a01
            + state1.state0 * a11
            + state1.state1 * a21
            + state1.state2 * b11
            + state1.state3 * b21;

        state1.state1 = state1.state0;
        state1.state0 = in1;
        state1.state3 = state1.state2;
        state1.state2 = out1;

        mix_state = ref_multiply_round_up(out1, volume);
        output[i] += mix_state;
        volume += ramp;
    }

    mix_state
}

// ----------------------------------------------------------------------------
// Step 1: Invariants, Boundary Checks, Struct Alignments
// ----------------------------------------------------------------------------

#[test]
fn test_step1_struct_layout_and_sizes() {
    assert_eq!(size_of::<BiquadFilterParameter>(), 0x18); // 24 bytes
    assert_eq!(size_of::<BiquadFilterState>(), 0x20); // 32 bytes
}

#[test]
fn test_step1_ffi_null_and_zero_safety() {
    unsafe {
        let mut state = BiquadFilterState::default();
        let param = BiquadFilterParameter::default();
        let mut buf = [0.0f32; 16];

        // Null pointer safety
        assert_eq!(
            ryu_biquad_process(
                std::ptr::null(),
                &mut state,
                buf.as_mut_ptr(),
                buf.as_ptr(),
                16
            ),
            -1
        );
        assert_eq!(
            ryu_biquad_process(
                &param,
                std::ptr::null_mut(),
                buf.as_mut_ptr(),
                buf.as_ptr(),
                16
            ),
            -1
        );
        assert_eq!(
            ryu_biquad_process(&param, &mut state, std::ptr::null_mut(), buf.as_ptr(), 16),
            -1
        );
        assert_eq!(
            ryu_biquad_process(&param, &mut state, buf.as_mut_ptr(), std::ptr::null(), 16),
            -1
        );

        // Zero sample count is a valid no-op
        assert_eq!(
            ryu_biquad_process(&param, &mut state, buf.as_mut_ptr(), buf.as_ptr(), 0),
            0
        );
        assert_eq!(
            ryu_biquad_process_and_mix(&param, &mut state, buf.as_mut_ptr(), buf.as_ptr(), 0, 1.0),
            0
        );
        assert_eq!(
            ryu_biquad_process_and_mix_ramp(
                &param,
                &mut state,
                buf.as_mut_ptr(),
                buf.as_ptr(),
                0,
                1.0,
                0.1,
                std::ptr::null_mut()
            ),
            0
        );
        assert_eq!(
            ryu_biquad_process_multi(&param, &mut state, 0, buf.as_mut_ptr(), buf.as_ptr(), 16),
            0
        );
        assert_eq!(
            ryu_biquad_process_double_and_mix(
                &param,
                &param,
                &mut state,
                &mut state,
                buf.as_mut_ptr(),
                buf.as_ptr(),
                0,
                1.0
            ),
            0
        );
        assert_eq!(
            ryu_biquad_process_double_and_mix_ramp(
                &param,
                &param,
                &mut state,
                &mut state,
                buf.as_mut_ptr(),
                buf.as_ptr(),
                0,
                1.0,
                0.1,
                std::ptr::null_mut()
            ),
            0
        );
    }
}

// ----------------------------------------------------------------------------
// Step 2: Deterministic Dual-Execution Bit-Identical Tests
// ----------------------------------------------------------------------------

fn create_sample_signals(len: usize) -> Vec<f32> {
    let mut sig = Vec::with_capacity(len);
    for i in 0..len {
        let t = i as f32;
        // Mixture of harmonics, impulse, and step
        sig.push((t * 0.1).sin() * 0.5 + (t * 0.35).cos() * 0.3 + if i == 5 { 1.0 } else { 0.0 });
    }
    sig
}

#[test]
fn test_step2_biquad_filter_bit_identical() {
    let param = BiquadFilterParameter {
        enable: 1,
        _reserved1: 0,
        _reserved2: 0,
        _reserved3: 0,
        numerator: [0.2929, 0.5858, 0.2929],
        denominator: [-0.0, 0.1716],
    };

    let sample_counts = [16, 160, 240, 480, 1024];

    for &cnt in &sample_counts {
        let input = create_sample_signals(cnt);
        let mut rust_out = vec![0.0f32; cnt];
        let mut ref_out = vec![0.0f32; cnt];

        let mut rust_state = BiquadFilterState::default();
        let mut ref_state = BiquadFilterState::default();

        process_biquad_filter(&param, &mut rust_state, &mut rust_out, &input, cnt);
        ref_process_biquad_filter(&param, &mut ref_state, &mut ref_out, &input, cnt);

        assert_eq!(
            rust_out, ref_out,
            "Biquad filter outputs must be 100% bit-identical for count {}",
            cnt
        );
        assert_eq!(rust_state, ref_state, "Biquad states must be bit-identical");
    }
}

#[test]
fn test_step2_biquad_filter_and_mix_bit_identical() {
    let param = BiquadFilterParameter {
        enable: 1,
        _reserved1: 0,
        _reserved2: 0,
        _reserved3: 0,
        numerator: [0.15, 0.30, 0.15],
        denominator: [0.4, -0.2],
    };

    let cnt = 240;
    let input = create_sample_signals(cnt);

    let mut rust_out = vec![1.5f32; cnt];
    let mut ref_out = vec![1.5f32; cnt];

    let mut rust_state = BiquadFilterState::default();
    let mut ref_state = BiquadFilterState::default();

    let volume = 0.75f32;

    process_biquad_filter_and_mix(&param, &mut rust_state, &mut rust_out, &input, cnt, volume);
    ref_process_biquad_filter_and_mix(&param, &mut ref_state, &mut ref_out, &input, cnt, volume);

    assert_eq!(rust_out, ref_out, "Mix outputs must be 100% bit-identical");
    assert_eq!(rust_state, ref_state, "Mix states must be bit-identical");
}

#[test]
fn test_step2_biquad_filter_and_mix_ramp_bit_identical() {
    let param = BiquadFilterParameter {
        enable: 1,
        _reserved1: 0,
        _reserved2: 0,
        _reserved3: 0,
        numerator: [0.12, 0.24, 0.12],
        denominator: [0.3, -0.1],
    };

    let cnt = 240;
    let input = create_sample_signals(cnt);

    let mut rust_out = vec![2.0f32; cnt];
    let mut ref_out = vec![2.0f32; cnt];

    let mut rust_state = BiquadFilterState::default();
    let mut ref_state = BiquadFilterState::default();

    let volume = 0.2f32;
    let ramp = (0.8f32 - 0.2f32) / (cnt as f32);

    let rust_last = process_biquad_filter_and_mix_ramp(
        &param,
        &mut rust_state,
        &mut rust_out,
        &input,
        cnt,
        volume,
        ramp,
    );
    let ref_last = ref_process_biquad_filter_and_mix_ramp(
        &param,
        &mut ref_state,
        &mut ref_out,
        &input,
        cnt,
        volume,
        ramp,
    );

    assert_eq!(rust_last, ref_last, "Last sample must be identical");
    assert_eq!(
        rust_out, ref_out,
        "Ramp mix outputs must be 100% bit-identical"
    );
    assert_eq!(rust_state, ref_state, "Ramp states must be bit-identical");
}

#[test]
fn test_step2_multi_biquad_filter_bit_identical() {
    let params = [
        BiquadFilterParameter {
            enable: 1,
            _reserved1: 0,
            _reserved2: 0,
            _reserved3: 0,
            numerator: [0.1, 0.2, 0.1],
            denominator: [0.2, -0.05],
        },
        BiquadFilterParameter {
            enable: 1,
            _reserved1: 0,
            _reserved2: 0,
            _reserved3: 0,
            numerator: [0.2, 0.4, 0.2],
            denominator: [0.1, -0.02],
        },
    ];

    let cnt = 160;
    let input = create_sample_signals(cnt);

    let mut rust_out = vec![0.0f32; cnt];
    let mut ref_out = vec![0.0f32; cnt];

    let mut rust_states = [BiquadFilterState::default(), BiquadFilterState::default()];
    let mut ref_states = [BiquadFilterState::default(), BiquadFilterState::default()];

    process_multi_biquad_filter(&params, &mut rust_states, &mut rust_out, &input, cnt);
    ref_process_multi_biquad_filter(&params, &mut ref_states, &mut ref_out, &input, cnt);

    assert_eq!(
        rust_out, ref_out,
        "Cascaded biquad outputs must be 100% bit-identical"
    );
    assert_eq!(
        rust_states, ref_states,
        "Cascaded states must be bit-identical"
    );
}

#[test]
fn test_step2_double_biquad_filter_and_mix_bit_identical() {
    let param0 = BiquadFilterParameter {
        enable: 1,
        _reserved1: 0,
        _reserved2: 0,
        _reserved3: 0,
        numerator: [0.08, 0.16, 0.08],
        denominator: [0.35, -0.15],
    };

    let param1 = BiquadFilterParameter {
        enable: 1,
        _reserved1: 0,
        _reserved2: 0,
        _reserved3: 0,
        numerator: [0.18, 0.36, 0.18],
        denominator: [0.25, -0.08],
    };

    let cnt = 240;
    let input = create_sample_signals(cnt);

    let mut rust_out = vec![0.5f32; cnt];
    let mut ref_out = vec![0.5f32; cnt];

    let mut rust_state0 = BiquadFilterState::default();
    let mut rust_state1 = BiquadFilterState::default();
    let mut ref_state0 = BiquadFilterState::default();
    let mut ref_state1 = BiquadFilterState::default();

    let volume = 0.85f32;

    process_double_biquad_filter_and_mix(
        &param0,
        &param1,
        &mut rust_state0,
        &mut rust_state1,
        &mut rust_out,
        &input,
        cnt,
        volume,
    );
    ref_process_double_biquad_filter_and_mix(
        &param0,
        &param1,
        &mut ref_state0,
        &mut ref_state1,
        &mut ref_out,
        &input,
        cnt,
        volume,
    );

    assert_eq!(
        rust_out, ref_out,
        "Double biquad mix outputs must be 100% bit-identical"
    );
    assert_eq!(rust_state0, ref_state0, "State0 must be identical");
    assert_eq!(rust_state1, ref_state1, "State1 must be identical");
}

#[test]
fn test_step2_double_biquad_filter_and_mix_ramp_bit_identical() {
    let param0 = BiquadFilterParameter {
        enable: 1,
        _reserved1: 0,
        _reserved2: 0,
        _reserved3: 0,
        numerator: [0.07, 0.14, 0.07],
        denominator: [0.3, -0.1],
    };

    let param1 = BiquadFilterParameter {
        enable: 1,
        _reserved1: 0,
        _reserved2: 0,
        _reserved3: 0,
        numerator: [0.14, 0.28, 0.14],
        denominator: [0.2, -0.05],
    };

    let cnt = 480;
    let input = create_sample_signals(cnt);

    let mut rust_out = vec![0.0f32; cnt];
    let mut ref_out = vec![0.0f32; cnt];

    let mut rust_state0 = BiquadFilterState::default();
    let mut rust_state1 = BiquadFilterState::default();
    let mut ref_state0 = BiquadFilterState::default();
    let mut ref_state1 = BiquadFilterState::default();

    let volume = 0.1f32;
    let ramp = (0.9f32 - 0.1f32) / (cnt as f32);

    let rust_last = process_double_biquad_filter_and_mix_ramp(
        &param0,
        &param1,
        &mut rust_state0,
        &mut rust_state1,
        &mut rust_out,
        &input,
        cnt,
        volume,
        ramp,
    );
    let ref_last = ref_process_double_biquad_filter_and_mix_ramp(
        &param0,
        &param1,
        &mut ref_state0,
        &mut ref_state1,
        &mut ref_out,
        &input,
        cnt,
        volume,
        ramp,
    );

    assert_eq!(rust_last, ref_last, "Last sample must be bit-identical");
    assert_eq!(
        rust_out, ref_out,
        "Double biquad ramp mix outputs must be 100% bit-identical"
    );
    assert_eq!(rust_state0, ref_state0, "State0 must be identical");
    assert_eq!(rust_state1, ref_state1, "State1 must be identical");
}
