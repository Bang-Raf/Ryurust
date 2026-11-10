/// Deinterlaces video fields using Weave (combines alternating fields from previous frame).
/// Matches `Scaler.DeinterlaceWeave(data, prevData, width, fieldSize, isTopField)`.
pub fn deinterlace_weave(
    data: &mut [u8],
    prev_data: &[u8],
    width: usize,
    field_size: usize,
    is_top_field: bool,
) {
    if field_size == 0 || width == 0 || width > field_size {
        return;
    }

    let step = field_size * 2;
    let mut offset = 0;

    if is_top_field {
        while offset + field_size + width <= data.len() {
            let ref_off = offset >> 1;
            if ref_off + width <= prev_data.len() {
                data[offset + field_size..offset + field_size + width]
                    .copy_from_slice(&prev_data[ref_off..ref_off + width]);
            }
            offset += step;
        }
    } else {
        while offset + width <= data.len() {
            let ref_off = offset >> 1;
            if ref_off + width <= prev_data.len() {
                data[offset..offset + width].copy_from_slice(&prev_data[ref_off..ref_off + width]);
            }
            offset += step;
        }
    }
}

/// Deinterlaces video fields using Bob (line duplication from adjacent field).
/// Matches `Scaler.DeinterlaceBob(data, width, fieldSize, isTopField)`.
pub fn deinterlace_bob(data: &mut [u8], width: usize, field_size: usize, is_top_field: bool) {
    if field_size == 0 || width == 0 || width > field_size {
        return;
    }

    let step = field_size * 2;
    let mut offset = 0;

    if is_top_field {
        while offset + field_size + width <= data.len() {
            let (src_slice, dst_slice) = data.split_at_mut(offset + field_size);
            dst_slice[..width].copy_from_slice(&src_slice[offset..offset + width]);
            offset += step;
        }
    } else {
        while offset + field_size + width <= data.len() {
            let (dst_slice, src_slice) = data.split_at_mut(offset + field_size);
            dst_slice[offset..offset + width].copy_from_slice(&src_slice[..width]);
            offset += step;
        }
    }
}

/// Deinterlaces video fields using motion adaptive detection (Bob on motion, Weave on static).
/// Matches `Scaler.DeinterlaceMotionAdaptive(data, prevData, nextData, width, fieldSize, isTopField)`.
pub fn deinterlace_motion_adaptive(
    data: &mut [u8],
    prev_data: &[u8],
    next_data: &[u8],
    width: usize,
    field_size: usize,
    is_top_field: bool,
) {
    if field_size == 0 || width == 0 || width > field_size {
        return;
    }

    let start = if is_top_field { field_size } else { 0 };
    let other_field_offset = if is_top_field {
        -(field_size as isize)
    } else {
        field_size as isize
    };

    let step = field_size * 2;
    let mut offset = start;

    let data_len = data.len();
    let prev_len = prev_data.len();
    let next_len = next_data.len();

    let p_data = data.as_mut_ptr();
    let p_prev = prev_data.as_ptr();
    let p_next = next_data.as_ptr();

    while offset + width <= data_len {
        let other_offset = (offset as isize + other_field_offset) as usize;
        if other_offset + width > data_len {
            offset += step;
            continue;
        }

        let ref_offset = (offset - start) >> 1;
        if ref_offset + width > prev_len || ref_offset + width > next_len {
            offset += step;
            continue;
        }

        unsafe {
            let curr_row = p_data.add(offset);
            let other_row = p_data.add(other_offset);
            let prev_row = p_prev.add(ref_offset);
            let next_row = p_next.add(ref_offset);

            let mut x = 0usize;

            // 16-byte chunk loop for vector optimization
            while x + 16 <= width {
                for i in 0..16 {
                    let px_idx = x + i;
                    let p = *prev_row.add(px_idx);
                    let n = *next_row.add(px_idx);
                    if n != p {
                        *curr_row.add(px_idx) = *other_row.add(px_idx);
                    } else {
                        *curr_row.add(px_idx) = p;
                    }
                }
                x += 16;
            }

            // Scalar remainder
            while x < width {
                let p = *prev_row.add(x);
                let n = *next_row.add(x);
                if n != p {
                    *curr_row.add(x) = *other_row.add(x);
                } else {
                    *curr_row.add(x) = p;
                }
                x += 1;
            }
        }

        offset += step;
    }
}
