//! Mixing kernels that replace `MixCommand.ProcessMix` and `MixRampCommand.ProcessMixRamp`
//! in `Ryujinx.Audio` (called through `bindings/csharp/NativeAudioMixer.cs`).
//!
//! # Rounding, bit-exact with the managed fallback
//!
//! `FloatingPointHelper.MultiplyRoundUp(a, b)` is `MathF.Round(a * b)`, i.e. round half to
//! even. The managed `MixCommand` however only uses it on its scalar paths: the vectorised
//! paths (`Avx`, `Sse41`, `AdvSimd`) apply `Ceiling` to every full vector and
//! `MultiplyRoundUp` to the remaining tail samples. `MixRampCommand` is scalar only and
//! always uses `MultiplyRoundUp`.
//!
//! To produce the same samples the C# code would have produced on the same machine,
//! [`mix_buffer`] picks the vector width the managed code would pick (8 with AVX, 4 with
//! SSE4.1 or AArch64 AdvSIMD, none otherwise), rounds the first `count / width * width`
//! samples with `ceil` and the tail with round-half-to-even. [`mix_ramp`] always rounds half
//! to even.

#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;
use std::sync::OnceLock;

/// `FloatingPointHelper.RoundUp` / `MultiplyRoundUp`: `MathF.Round(x)` (half to even).
#[inline(always)]
fn round_up(val: f32) -> f32 {
    val.round_ties_even()
}

/// Which managed `MixCommand.ProcessMix*` variant this host would run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MixPath {
    /// `ProcessMixAvx`: `Ceiling` on 8-wide vectors, `MultiplyRoundUp` on the tail.
    #[cfg(target_arch = "x86_64")]
    Avx,
    /// `ProcessMixSse41` / `ProcessMixAdvSimd`: `Ceiling` on 4-wide vectors,
    /// `MultiplyRoundUp` on the tail.
    #[cfg_attr(
        not(any(target_arch = "x86_64", target_arch = "aarch64")),
        allow(dead_code)
    )]
    Width4,
    /// `ProcessMixSlowPath`: `MultiplyRoundUp` on every sample.
    #[cfg_attr(target_arch = "aarch64", allow(dead_code))]
    Scalar,
}

fn detect_mix_path() -> MixPath {
    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx") {
            return MixPath::Avx;
        }
        if is_x86_feature_detected!("sse4.1") {
            return MixPath::Width4;
        }
        MixPath::Scalar
    }
    #[cfg(target_arch = "aarch64")]
    {
        MixPath::Width4
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        MixPath::Scalar
    }
}

/// CPU feature detection is done once per process and cached here.
#[inline]
fn mix_path() -> MixPath {
    static PATH: OnceLock<MixPath> = OnceLock::new();
    *PATH.get_or_init(detect_mix_path)
}

/// Mixes `input * volume` into `output`, bit-identical to the managed `MixCommand.ProcessMix`
/// on this host (see the module docs for the rounding rules).
pub fn mix_buffer(output: &mut [f32], input: &[f32], volume: f32) {
    mix_buffer_with_path(mix_path(), output, input, volume);
}

#[inline]
fn mix_buffer_with_path(path: MixPath, output: &mut [f32], input: &[f32], volume: f32) {
    let count = output.len().min(input.len());
    if count == 0 {
        return;
    }
    let output = &mut output[..count];
    let input = &input[..count];

    match path {
        #[cfg(target_arch = "x86_64")]
        MixPath::Avx => {
            // SAFETY: `MixPath::Avx` is only selected after `is_x86_feature_detected!("avx")`
            // returned true, and both slices have exactly `count` elements.
            unsafe {
                mix_buffer_avx(output.as_mut_ptr(), input.as_ptr(), count, volume);
            }
        }
        MixPath::Width4 => mix_buffer_ceil_chunks(output, input, volume, 4),
        MixPath::Scalar => mix_buffer_scalar(output, input, volume),
    }
}

/// `Ceiling` on full `width`-sample chunks, `MultiplyRoundUp` on the tail.
#[inline]
fn mix_buffer_ceil_chunks(output: &mut [f32], input: &[f32], volume: f32, width: usize) {
    let count = output.len().min(input.len());
    let vec_end = count / width * width;
    for (o, &i) in output[..vec_end].iter_mut().zip(&input[..vec_end]) {
        *o += (i * volume).ceil();
    }
    for (o, &i) in output[vec_end..count]
        .iter_mut()
        .zip(&input[vec_end..count])
    {
        *o += round_up(i * volume);
    }
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx")]
unsafe fn mix_buffer_avx(out_ptr: *mut f32, in_ptr: *const f32, count: usize, volume: f32) {
    let chunks = count / 8;
    let remainder = count % 8;

    let v_vol = _mm256_set1_ps(volume);

    for i in 0..chunks {
        let offset = i * 8;
        // SAFETY: the caller guarantees `count` readable/writable floats behind both pointers
        // and `offset + 8 <= chunks * 8 <= count`.
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
        // SAFETY: `idx < start + remainder == count`.
        let val = *in_ptr.add(idx) * volume;
        *out_ptr.add(idx) += round_up(val);
    }
}

/// `MixCommand.ProcessMixSlowPath`: `MultiplyRoundUp` (round half to even) on every sample.
pub fn mix_buffer_scalar(output: &mut [f32], input: &[f32], volume: f32) {
    let count = output.len().min(input.len());
    for i in 0..count {
        output[i] += round_up(input[i] * volume);
    }
}

/// `MixRampCommand.ProcessMixRamp`: linear volume ramp, `MultiplyRoundUp` on every sample.
/// Returns the last mixed sample (stored by C# into `LastSamples`).
pub fn mix_ramp(output: &mut [f32], input: &[f32], volume0: f32, volume1: f32) -> f32 {
    let sample_count = output.len().min(input.len());
    if sample_count == 0 {
        return 0.0;
    }

    let ramp = (volume1 - volume0) / (sample_count as f32);
    let mut volume = volume0;
    let mut last_sample = 0.0f32;

    for i in 0..sample_count {
        last_sample = round_up(input[i] * volume);
        output[i] += last_sample;
        volume += ramp;
    }

    last_sample
}

pub fn mix_batch(output: &mut [f32], inputs: &[&[f32]], volumes: &[f32]) {
    let path = mix_path();
    let voice_count = inputs.len().min(volumes.len());
    for v in 0..voice_count {
        mix_buffer_with_path(path, output, inputs[v], volumes[v]);
    }
}

pub fn mix_matrix(outputs: &mut [&mut [f32]], input: &[f32], gains: &[f32]) {
    let path = mix_path();
    let channel_count = outputs.len().min(gains.len());
    for c in 0..channel_count {
        mix_buffer_with_path(path, outputs[c], input, gains[c]);
    }
}

/// Vector width (8, 4, or 0 for scalar) of the managed `MixCommand` path this host selects.
#[doc(hidden)]
pub fn host_mix_vector_width() -> usize {
    match mix_path() {
        #[cfg(target_arch = "x86_64")]
        MixPath::Avx => 8,
        MixPath::Width4 => 4,
        MixPath::Scalar => 0,
    }
}
