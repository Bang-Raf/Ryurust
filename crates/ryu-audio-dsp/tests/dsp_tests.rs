use ryu_audio_dsp::batch::{process_command_batch, DspCommandEntry, DspOpcode};
use ryu_audio_dsp::dsp;
use ryu_audio_dsp::ffi::*;
use ryu_audio_dsp::filter::{
    process_biquad_filter_and_mix_df1, process_biquad_filter_and_mix_ramp_df1,
    process_biquad_filter_df2, process_double_biquad_filter_and_mix,
    process_double_biquad_filter_and_mix_ramp, process_multi_tap_biquad_filter,
    BiquadFilterParameter, BiquadFilterState,
};
use std::mem::size_of;

// =========================================================================
// STEP 1: SAFETY, INVARIANTS, NULL POINTER & BOUNDARY VERIFICATION
// =========================================================================

#[test]
fn test_step1_struct_sizes_and_alignments() {
    assert_eq!(
        size_of::<BiquadFilterParameter>(),
        24,
        "BiquadFilterParameter must be 24 bytes (0x18)"
    );
    assert_eq!(
        size_of::<BiquadFilterState>(),
        32,
        "BiquadFilterState must be 32 bytes (0x20)"
    );
}

#[test]
fn test_step1_null_pointer_safety() {
    unsafe {
        assert_eq!(
            ryu_dsp_clear_buffer(std::ptr::null_mut(), 100),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_clear_buffers(std::ptr::null_mut(), 100),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_copy_buffer(std::ptr::null_mut(), 10, std::ptr::null(), 10),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_mix(std::ptr::null_mut(), 10, std::ptr::null(), 10, 1.0),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_mix_ramp(
                std::ptr::null_mut(),
                10,
                std::ptr::null(),
                10,
                0.0,
                1.0,
                std::ptr::null_mut()
            ),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_volume(std::ptr::null_mut(), 10, std::ptr::null(), 10, 1.0),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_volume_ramp(std::ptr::null_mut(), 10, std::ptr::null(), 10, 0.0, 1.0),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_biquad_filter(
                std::ptr::null_mut(),
                10,
                std::ptr::null(),
                10,
                std::ptr::null(),
                std::ptr::null_mut()
            ),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_biquad_filter_and_mix(
                std::ptr::null_mut(),
                10,
                std::ptr::null(),
                10,
                std::ptr::null(),
                std::ptr::null_mut(),
                1.0
            ),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_biquad_filter_and_mix_ramp(
                std::ptr::null_mut(),
                10,
                std::ptr::null(),
                10,
                std::ptr::null(),
                std::ptr::null_mut(),
                0.0,
                1.0,
                std::ptr::null_mut()
            ),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_double_biquad_filter_and_mix(
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                1.0
            ),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_double_biquad_filter_and_mix_ramp(
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0.0,
                1.0,
                std::ptr::null_mut()
            ),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_multi_tap_biquad_filter(
                std::ptr::null_mut(),
                0,
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
                0
            ),
            DSP_STATUS_NULL_POINTER
        );
        assert_eq!(
            ryu_dsp_process_command_batch(
                std::ptr::null_mut(),
                0,
                0,
                std::ptr::null(),
                0,
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
                0
            ),
            DSP_STATUS_NULL_POINTER
        );
    }
}

#[test]
fn test_step1_empty_buffer_handling() {
    let mut out = [0.0f32; 16];
    let inp = [1.0f32; 16];
    let param = BiquadFilterParameter::default();
    let mut state = BiquadFilterState::default();

    unsafe {
        assert_eq!(ryu_dsp_clear_buffer(out.as_mut_ptr(), 0), DSP_STATUS_OK);
        assert_eq!(ryu_dsp_clear_buffers(out.as_mut_ptr(), 0), DSP_STATUS_OK);
        assert_eq!(
            ryu_dsp_copy_buffer(out.as_mut_ptr(), 0, inp.as_ptr(), 0),
            DSP_STATUS_OK
        );
        assert_eq!(
            ryu_dsp_mix(out.as_mut_ptr(), 0, inp.as_ptr(), 0, 0.5),
            DSP_STATUS_OK
        );
        assert_eq!(
            ryu_dsp_mix_ramp(
                out.as_mut_ptr(),
                0,
                inp.as_ptr(),
                0,
                0.0,
                1.0,
                std::ptr::null_mut()
            ),
            DSP_STATUS_OK
        );
        assert_eq!(
            ryu_dsp_volume(out.as_mut_ptr(), 0, inp.as_ptr(), 0, 0.5),
            DSP_STATUS_OK
        );
        assert_eq!(
            ryu_dsp_volume_ramp(out.as_mut_ptr(), 0, inp.as_ptr(), 0, 0.0, 1.0),
            DSP_STATUS_OK
        );
        assert_eq!(
            ryu_dsp_biquad_filter(out.as_mut_ptr(), 0, inp.as_ptr(), 0, &param, &mut state),
            DSP_STATUS_OK
        );
    }
}

// =========================================================================
// STEP 2: DETERMINISTIC DUAL-EXECUTION & BIT-IDENTICAL VERIFICATION
// =========================================================================

#[test]
fn test_step2_mix_and_volume_bit_identical() {
    let sample_count = 240; // 5ms quantum @ 48kHz
    let mut input = vec![0.0f32; sample_count];
    for (i, x) in input.iter_mut().enumerate() {
        *x = ((i as f32) * 137.0 % 65536.0) - 32768.0;
    }

    let volume_val = 0.725f32;

    // Test Volume
    let mut out_vol = vec![0.0f32; sample_count];
    dsp::volume(&mut out_vol, &input, volume_val);

    for i in 0..sample_count {
        let expected = (input[i] * volume_val).round_ties_even();
        assert_eq!(out_vol[i], expected, "Volume mismatch at sample {}", i);
    }

    // Test Mix
    let mut out_mix = vec![1000.0f32; sample_count];
    dsp::mix(&mut out_mix, &input, volume_val);

    for i in 0..sample_count {
        let expected = 1000.0f32 + (input[i] * volume_val).round_ties_even();
        assert_eq!(out_mix[i], expected, "Mix mismatch at sample {}", i);
    }
}

#[test]
fn test_step2_mix_ramp_and_volume_ramp_bit_identical() {
    let sample_count = 240;
    let mut input = vec![0.0f32; sample_count];
    for (i, x) in input.iter_mut().enumerate() {
        *x = (i as f32 * 31.0) - 100.0;
    }

    let vol0 = 0.1f32;
    let vol1 = 0.9f32;
    let ramp = (vol1 - vol0) / sample_count as f32;

    // Volume Ramp
    let mut out_vramp = vec![0.0f32; sample_count];
    dsp::volume_ramp(&mut out_vramp, &input, vol0, vol1);

    let mut expected_vol = vol0;
    for i in 0..sample_count {
        let expected = (input[i] * expected_vol).round_ties_even();
        assert_eq!(
            out_vramp[i], expected,
            "VolumeRamp mismatch at sample {}",
            i
        );
        expected_vol += ramp;
    }

    // Mix Ramp
    let mut out_mramp = vec![50.0f32; sample_count];
    let last_sample = dsp::mix_ramp(&mut out_mramp, &input, vol0, vol1);

    expected_vol = vol0;
    let mut last_expected = 0.0f32;
    for i in 0..sample_count {
        let state = (input[i] * expected_vol).round_ties_even();
        let expected = 50.0f32 + state;
        assert_eq!(out_mramp[i], expected, "MixRamp mismatch at sample {}", i);
        last_expected = state;
        expected_vol += ramp;
    }
    assert_eq!(
        last_sample, last_expected,
        "Last sample mismatch in MixRamp"
    );
}

#[test]
fn test_step2_biquad_filter_df2_deterministic() {
    let sample_count = 240;
    let mut input = vec![0.0f32; sample_count];
    for (i, x) in input.iter_mut().enumerate() {
        *x = ((i as f32 * 0.1).sin()) * 10000.0;
    }

    let param = BiquadFilterParameter {
        enable: 1,
        _reserved: [0; 3],
        numerator: [0.067455, 0.134911, 0.067455],
        denominator: [-1.14298, 0.41280],
    };

    let mut state1 = BiquadFilterState::default();
    let mut out1 = vec![0.0f32; sample_count];
    process_biquad_filter_df2(&param, &mut state1, &mut out1, &input);

    // Reference Direct Form 2 evaluation
    let mut ref_s0 = 0.0f32;
    let mut ref_s1 = 0.0f32;
    for i in 0..sample_count {
        let in_samp = input[i];
        let expected_out = in_samp * param.numerator[0] + ref_s0;
        ref_s0 = in_samp * param.numerator[1] + expected_out * param.denominator[0] + ref_s1;
        ref_s1 = in_samp * param.numerator[2] + expected_out * param.denominator[1];

        assert_eq!(out1[i], expected_out, "Biquad DF2 mismatch at sample {}", i);
    }

    assert_eq!(state1.state0, ref_s0);
    assert_eq!(state1.state1, ref_s1);
}

#[test]
fn test_step2_biquad_filter_and_mix_df1_deterministic() {
    let sample_count = 240;
    let mut input = vec![0.0f32; sample_count];
    for (i, x) in input.iter_mut().enumerate() {
        *x = ((i as f32 * 0.15).cos()) * 8000.0;
    }

    let param = BiquadFilterParameter {
        enable: 1,
        _reserved: [0; 3],
        numerator: [0.1, 0.2, 0.1],
        denominator: [-0.5, 0.25],
    };

    let mut state = BiquadFilterState::default();
    let mut output = vec![100.0f32; sample_count];
    let vol = 0.8f32;

    process_biquad_filter_and_mix_df1(&param, &mut state, &mut output, &input, vol);

    // Reference Direct Form 1 evaluation
    let mut s0 = 0.0f32;
    let mut s1 = 0.0f32;
    let mut s2 = 0.0f32;
    let mut s3 = 0.0f32;

    for i in 0..sample_count {
        let in_samp = input[i];
        let out_samp = in_samp * param.numerator[0]
            + s0 * param.numerator[1]
            + s1 * param.numerator[2]
            + s2 * param.denominator[0]
            + s3 * param.denominator[1];

        s1 = s0;
        s0 = in_samp;
        s3 = s2;
        s2 = out_samp;

        let expected = 100.0f32 + (out_samp * vol).round_ties_even();
        assert_eq!(output[i], expected, "Biquad DF1 mismatch at sample {}", i);
    }
}

#[test]
fn test_step2_biquad_filter_and_mix_ramp_df1_deterministic() {
    let sample_count = 240;
    let mut input = vec![0.0f32; sample_count];
    for (i, x) in input.iter_mut().enumerate() {
        *x = ((i as f32 * 0.2).sin()) * 5000.0;
    }

    let param = BiquadFilterParameter {
        enable: 1,
        _reserved: [0; 3],
        numerator: [0.08, 0.16, 0.08],
        denominator: [-0.6, 0.3],
    };

    let mut state = BiquadFilterState::default();
    let mut output = vec![0.0f32; sample_count];
    let vol0 = 0.2f32;
    let vol1 = 0.8f32;

    let last_s =
        process_biquad_filter_and_mix_ramp_df1(&param, &mut state, &mut output, &input, vol0, vol1);

    let ramp = (vol1 - vol0) / sample_count as f32;
    let mut expected_vol = vol0;
    let mut ref_s0 = 0.0f32;
    let mut ref_s1 = 0.0f32;
    let mut ref_s2 = 0.0f32;
    let mut ref_s3 = 0.0f32;
    let mut expected_last = 0.0f32;

    for i in 0..sample_count {
        let in_samp = input[i];
        let out_samp = in_samp * param.numerator[0]
            + ref_s0 * param.numerator[1]
            + ref_s1 * param.numerator[2]
            + ref_s2 * param.denominator[0]
            + ref_s3 * param.denominator[1];

        ref_s1 = ref_s0;
        ref_s0 = in_samp;
        ref_s3 = ref_s2;
        ref_s2 = out_samp;

        expected_last = (out_samp * expected_vol).round_ties_even();
        assert_eq!(
            output[i], expected_last,
            "Biquad DF1 ramp mismatch at sample {}",
            i
        );
        expected_vol += ramp;
    }

    assert_eq!(last_s, expected_last);
}

#[test]
fn test_step2_multi_tap_and_double_biquad_cascade() {
    let sample_count = 240;
    let input: Vec<f32> = (0..sample_count)
        .map(|i| (i as f32 * 0.05).sin() * 1000.0)
        .collect();

    let param0 = BiquadFilterParameter {
        enable: 1,
        _reserved: [0; 3],
        numerator: [0.05, 0.1, 0.05],
        denominator: [-0.8, 0.2],
    };
    let param1 = BiquadFilterParameter {
        enable: 1,
        _reserved: [0; 3],
        numerator: [0.06, 0.12, 0.06],
        denominator: [-0.7, 0.25],
    };

    // Double biquad
    let mut st0 = BiquadFilterState::default();
    let mut st1 = BiquadFilterState::default();
    let mut out_double = vec![0.0f32; sample_count];
    process_double_biquad_filter_and_mix(
        &param0,
        &param1,
        &mut st0,
        &mut st1,
        &mut out_double,
        &input,
        0.75,
    );

    assert!(
        out_double.iter().any(|&x| x != 0.0),
        "Double biquad must produce non-zero output"
    );

    // Double biquad ramp
    let mut st0_r = BiquadFilterState::default();
    let mut st1_r = BiquadFilterState::default();
    let mut out_double_r = vec![0.0f32; sample_count];
    let last_r = process_double_biquad_filter_and_mix_ramp(
        &param0,
        &param1,
        &mut st0_r,
        &mut st1_r,
        &mut out_double_r,
        &input,
        0.1,
        0.9,
    );
    assert_ne!(last_r, 0.0);

    // Multi-tap biquad
    let params = [param0, param1];
    let mut states = [BiquadFilterState::default(), BiquadFilterState::default()];
    let mut out_multitap = vec![0.0f32; sample_count];
    process_multi_tap_biquad_filter(&params, &mut states, &mut out_multitap, &input);
    assert!(
        out_multitap.iter().any(|&x| x != 0.0),
        "Multi-tap biquad must produce non-zero output"
    );
}

#[test]
fn test_step2_batch_processor_differential_execution() {
    let buffer_count = 64;
    let sample_count = 240;
    let total_floats = buffer_count * sample_count;

    let mut buffers_batch = vec![0.0f32; total_floats];
    let mut buffers_ref = vec![0.0f32; total_floats];

    // Seed buffers with non-trivial waveforms
    for ch in 0..buffer_count {
        for s in 0..sample_count {
            let val = ((ch * 73 + s * 19) as f32 % 500.0) - 250.0;
            buffers_batch[ch * sample_count + s] = val;
            buffers_ref[ch * sample_count + s] = val;
        }
    }

    let filter_param = BiquadFilterParameter {
        enable: 1,
        _reserved: [0; 3],
        numerator: [0.067455, 0.134911, 0.067455],
        denominator: [-1.14298, 0.41280],
    };

    let mut commands = Vec::new();
    // Build 60 diverse commands simulating realistic Nintendo Switch audio quantum:
    for i in 0..10 {
        // Clear buffer
        commands.push(DspCommandEntry {
            opcode: DspOpcode::ClearBuffer as u32,
            input_buffer_index: 0,
            output_buffer_index: (i * 3) as u16,
            ..Default::default()
        });
        // Copy buffer
        commands.push(DspCommandEntry {
            opcode: DspOpcode::CopyBuffer as u32,
            input_buffer_index: (i + 1) as u16,
            output_buffer_index: (i * 3) as u16,
            ..Default::default()
        });
        // Volume
        commands.push(DspCommandEntry {
            opcode: DspOpcode::Volume as u32,
            input_buffer_index: (i * 3) as u16,
            output_buffer_index: (i * 3) as u16,
            volume0: 0.85,
            ..Default::default()
        });
        // Volume Ramp
        commands.push(DspCommandEntry {
            opcode: DspOpcode::VolumeRamp as u32,
            input_buffer_index: (i * 3) as u16,
            output_buffer_index: (i * 3) as u16,
            volume0: 0.2,
            volume1: 0.9,
            ..Default::default()
        });
        // Mix
        commands.push(DspCommandEntry {
            opcode: DspOpcode::Mix as u32,
            input_buffer_index: (i * 3) as u16,
            output_buffer_index: 63,
            volume0: 0.5,
            ..Default::default()
        });
        // Mix Ramp
        commands.push(DspCommandEntry {
            opcode: DspOpcode::MixRamp as u32,
            input_buffer_index: (i * 3) as u16,
            output_buffer_index: 62,
            volume0: 0.1,
            volume1: 0.7,
            last_sample_index: i as u32,
            ..Default::default()
        });
        // Biquad filter
        commands.push(DspCommandEntry {
            opcode: DspOpcode::BiquadFilter as u32,
            input_buffer_index: (i * 3) as u16,
            output_buffer_index: (i * 3) as u16,
            filter_param,
            state_index: (i % 16) as u32,
            ..Default::default()
        });
    }

    let mut states_batch = vec![BiquadFilterState::default(); 16];
    let mut states_ref = vec![BiquadFilterState::default(); 16];
    let mut last_samples_batch = vec![0.0f32; 16];
    let mut last_samples_ref = vec![0.0f32; 16];

    // 1. Run batch processor
    let count = process_command_batch(
        &mut buffers_batch,
        buffer_count,
        sample_count,
        &commands,
        &mut states_batch,
        &mut last_samples_batch,
    )
    .expect("Batch processing failed");

    assert_eq!(count, commands.len());

    // 2. Run step-by-step sequential reference
    for cmd in &commands {
        let in_idx = cmd.input_buffer_index as usize;
        let out_idx = cmd.output_buffer_index as usize;

        match DspOpcode::from_u32(cmd.opcode) {
            DspOpcode::ClearBuffer => {
                let off = out_idx * sample_count;
                buffers_ref[off..off + sample_count].fill(0.0);
            }
            DspOpcode::CopyBuffer => {
                let src_off = in_idx * sample_count;
                let dst_off = out_idx * sample_count;
                let src = buffers_ref[src_off..src_off + sample_count].to_vec();
                buffers_ref[dst_off..dst_off + sample_count].copy_from_slice(&src);
            }
            DspOpcode::Volume => {
                let src_off = in_idx * sample_count;
                let dst_off = out_idx * sample_count;
                let src = buffers_ref[src_off..src_off + sample_count].to_vec();
                dsp::volume(
                    &mut buffers_ref[dst_off..dst_off + sample_count],
                    &src,
                    cmd.volume0,
                );
            }
            DspOpcode::VolumeRamp => {
                let src_off = in_idx * sample_count;
                let dst_off = out_idx * sample_count;
                let src = buffers_ref[src_off..src_off + sample_count].to_vec();
                dsp::volume_ramp(
                    &mut buffers_ref[dst_off..dst_off + sample_count],
                    &src,
                    cmd.volume0,
                    cmd.volume1,
                );
            }
            DspOpcode::Mix => {
                let src_off = in_idx * sample_count;
                let dst_off = out_idx * sample_count;
                let src = buffers_ref[src_off..src_off + sample_count].to_vec();
                dsp::mix(
                    &mut buffers_ref[dst_off..dst_off + sample_count],
                    &src,
                    cmd.volume0,
                );
            }
            DspOpcode::MixRamp => {
                let src_off = in_idx * sample_count;
                let dst_off = out_idx * sample_count;
                let src = buffers_ref[src_off..src_off + sample_count].to_vec();
                let last = dsp::mix_ramp(
                    &mut buffers_ref[dst_off..dst_off + sample_count],
                    &src,
                    cmd.volume0,
                    cmd.volume1,
                );
                last_samples_ref[cmd.last_sample_index as usize] = last;
            }
            DspOpcode::BiquadFilter => {
                let src_off = in_idx * sample_count;
                let dst_off = out_idx * sample_count;
                let src = buffers_ref[src_off..src_off + sample_count].to_vec();
                let st_idx = cmd.state_index as usize;
                process_biquad_filter_df2(
                    &cmd.filter_param,
                    &mut states_ref[st_idx],
                    &mut buffers_ref[dst_off..dst_off + sample_count],
                    &src,
                );
            }
            _ => {}
        }
    }

    // 3. Verify bit-identical match across all 64 mix buffers (15,360 floats)
    assert_eq!(
        buffers_batch, buffers_ref,
        "Batch processor output diverged from reference!"
    );
    assert_eq!(
        last_samples_batch, last_samples_ref,
        "Batch processor last samples diverged!"
    );
    assert_eq!(
        states_batch, states_ref,
        "Batch processor filter states diverged!"
    );
}
