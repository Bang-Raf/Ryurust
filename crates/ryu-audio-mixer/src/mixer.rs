#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[inline(always)]
fn round_sample(val: f32) -> f32 {
    val.ceil()
}

pub fn mix_buffer(output: &mut [f32], input: &[f32], volume: f32) {
    let count = output.len().min(input.len());
    if count == 0 || volume == 0.0 {
        return;
    }

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx") {
            unsafe {
                mix_buffer_avx(output.as_mut_ptr(), input.as_ptr(), count, volume);
            }
            return;
        }
    }

    mix_buffer_scalar(output, input, volume);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn mix_buffer_avx(out_ptr: *mut f32, in_ptr: *const f32, count: usize, volume: f32) {
    let chunks = count / 8;
    let remainder = count % 8;

    let v_vol = _mm256_set1_ps(volume);

    for i in 0..chunks {
        let offset = i * 8;
        let v_in = _mm256_loadu_ps(in_ptr.add(offset));
        let v_out = _mm256_loadu_ps(out_ptr.add(offset));

        let v_mul = _mm256_mul_ps(v_in, v_vol);
        let v_ceil = _mm256_ceil_ps(v_mul);
        let v_res = _mm256_add_ps(v_out, v_ceil);

        _mm256_storeu_ps(out_ptr.add(offset), v_res);
    }

    let start = chunks * 8;
    for i in 0..remainder {
        let idx = start + i;
        let val = *in_ptr.add(idx) * volume;
        *out_ptr.add(idx) += round_sample(val);
    }
}

pub fn mix_buffer_scalar(output: &mut [f32], input: &[f32], volume: f32) {
    let count = output.len().min(input.len());
    for i in 0..count {
        output[i] += round_sample(input[i] * volume);
    }
}

pub fn mix_ramp(output: &mut [f32], input: &[f32], volume0: f32, volume1: f32) -> f32 {
    let sample_count = output.len().min(input.len());
    if sample_count == 0 {
        return 0.0;
    }

    let ramp = (volume1 - volume0) / (sample_count as f32);
    let mut volume = volume0;
    let mut last_sample = 0.0f32;

    for i in 0..sample_count {
        last_sample = round_sample(input[i] * volume);
        output[i] += last_sample;
        volume += ramp;
    }

    last_sample
}

pub fn mix_batch(output: &mut [f32], inputs: &[&[f32]], volumes: &[f32]) {
    let voice_count = inputs.len().min(volumes.len());
    for v in 0..voice_count {
        mix_buffer(output, inputs[v], volumes[v]);
    }
}

pub fn mix_matrix(outputs: &mut [&mut [f32]], input: &[f32], gains: &[f32]) {
    let channel_count = outputs.len().min(gains.len());
    for c in 0..channel_count {
        mix_buffer(outputs[c], input, gains[c]);
    }
}
