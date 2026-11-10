use ryu_audio_mixer::*;

/// C# `FloatingPointHelper.MultiplyRoundUp(a, b)` == `MathF.Round(a * b)`
/// (MidpointRounding.ToEven).
fn csharp_multiply_round_up(a: f32, b: f32) -> f32 {
    (a * b).round_ties_even()
}

/// C# `MixCommand.ProcessMix` as the managed code runs it: `Ceiling` on full vectors of
/// `width` floats (`Avx`/`Sse41`/`AdvSimd` paths), `MultiplyRoundUp` on the tail; width 0
/// is `ProcessMixSlowPath`.
fn csharp_process_mix(output: &mut [f32], input: &[f32], volume: f32, width: usize) {
    let sisd_start = if width == 0 {
        0
    } else {
        input.len() / width * width
    };
    for i in 0..sisd_start {
        output[i] += (input[i] * volume).ceil();
    }
    for i in sisd_start..input.len() {
        output[i] += csharp_multiply_round_up(input[i], volume);
    }
}

/// C# `MixRampCommand.ProcessMixRamp`.
fn csharp_process_mix_ramp(output: &mut [f32], input: &[f32], v0: f32, v1: f32) -> f32 {
    let sample_count = input.len();
    let ramp = (v1 - v0) / sample_count as f32;
    let mut volume = v0;
    let mut state = 0.0f32;
    for i in 0..sample_count {
        state = csharp_multiply_round_up(input[i], volume);
        output[i] += state;
        volume += ramp;
    }
    state
}

#[test]
fn test_version() {
    assert_eq!(ryu_audio_mixer_version(), 0x0001_0000);
}

#[test]
fn test_mix_buffer_basic() {
    let input = vec![
        1.0f32, 2.0f32, -1.5f32, 0.25f32, 10.0f32, -5.2f32, 0.0f32, 3.14f32,
    ];
    let mut output = vec![0.0f32; 8];
    let volume = 0.5f32;

    mix_buffer(&mut output, &input, volume);

    let mut expected = vec![0.0f32; 8];
    csharp_process_mix(&mut expected, &input, volume, host_mix_vector_width());
    assert_eq!(output, expected);
}

#[test]
fn test_mix_ramp() {
    let count = 16;
    let input = vec![1.0f32; count];
    let mut output = vec![0.0f32; count];
    let v0 = 0.0f32;
    let v1 = 1.0f32;

    let last = mix_ramp(&mut output, &input, v0, v1);
    assert!(last > 0.0);

    // Verify monotonically increasing volume ramp
    for i in 1..count {
        assert!(output[i] >= output[i - 1]);
    }
}

#[test]
fn test_mix_batch() {
    let sample_count = 64;
    let v1 = vec![1.0f32; sample_count];
    let v2 = vec![2.0f32; sample_count];
    let v3 = vec![3.0f32; sample_count];
    let mut output = vec![0.0f32; sample_count];

    let inputs = [&v1[..], &v2[..], &v3[..]];
    let volumes = [0.5f32, 0.25f32, 0.1f32];

    mix_batch(&mut output, &inputs, &volumes);

    let mut expected = vec![0.0f32; sample_count];
    for (input, &vol) in inputs.iter().zip(&volumes) {
        csharp_process_mix(&mut expected, input, vol, host_mix_vector_width());
    }
    assert_eq!(output, expected);
}

#[test]
fn test_mix_matrix() {
    let sample_count = 32;
    let input = vec![2.0f32; sample_count];
    let mut ch0 = vec![0.0f32; sample_count];
    let mut ch1 = vec![0.0f32; sample_count];

    let gains = [0.5f32, 1.0f32];
    let mut outputs = [&mut ch0[..], &mut ch1[..]];

    mix_matrix(&mut outputs, &input, &gains);

    for s in ch0 {
        assert_eq!(s, 1.0);
    }
    for s in ch1 {
        assert_eq!(s, 2.0);
    }
}

#[test]
fn test_c_abi() {
    let input = [1.0f32, 2.0f32, 3.0f32, 4.0f32];
    let mut output = [0.0f32; 4];
    let mut last = 0.0f32;

    unsafe {
        assert_eq!(
            ryu_audio_mix(output.as_mut_ptr(), input.as_ptr(), 4, 1.0),
            0
        );
        assert_eq!(output, [1.0, 2.0, 3.0, 4.0]);

        assert_eq!(
            ryu_audio_mix_ramp(output.as_mut_ptr(), input.as_ptr(), 4, 0.0, 1.0, &mut last),
            0
        );
        assert!(last > 0.0);
    }
}

#[test]
fn test_differential_bit_identical_to_csharp_mix() {
    // Lengths cover: no full vector, exact multiples of 4 and 8, and ragged tails.
    for &len in &[1usize, 3, 4, 7, 8, 13, 240, 241, 247] {
        let samples: Vec<f32> = (0..len).map(|i| (i as f32 - 120.0) * 0.125).collect();
        for &volume in &[0.825f32, 0.5, 1.0, 0.0, -0.75, 1.5] {
            let mut rust_output: Vec<f32> = (0..len).map(|i| i as f32 * 0.5).collect();
            let mut csharp_expected = rust_output.clone();
            csharp_process_mix(
                &mut csharp_expected,
                &samples,
                volume,
                host_mix_vector_width(),
            );
            mix_buffer(&mut rust_output, &samples, volume);
            for i in 0..len {
                assert_eq!(
                    rust_output[i].to_bits(),
                    csharp_expected[i].to_bits(),
                    "len {len} volume {volume} sample {i}"
                );
            }
        }
    }
}

#[test]
fn test_scalar_path_is_round_half_to_even() {
    // ProcessMixSlowPath: MathF.Round(x) rounds .5 to even, unlike ceil.
    let input = [0.5f32, 1.5, 2.5, -0.5, -1.5, -2.5, 0.25, -0.75];
    let mut output = [0.0f32; 8];
    mix_buffer_scalar(&mut output, &input, 1.0);
    assert_eq!(output, [0.0, 2.0, 2.0, -0.0, -2.0, -2.0, 0.0, -1.0]);
}

#[test]
fn test_mix_tail_uses_multiply_round_up() {
    // 9 samples: on AVX the 9th, on SSE4.1/AdvSimd the 9th, and on the slow path all of them
    // go through MultiplyRoundUp. 2.5 rounds to 2 (half to even) rather than ceil's 3.
    let input = [2.5f32; 9];
    let mut output = [0.0f32; 9];
    mix_buffer(&mut output, &input, 1.0);
    assert_eq!(output[8], 2.0);
    let expected_head = if host_mix_vector_width() == 0 {
        2.0
    } else {
        3.0
    };
    assert!(output[..8].iter().all(|&s| s == expected_head));
}

#[test]
fn test_differential_bit_identical_to_csharp_mix_ramp() {
    for &len in &[1usize, 5, 240] {
        let samples: Vec<f32> = (0..len)
            .map(|i| ((i as f32) * 0.37).sin() * 1000.0 + 0.5)
            .collect();
        for &(v0, v1) in &[(0.0f32, 1.0f32), (0.1, 0.9), (1.0, 0.25), (0.5, 0.5)] {
            let mut rust_output = vec![1.0f32; len];
            let mut csharp_expected = rust_output.clone();
            let expected_last = csharp_process_mix_ramp(&mut csharp_expected, &samples, v0, v1);
            let last = mix_ramp(&mut rust_output, &samples, v0, v1);
            assert_eq!(last.to_bits(), expected_last.to_bits());
            for i in 0..len {
                assert_eq!(
                    rust_output[i].to_bits(),
                    csharp_expected[i].to_bits(),
                    "len {len} ramp {v0}->{v1} sample {i}"
                );
            }
        }
    }
    // Half-way values must round to even in the ramp (C# MixRampCommand is scalar only).
    let mut out = [0.0f32; 2];
    let last = mix_ramp(&mut out, &[2.5, 2.5], 1.0, 1.0);
    assert_eq!(out, [2.0, 2.0]);
    assert_eq!(last, 2.0);
}
