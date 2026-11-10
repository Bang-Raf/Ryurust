//! High-throughput Tegra Block-Linear swizzle and deswizzle implementations.

use crate::constants::{align_down, align_up, GOB_HEIGHT, GOB_STRIDE};
use crate::layout::BlockLinearLayout;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SwizzleError {
    InvalidDimensions,
    InvalidBytesPerPixel,
    BufferTooSmall,
    InvalidGobBlocks,
}

#[inline(always)]
unsafe fn copy_16(dst: *mut u8, src: *const u8) {
    std::ptr::copy_nonoverlapping(src, dst, 16);
}

#[inline(always)]
unsafe fn copy_bytes(dst: *mut u8, src: *const u8, count: usize) {
    std::ptr::copy_nonoverlapping(src, dst, count);
}

/// Computes the required block-linear buffer size for given dimensions.
pub fn query_block_linear_size(
    width: usize,
    height: usize,
    depth: usize,
    bytes_per_pixel: usize,
    gob_blocks_in_y: usize,
    gob_blocks_in_z: usize,
) -> Result<usize, SwizzleError> {
    if bytes_per_pixel == 0 || bytes_per_pixel > 16 {
        return Err(SwizzleError::InvalidBytesPerPixel);
    }
    if gob_blocks_in_y == 0 || !gob_blocks_in_y.is_power_of_two() {
        return Err(SwizzleError::InvalidGobBlocks);
    }
    if gob_blocks_in_z == 0 || !gob_blocks_in_z.is_power_of_two() {
        return Err(SwizzleError::InvalidGobBlocks);
    }

    let alignment = GOB_STRIDE / bytes_per_pixel;
    let w_aligned = align_up(width, alignment);
    let layout = BlockLinearLayout::new(
        w_aligned,
        height,
        gob_blocks_in_y,
        gob_blocks_in_z,
        bytes_per_pixel,
    );
    Ok(layout.slice_size * depth)
}

/// Converts Tegra Block-Linear surface to standard linear pitch layout (Sequential).
pub fn convert_block_linear_to_linear(
    dst: &mut [u8],
    src: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bytes_per_pixel: usize,
    gob_blocks_in_y: usize,
) -> Result<(), SwizzleError> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    if bytes_per_pixel == 0 || bytes_per_pixel > 16 {
        return Err(SwizzleError::InvalidBytesPerPixel);
    }
    if gob_blocks_in_y == 0 || !gob_blocks_in_y.is_power_of_two() {
        return Err(SwizzleError::InvalidGobBlocks);
    }

    let alignment = GOB_STRIDE / bytes_per_pixel;
    let w_aligned = align_up(width, alignment);
    let layout = BlockLinearLayout::new(w_aligned, height, gob_blocks_in_y, 1, bytes_per_pixel);

    let min_src_len = layout.slice_size;
    let min_dst_len = (height - 1) * stride + width * bytes_per_pixel;

    if src.len() < min_src_len || dst.len() < min_dst_len {
        return Err(SwizzleError::BufferTooSmall);
    }

    let stride_trunc = align_down(width * bytes_per_pixel, 16);
    let stride_trunc64 = align_down(width * bytes_per_pixel, 64);
    let x_start = stride_trunc / bytes_per_pixel;

    let src_ptr = src.as_ptr();
    let dst_ptr = dst.as_mut_ptr();

    for y in 0..height {
        let yz_part = layout.compute_yz_part(y, 0);
        let mut out_ptr = unsafe { dst_ptr.add(y * stride) };

        // 64-byte chunks (4x 16-byte SIMD blocks)
        let mut x = 0;
        while x < stride_trunc64 {
            let base_offset = layout.get_offset_64(x, yz_part);
            unsafe {
                let off0 = src_ptr.add(base_offset);
                let off1 = src_ptr.add(base_offset + 0x20);
                let off2 = src_ptr.add(base_offset + 0x100);
                let off3 = src_ptr.add(base_offset + 0x120);

                copy_16(out_ptr, off0);
                copy_16(out_ptr.add(16), off1);
                copy_16(out_ptr.add(32), off2);
                copy_16(out_ptr.add(48), off3);

                out_ptr = out_ptr.add(64);
            }
            x += 64;
        }

        // 16-byte chunks
        while x < stride_trunc {
            let offset = layout.get_offset_16(x, yz_part);
            unsafe {
                copy_16(out_ptr, src_ptr.add(offset));
                out_ptr = out_ptr.add(16);
            }
            x += 16;
        }

        // Remaining individual pixels
        for px in x_start..width {
            let offset = layout.get_offset_pixel(px, yz_part);
            unsafe {
                copy_bytes(out_ptr, src_ptr.add(offset), bytes_per_pixel);
                out_ptr = out_ptr.add(bytes_per_pixel);
            }
        }
    }

    Ok(())
}

/// Converts linear pitch surface to Tegra Block-Linear layout (Sequential).
pub fn convert_linear_to_block_linear(
    dst: &mut [u8],
    src: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bytes_per_pixel: usize,
    gob_blocks_in_y: usize,
) -> Result<(), SwizzleError> {
    if width == 0 || height == 0 {
        return Ok(());
    }
    if bytes_per_pixel == 0 || bytes_per_pixel > 16 {
        return Err(SwizzleError::InvalidBytesPerPixel);
    }
    if gob_blocks_in_y == 0 || !gob_blocks_in_y.is_power_of_two() {
        return Err(SwizzleError::InvalidGobBlocks);
    }

    let alignment = GOB_STRIDE / bytes_per_pixel;
    let w_aligned = align_up(width, alignment);
    let layout = BlockLinearLayout::new(w_aligned, height, gob_blocks_in_y, 1, bytes_per_pixel);

    let min_dst_len = layout.slice_size;
    let min_src_len = (height - 1) * stride + width * bytes_per_pixel;

    if dst.len() < min_dst_len || src.len() < min_src_len {
        return Err(SwizzleError::BufferTooSmall);
    }

    let stride_trunc = align_down(width * bytes_per_pixel, 16);
    let stride_trunc64 = align_down(width * bytes_per_pixel, 64);
    let x_start = stride_trunc / bytes_per_pixel;

    let src_ptr = src.as_ptr();
    let dst_ptr = dst.as_mut_ptr();

    for y in 0..height {
        let yz_part = layout.compute_yz_part(y, 0);
        let mut in_ptr = unsafe { src_ptr.add(y * stride) };

        // 64-byte chunks
        let mut x = 0;
        while x < stride_trunc64 {
            let base_offset = layout.get_offset_64(x, yz_part);
            unsafe {
                let off0 = dst_ptr.add(base_offset);
                let off1 = dst_ptr.add(base_offset + 0x20);
                let off2 = dst_ptr.add(base_offset + 0x100);
                let off3 = dst_ptr.add(base_offset + 0x120);

                copy_16(off0, in_ptr);
                copy_16(off1, in_ptr.add(16));
                copy_16(off2, in_ptr.add(32));
                copy_16(off3, in_ptr.add(48));

                in_ptr = in_ptr.add(64);
            }
            x += 64;
        }

        // 16-byte chunks
        while x < stride_trunc {
            let offset = layout.get_offset_16(x, yz_part);
            unsafe {
                copy_16(dst_ptr.add(offset), in_ptr);
                in_ptr = in_ptr.add(16);
            }
            x += 16;
        }

        // Remaining pixels
        for px in x_start..width {
            let offset = layout.get_offset_pixel(px, yz_part);
            unsafe {
                copy_bytes(dst_ptr.add(offset), in_ptr, bytes_per_pixel);
                in_ptr = in_ptr.add(bytes_per_pixel);
            }
        }
    }

    Ok(())
}

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Converts Tegra Block-Linear surface to linear pitch layout using Rayon multi-threading.
pub fn convert_block_linear_to_linear_parallel(
    dst: &mut [u8],
    src: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bytes_per_pixel: usize,
    gob_blocks_in_y: usize,
) -> Result<(), SwizzleError> {
    #[cfg(feature = "parallel")]
    {
        // For small images (<128 rows), thread dispatch overhead outweighs gain
        if height < 128 {
            return convert_block_linear_to_linear(
                dst,
                src,
                width,
                height,
                stride,
                bytes_per_pixel,
                gob_blocks_in_y,
            );
        }

        if width == 0 || height == 0 {
            return Ok(());
        }
        if bytes_per_pixel == 0 || bytes_per_pixel > 16 {
            return Err(SwizzleError::InvalidBytesPerPixel);
        }
        if gob_blocks_in_y == 0 || !gob_blocks_in_y.is_power_of_two() {
            return Err(SwizzleError::InvalidGobBlocks);
        }

        let alignment = GOB_STRIDE / bytes_per_pixel;
        let w_aligned = align_up(width, alignment);
        let layout = BlockLinearLayout::new(w_aligned, height, gob_blocks_in_y, 1, bytes_per_pixel);

        let min_src_len = layout.slice_size;
        let min_dst_len = (height - 1) * stride + width * bytes_per_pixel;

        if src.len() < min_src_len || dst.len() < min_dst_len {
            return Err(SwizzleError::BufferTooSmall);
        }

        let stride_trunc = align_down(width * bytes_per_pixel, 16);
        let stride_trunc64 = align_down(width * bytes_per_pixel, 64);
        let x_start = stride_trunc / bytes_per_pixel;

        let src_ptr_val = src.as_ptr() as usize;
        let dst_ptr_val = dst.as_mut_ptr() as usize;

        // Parallelize across rows: each row y writes to completely non-overlapping memory dst[y * stride ..]
        (0..height).into_par_iter().for_each(|y| {
            let src_ptr = src_ptr_val as *const u8;
            let dst_ptr = dst_ptr_val as *mut u8;

            let yz_part = layout.compute_yz_part(y, 0);
            let mut out_ptr = unsafe { dst_ptr.add(y * stride) };

            let mut x = 0;
            while x < stride_trunc64 {
                let base_offset = layout.get_offset_64(x, yz_part);
                unsafe {
                    let off0 = src_ptr.add(base_offset);
                    let off1 = src_ptr.add(base_offset + 0x20);
                    let off2 = src_ptr.add(base_offset + 0x100);
                    let off3 = src_ptr.add(base_offset + 0x120);

                    copy_16(out_ptr, off0);
                    copy_16(out_ptr.add(16), off1);
                    copy_16(out_ptr.add(32), off2);
                    copy_16(out_ptr.add(48), off3);

                    out_ptr = out_ptr.add(64);
                }
                x += 64;
            }

            while x < stride_trunc {
                let offset = layout.get_offset_16(x, yz_part);
                unsafe {
                    copy_16(out_ptr, src_ptr.add(offset));
                    out_ptr = out_ptr.add(16);
                }
                x += 16;
            }

            for px in x_start..width {
                let offset = layout.get_offset_pixel(px, yz_part);
                unsafe {
                    copy_bytes(out_ptr, src_ptr.add(offset), bytes_per_pixel);
                    out_ptr = out_ptr.add(bytes_per_pixel);
                }
            }
        });

        Ok(())
    }

    #[cfg(not(feature = "parallel"))]
    {
        convert_block_linear_to_linear(
            dst,
            src,
            width,
            height,
            stride,
            bytes_per_pixel,
            gob_blocks_in_y,
        )
    }
}

/// Converts linear pitch surface to Tegra Block-Linear layout using Rayon multi-threading.
pub fn convert_linear_to_block_linear_parallel(
    dst: &mut [u8],
    src: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    bytes_per_pixel: usize,
    gob_blocks_in_y: usize,
) -> Result<(), SwizzleError> {
    #[cfg(feature = "parallel")]
    {
        let rob_height = gob_blocks_in_y * GOB_HEIGHT;
        if height < rob_height * 2 {
            return convert_linear_to_block_linear(
                dst,
                src,
                width,
                height,
                stride,
                bytes_per_pixel,
                gob_blocks_in_y,
            );
        }

        if width == 0 || height == 0 {
            return Ok(());
        }
        if bytes_per_pixel == 0 || bytes_per_pixel > 16 {
            return Err(SwizzleError::InvalidBytesPerPixel);
        }
        if gob_blocks_in_y == 0 || !gob_blocks_in_y.is_power_of_two() {
            return Err(SwizzleError::InvalidGobBlocks);
        }

        let alignment = GOB_STRIDE / bytes_per_pixel;
        let w_aligned = align_up(width, alignment);
        let layout = BlockLinearLayout::new(w_aligned, height, gob_blocks_in_y, 1, bytes_per_pixel);

        let min_dst_len = layout.slice_size;
        let min_src_len = (height - 1) * stride + width * bytes_per_pixel;

        if dst.len() < min_dst_len || src.len() < min_src_len {
            return Err(SwizzleError::BufferTooSmall);
        }

        let stride_trunc = align_down(width * bytes_per_pixel, 16);
        let stride_trunc64 = align_down(width * bytes_per_pixel, 64);
        let x_start = stride_trunc / bytes_per_pixel;

        let src_ptr_val = src.as_ptr() as usize;
        let dst_ptr_val = dst.as_mut_ptr() as usize;

        // Parallelize across ROB chunks (multiples of rob_height = gob_blocks_in_y * 8)
        // Each ROB chunk writes to completely disjoint slices of the block-linear output!
        let num_robs = (height + rob_height - 1) / rob_height;

        (0..num_robs).into_par_iter().for_each(|rob_idx| {
            let y_start = rob_idx * rob_height;
            let y_end = (y_start + rob_height).min(height);

            let src_ptr = src_ptr_val as *const u8;
            let dst_ptr = dst_ptr_val as *mut u8;

            for y in y_start..y_end {
                let yz_part = layout.compute_yz_part(y, 0);
                let mut in_ptr = unsafe { src_ptr.add(y * stride) };

                let mut x = 0;
                while x < stride_trunc64 {
                    let base_offset = layout.get_offset_64(x, yz_part);
                    unsafe {
                        let off0 = dst_ptr.add(base_offset);
                        let off1 = dst_ptr.add(base_offset + 0x20);
                        let off2 = dst_ptr.add(base_offset + 0x100);
                        let off3 = dst_ptr.add(base_offset + 0x120);

                        copy_16(off0, in_ptr);
                        copy_16(off1, in_ptr.add(16));
                        copy_16(off2, in_ptr.add(32));
                        copy_16(off3, in_ptr.add(48));

                        in_ptr = in_ptr.add(64);
                    }
                    x += 64;
                }

                while x < stride_trunc {
                    let offset = layout.get_offset_16(x, yz_part);
                    unsafe {
                        copy_16(dst_ptr.add(offset), in_ptr);
                        in_ptr = in_ptr.add(16);
                    }
                    x += 16;
                }

                for px in x_start..width {
                    let offset = layout.get_offset_pixel(px, yz_part);
                    unsafe {
                        copy_bytes(dst_ptr.add(offset), in_ptr, bytes_per_pixel);
                        in_ptr = in_ptr.add(bytes_per_pixel);
                    }
                }
            }
        });

        Ok(())
    }

    #[cfg(not(feature = "parallel"))]
    {
        convert_linear_to_block_linear(
            dst,
            src,
            width,
            height,
            stride,
            bytes_per_pixel,
            gob_blocks_in_y,
        )
    }
}
