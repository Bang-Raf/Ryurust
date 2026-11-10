// VP9 In-Loop Deblocking Filter
// Fixed-point exact edge deblocking filters matching libvpx / VP9 specification

use crate::idct::round_power_of_two;

#[inline(always)]
pub fn clamp_sbyte(t: i32) -> i8 {
    if t > 127 {
        127
    } else if t < -128 {
        -128
    } else {
        t as i8
    }
}

#[inline(always)]
pub fn filter_mask(
    limit: u8,
    blimit: u8,
    p3: u8,
    p2: u8,
    p1: u8,
    p0: u8,
    q0: u8,
    q1: u8,
    q2: u8,
    q3: u8,
) -> i8 {
    let mut mask = 0i32;
    mask |= if (p3 as i32 - p2 as i32).abs() > limit as i32 {
        -1
    } else {
        0
    };
    mask |= if (p2 as i32 - p1 as i32).abs() > limit as i32 {
        -1
    } else {
        0
    };
    mask |= if (p1 as i32 - p0 as i32).abs() > limit as i32 {
        -1
    } else {
        0
    };
    mask |= if (q1 as i32 - q0 as i32).abs() > limit as i32 {
        -1
    } else {
        0
    };
    mask |= if (q2 as i32 - q1 as i32).abs() > limit as i32 {
        -1
    } else {
        0
    };
    mask |= if (q3 as i32 - q2 as i32).abs() > limit as i32 {
        -1
    } else {
        0
    };
    mask |= if ((p0 as i32 - q0 as i32).abs() * 2) + ((p1 as i32 - q1 as i32).abs() / 2)
        > blimit as i32
    {
        -1
    } else {
        0
    };
    (!mask) as i8
}

#[inline(always)]
pub fn flat_mask4(
    thresh: u8,
    p3: u8,
    p2: u8,
    p1: u8,
    p0: u8,
    q0: u8,
    q1: u8,
    q2: u8,
    q3: u8,
) -> i8 {
    let mut mask = 0i32;
    mask |= if (p1 as i32 - p0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    mask |= if (q1 as i32 - q0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    mask |= if (p2 as i32 - p0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    mask |= if (q2 as i32 - q0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    mask |= if (p3 as i32 - p0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    mask |= if (q3 as i32 - q0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    (!mask) as i8
}

#[inline(always)]
pub fn flat_mask5(
    thresh: u8,
    p4: u8,
    p3: u8,
    p2: u8,
    p1: u8,
    p0: u8,
    q0: u8,
    q1: u8,
    q2: u8,
    q3: u8,
    q4: u8,
) -> i8 {
    let mut mask = !flat_mask4(thresh, p3, p2, p1, p0, q0, q1, q2, q3) as i32;
    mask |= if (p4 as i32 - p0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    mask |= if (q4 as i32 - q0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    (!mask) as i8
}

#[inline(always)]
pub fn hev_mask(thresh: u8, p1: u8, p0: u8, q0: u8, q1: u8) -> i8 {
    let mut hev = 0i32;
    hev |= if (p1 as i32 - p0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    hev |= if (q1 as i32 - q0 as i32).abs() > thresh as i32 {
        -1
    } else {
        0
    };
    hev as i8
}

#[inline(always)]
pub fn filter4(mask: i8, thresh: u8, op1: &mut u8, op0: &mut u8, oq0: &mut u8, oq1: &mut u8) {
    let ps1 = (*op1 ^ 0x80) as i8;
    let ps0 = (*op0 ^ 0x80) as i8;
    let qs0 = (*oq0 ^ 0x80) as i8;
    let qs1 = (*oq1 ^ 0x80) as i8;
    let hev = hev_mask(thresh, *op1, *op0, *oq0, *oq1);

    // Outer taps if high edge variance
    let filter = clamp_sbyte(ps1 as i32 - qs1 as i32) & hev;

    // Inner taps
    let filter = clamp_sbyte(filter as i32 + (3 * (qs0 as i32 - ps0 as i32))) & mask;

    let filter1 = clamp_sbyte(filter as i32 + 4) >> 3;
    let filter2 = clamp_sbyte(filter as i32 + 3) >> 3;

    *oq0 = (clamp_sbyte(qs0 as i32 - filter1 as i32) as u8) ^ 0x80;
    *op0 = (clamp_sbyte(ps0 as i32 + filter2 as i32) as u8) ^ 0x80;
}

#[inline(always)]
pub fn filter8(
    mask: i8,
    thresh: u8,
    flat: bool,
    op3: &mut u8,
    op2: &mut u8,
    op1: &mut u8,
    op0: &mut u8,
    oq0: &mut u8,
    oq1: &mut u8,
    oq2: &mut u8,
    oq3: &mut u8,
) {
    if flat && mask != 0 {
        let p3 = *op3 as i64;
        let p2 = *op2 as i64;
        let p1 = *op1 as i64;
        let p0 = *op0 as i64;
        let q0 = *oq0 as i64;
        let q1 = *oq1 as i64;
        let q2 = *oq2 as i64;
        let q3 = *oq3 as i64;

        // 7-tap filter [1, 1, 1, 2, 1, 1, 1]
        *op2 = round_power_of_two(p3 + p3 + p3 + (2 * p2) + p1 + p0 + q0, 3) as u8;
        *op1 = round_power_of_two(p3 + p3 + p2 + (2 * p1) + p0 + q0 + q1, 3) as u8;
        *op0 = round_power_of_two(p3 + p2 + p1 + (2 * p0) + q0 + q1 + q2, 3) as u8;
        *oq0 = round_power_of_two(p2 + p1 + p0 + (2 * q0) + q1 + q2 + q3, 3) as u8;
        *oq1 = round_power_of_two(p1 + p0 + q0 + (2 * q1) + q2 + q3 + q3, 3) as u8;
        *oq2 = round_power_of_two(p0 + q0 + q1 + (2 * q2) + q3 + q3 + q3, 3) as u8;
    } else {
        filter4(mask, thresh, op1, op0, oq0, oq1);
    }
}

#[inline(always)]
pub fn filter16(
    mask: i8,
    thresh: u8,
    flat: bool,
    flat2: bool,
    op7: &mut u8,
    op6: &mut u8,
    op5: &mut u8,
    op4: &mut u8,
    op3: &mut u8,
    op2: &mut u8,
    op1: &mut u8,
    op0: &mut u8,
    oq0: &mut u8,
    oq1: &mut u8,
    oq2: &mut u8,
    oq3: &mut u8,
    oq4: &mut u8,
    oq5: &mut u8,
    oq6: &mut u8,
    oq7: &mut u8,
) {
    if flat2 && flat && mask != 0 {
        let p7 = *op7 as i64;
        let p6 = *op6 as i64;
        let p5 = *op5 as i64;
        let p4 = *op4 as i64;
        let p3 = *op3 as i64;
        let p2 = *op2 as i64;
        let p1 = *op1 as i64;
        let p0 = *op0 as i64;
        let q0 = *oq0 as i64;
        let q1 = *oq1 as i64;
        let q2 = *oq2 as i64;
        let q3 = *oq3 as i64;
        let q4 = *oq4 as i64;
        let q5 = *oq5 as i64;
        let q6 = *oq6 as i64;
        let q7 = *oq7 as i64;

        // 15-tap filter
        *op6 = round_power_of_two((p7 * 7) + (p6 * 2) + p5 + p4 + p3 + p2 + p1 + p0 + q0, 4) as u8;
        *op5 = round_power_of_two(
            (p7 * 6) + p6 + (p5 * 2) + p4 + p3 + p2 + p1 + p0 + q0 + q1,
            4,
        ) as u8;
        *op4 = round_power_of_two(
            (p7 * 5) + p6 + p5 + (p4 * 2) + p3 + p2 + p1 + p0 + q0 + q1 + q2,
            4,
        ) as u8;
        *op3 = round_power_of_two(
            (p7 * 4) + p6 + p5 + p4 + (p3 * 2) + p2 + p1 + p0 + q0 + q1 + q2 + q3,
            4,
        ) as u8;
        *op2 = round_power_of_two(
            (p7 * 3) + p6 + p5 + p4 + p3 + (p2 * 2) + p1 + p0 + q0 + q1 + q2 + q3 + q4,
            4,
        ) as u8;
        *op1 = round_power_of_two(
            (p7 * 2) + p6 + p5 + p4 + p3 + p2 + (p1 * 2) + p0 + q0 + q1 + q2 + q3 + q4 + q5,
            4,
        ) as u8;
        *op0 = round_power_of_two(
            p7 + p6 + p5 + p4 + p3 + p2 + p1 + (p0 * 2) + q0 + q1 + q2 + q3 + q4 + q5 + q6,
            4,
        ) as u8;
        *oq0 = round_power_of_two(
            p6 + p5 + p4 + p3 + p2 + p1 + p0 + (q0 * 2) + q1 + q2 + q3 + q4 + q5 + q6 + q7,
            4,
        ) as u8;
        *oq1 = round_power_of_two(
            p5 + p4 + p3 + p2 + p1 + p0 + q0 + (q1 * 2) + q2 + q3 + q4 + q5 + q6 + (q7 * 2),
            4,
        ) as u8;
        *oq2 = round_power_of_two(
            p4 + p3 + p2 + p1 + p0 + q0 + q1 + (q2 * 2) + q3 + q4 + q5 + (q7 * 3),
            4,
        ) as u8;
        *oq3 = round_power_of_two(
            p3 + p2 + p1 + p0 + q0 + q1 + q2 + (q3 * 2) + q4 + q5 + (q7 * 4),
            4,
        ) as u8;
        *oq4 = round_power_of_two(
            p2 + p1 + p0 + q0 + q1 + q2 + q3 + (q4 * 2) + q5 + (q7 * 5),
            4,
        ) as u8;
        *oq5 = round_power_of_two(p1 + p0 + q0 + q1 + q2 + q3 + q4 + (q5 * 2) + (q7 * 6), 4) as u8;
        *oq6 = round_power_of_two(p0 + q0 + q1 + q2 + q3 + q4 + q5 + (q6 * 2) + (q7 * 7), 4) as u8;
    } else {
        filter8(mask, thresh, flat, op3, op2, op1, op0, oq0, oq1, oq2, oq3);
    }
}

// -------------------------------------------------------------
// Horizontal and Vertical Edge Deblocking Routines (8 samples wide)
// -------------------------------------------------------------

pub fn lpf_horizontal_4(
    s: &mut [u8],
    offset: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
) {
    if offset < 4 * pitch || offset + 4 * pitch > s.len() {
        return;
    }

    for i in 0..8 {
        let p3 = s[offset - 4 * pitch + i];
        let p2 = s[offset - 3 * pitch + i];
        let p1 = s[offset - 2 * pitch + i];
        let p0 = s[offset - pitch + i];
        let q0 = s[offset + i];
        let q1 = s[offset + pitch + i];
        let q2 = s[offset + 2 * pitch + i];
        let q3 = s[offset + 3 * pitch + i];

        let mask = filter_mask(limit, blimit, p3, p2, p1, p0, q0, q1, q2, q3);

        let mut op1 = p1;
        let mut op0 = p0;
        let mut oq0 = q0;
        let mut oq1 = q1;

        filter4(mask, thresh, &mut op1, &mut op0, &mut oq0, &mut oq1);

        s[offset - 2 * pitch + i] = op1;
        s[offset - pitch + i] = op0;
        s[offset + i] = oq0;
        s[offset + pitch + i] = oq1;
    }
}

pub fn lpf_vertical_4(
    s: &mut [u8],
    offset: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
) {
    if offset < 4 || offset + 4 > s.len() {
        return;
    }

    for i in 0..8 {
        let row = offset + i * pitch;
        if row + 4 > s.len() || row < 4 {
            break;
        }

        let p3 = s[row - 4];
        let p2 = s[row - 3];
        let p1 = s[row - 2];
        let p0 = s[row - 1];
        let q0 = s[row];
        let q1 = s[row + 1];
        let q2 = s[row + 2];
        let q3 = s[row + 3];

        let mask = filter_mask(limit, blimit, p3, p2, p1, p0, q0, q1, q2, q3);

        let mut op1 = p1;
        let mut op0 = p0;
        let mut oq0 = q0;
        let mut oq1 = q1;

        filter4(mask, thresh, &mut op1, &mut op0, &mut oq0, &mut oq1);

        s[row - 2] = op1;
        s[row - 1] = op0;
        s[row] = oq0;
        s[row + 1] = oq1;
    }
}

pub fn lpf_horizontal_8(
    s: &mut [u8],
    offset: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
) {
    if offset < 4 * pitch || offset + 4 * pitch > s.len() {
        return;
    }

    for i in 0..8 {
        let mut p3 = s[offset - 4 * pitch + i];
        let mut p2 = s[offset - 3 * pitch + i];
        let mut p1 = s[offset - 2 * pitch + i];
        let mut p0 = s[offset - pitch + i];
        let mut q0 = s[offset + i];
        let mut q1 = s[offset + pitch + i];
        let mut q2 = s[offset + 2 * pitch + i];
        let mut q3 = s[offset + 3 * pitch + i];

        let mask = filter_mask(limit, blimit, p3, p2, p1, p0, q0, q1, q2, q3);
        let flat = flat_mask4(1, p3, p2, p1, p0, q0, q1, q2, q3);

        filter8(
            mask,
            thresh,
            flat != 0,
            &mut p3,
            &mut p2,
            &mut p1,
            &mut p0,
            &mut q0,
            &mut q1,
            &mut q2,
            &mut q3,
        );

        s[offset - 4 * pitch + i] = p3;
        s[offset - 3 * pitch + i] = p2;
        s[offset - 2 * pitch + i] = p1;
        s[offset - pitch + i] = p0;
        s[offset + i] = q0;
        s[offset + pitch + i] = q1;
        s[offset + 2 * pitch + i] = q2;
        s[offset + 3 * pitch + i] = q3;
    }
}

pub fn lpf_vertical_8(
    s: &mut [u8],
    offset: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
) {
    if offset < 4 || offset + 4 > s.len() {
        return;
    }

    for i in 0..8 {
        let row = offset + i * pitch;
        if row + 4 > s.len() || row < 4 {
            break;
        }

        let mut p3 = s[row - 4];
        let mut p2 = s[row - 3];
        let mut p1 = s[row - 2];
        let mut p0 = s[row - 1];
        let mut q0 = s[row];
        let mut q1 = s[row + 1];
        let mut q2 = s[row + 2];
        let mut q3 = s[row + 3];

        let mask = filter_mask(limit, blimit, p3, p2, p1, p0, q0, q1, q2, q3);
        let flat = flat_mask4(1, p3, p2, p1, p0, q0, q1, q2, q3);

        filter8(
            mask,
            thresh,
            flat != 0,
            &mut p3,
            &mut p2,
            &mut p1,
            &mut p0,
            &mut q0,
            &mut q1,
            &mut q2,
            &mut q3,
        );

        s[row - 4] = p3;
        s[row - 3] = p2;
        s[row - 2] = p1;
        s[row - 1] = p0;
        s[row] = q0;
        s[row + 1] = q1;
        s[row + 2] = q2;
        s[row + 3] = q3;
    }
}

pub fn lpf_horizontal_16(
    s: &mut [u8],
    offset: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
) {
    if offset < 8 * pitch || offset + 8 * pitch > s.len() {
        return;
    }

    for i in 0..8 {
        let mut p7 = s[offset - 8 * pitch + i];
        let mut p6 = s[offset - 7 * pitch + i];
        let mut p5 = s[offset - 6 * pitch + i];
        let mut p4 = s[offset - 5 * pitch + i];
        let mut p3 = s[offset - 4 * pitch + i];
        let mut p2 = s[offset - 3 * pitch + i];
        let mut p1 = s[offset - 2 * pitch + i];
        let mut p0 = s[offset - pitch + i];
        let mut q0 = s[offset + i];
        let mut q1 = s[offset + pitch + i];
        let mut q2 = s[offset + 2 * pitch + i];
        let mut q3 = s[offset + 3 * pitch + i];
        let mut q4 = s[offset + 4 * pitch + i];
        let mut q5 = s[offset + 5 * pitch + i];
        let mut q6 = s[offset + 6 * pitch + i];
        let mut q7 = s[offset + 7 * pitch + i];

        let mask = filter_mask(limit, blimit, p3, p2, p1, p0, q0, q1, q2, q3);
        let flat = flat_mask4(1, p3, p2, p1, p0, q0, q1, q2, q3);
        let flat2 = flat_mask5(1, p7, p6, p5, p4, p0, q0, q4, q5, q6, q7);

        filter16(
            mask,
            thresh,
            flat != 0,
            flat2 != 0,
            &mut p7,
            &mut p6,
            &mut p5,
            &mut p4,
            &mut p3,
            &mut p2,
            &mut p1,
            &mut p0,
            &mut q0,
            &mut q1,
            &mut q2,
            &mut q3,
            &mut q4,
            &mut q5,
            &mut q6,
            &mut q7,
        );

        s[offset - 8 * pitch + i] = p7;
        s[offset - 7 * pitch + i] = p6;
        s[offset - 6 * pitch + i] = p5;
        s[offset - 5 * pitch + i] = p4;
        s[offset - 4 * pitch + i] = p3;
        s[offset - 3 * pitch + i] = p2;
        s[offset - 2 * pitch + i] = p1;
        s[offset - pitch + i] = p0;
        s[offset + i] = q0;
        s[offset + pitch + i] = q1;
        s[offset + 2 * pitch + i] = q2;
        s[offset + 3 * pitch + i] = q3;
        s[offset + 4 * pitch + i] = q4;
        s[offset + 5 * pitch + i] = q5;
        s[offset + 6 * pitch + i] = q6;
        s[offset + 7 * pitch + i] = q7;
    }
}

pub fn lpf_vertical_16(
    s: &mut [u8],
    offset: usize,
    pitch: usize,
    blimit: u8,
    limit: u8,
    thresh: u8,
) {
    if offset < 8 || offset + 8 > s.len() {
        return;
    }

    for i in 0..8 {
        let row = offset + i * pitch;
        if row + 8 > s.len() || row < 8 {
            break;
        }

        let mut p7 = s[row - 8];
        let mut p6 = s[row - 7];
        let mut p5 = s[row - 6];
        let mut p4 = s[row - 5];
        let mut p3 = s[row - 4];
        let mut p2 = s[row - 3];
        let mut p1 = s[row - 2];
        let mut p0 = s[row - 1];
        let mut q0 = s[row];
        let mut q1 = s[row + 1];
        let mut q2 = s[row + 2];
        let mut q3 = s[row + 3];
        let mut q4 = s[row + 4];
        let mut q5 = s[row + 5];
        let mut q6 = s[row + 6];
        let mut q7 = s[row + 7];

        let mask = filter_mask(limit, blimit, p3, p2, p1, p0, q0, q1, q2, q3);
        let flat = flat_mask4(1, p3, p2, p1, p0, q0, q1, q2, q3);
        let flat2 = flat_mask5(1, p7, p6, p5, p4, p0, q0, q4, q5, q6, q7);

        filter16(
            mask,
            thresh,
            flat != 0,
            flat2 != 0,
            &mut p7,
            &mut p6,
            &mut p5,
            &mut p4,
            &mut p3,
            &mut p2,
            &mut p1,
            &mut p0,
            &mut q0,
            &mut q1,
            &mut q2,
            &mut q3,
            &mut q4,
            &mut q5,
            &mut q6,
            &mut q7,
        );

        s[row - 8] = p7;
        s[row - 7] = p6;
        s[row - 6] = p5;
        s[row - 5] = p4;
        s[row - 4] = p3;
        s[row - 3] = p2;
        s[row - 2] = p1;
        s[row - 1] = p0;
        s[row] = q0;
        s[row + 1] = q1;
        s[row + 2] = q2;
        s[row + 3] = q3;
        s[row + 4] = q4;
        s[row + 5] = q5;
        s[row + 6] = q6;
        s[row + 7] = q7;
    }
}

// -------------------------------------------------------------
// Frame-level Loop Filtering
// -------------------------------------------------------------

pub fn compute_loop_filter_limits(level: u8, sharpness: u8) -> (u8, u8, u8) {
    if level == 0 {
        return (0, 0, 0);
    }

    let mut inside_limit = (level as i32)
        >> ((if sharpness > 0 { 1 } else { 0 }) + (if sharpness > 4 { 1 } else { 0 }));
    if sharpness > 0 && inside_limit > 9 - (sharpness as i32) {
        inside_limit = 9 - (sharpness as i32);
    }
    if inside_limit < 1 {
        inside_limit = 1;
    }

    let limit = inside_limit as u8;
    let blimit = ((2 * (level as i32 + 2)) + inside_limit).min(255) as u8;
    let thresh = (level >> 4) as u8;

    (blimit, limit, thresh)
}

pub fn loop_filter_plane(
    plane: &mut [u8],
    width: usize,
    height: usize,
    stride: usize,
    filter_level: u8,
    sharpness: u8,
) {
    if filter_level == 0 || width < 8 || height < 8 {
        return;
    }

    let (blimit, limit, thresh) = compute_loop_filter_limits(filter_level, sharpness);

    // Vertical edges (filtering across vertical block boundaries, x = 8, 16, 24, ...)
    let cols = width / 8;
    let rows = height / 8;

    for r in 0..rows {
        for c in 1..cols {
            let offset = r * 8 * stride + c * 8;
            lpf_vertical_8(plane, offset, stride, blimit, limit, thresh);
        }
    }

    // Horizontal edges (filtering across horizontal block boundaries, y = 8, 16, 24, ...)
    for r in 1..rows {
        for c in 0..cols {
            let offset = r * 8 * stride + c * 8;
            lpf_horizontal_8(plane, offset, stride, blimit, limit, thresh);
        }
    }
}

pub fn loop_filter_frame(
    y_plane: &mut [u8],
    u_plane: &mut [u8],
    v_plane: &mut [u8],
    width: usize,
    height: usize,
    y_stride: usize,
    uv_stride: usize,
    filter_level: u8,
    sharpness: u8,
) {
    loop_filter_plane(y_plane, width, height, y_stride, filter_level, sharpness);
    loop_filter_plane(
        u_plane,
        width / 2,
        height / 2,
        uv_stride,
        filter_level,
        sharpness,
    );
    loop_filter_plane(
        v_plane,
        width / 2,
        height / 2,
        uv_stride,
        filter_level,
        sharpness,
    );
}
