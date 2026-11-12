//! Hardware ADPCM Decoder matching Nintendo Switch Audio Renderer specifications.

pub const SAMPLES_PER_FRAME: usize = 14;
pub const NIBBLES_PER_FRAME: usize = SAMPLES_PER_FRAME + 2; // 16 nibbles (8 bytes)
pub const BYTES_PER_FRAME: usize = 8;
pub const FIXED_POINT_PRECISION: i32 = 11;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AdpcmLoopContext {
    pub pred_scale: i16,
    pub history0: i16,
    pub history1: i16,
}

#[inline(always)]
pub fn get_adpcm_data_size(sample_count: usize) -> usize {
    let frames = sample_count / SAMPLES_PER_FRAME;
    let remainder = sample_count % SAMPLES_PER_FRAME;
    let extra_size = if remainder != 0 {
        remainder / 2 + 1 + (remainder % 2)
    } else {
        0
    };
    BYTES_PER_FRAME * frames + extra_size
}

#[inline(always)]
pub fn get_nibbles_from_sample_count(sample_count: usize) -> usize {
    let header_size = if !sample_count.is_multiple_of(SAMPLES_PER_FRAME) {
        2
    } else {
        0
    };
    (sample_count / SAMPLES_PER_FRAME) * NIBBLES_PER_FRAME
        + (sample_count % SAMPLES_PER_FRAME)
        + header_size
}

#[inline(always)]
fn saturate(val: i32) -> i16 {
    val.clamp(i16::MIN as i32, i16::MAX as i32) as i16
}

#[inline(always)]
fn get_coefficient(coefficients: &[i16], index: usize) -> i16 {
    if index < coefficients.len() {
        coefficients[index]
    } else {
        0
    }
}

/// Decodes Nintendo DSP ADPCM compressed audio stream into linear 16-bit PCM samples.
#[allow(clippy::too_many_arguments)] // mirrors C# `AdpcmHelper.Decode` and backs `ryu_adpcm_decode`
pub fn decode_adpcm(
    output: &mut [i16],
    input: &[u8],
    start_sample_offset: i32,
    end_sample_offset: i32,
    offset: i32,
    count: i32,
    coefficients: &[i16],
    loop_context: &mut AdpcmLoopContext,
) -> i32 {
    if input.is_empty() || end_sample_offset < start_sample_offset || count <= 0 {
        return 0;
    }

    let mut pred_scale = loop_context.pred_scale as u8;
    let mut scale = pred_scale & 0x0F;
    let mut coefficient_index = (pred_scale >> 4) & 0x0F;
    let mut history0 = loop_context.history0;
    let mut history1 = loop_context.history1;

    let mut coeff0 = get_coefficient(coefficients, (coefficient_index as usize) * 2);
    let mut coeff1 = get_coefficient(coefficients, (coefficient_index as usize) * 2 + 1);

    let max_avail = (end_sample_offset - start_sample_offset - offset).max(0);
    let decoded_count = (count as usize).min(max_avail as usize).min(output.len());
    let mut nibbles = get_nibbles_from_sample_count((offset + start_sample_offset).max(0) as usize);
    let mut remaining = decoded_count;
    let mut output_idx = 0;

    let start_byte = nibbles / 2;
    if start_byte >= input.len() {
        return 0;
    }
    let target_input = &input[start_byte..];
    let mut input_idx = 0;

    while remaining > 0 && input_idx < target_input.len() {
        let samples_count = if nibbles.is_multiple_of(NIBBLES_PER_FRAME) {
            pred_scale = target_input[input_idx];
            input_idx += 1;

            scale = pred_scale & 0x0F;
            coefficient_index = (pred_scale >> 4) & 0x0F;

            coeff0 = get_coefficient(coefficients, (coefficient_index as usize) * 2);
            coeff1 = get_coefficient(coefficients, (coefficient_index as usize) * 2 + 1);

            nibbles += 2;
            remaining.min(SAMPLES_PER_FRAME)
        } else {
            1
        };

        let scale_fixed = (1 << FIXED_POINT_PRECISION) << scale;

        if samples_count < SAMPLES_PER_FRAME {
            for _ in 0..samples_count {
                if input_idx >= target_input.len() || output_idx >= output.len() {
                    break;
                }

                let value = target_input[input_idx];
                let sample_val = if (nibbles & 1) != 0 {
                    input_idx += 1;
                    ((value as i8) << 4) >> 4
                } else {
                    (value as i8) >> 4
                } as i32;

                nibbles += 1;

                let prediction =
                    (coeff0 as i32) * (history0 as i32) + (coeff1 as i32) * (history1 as i32);
                let sample =
                    (sample_val * scale_fixed + prediction + 1024) >> FIXED_POINT_PRECISION;
                let sat = saturate(sample);

                history1 = history0;
                history0 = sat;

                output[output_idx] = sat;
                output_idx += 1;
                remaining -= 1;
            }
        } else {
            // Full 14-sample frame optimized unroll
            let half_frames = SAMPLES_PER_FRAME / 2;
            for _ in 0..half_frames {
                if input_idx >= target_input.len() || output_idx + 1 >= output.len() {
                    break;
                }

                let value = target_input[input_idx];
                input_idx += 1;

                let sample0_val = ((value as i8) >> 4) as i32;
                let sample1_val = (((value as i8) << 4) >> 4) as i32;

                let pred0 =
                    (coeff0 as i32) * (history0 as i32) + (coeff1 as i32) * (history1 as i32);
                let s0 = (sample0_val * scale_fixed + pred0 + 1024) >> FIXED_POINT_PRECISION;
                let sat0 = saturate(s0);

                let pred1 = (coeff0 as i32) * (sat0 as i32) + (coeff1 as i32) * (history0 as i32);
                let s1 = (sample1_val * scale_fixed + pred1 + 1024) >> FIXED_POINT_PRECISION;
                let sat1 = saturate(s1);

                history1 = sat0;
                history0 = sat1;

                output[output_idx] = sat0;
                output[output_idx + 1] = sat1;
                output_idx += 2;
            }

            nibbles += SAMPLES_PER_FRAME;
            remaining = remaining.saturating_sub(SAMPLES_PER_FRAME);
        }
    }

    loop_context.pred_scale = pred_scale as i16;
    loop_context.history0 = history0;
    loop_context.history1 = history1;

    output_idx as i32
}
