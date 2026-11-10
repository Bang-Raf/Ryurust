use ryu_audio_mixer::*;

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

    // Each sample should be (in * 0.5).ceil()
    for i in 0..8 {
        let expected = (input[i] * volume).ceil();
        assert_eq!(output[i], expected);
    }
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

    let expected_sample = (1.0 * 0.5f32).ceil() + (2.0 * 0.25f32).ceil() + (3.0 * 0.1f32).ceil();
    for sample in output {
        assert_eq!(sample, expected_sample);
    }
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
        assert_eq!(s, (2.0 * 0.5f32).ceil());
    }
    for s in ch1 {
        assert_eq!(s, (2.0 * 1.0f32).ceil());
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
fn test_differential_bit_identical_to_csharp_avx() {
    // In C# Avx.Ceiling(Avx.Multiply(in, vol))
    let samples: Vec<f32> = (0..240).map(|i| (i as f32 - 120.0) * 0.125).collect();
    let volume = 0.825f32;

    let mut rust_output = vec![0.0f32; 240];
    let mut csharp_expected = vec![0.0f32; 240];

    // Reference calculation matching C#
    for i in 0..240 {
        csharp_expected[i] = (samples[i] * volume).ceil();
    }

    mix_buffer(&mut rust_output, &samples, volume);

    for i in 0..240 {
        assert_eq!(
            rust_output[i], csharp_expected[i],
            "Mismatch at sample index {}",
            i
        );
    }
}
