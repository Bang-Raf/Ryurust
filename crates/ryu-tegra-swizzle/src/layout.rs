//! Tegra Block-Linear layout offset calculation and ROB/Slice geometry.

use crate::constants::{div_round_up, GOB_HEIGHT, GOB_SIZE, GOB_STRIDE};

#[derive(Debug, Clone, Copy)]
pub struct BlockLinearLayout {
    pub tex_bpp: usize,
    pub bpp_shift: u32,
    pub bh_mask: usize,
    pub bd_mask: usize,
    pub bh_shift: u32,
    pub bd_shift: u32,
    pub x_shift: u32,
    pub rob_size: usize,
    pub slice_size: usize,
}

impl BlockLinearLayout {
    pub fn new(
        width: usize,
        height: usize,
        gob_blocks_in_y: usize,
        gob_blocks_in_z: usize,
        bpp: usize,
    ) -> Self {
        let bpp_shift = bpp.trailing_zeros();
        let bh_mask = gob_blocks_in_y.saturating_sub(1);
        let bd_mask = gob_blocks_in_z.saturating_sub(1);
        let bh_shift = gob_blocks_in_y.trailing_zeros();
        let bd_shift = gob_blocks_in_z.trailing_zeros();

        let block_volume = GOB_SIZE * gob_blocks_in_y * gob_blocks_in_z;
        let x_shift = block_volume.trailing_zeros();

        let width_in_gobs = div_round_up(width * bpp, GOB_STRIDE);
        let rob_size = block_volume * width_in_gobs;
        let slice_size = div_round_up(height, gob_blocks_in_y * GOB_HEIGHT) * rob_size;

        Self {
            tex_bpp: bpp,
            bpp_shift,
            bh_mask,
            bd_mask,
            bh_shift,
            bd_shift,
            x_shift,
            rob_size,
            slice_size,
        }
    }

    #[inline(always)]
    pub fn compute_yz_part(&self, y: usize, z: usize) -> usize {
        let yh = y / GOB_HEIGHT;
        let mut offset =
            (z >> self.bd_shift) * self.slice_size + (yh >> self.bh_shift) * self.rob_size;
        offset += (yh & self.bh_mask) * GOB_SIZE;
        offset += ((z & self.bd_mask) * GOB_SIZE) << self.bh_shift;
        offset += ((y & 0x07) >> 1) << 6;
        offset += (y & 0x01) << 4;
        offset
    }

    #[inline(always)]
    pub fn get_offset_64(&self, x: usize, yz_part: usize) -> usize {
        ((x / GOB_STRIDE) << self.x_shift) + yz_part
    }

    #[inline(always)]
    pub fn get_offset_16(&self, x: usize, yz_part: usize) -> usize {
        let x_part = ((x / GOB_STRIDE) << self.x_shift)
            + (((x & 0x3f) >> 5) << 8)
            + (((x & 0x1f) >> 4) << 5);
        x_part + yz_part
    }

    #[inline(always)]
    pub fn get_offset_pixel(&self, x_pixel: usize, yz_part: usize) -> usize {
        let x = if self.tex_bpp.is_power_of_two() {
            x_pixel << self.bpp_shift
        } else {
            x_pixel * self.tex_bpp
        };

        let x_part = ((x / GOB_STRIDE) << self.x_shift)
            + (((x & 0x3f) >> 5) << 8)
            + (((x & 0x1f) >> 4) << 5)
            + (x & 0x0f);
        x_part + yz_part
    }

    #[inline(always)]
    pub fn get_offset_with_line_offset(&self, x: usize, y: usize, z: usize) -> usize {
        let yz_part = self.compute_yz_part(y, z);
        let x_part = ((x / GOB_STRIDE) << self.x_shift)
            + (((x & 0x3f) >> 5) << 8)
            + (((x & 0x1f) >> 4) << 5)
            + (x & 0x0f);
        x_part + yz_part
    }
}
