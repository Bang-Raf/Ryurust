#[inline(always)]
fn round_half_to_even(val: f32) -> f32 {
    val.round_ties_even()
}

/// Per-sample access used by the kernels so that one implementation serves both the
/// separate-buffer form (`output`, `input`) and the in-place form (one `&mut [f32]` that is
/// both input and output, as when a renderer command has `InputBufferIndex ==
/// OutputBufferIndex`).
///
/// Every kernel reads `input(i)` before it writes `output(i)` and never touches another
/// index in between, so the in-place form gives the same result as the C# code running
/// with aliased `Span`s.
pub(crate) trait SampleIo {
    fn len(&self) -> usize;
    fn input(&self, i: usize) -> f32;
    fn output(&mut self, i: usize) -> &mut f32;
}

/// Distinct output and input buffers; length is the shorter of the two.
pub(crate) struct Split<'a> {
    output: &'a mut [f32],
    input: &'a [f32],
}

impl<'a> Split<'a> {
    #[inline(always)]
    pub(crate) fn new(output: &'a mut [f32], input: &'a [f32]) -> Self {
        let n = output.len().min(input.len());
        Self {
            output: &mut output[..n],
            input: &input[..n],
        }
    }
}

impl SampleIo for Split<'_> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.output.len()
    }
    #[inline(always)]
    fn input(&self, i: usize) -> f32 {
        self.input[i]
    }
    #[inline(always)]
    fn output(&mut self, i: usize) -> &mut f32 {
        &mut self.output[i]
    }
}

/// One buffer that is both input and output.
pub(crate) struct InPlace<'a>(pub(crate) &'a mut [f32]);

impl SampleIo for InPlace<'_> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.0.len()
    }
    #[inline(always)]
    fn input(&self, i: usize) -> f32 {
        self.0[i]
    }
    #[inline(always)]
    fn output(&mut self, i: usize) -> &mut f32 {
        &mut self.0[i]
    }
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
/// Matches `MixCommand.ProcessMix(outputBuffer, inputBuffer)` (scalar `MultiplyRoundUp`,
/// i.e. round half to even).
#[inline]
pub fn mix(output: &mut [f32], input: &[f32], volume: f32) {
    mix_io(&mut Split::new(output, input), volume);
}

/// [`mix`] with `buffer` as both input and output (`buffer[i] += round(buffer[i] * volume)`).
#[inline]
pub fn mix_in_place(buffer: &mut [f32], volume: f32) {
    mix_io(&mut InPlace(buffer), volume);
}

#[inline(always)]
pub(crate) fn mix_io<I: SampleIo>(io: &mut I, volume: f32) {
    for i in 0..io.len() {
        let v = round_half_to_even(io.input(i) * volume);
        *io.output(i) += v;
    }
}

/// Mixes input buffer into output buffer with linear volume ramping.
/// Matches `MixRampCommand.ProcessMixRamp(outputBuffer, inputBuffer, sampleCount)`.
/// Returns the last computed mixed sample (for depop state tracking).
#[inline]
pub fn mix_ramp(output: &mut [f32], input: &[f32], volume0: f32, volume1: f32) -> f32 {
    mix_ramp_io(&mut Split::new(output, input), volume0, volume1)
}

/// [`mix_ramp`] with `buffer` as both input and output.
#[inline]
pub fn mix_ramp_in_place(buffer: &mut [f32], volume0: f32, volume1: f32) -> f32 {
    mix_ramp_io(&mut InPlace(buffer), volume0, volume1)
}

#[inline(always)]
pub(crate) fn mix_ramp_io<I: SampleIo>(io: &mut I, volume0: f32, volume1: f32) -> f32 {
    let count = io.len();
    if count == 0 {
        return 0.0;
    }

    let ramp = (volume1 - volume0) / count as f32;
    let mut volume = volume0;
    let mut state = 0.0f32;

    for i in 0..count {
        state = round_half_to_even(io.input(i) * volume);
        *io.output(i) += state;
        volume += ramp;
    }

    state
}

/// Scales input buffer by volume into output buffer.
/// Matches `VolumeCommand.ProcessVolume(outputBuffer, inputBuffer)`.
#[inline]
pub fn volume(output: &mut [f32], input: &[f32], volume: f32) {
    volume_io(&mut Split::new(output, input), volume);
}

/// [`volume`] with `buffer` as both input and output.
#[inline]
pub fn volume_in_place(buffer: &mut [f32], volume: f32) {
    volume_io(&mut InPlace(buffer), volume);
}

#[inline(always)]
pub(crate) fn volume_io<I: SampleIo>(io: &mut I, volume: f32) {
    for i in 0..io.len() {
        *io.output(i) = round_half_to_even(io.input(i) * volume);
    }
}

/// Scales input buffer with linear volume ramping into output buffer.
/// Matches `VolumeRampCommand.ProcessVolumeRamp(outputBuffer, inputBuffer, sampleCount)`.
#[inline]
pub fn volume_ramp(output: &mut [f32], input: &[f32], volume0: f32, volume1: f32) {
    volume_ramp_io(&mut Split::new(output, input), volume0, volume1);
}

/// [`volume_ramp`] with `buffer` as both input and output.
#[inline]
pub fn volume_ramp_in_place(buffer: &mut [f32], volume0: f32, volume1: f32) {
    volume_ramp_io(&mut InPlace(buffer), volume0, volume1);
}

#[inline(always)]
pub(crate) fn volume_ramp_io<I: SampleIo>(io: &mut I, volume0: f32, volume1: f32) {
    let count = io.len();
    if count == 0 {
        return;
    }

    let ramp = (volume1 - volume0) / count as f32;
    let mut volume = volume0;

    for i in 0..count {
        *io.output(i) = round_half_to_even(io.input(i) * volume);
        volume += ramp;
    }
}
