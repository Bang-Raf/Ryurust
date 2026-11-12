//! Polyphase Sinc FIR Resampler implementation matching Ryubing Audio Renderer.

use crate::lut::*;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResampleQuality {
    Default = 0,
    High = 1,
    Low = 2,
}

impl From<u8> for ResampleQuality {
    fn from(val: u8) -> Self {
        match val {
            1 => ResampleQuality::High,
            2 => ResampleQuality::Low,
            _ => ResampleQuality::Default,
        }
    }
}

#[inline(always)]
fn get_default_parameters(ratio: f32) -> &'static [f32; 512] {
    if ratio <= 1.0 {
        &NORMAL_LUT_1
    } else if ratio > 1.333313 {
        &NORMAL_LUT_0
    } else {
        &NORMAL_LUT_2
    }
}

#[inline(always)]
fn get_high_parameters(ratio: f32) -> &'static [f32; 1024] {
    if ratio <= 1.0 {
        &HIGH_LUT_1
    } else if ratio > 1.333313 {
        &HIGH_LUT_0
    } else {
        &HIGH_LUT_2
    }
}

/// Resamples 16-bit PCM input to 32-bit floating point output using 4-tap polyphase sinc filter.
pub fn resample_default(
    output: &mut [f32],
    input: &[i16],
    ratio: f32,
    fraction: &mut f32,
    sample_count: usize,
) {
    let parameters = get_default_parameters(ratio);
    let mut input_idx = 0usize;
    let mut frac = *fraction;

    for out in output.iter_mut().take(sample_count) {
        let base_idx = (((frac * 128.0) as usize) & 127) * 4;
        let p = &parameters[base_idx..base_idx + 4];

        let in0 = if input_idx < input.len() {
            input[input_idx] as f32
        } else {
            0.0
        };
        let in1 = if input_idx + 1 < input.len() {
            input[input_idx + 1] as f32
        } else {
            0.0
        };
        let in2 = if input_idx + 2 < input.len() {
            input[input_idx + 2] as f32
        } else {
            0.0
        };
        let in3 = if input_idx + 3 < input.len() {
            input[input_idx + 3] as f32
        } else {
            0.0
        };

        *out = (in0 * p[0] + in1 * p[1] + in2 * p[2] + in3 * p[3]).round();

        frac += ratio;
        input_idx += frac as usize;
        frac -= (frac as u32) as f32;
    }

    *fraction = frac;
}

/// Resamples 16-bit PCM input using 8-tap high quality polyphase sinc filter.
pub fn resample_high(
    output: &mut [f32],
    input: &[i16],
    ratio: f32,
    fraction: &mut f32,
    sample_count: usize,
) {
    let parameters = get_high_parameters(ratio);
    let mut input_idx = 0usize;
    let mut frac = *fraction;

    for out in output.iter_mut().take(sample_count) {
        let base_idx = (((frac * 128.0) as usize) & 127) * 8;
        let p = &parameters[base_idx..base_idx + 8];

        let mut sum = 0.0f32;
        for k in 0..8 {
            let in_val = if input_idx + k < input.len() {
                input[input_idx + k] as f32
            } else {
                0.0
            };
            sum += in_val * p[k];
        }

        *out = sum.round();

        frac += ratio;
        input_idx += frac as usize;
        frac -= (frac as u32) as f32;
    }

    *fraction = frac;
}

/// Fast low-quality nearest-neighbor resampling.
pub fn resample_low(
    output: &mut [f32],
    input: &[i16],
    ratio: f32,
    fraction: &mut f32,
    sample_count: usize,
) {
    let mut input_idx = 0usize;
    let mut frac = *fraction;

    for out in output.iter_mut().take(sample_count) {
        let val = if frac > 1.0 {
            if input_idx + 1 < input.len() {
                input[input_idx + 1]
            } else {
                0
            }
        } else {
            if input_idx < input.len() {
                input[input_idx]
            } else {
                0
            }
        };

        *out = val as f32;

        frac += ratio;
        input_idx += frac as usize;
        frac -= (frac as u32) as f32;
    }

    *fraction = frac;
}

/// Unified entry point for audio resampling.
pub fn resample(
    output: &mut [f32],
    input: &[i16],
    ratio: f32,
    fraction: &mut f32,
    sample_count: usize,
    quality: ResampleQuality,
) {
    match quality {
        ResampleQuality::Default => resample_default(output, input, ratio, fraction, sample_count),
        ResampleQuality::High => resample_high(output, input, ratio, fraction, sample_count),
        ResampleQuality::Low => resample_low(output, input, ratio, fraction, sample_count),
    }
}
