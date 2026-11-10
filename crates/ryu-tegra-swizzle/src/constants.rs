//! Constants for Nvidia Tegra X1 Block-Linear texture memory layout.

pub const GOB_STRIDE: usize = 64;
pub const GOB_HEIGHT: usize = 8;
pub const GOB_SIZE: usize = GOB_STRIDE * GOB_HEIGHT; // 512 bytes

pub const HOST_STRIDE_ALIGNMENT: usize = 4;

#[inline(always)]
pub const fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

#[inline(always)]
pub const fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

#[inline(always)]
pub const fn div_round_up(value: usize, divisor: usize) -> usize {
    (value + divisor - 1) / divisor
}
