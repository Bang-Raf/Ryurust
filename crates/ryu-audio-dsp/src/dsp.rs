#[inline(always)]
fn round_half_to_even(val: f32) -> f32 {
    val.round_ties_even()
}

/// Clears an audio buffer with zeroes.
#[inline]
pub fn clear_buffer(buffer: &mut [f32]) {
    buffer.fill(0.0);
}

/// Copies samples from input buffer to output buffer.
#[inline]
pub fn copy_buffer(output: &mut [f32], input: &[f32]) {
    let n = output.len().min(input.len());
    output[..n].copy_from_slice(&input[..n]);
}

/// Mixes input buffer into output buffer with a constant volume.
/// Matches `MixCommand.ProcessMix(outputBuffer, inputBuffer)`.
#[inline]
pub fn mix(output: &mut [f32], input: &[f32], volume: f32) {
    let count = output.len().min(input.len());
    for i in 0..count {
        output[i] += round_half_to_even(input[i] * volume);
    }
}

/// Mixes input buffer into output buffer with linear volume ramping.
/// Matches `MixRampCommand.ProcessMixRamp(outputBuffer, inputBuffer, sampleCount)`.
/// Returns the last computed mixed sample (for depop state tracking).
#[inline]
pub fn mix_ramp(output: &mut [f32], input: &[f32], volume0: f32, volume1: f32) -> f32 {
    let count = output.len().min(input.len());
    if count == 0 {
        return 0.0;
    }

    let ramp = (volume1 - volume0) / count as f32;
    let mut volume = volume0;
    let mut state = 0.0f32;

    for i in 0..count {
        state = round_half_to_even(input[i] * volume);
        output[i] += state;
        volume += ramp;
    }

    state
}

/// Scales input buffer by volume into output buffer.
/// Matches `VolumeCommand.ProcessVolume(outputBuffer, inputBuffer)`.
#[inline]
pub fn volume(output: &mut [f32], input: &[f32], volume: f32) {
    let count = output.len().min(input.len());
    for i in 0..count {
        output[i] = round_half_to_even(input[i] * volume);
    }
}

/// Scales input buffer with linear volume ramping into output buffer.
/// Matches `VolumeRampCommand.ProcessVolumeRamp(outputBuffer, inputBuffer, sampleCount)`.
#[inline]
pub fn volume_ramp(output: &mut [f32], input: &[f32], volume0: f32, volume1: f32) {
    let count = output.len().min(input.len());
    if count == 0 {
        return;
    }

    let ramp = (volume1 - volume0) / count as f32;
    let mut volume = volume0;

    for i in 0..count {
        output[i] = round_half_to_even(input[i] * volume);
        volume += ramp;
    }
}
