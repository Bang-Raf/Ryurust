// VP9 Inverse Discrete Cosine Transform & Inverse Asymmetric Discrete Sine Transform
// Fixed-point exact integer math matching libvpx / VP9 spec

pub const DCT_CONST_BITS: u32 = 14;

pub const COS_PI_1_64: i16 = 16364;
pub const COS_PI_2_64: i16 = 16305;
pub const COS_PI_3_64: i16 = 16207;
pub const COS_PI_4_64: i16 = 16069;
pub const COS_PI_5_64: i16 = 15893;
pub const COS_PI_6_64: i16 = 15679;
pub const COS_PI_7_64: i16 = 15426;
pub const COS_PI_8_64: i16 = 15137;
pub const COS_PI_9_64: i16 = 14811;
pub const COS_PI_10_64: i16 = 14449;
pub const COS_PI_11_64: i16 = 14053;
pub const COS_PI_12_64: i16 = 13623;
pub const COS_PI_13_64: i16 = 13160;
pub const COS_PI_14_64: i16 = 12665;
pub const COS_PI_15_64: i16 = 12140;
pub const COS_PI_16_64: i16 = 11585;
pub const COS_PI_17_64: i16 = 11003;
pub const COS_PI_18_64: i16 = 10394;
pub const COS_PI_19_64: i16 = 9760;
pub const COS_PI_20_64: i16 = 9102;
pub const COS_PI_21_64: i16 = 8423;
pub const COS_PI_22_64: i16 = 7723;
pub const COS_PI_23_64: i16 = 7005;
pub const COS_PI_24_64: i16 = 6270;
pub const COS_PI_25_64: i16 = 5520;
pub const COS_PI_26_64: i16 = 4756;
pub const COS_PI_27_64: i16 = 3981;
pub const COS_PI_28_64: i16 = 3196;
pub const COS_PI_29_64: i16 = 2404;
pub const COS_PI_30_64: i16 = 1606;
pub const COS_PI_31_64: i16 = 804;

pub const SIN_PI_1_9: i32 = 5283;
pub const SIN_PI_2_9: i32 = 9929;
pub const SIN_PI_3_9: i32 = 13377;
pub const SIN_PI_4_9: i32 = 15212;

#[inline(always)]
pub fn wrap_low(x: i64) -> i32 {
    (x as i16) as i32
}

#[inline(always)]
pub fn dct_const_round_shift(input: i64) -> i64 {
    round_power_of_two(input, DCT_CONST_BITS)
}

#[inline(always)]
pub fn round_power_of_two(value: i64, n: u32) -> i64 {
    (value + (1i64 << (n - 1))) >> n
}

#[inline(always)]
pub fn clip_pixel(val: i32) -> u8 {
    if val > 255 {
        255
    } else if val < 0 {
        0
    } else {
        val as u8
    }
}

#[inline(always)]
pub fn clip_pixel_add(dest: u8, trans: i64) -> u8 {
    let t = wrap_low(trans);
    clip_pixel(dest as i32 + t)
}

pub fn idct4(input: &[i32], output: &mut [i32]) {
    let mut step = [0i16; 4];

    // stage 1
    let temp1 = ((input[0] as i16 as i64) + (input[2] as i16 as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((input[0] as i16 as i64) - (input[2] as i16 as i64)) * (COS_PI_16_64 as i64);
    step[0] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step[1] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[1] as i16 as i64) * (COS_PI_24_64 as i64))
        - ((input[3] as i16 as i64) * (COS_PI_8_64 as i64));
    let temp2 = ((input[1] as i16 as i64) * (COS_PI_8_64 as i64))
        + ((input[3] as i16 as i64) * (COS_PI_24_64 as i64));
    step[2] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step[3] = wrap_low(dct_const_round_shift(temp2)) as i16;

    // stage 2
    output[0] = wrap_low((step[0] as i64) + (step[3] as i64));
    output[1] = wrap_low((step[1] as i64) + (step[2] as i64));
    output[2] = wrap_low((step[1] as i64) - (step[2] as i64));
    output[3] = wrap_low((step[0] as i64) - (step[3] as i64));
}

pub fn iadst4(input: &[i32], output: &mut [i32]) {
    let x0 = input[0] as i64;
    let x1 = input[1] as i64;
    let x2 = input[2] as i64;
    let x3 = input[3] as i64;

    if (x0 | x1 | x2 | x3) == 0 {
        output[..4].fill(0);
        return;
    }

    let mut s0 = (SIN_PI_1_9 as i64) * x0;
    let mut s1 = (SIN_PI_2_9 as i64) * x0;
    let s2 = (SIN_PI_3_9 as i64) * x1;
    let s3 = (SIN_PI_4_9 as i64) * x2;
    let s4 = (SIN_PI_1_9 as i64) * x2;
    let s5 = (SIN_PI_2_9 as i64) * x3;
    let s6 = (SIN_PI_4_9 as i64) * x3;
    let s7 = wrap_low(x0 - x2 + x3) as i64;

    s0 = s0 + s3 + s5;
    s1 = s1 - s4 - s6;
    let s3_val = s2;
    let s2_val = (SIN_PI_3_9 as i64) * s7;

    output[0] = wrap_low(dct_const_round_shift(s0 + s3_val));
    output[1] = wrap_low(dct_const_round_shift(s1 + s3_val));
    output[2] = wrap_low(dct_const_round_shift(s2_val));
    output[3] = wrap_low(dct_const_round_shift(s0 + s1 - s3_val));
}

pub fn idct8(input: &[i32], output: &mut [i32]) {
    let mut step1 = [0i16; 8];
    let mut step2 = [0i16; 8];

    // stage 1
    step1[0] = input[0] as i16;
    step1[2] = input[4] as i16;
    step1[1] = input[2] as i16;
    step1[3] = input[6] as i16;

    let temp1 = ((input[1] as i16 as i64) * (COS_PI_28_64 as i64))
        - ((input[7] as i16 as i64) * (COS_PI_4_64 as i64));
    let temp2 = ((input[1] as i16 as i64) * (COS_PI_4_64 as i64))
        + ((input[7] as i16 as i64) * (COS_PI_28_64 as i64));
    step1[4] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[7] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[5] as i16 as i64) * (COS_PI_12_64 as i64))
        - ((input[3] as i16 as i64) * (COS_PI_20_64 as i64));
    let temp2 = ((input[5] as i16 as i64) * (COS_PI_20_64 as i64))
        + ((input[3] as i16 as i64) * (COS_PI_12_64 as i64));
    step1[5] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[6] = wrap_low(dct_const_round_shift(temp2)) as i16;

    // stage 2
    let temp1 = ((step1[0] as i64) + (step1[2] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step1[0] as i64) - (step1[2] as i64)) * (COS_PI_16_64 as i64);
    step2[0] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[1] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[1] as i64) * (COS_PI_24_64 as i64)) - ((step1[3] as i64) * (COS_PI_8_64 as i64));
    let temp2 =
        ((step1[1] as i64) * (COS_PI_8_64 as i64)) + ((step1[3] as i64) * (COS_PI_24_64 as i64));
    step2[2] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[3] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step2[4] = wrap_low((step1[4] as i64) + (step1[5] as i64)) as i16;
    step2[5] = wrap_low((step1[4] as i64) - (step1[5] as i64)) as i16;
    step2[6] = wrap_low(-(step1[6] as i64) + (step1[7] as i64)) as i16;
    step2[7] = wrap_low((step1[6] as i64) + (step1[7] as i64)) as i16;

    // stage 3
    step1[0] = wrap_low((step2[0] as i64) + (step2[3] as i64)) as i16;
    step1[1] = wrap_low((step2[1] as i64) + (step2[2] as i64)) as i16;
    step1[2] = wrap_low((step2[1] as i64) - (step2[2] as i64)) as i16;
    step1[3] = wrap_low((step2[0] as i64) - (step2[3] as i64)) as i16;
    step1[4] = step2[4];
    let temp1 = ((step2[6] as i64) - (step2[5] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step2[5] as i64) + (step2[6] as i64)) * (COS_PI_16_64 as i64);
    step1[5] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[6] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step1[7] = step2[7];

    // stage 4
    output[0] = wrap_low((step1[0] as i64) + (step1[7] as i64));
    output[1] = wrap_low((step1[1] as i64) + (step1[6] as i64));
    output[2] = wrap_low((step1[2] as i64) + (step1[5] as i64));
    output[3] = wrap_low((step1[3] as i64) + (step1[4] as i64));
    output[4] = wrap_low((step1[3] as i64) - (step1[4] as i64));
    output[5] = wrap_low((step1[2] as i64) - (step1[5] as i64));
    output[6] = wrap_low((step1[1] as i64) - (step1[6] as i64));
    output[7] = wrap_low((step1[0] as i64) - (step1[7] as i64));
}

pub fn iadst8(input: &[i32], output: &mut [i32]) {
    let mut x0 = input[7] as i64;
    let mut x1 = input[0] as i64;
    let mut x2 = input[5] as i64;
    let mut x3 = input[2] as i64;
    let mut x4 = input[3] as i64;
    let mut x5 = input[4] as i64;
    let mut x6 = input[1] as i64;
    let mut x7 = input[6] as i64;

    if (x0 | x1 | x2 | x3 | x4 | x5 | x6 | x7) == 0 {
        output[..8].fill(0);
        return;
    }

    // stage 1
    let s0 = ((COS_PI_2_64 as i64) * x0) + ((COS_PI_30_64 as i64) * x1);
    let s1 = ((COS_PI_30_64 as i64) * x0) - ((COS_PI_2_64 as i64) * x1);
    let s2 = ((COS_PI_10_64 as i64) * x2) + ((COS_PI_22_64 as i64) * x3);
    let s3 = ((COS_PI_22_64 as i64) * x2) - ((COS_PI_10_64 as i64) * x3);
    let s4 = ((COS_PI_18_64 as i64) * x4) + ((COS_PI_14_64 as i64) * x5);
    let s5 = ((COS_PI_14_64 as i64) * x4) - ((COS_PI_18_64 as i64) * x5);
    let s6 = ((COS_PI_26_64 as i64) * x6) + ((COS_PI_6_64 as i64) * x7);
    let s7 = ((COS_PI_6_64 as i64) * x6) - ((COS_PI_26_64 as i64) * x7);

    x0 = wrap_low(dct_const_round_shift(s0 + s4)) as i64;
    x1 = wrap_low(dct_const_round_shift(s1 + s5)) as i64;
    x2 = wrap_low(dct_const_round_shift(s2 + s6)) as i64;
    x3 = wrap_low(dct_const_round_shift(s3 + s7)) as i64;
    x4 = wrap_low(dct_const_round_shift(s0 - s4)) as i64;
    x5 = wrap_low(dct_const_round_shift(s1 - s5)) as i64;
    x6 = wrap_low(dct_const_round_shift(s2 - s6)) as i64;
    x7 = wrap_low(dct_const_round_shift(s3 - s7)) as i64;

    // stage 2
    let s0 = x0;
    let s1 = x1;
    let s2 = x2;
    let s3 = x3;
    let s4 = ((COS_PI_8_64 as i64) * x4) + ((COS_PI_24_64 as i64) * x5);
    let s5 = ((COS_PI_24_64 as i64) * x4) - ((COS_PI_8_64 as i64) * x5);
    let s6 = ((-COS_PI_24_64 as i64) * x6) + ((COS_PI_8_64 as i64) * x7);
    let s7 = ((COS_PI_8_64 as i64) * x6) + ((COS_PI_24_64 as i64) * x7);

    x0 = wrap_low(s0 + s2) as i64;
    x1 = wrap_low(s1 + s3) as i64;
    x2 = wrap_low(s0 - s2) as i64;
    x3 = wrap_low(s1 - s3) as i64;
    x4 = wrap_low(dct_const_round_shift(s4 + s6)) as i64;
    x5 = wrap_low(dct_const_round_shift(s5 + s7)) as i64;
    x6 = wrap_low(dct_const_round_shift(s4 - s6)) as i64;
    x7 = wrap_low(dct_const_round_shift(s5 - s7)) as i64;

    // stage 3
    let s2 = (COS_PI_16_64 as i64) * (x2 + x3);
    let s3 = (COS_PI_16_64 as i64) * (x2 - x3);
    let s6 = (COS_PI_16_64 as i64) * (x6 + x7);
    let s7 = (COS_PI_16_64 as i64) * (x6 - x7);

    x2 = wrap_low(dct_const_round_shift(s2)) as i64;
    x3 = wrap_low(dct_const_round_shift(s3)) as i64;
    x6 = wrap_low(dct_const_round_shift(s6)) as i64;
    x7 = wrap_low(dct_const_round_shift(s7)) as i64;

    output[0] = wrap_low(x0);
    output[1] = wrap_low(-x4);
    output[2] = wrap_low(x6);
    output[3] = wrap_low(-x2);
    output[4] = wrap_low(x3);
    output[5] = wrap_low(-x7);
    output[6] = wrap_low(x5);
    output[7] = wrap_low(-x1);
}

pub fn idct16(input: &[i32], output: &mut [i32]) {
    let mut step1 = [0i16; 16];
    let mut step2 = [0i16; 16];

    // stage 1
    step1[0] = input[0] as i16;
    step1[1] = input[8] as i16;
    step1[2] = input[4] as i16;
    step1[3] = input[12] as i16;
    step1[4] = input[2] as i16;
    step1[5] = input[10] as i16;
    step1[6] = input[6] as i16;
    step1[7] = input[14] as i16;
    step1[8] = input[1] as i16;
    step1[9] = input[9] as i16;
    step1[10] = input[5] as i16;
    step1[11] = input[13] as i16;
    step1[12] = input[3] as i16;
    step1[13] = input[11] as i16;
    step1[14] = input[7] as i16;
    step1[15] = input[15] as i16;

    // stage 2
    step2[..8].copy_from_slice(&step1[..8]);

    let temp1 =
        ((step1[8] as i64) * (COS_PI_30_64 as i64)) - ((step1[15] as i64) * (COS_PI_2_64 as i64));
    let temp2 =
        ((step1[8] as i64) * (COS_PI_2_64 as i64)) + ((step1[15] as i64) * (COS_PI_30_64 as i64));
    step2[8] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[15] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[9] as i64) * (COS_PI_14_64 as i64)) - ((step1[14] as i64) * (COS_PI_18_64 as i64));
    let temp2 =
        ((step1[9] as i64) * (COS_PI_18_64 as i64)) + ((step1[14] as i64) * (COS_PI_14_64 as i64));
    step2[9] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[14] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[10] as i64) * (COS_PI_22_64 as i64)) - ((step1[13] as i64) * (COS_PI_10_64 as i64));
    let temp2 =
        ((step1[10] as i64) * (COS_PI_10_64 as i64)) + ((step1[13] as i64) * (COS_PI_22_64 as i64));
    step2[10] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[13] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[11] as i64) * (COS_PI_6_64 as i64)) - ((step1[12] as i64) * (COS_PI_26_64 as i64));
    let temp2 =
        ((step1[11] as i64) * (COS_PI_26_64 as i64)) + ((step1[12] as i64) * (COS_PI_6_64 as i64));
    step2[11] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[12] = wrap_low(dct_const_round_shift(temp2)) as i16;

    // stage 3
    step1[..4].copy_from_slice(&step2[..4]);

    let temp1 =
        ((step2[4] as i64) * (COS_PI_28_64 as i64)) - ((step2[7] as i64) * (COS_PI_4_64 as i64));
    let temp2 =
        ((step2[4] as i64) * (COS_PI_4_64 as i64)) + ((step2[7] as i64) * (COS_PI_28_64 as i64));
    step1[4] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[7] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step2[5] as i64) * (COS_PI_12_64 as i64)) - ((step2[6] as i64) * (COS_PI_20_64 as i64));
    let temp2 =
        ((step2[5] as i64) * (COS_PI_20_64 as i64)) + ((step2[6] as i64) * (COS_PI_12_64 as i64));
    step1[5] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[6] = wrap_low(dct_const_round_shift(temp2)) as i16;

    step1[8] = wrap_low((step2[8] as i64) + (step2[9] as i64)) as i16;
    step1[9] = wrap_low((step2[8] as i64) - (step2[9] as i64)) as i16;
    step1[10] = wrap_low(-(step2[10] as i64) + (step2[11] as i64)) as i16;
    step1[11] = wrap_low((step2[10] as i64) + (step2[11] as i64)) as i16;
    step1[12] = wrap_low((step2[12] as i64) + (step2[13] as i64)) as i16;
    step1[13] = wrap_low((step2[12] as i64) - (step2[13] as i64)) as i16;
    step1[14] = wrap_low(-(step2[14] as i64) + (step2[15] as i64)) as i16;
    step1[15] = wrap_low((step2[14] as i64) + (step2[15] as i64)) as i16;

    // stage 4
    let temp1 = ((step1[0] as i64) + (step1[1] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step1[0] as i64) - (step1[1] as i64)) * (COS_PI_16_64 as i64);
    step2[0] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[1] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[2] as i64) * (COS_PI_24_64 as i64)) - ((step1[3] as i64) * (COS_PI_8_64 as i64));
    let temp2 =
        ((step1[2] as i64) * (COS_PI_8_64 as i64)) + ((step1[3] as i64) * (COS_PI_24_64 as i64));
    step2[2] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[3] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step2[4] = wrap_low((step1[4] as i64) + (step1[5] as i64)) as i16;
    step2[5] = wrap_low((step1[4] as i64) - (step1[5] as i64)) as i16;
    step2[6] = wrap_low(-(step1[6] as i64) + (step1[7] as i64)) as i16;
    step2[7] = wrap_low((step1[6] as i64) + (step1[7] as i64)) as i16;

    step2[8] = step1[8];
    step2[15] = step1[15];
    let temp1 =
        (-(step1[9] as i64) * (COS_PI_8_64 as i64)) + ((step1[14] as i64) * (COS_PI_24_64 as i64));
    let temp2 =
        ((step1[9] as i64) * (COS_PI_24_64 as i64)) + ((step1[14] as i64) * (COS_PI_8_64 as i64));
    step2[9] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[14] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        (-(step1[10] as i64) * (COS_PI_24_64 as i64)) - ((step1[13] as i64) * (COS_PI_8_64 as i64));
    let temp2 =
        (-(step1[10] as i64) * (COS_PI_8_64 as i64)) + ((step1[13] as i64) * (COS_PI_24_64 as i64));
    step2[10] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[13] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step2[11] = step1[11];
    step2[12] = step1[12];

    // stage 5
    step1[0] = wrap_low((step2[0] as i64) + (step2[3] as i64)) as i16;
    step1[1] = wrap_low((step2[1] as i64) + (step2[2] as i64)) as i16;
    step1[2] = wrap_low((step2[1] as i64) - (step2[2] as i64)) as i16;
    step1[3] = wrap_low((step2[0] as i64) - (step2[3] as i64)) as i16;
    step1[4] = step2[4];
    let temp1 = ((step2[6] as i64) - (step2[5] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step2[5] as i64) + (step2[6] as i64)) * (COS_PI_16_64 as i64);
    step1[5] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[6] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step1[7] = step2[7];

    step1[8] = wrap_low((step2[8] as i64) + (step2[11] as i64)) as i16;
    step1[9] = wrap_low((step2[9] as i64) + (step2[10] as i64)) as i16;
    step1[10] = wrap_low((step2[9] as i64) - (step2[10] as i64)) as i16;
    step1[11] = wrap_low((step2[8] as i64) - (step2[11] as i64)) as i16;
    step1[12] = wrap_low(-(step2[12] as i64) + (step2[15] as i64)) as i16;
    step1[13] = wrap_low(-(step2[13] as i64) + (step2[14] as i64)) as i16;
    step1[14] = wrap_low((step2[13] as i64) + (step2[14] as i64)) as i16;
    step1[15] = wrap_low((step2[12] as i64) + (step2[15] as i64)) as i16;

    // stage 6
    step2[0] = wrap_low((step1[0] as i64) + (step1[7] as i64)) as i16;
    step2[1] = wrap_low((step1[1] as i64) + (step1[6] as i64)) as i16;
    step2[2] = wrap_low((step1[2] as i64) + (step1[5] as i64)) as i16;
    step2[3] = wrap_low((step1[3] as i64) + (step1[4] as i64)) as i16;
    step2[4] = wrap_low((step1[3] as i64) - (step1[4] as i64)) as i16;
    step2[5] = wrap_low((step1[2] as i64) - (step1[5] as i64)) as i16;
    step2[6] = wrap_low((step1[1] as i64) - (step1[6] as i64)) as i16;
    step2[7] = wrap_low((step1[0] as i64) - (step1[7] as i64)) as i16;
    step2[8] = step1[8];
    step2[9] = step1[9];

    let temp1 = (-(step1[10] as i64) + (step1[13] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step1[10] as i64) + (step1[13] as i64)) * (COS_PI_16_64 as i64);
    step2[10] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[13] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = (-(step1[11] as i64) + (step1[12] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step1[11] as i64) + (step1[12] as i64)) * (COS_PI_16_64 as i64);
    step2[11] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[12] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step2[14] = step1[14];
    step2[15] = step1[15];

    // stage 7
    output[0] = wrap_low((step2[0] as i64) + (step2[15] as i64));
    output[1] = wrap_low((step2[1] as i64) + (step2[14] as i64));
    output[2] = wrap_low((step2[2] as i64) + (step2[13] as i64));
    output[3] = wrap_low((step2[3] as i64) + (step2[12] as i64));
    output[4] = wrap_low((step2[4] as i64) + (step2[11] as i64));
    output[5] = wrap_low((step2[5] as i64) + (step2[10] as i64));
    output[6] = wrap_low((step2[6] as i64) + (step2[9] as i64));
    output[7] = wrap_low((step2[7] as i64) + (step2[8] as i64));
    output[8] = wrap_low((step2[7] as i64) - (step2[8] as i64));
    output[9] = wrap_low((step2[6] as i64) - (step2[9] as i64));
    output[10] = wrap_low((step2[5] as i64) - (step2[10] as i64));
    output[11] = wrap_low((step2[4] as i64) - (step2[11] as i64));
    output[12] = wrap_low((step2[3] as i64) - (step2[12] as i64));
    output[13] = wrap_low((step2[2] as i64) - (step2[13] as i64));
    output[14] = wrap_low((step2[1] as i64) - (step2[14] as i64));
    output[15] = wrap_low((step2[0] as i64) - (step2[15] as i64));
}

pub fn idct32(input: &[i32], output: &mut [i32]) {
    let mut step1 = [0i16; 32];
    let mut step2 = [0i16; 32];

    // stage 1
    step1[0] = input[0] as i16;
    step1[1] = input[16] as i16;
    step1[2] = input[8] as i16;
    step1[3] = input[24] as i16;
    step1[4] = input[4] as i16;
    step1[5] = input[20] as i16;
    step1[6] = input[12] as i16;
    step1[7] = input[28] as i16;
    step1[8] = input[2] as i16;
    step1[9] = input[18] as i16;
    step1[10] = input[10] as i16;
    step1[11] = input[26] as i16;
    step1[12] = input[6] as i16;
    step1[13] = input[22] as i16;
    step1[14] = input[14] as i16;
    step1[15] = input[30] as i16;

    let temp1 = ((input[1] as i16 as i64) * (COS_PI_31_64 as i64))
        - ((input[31] as i16 as i64) * (COS_PI_1_64 as i64));
    let temp2 = ((input[1] as i16 as i64) * (COS_PI_1_64 as i64))
        + ((input[31] as i16 as i64) * (COS_PI_31_64 as i64));
    step1[16] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[31] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[17] as i16 as i64) * (COS_PI_15_64 as i64))
        - ((input[15] as i16 as i64) * (COS_PI_17_64 as i64));
    let temp2 = ((input[17] as i16 as i64) * (COS_PI_17_64 as i64))
        + ((input[15] as i16 as i64) * (COS_PI_15_64 as i64));
    step1[17] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[30] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[9] as i16 as i64) * (COS_PI_23_64 as i64))
        - ((input[23] as i16 as i64) * (COS_PI_9_64 as i64));
    let temp2 = ((input[9] as i16 as i64) * (COS_PI_9_64 as i64))
        + ((input[23] as i16 as i64) * (COS_PI_23_64 as i64));
    step1[18] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[29] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[25] as i16 as i64) * (COS_PI_7_64 as i64))
        - ((input[7] as i16 as i64) * (COS_PI_25_64 as i64));
    let temp2 = ((input[25] as i16 as i64) * (COS_PI_25_64 as i64))
        + ((input[7] as i16 as i64) * (COS_PI_7_64 as i64));
    step1[19] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[28] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[5] as i16 as i64) * (COS_PI_27_64 as i64))
        - ((input[27] as i16 as i64) * (COS_PI_5_64 as i64));
    let temp2 = ((input[5] as i16 as i64) * (COS_PI_5_64 as i64))
        + ((input[27] as i16 as i64) * (COS_PI_27_64 as i64));
    step1[20] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[27] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[21] as i16 as i64) * (COS_PI_11_64 as i64))
        - ((input[11] as i16 as i64) * (COS_PI_21_64 as i64));
    let temp2 = ((input[21] as i16 as i64) * (COS_PI_21_64 as i64))
        + ((input[11] as i16 as i64) * (COS_PI_11_64 as i64));
    step1[21] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[26] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[13] as i16 as i64) * (COS_PI_19_64 as i64))
        - ((input[19] as i16 as i64) * (COS_PI_13_64 as i64));
    let temp2 = ((input[13] as i16 as i64) * (COS_PI_13_64 as i64))
        + ((input[19] as i16 as i64) * (COS_PI_19_64 as i64));
    step1[22] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[25] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = ((input[29] as i16 as i64) * (COS_PI_3_64 as i64))
        - ((input[3] as i16 as i64) * (COS_PI_29_64 as i64));
    let temp2 = ((input[29] as i16 as i64) * (COS_PI_29_64 as i64))
        + ((input[3] as i16 as i64) * (COS_PI_3_64 as i64));
    step1[23] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[24] = wrap_low(dct_const_round_shift(temp2)) as i16;

    // stage 2
    step2[..8].copy_from_slice(&step1[..8]);

    let temp1 =
        ((step1[8] as i64) * (COS_PI_30_64 as i64)) - ((step1[15] as i64) * (COS_PI_2_64 as i64));
    let temp2 =
        ((step1[8] as i64) * (COS_PI_2_64 as i64)) + ((step1[15] as i64) * (COS_PI_30_64 as i64));
    step2[8] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[15] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[9] as i64) * (COS_PI_14_64 as i64)) - ((step1[14] as i64) * (COS_PI_18_64 as i64));
    let temp2 =
        ((step1[9] as i64) * (COS_PI_18_64 as i64)) + ((step1[14] as i64) * (COS_PI_14_64 as i64));
    step2[9] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[14] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[10] as i64) * (COS_PI_22_64 as i64)) - ((step1[13] as i64) * (COS_PI_10_64 as i64));
    let temp2 =
        ((step1[10] as i64) * (COS_PI_10_64 as i64)) + ((step1[13] as i64) * (COS_PI_22_64 as i64));
    step2[10] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[13] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[11] as i64) * (COS_PI_6_64 as i64)) - ((step1[12] as i64) * (COS_PI_26_64 as i64));
    let temp2 =
        ((step1[11] as i64) * (COS_PI_26_64 as i64)) + ((step1[12] as i64) * (COS_PI_6_64 as i64));
    step2[11] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[12] = wrap_low(dct_const_round_shift(temp2)) as i16;

    step2[16] = wrap_low((step1[16] as i64) + (step1[17] as i64)) as i16;
    step2[17] = wrap_low((step1[16] as i64) - (step1[17] as i64)) as i16;
    step2[18] = wrap_low(-(step1[18] as i64) + (step1[19] as i64)) as i16;
    step2[19] = wrap_low((step1[18] as i64) + (step1[19] as i64)) as i16;
    step2[20] = wrap_low((step1[20] as i64) + (step1[21] as i64)) as i16;
    step2[21] = wrap_low((step1[20] as i64) - (step1[21] as i64)) as i16;
    step2[22] = wrap_low(-(step1[22] as i64) + (step1[23] as i64)) as i16;
    step2[23] = wrap_low((step1[22] as i64) + (step1[23] as i64)) as i16;
    step2[24] = wrap_low((step1[24] as i64) + (step1[25] as i64)) as i16;
    step2[25] = wrap_low((step1[24] as i64) - (step1[25] as i64)) as i16;
    step2[26] = wrap_low(-(step1[26] as i64) + (step1[27] as i64)) as i16;
    step2[27] = wrap_low((step1[26] as i64) + (step1[27] as i64)) as i16;
    step2[28] = wrap_low((step1[28] as i64) + (step1[29] as i64)) as i16;
    step2[29] = wrap_low((step1[28] as i64) - (step1[29] as i64)) as i16;
    step2[30] = wrap_low(-(step1[30] as i64) + (step1[31] as i64)) as i16;
    step2[31] = wrap_low((step1[30] as i64) + (step1[31] as i64)) as i16;

    // stage 3
    step1[..4].copy_from_slice(&step2[..4]);

    let temp1 =
        ((step2[4] as i64) * (COS_PI_28_64 as i64)) - ((step2[7] as i64) * (COS_PI_4_64 as i64));
    let temp2 =
        ((step2[4] as i64) * (COS_PI_4_64 as i64)) + ((step2[7] as i64) * (COS_PI_28_64 as i64));
    step1[4] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[7] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step2[5] as i64) * (COS_PI_12_64 as i64)) - ((step2[6] as i64) * (COS_PI_20_64 as i64));
    let temp2 =
        ((step2[5] as i64) * (COS_PI_20_64 as i64)) + ((step2[6] as i64) * (COS_PI_12_64 as i64));
    step1[5] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[6] = wrap_low(dct_const_round_shift(temp2)) as i16;

    step1[8] = wrap_low((step2[8] as i64) + (step2[9] as i64)) as i16;
    step1[9] = wrap_low((step2[8] as i64) - (step2[9] as i64)) as i16;
    step1[10] = wrap_low(-(step2[10] as i64) + (step2[11] as i64)) as i16;
    step1[11] = wrap_low((step2[10] as i64) + (step2[11] as i64)) as i16;
    step1[12] = wrap_low((step2[12] as i64) + (step2[13] as i64)) as i16;
    step1[13] = wrap_low((step2[12] as i64) - (step2[13] as i64)) as i16;
    step1[14] = wrap_low(-(step2[14] as i64) + (step2[15] as i64)) as i16;
    step1[15] = wrap_low((step2[14] as i64) + (step2[15] as i64)) as i16;

    step1[16] = step2[16];
    step1[31] = step2[31];
    let temp1 =
        (-(step2[17] as i64) * (COS_PI_4_64 as i64)) + ((step2[30] as i64) * (COS_PI_28_64 as i64));
    let temp2 =
        ((step2[17] as i64) * (COS_PI_28_64 as i64)) + ((step2[30] as i64) * (COS_PI_4_64 as i64));
    step1[17] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[30] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        (-(step2[18] as i64) * (COS_PI_28_64 as i64)) - ((step2[29] as i64) * (COS_PI_4_64 as i64));
    let temp2 =
        (-(step2[18] as i64) * (COS_PI_4_64 as i64)) + ((step2[29] as i64) * (COS_PI_28_64 as i64));
    step1[18] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[29] = wrap_low(dct_const_round_shift(temp2)) as i16;

    step1[19] = step2[19];
    step1[20] = step2[20];
    let temp1 = (-(step2[21] as i64) * (COS_PI_20_64 as i64))
        + ((step2[26] as i64) * (COS_PI_12_64 as i64));
    let temp2 =
        ((step2[21] as i64) * (COS_PI_12_64 as i64)) + ((step2[26] as i64) * (COS_PI_20_64 as i64));
    step1[21] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[26] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = (-(step2[22] as i64) * (COS_PI_12_64 as i64))
        - ((step2[25] as i64) * (COS_PI_20_64 as i64));
    let temp2 = (-(step2[22] as i64) * (COS_PI_20_64 as i64))
        + ((step2[25] as i64) * (COS_PI_12_64 as i64));
    step1[22] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[25] = wrap_low(dct_const_round_shift(temp2)) as i16;

    step1[23] = step2[23];
    step1[24] = step2[24];
    step1[27] = step2[27];
    step1[28] = step2[28];

    // stage 4
    let temp1 = ((step1[0] as i64) + (step1[1] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step1[0] as i64) - (step1[1] as i64)) * (COS_PI_16_64 as i64);
    step2[0] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[1] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        ((step1[2] as i64) * (COS_PI_24_64 as i64)) - ((step1[3] as i64) * (COS_PI_8_64 as i64));
    let temp2 =
        ((step1[2] as i64) * (COS_PI_8_64 as i64)) + ((step1[3] as i64) * (COS_PI_24_64 as i64));
    step2[2] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[3] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step2[4] = wrap_low((step1[4] as i64) + (step1[5] as i64)) as i16;
    step2[5] = wrap_low((step1[4] as i64) - (step1[5] as i64)) as i16;
    step2[6] = wrap_low(-(step1[6] as i64) + (step1[7] as i64)) as i16;
    step2[7] = wrap_low((step1[6] as i64) + (step1[7] as i64)) as i16;

    step2[8] = step1[8];
    step2[15] = step1[15];
    let temp1 =
        (-(step1[9] as i64) * (COS_PI_8_64 as i64)) + ((step1[14] as i64) * (COS_PI_24_64 as i64));
    let temp2 =
        ((step1[9] as i64) * (COS_PI_24_64 as i64)) + ((step1[14] as i64) * (COS_PI_8_64 as i64));
    step2[9] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[14] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        (-(step1[10] as i64) * (COS_PI_24_64 as i64)) - ((step1[13] as i64) * (COS_PI_8_64 as i64));
    let temp2 =
        (-(step1[10] as i64) * (COS_PI_8_64 as i64)) + ((step1[13] as i64) * (COS_PI_24_64 as i64));
    step2[10] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[13] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step2[11] = step1[11];
    step2[12] = step1[12];

    step2[16] = wrap_low((step1[16] as i64) + (step1[19] as i64)) as i16;
    step2[17] = wrap_low((step1[17] as i64) + (step1[18] as i64)) as i16;
    step2[18] = wrap_low((step1[17] as i64) - (step1[18] as i64)) as i16;
    step2[19] = wrap_low((step1[16] as i64) - (step1[19] as i64)) as i16;
    step2[20] = wrap_low(-(step1[20] as i64) + (step1[23] as i64)) as i16;
    step2[21] = wrap_low(-(step1[21] as i64) + (step1[22] as i64)) as i16;
    step2[22] = wrap_low((step1[21] as i64) + (step1[22] as i64)) as i16;
    step2[23] = wrap_low((step1[20] as i64) + (step1[23] as i64)) as i16;

    step2[24] = wrap_low((step1[24] as i64) + (step1[27] as i64)) as i16;
    step2[25] = wrap_low((step1[25] as i64) + (step1[26] as i64)) as i16;
    step2[26] = wrap_low((step1[25] as i64) - (step1[26] as i64)) as i16;
    step2[27] = wrap_low((step1[24] as i64) - (step1[27] as i64)) as i16;
    step2[28] = wrap_low(-(step1[28] as i64) + (step1[31] as i64)) as i16;
    step2[29] = wrap_low(-(step1[29] as i64) + (step1[30] as i64)) as i16;
    step2[30] = wrap_low((step1[29] as i64) + (step1[30] as i64)) as i16;
    step2[31] = wrap_low((step1[28] as i64) + (step1[31] as i64)) as i16;

    // stage 5
    step1[0] = wrap_low((step2[0] as i64) + (step2[3] as i64)) as i16;
    step1[1] = wrap_low((step2[1] as i64) + (step2[2] as i64)) as i16;
    step1[2] = wrap_low((step2[1] as i64) - (step2[2] as i64)) as i16;
    step1[3] = wrap_low((step2[0] as i64) - (step2[3] as i64)) as i16;
    step1[4] = step2[4];
    let temp1 = ((step2[6] as i64) - (step2[5] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step2[5] as i64) + (step2[6] as i64)) * (COS_PI_16_64 as i64);
    step1[5] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[6] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step1[7] = step2[7];

    step1[8] = wrap_low((step2[8] as i64) + (step2[11] as i64)) as i16;
    step1[9] = wrap_low((step2[9] as i64) + (step2[10] as i64)) as i16;
    step1[10] = wrap_low((step2[9] as i64) - (step2[10] as i64)) as i16;
    step1[11] = wrap_low((step2[8] as i64) - (step2[11] as i64)) as i16;
    step1[12] = wrap_low(-(step2[12] as i64) + (step2[15] as i64)) as i16;
    step1[13] = wrap_low(-(step2[13] as i64) + (step2[14] as i64)) as i16;
    step1[14] = wrap_low((step2[13] as i64) + (step2[14] as i64)) as i16;
    step1[15] = wrap_low((step2[12] as i64) + (step2[15] as i64)) as i16;

    step1[16] = step2[16];
    step1[17] = step2[17];
    let temp1 =
        (-(step2[18] as i64) * (COS_PI_8_64 as i64)) + ((step2[29] as i64) * (COS_PI_24_64 as i64));
    let temp2 =
        ((step2[18] as i64) * (COS_PI_24_64 as i64)) + ((step2[29] as i64) * (COS_PI_8_64 as i64));
    step1[18] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[29] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        (-(step2[19] as i64) * (COS_PI_8_64 as i64)) + ((step2[28] as i64) * (COS_PI_24_64 as i64));
    let temp2 =
        ((step2[19] as i64) * (COS_PI_24_64 as i64)) + ((step2[28] as i64) * (COS_PI_8_64 as i64));
    step1[19] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[28] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        (-(step2[20] as i64) * (COS_PI_24_64 as i64)) - ((step2[27] as i64) * (COS_PI_8_64 as i64));
    let temp2 =
        (-(step2[20] as i64) * (COS_PI_8_64 as i64)) + ((step2[27] as i64) * (COS_PI_24_64 as i64));
    step1[20] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[27] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 =
        (-(step2[21] as i64) * (COS_PI_24_64 as i64)) - ((step2[26] as i64) * (COS_PI_8_64 as i64));
    let temp2 =
        (-(step2[21] as i64) * (COS_PI_8_64 as i64)) + ((step2[26] as i64) * (COS_PI_24_64 as i64));
    step1[21] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[26] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step1[22] = step2[22];
    step1[23] = step2[23];
    step1[24] = step2[24];
    step1[25] = step2[25];
    step1[30] = step2[30];
    step1[31] = step2[31];

    // stage 6
    step2[0] = wrap_low((step1[0] as i64) + (step1[7] as i64)) as i16;
    step2[1] = wrap_low((step1[1] as i64) + (step1[6] as i64)) as i16;
    step2[2] = wrap_low((step1[2] as i64) + (step1[5] as i64)) as i16;
    step2[3] = wrap_low((step1[3] as i64) + (step1[4] as i64)) as i16;
    step2[4] = wrap_low((step1[3] as i64) - (step1[4] as i64)) as i16;
    step2[5] = wrap_low((step1[2] as i64) - (step1[5] as i64)) as i16;
    step2[6] = wrap_low((step1[1] as i64) - (step1[6] as i64)) as i16;
    step2[7] = wrap_low((step1[0] as i64) - (step1[7] as i64)) as i16;
    step2[8] = step1[8];
    step2[9] = step1[9];

    let temp1 = (-(step1[10] as i64) + (step1[13] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step1[10] as i64) + (step1[13] as i64)) * (COS_PI_16_64 as i64);
    step2[10] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[13] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = (-(step1[11] as i64) + (step1[12] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step1[11] as i64) + (step1[12] as i64)) * (COS_PI_16_64 as i64);
    step2[11] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step2[12] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step2[14] = step1[14];
    step2[15] = step1[15];

    step2[16] = wrap_low((step1[16] as i64) + (step1[23] as i64)) as i16;
    step2[17] = wrap_low((step1[17] as i64) + (step1[22] as i64)) as i16;
    step2[18] = wrap_low((step1[18] as i64) + (step1[21] as i64)) as i16;
    step2[19] = wrap_low((step1[19] as i64) + (step1[20] as i64)) as i16;
    step2[20] = wrap_low((step1[19] as i64) - (step1[20] as i64)) as i16;
    step2[21] = wrap_low((step1[18] as i64) - (step1[21] as i64)) as i16;
    step2[22] = wrap_low((step1[17] as i64) - (step1[22] as i64)) as i16;
    step2[23] = wrap_low((step1[16] as i64) - (step1[23] as i64)) as i16;

    step2[24] = wrap_low(-(step1[24] as i64) + (step1[31] as i64)) as i16;
    step2[25] = wrap_low(-(step1[25] as i64) + (step1[30] as i64)) as i16;
    step2[26] = wrap_low(-(step1[26] as i64) + (step1[29] as i64)) as i16;
    step2[27] = wrap_low(-(step1[27] as i64) + (step1[28] as i64)) as i16;
    step2[28] = wrap_low((step1[27] as i64) + (step1[28] as i64)) as i16;
    step2[29] = wrap_low((step1[26] as i64) + (step1[29] as i64)) as i16;
    step2[30] = wrap_low((step1[25] as i64) + (step1[30] as i64)) as i16;
    step2[31] = wrap_low((step1[24] as i64) + (step1[31] as i64)) as i16;

    // stage 7
    step1[0] = wrap_low((step2[0] as i64) + (step2[15] as i64)) as i16;
    step1[1] = wrap_low((step2[1] as i64) + (step2[14] as i64)) as i16;
    step1[2] = wrap_low((step2[2] as i64) + (step2[13] as i64)) as i16;
    step1[3] = wrap_low((step2[3] as i64) + (step2[12] as i64)) as i16;
    step1[4] = wrap_low((step2[4] as i64) + (step2[11] as i64)) as i16;
    step1[5] = wrap_low((step2[5] as i64) + (step2[10] as i64)) as i16;
    step1[6] = wrap_low((step2[6] as i64) + (step2[9] as i64)) as i16;
    step1[7] = wrap_low((step2[7] as i64) + (step2[8] as i64)) as i16;
    step1[8] = wrap_low((step2[7] as i64) - (step2[8] as i64)) as i16;
    step1[9] = wrap_low((step2[6] as i64) - (step2[9] as i64)) as i16;
    step1[10] = wrap_low((step2[5] as i64) - (step2[10] as i64)) as i16;
    step1[11] = wrap_low((step2[4] as i64) - (step2[11] as i64)) as i16;
    step1[12] = wrap_low((step2[3] as i64) - (step2[12] as i64)) as i16;
    step1[13] = wrap_low((step2[2] as i64) - (step2[13] as i64)) as i16;
    step1[14] = wrap_low((step2[1] as i64) - (step2[14] as i64)) as i16;
    step1[15] = wrap_low((step2[0] as i64) - (step2[15] as i64)) as i16;

    step1[16] = step2[16];
    step1[17] = step2[17];
    step1[18] = step2[18];
    step1[19] = step2[19];
    let temp1 = (-(step2[20] as i64) + (step2[27] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step2[20] as i64) + (step2[27] as i64)) * (COS_PI_16_64 as i64);
    step1[20] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[27] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = (-(step2[21] as i64) + (step2[26] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step2[21] as i64) + (step2[26] as i64)) * (COS_PI_16_64 as i64);
    step1[21] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[26] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = (-(step2[22] as i64) + (step2[25] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step2[22] as i64) + (step2[25] as i64)) * (COS_PI_16_64 as i64);
    step1[22] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[25] = wrap_low(dct_const_round_shift(temp2)) as i16;

    let temp1 = (-(step2[23] as i64) + (step2[24] as i64)) * (COS_PI_16_64 as i64);
    let temp2 = ((step2[23] as i64) + (step2[24] as i64)) * (COS_PI_16_64 as i64);
    step1[23] = wrap_low(dct_const_round_shift(temp1)) as i16;
    step1[24] = wrap_low(dct_const_round_shift(temp2)) as i16;
    step1[28] = step2[28];
    step1[29] = step2[29];
    step1[30] = step2[30];
    step1[31] = step2[31];

    // final stage
    output[0] = wrap_low((step1[0] as i64) + (step1[31] as i64));
    output[1] = wrap_low((step1[1] as i64) + (step1[30] as i64));
    output[2] = wrap_low((step1[2] as i64) + (step1[29] as i64));
    output[3] = wrap_low((step1[3] as i64) + (step1[28] as i64));
    output[4] = wrap_low((step1[4] as i64) + (step1[27] as i64));
    output[5] = wrap_low((step1[5] as i64) + (step1[26] as i64));
    output[6] = wrap_low((step1[6] as i64) + (step1[25] as i64));
    output[7] = wrap_low((step1[7] as i64) + (step1[24] as i64));
    output[8] = wrap_low((step1[8] as i64) + (step1[23] as i64));
    output[9] = wrap_low((step1[9] as i64) + (step1[22] as i64));
    output[10] = wrap_low((step1[10] as i64) + (step1[21] as i64));
    output[11] = wrap_low((step1[11] as i64) + (step1[20] as i64));
    output[12] = wrap_low((step1[12] as i64) + (step1[19] as i64));
    output[13] = wrap_low((step1[13] as i64) + (step1[18] as i64));
    output[14] = wrap_low((step1[14] as i64) + (step1[17] as i64));
    output[15] = wrap_low((step1[15] as i64) + (step1[16] as i64));
    output[16] = wrap_low((step1[15] as i64) - (step1[16] as i64));
    output[17] = wrap_low((step1[14] as i64) - (step1[17] as i64));
    output[18] = wrap_low((step1[13] as i64) - (step1[18] as i64));
    output[19] = wrap_low((step1[12] as i64) - (step1[19] as i64));
    output[20] = wrap_low((step1[11] as i64) - (step1[20] as i64));
    output[21] = wrap_low((step1[10] as i64) - (step1[21] as i64));
    output[22] = wrap_low((step1[9] as i64) - (step1[22] as i64));
    output[23] = wrap_low((step1[8] as i64) - (step1[23] as i64));
    output[24] = wrap_low((step1[7] as i64) - (step1[24] as i64));
    output[25] = wrap_low((step1[6] as i64) - (step1[25] as i64));
    output[26] = wrap_low((step1[5] as i64) - (step1[26] as i64));
    output[27] = wrap_low((step1[4] as i64) - (step1[27] as i64));
    output[28] = wrap_low((step1[3] as i64) - (step1[28] as i64));
    output[29] = wrap_low((step1[2] as i64) - (step1[29] as i64));
    output[30] = wrap_low((step1[1] as i64) - (step1[30] as i64));
    output[31] = wrap_low((step1[0] as i64) - (step1[31] as i64));
}

// -------------------------------------------------------------
// 2D Transforms with Add-to-Destination
// -------------------------------------------------------------

pub fn idct4x4_1_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let output = wrap_low(dct_const_round_shift(
        (input[0] as i16 as i64) * (COS_PI_16_64 as i64),
    ));
    let output = wrap_low(dct_const_round_shift(
        (output as i64) * (COS_PI_16_64 as i64),
    ));
    let a1 = round_power_of_two(output as i64, 4);

    for j in 0..4 {
        let row_offset = j * stride;
        for i in 0..4 {
            dest[row_offset + i] = clip_pixel_add(dest[row_offset + i], a1);
        }
    }
}

pub fn idct4x4_16_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let mut intermediate = [0i32; 16];
    let mut temp_in = [0i32; 4];
    let mut temp_out = [0i32; 4];

    // Rows
    for i in 0..4 {
        idct4(
            &input[i * 4..(i + 1) * 4],
            &mut intermediate[i * 4..(i + 1) * 4],
        );
    }

    // Columns
    for i in 0..4 {
        for j in 0..4 {
            temp_in[j] = intermediate[j * 4 + i];
        }
        idct4(&temp_in, &mut temp_out);
        for (j, &out) in temp_out.iter().enumerate() {
            let row_offset = j * stride;
            dest[row_offset + i] =
                clip_pixel_add(dest[row_offset + i], round_power_of_two(out as i64, 4));
        }
    }
}

pub fn iht4x4_16_add(input: &[i32], dest: &mut [u8], stride: usize, tx_type: i32) {
    let mut intermediate = [0i32; 16];
    let mut temp_in = [0i32; 4];
    let mut temp_out = [0i32; 4];

    // Rows: tx_type 0 or 1 uses idct4, 2 or 3 uses iadst4
    for i in 0..4 {
        let slice = &input[i * 4..(i + 1) * 4];
        let out_slice = &mut intermediate[i * 4..(i + 1) * 4];
        match tx_type {
            0 | 1 => idct4(slice, out_slice),
            2 | 3 => iadst4(slice, out_slice),
            _ => idct4(slice, out_slice),
        }
    }

    // Columns: tx_type 0 or 2 uses idct4, 1 or 3 uses iadst4
    for i in 0..4 {
        for j in 0..4 {
            temp_in[j] = intermediate[j * 4 + i];
        }
        match tx_type {
            0 | 2 => idct4(&temp_in, &mut temp_out),
            1 | 3 => iadst4(&temp_in, &mut temp_out),
            _ => idct4(&temp_in, &mut temp_out),
        }
        for (j, &out) in temp_out.iter().enumerate() {
            let row_offset = j * stride;
            dest[row_offset + i] =
                clip_pixel_add(dest[row_offset + i], round_power_of_two(out as i64, 4));
        }
    }
}

pub fn idct4x4_add(input: &[i32], dest: &mut [u8], stride: usize, eob: i32) {
    if eob > 1 {
        idct4x4_16_add(input, dest, stride);
    } else {
        idct4x4_1_add(input, dest, stride);
    }
}

pub fn iht4x4_add(tx_type: i32, input: &[i32], dest: &mut [u8], stride: usize, eob: i32) {
    if eob > 1 {
        iht4x4_16_add(input, dest, stride, tx_type);
    } else {
        idct4x4_1_add(input, dest, stride);
    }
}

pub fn idct8x8_1_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let output = wrap_low(dct_const_round_shift(
        (input[0] as i16 as i64) * (COS_PI_16_64 as i64),
    ));
    let output = wrap_low(dct_const_round_shift(
        (output as i64) * (COS_PI_16_64 as i64),
    ));
    let a1 = round_power_of_two(output as i64, 5);

    for j in 0..8 {
        let row_offset = j * stride;
        for i in 0..8 {
            dest[row_offset + i] = clip_pixel_add(dest[row_offset + i], a1);
        }
    }
}

pub fn idct8x8_64_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let mut intermediate = [0i32; 64];
    let mut temp_in = [0i32; 8];
    let mut temp_out = [0i32; 8];

    // Rows
    for i in 0..8 {
        idct8(
            &input[i * 8..(i + 1) * 8],
            &mut intermediate[i * 8..(i + 1) * 8],
        );
    }

    // Columns
    for i in 0..8 {
        for j in 0..8 {
            temp_in[j] = intermediate[j * 8 + i];
        }
        idct8(&temp_in, &mut temp_out);
        for (j, &out) in temp_out.iter().enumerate() {
            let row_offset = j * stride;
            dest[row_offset + i] =
                clip_pixel_add(dest[row_offset + i], round_power_of_two(out as i64, 5));
        }
    }
}

pub fn iht8x8_64_add(input: &[i32], dest: &mut [u8], stride: usize, tx_type: i32) {
    let mut intermediate = [0i32; 64];
    let mut temp_in = [0i32; 8];
    let mut temp_out = [0i32; 8];

    // Rows
    for i in 0..8 {
        let slice = &input[i * 8..(i + 1) * 8];
        let out_slice = &mut intermediate[i * 8..(i + 1) * 8];
        match tx_type {
            0 | 1 => idct8(slice, out_slice),
            2 | 3 => iadst8(slice, out_slice),
            _ => idct8(slice, out_slice),
        }
    }

    // Columns
    for i in 0..8 {
        for j in 0..8 {
            temp_in[j] = intermediate[j * 8 + i];
        }
        match tx_type {
            0 | 2 => idct8(&temp_in, &mut temp_out),
            1 | 3 => iadst8(&temp_in, &mut temp_out),
            _ => idct8(&temp_in, &mut temp_out),
        }
        for (j, &out) in temp_out.iter().enumerate() {
            let row_offset = j * stride;
            dest[row_offset + i] =
                clip_pixel_add(dest[row_offset + i], round_power_of_two(out as i64, 5));
        }
    }
}

pub fn idct8x8_add(input: &[i32], dest: &mut [u8], stride: usize, eob: i32) {
    if eob == 1 {
        idct8x8_1_add(input, dest, stride);
    } else {
        idct8x8_64_add(input, dest, stride);
    }
}

pub fn iht8x8_add(tx_type: i32, input: &[i32], dest: &mut [u8], stride: usize, eob: i32) {
    if eob == 1 {
        idct8x8_1_add(input, dest, stride);
    } else {
        iht8x8_64_add(input, dest, stride, tx_type);
    }
}

pub fn idct16x16_1_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let output = wrap_low(dct_const_round_shift(
        (input[0] as i16 as i64) * (COS_PI_16_64 as i64),
    ));
    let output = wrap_low(dct_const_round_shift(
        (output as i64) * (COS_PI_16_64 as i64),
    ));
    let a1 = round_power_of_two(output as i64, 6);

    for j in 0..16 {
        let row_offset = j * stride;
        for i in 0..16 {
            dest[row_offset + i] = clip_pixel_add(dest[row_offset + i], a1);
        }
    }
}

pub fn idct16x16_256_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let mut intermediate = [0i32; 256];
    let mut temp_in = [0i32; 16];
    let mut temp_out = [0i32; 16];

    // Rows
    for i in 0..16 {
        idct16(
            &input[i * 16..(i + 1) * 16],
            &mut intermediate[i * 16..(i + 1) * 16],
        );
    }

    // Columns
    for i in 0..16 {
        for j in 0..16 {
            temp_in[j] = intermediate[j * 16 + i];
        }
        idct16(&temp_in, &mut temp_out);
        for (j, &out) in temp_out.iter().enumerate() {
            let row_offset = j * stride;
            dest[row_offset + i] =
                clip_pixel_add(dest[row_offset + i], round_power_of_two(out as i64, 6));
        }
    }
}

pub fn idct16x16_add(input: &[i32], dest: &mut [u8], stride: usize, eob: i32) {
    if eob == 1 {
        idct16x16_1_add(input, dest, stride);
    } else {
        idct16x16_256_add(input, dest, stride);
    }
}

pub fn idct32x32_1_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let output = wrap_low(dct_const_round_shift(
        (input[0] as i16 as i64) * (COS_PI_16_64 as i64),
    ));
    let output = wrap_low(dct_const_round_shift(
        (output as i64) * (COS_PI_16_64 as i64),
    ));
    let a1 = round_power_of_two(output as i64, 6);

    for j in 0..32 {
        let row_offset = j * stride;
        for i in 0..32 {
            dest[row_offset + i] = clip_pixel_add(dest[row_offset + i], a1);
        }
    }
}

/// True when every coefficient of `row` is exactly zero (as `i32`).
#[inline(always)]
pub fn row_is_zero(row: &[i32]) -> bool {
    row.iter().fold(0i32, |acc, &c| acc | c) == 0
}

pub fn idct32x32_1024_add(input: &[i32], dest: &mut [u8], stride: usize) {
    let mut intermediate = [0i32; 1024];
    let mut temp_in = [0i32; 32];
    let mut temp_out = [0i32; 32];

    // Rows
    for i in 0..32 {
        // Test the full 32-bit coefficients: truncating to i16 first would treat any
        // multiple of 65536 as zero and skip a non-zero row.
        let row = &input[i * 32..(i + 1) * 32];
        if !row_is_zero(row) {
            idct32(
                &input[i * 32..(i + 1) * 32],
                &mut intermediate[i * 32..(i + 1) * 32],
            );
        } else {
            intermediate[i * 32..(i + 1) * 32].fill(0);
        }
    }

    // Columns
    for i in 0..32 {
        for j in 0..32 {
            temp_in[j] = intermediate[j * 32 + i];
        }
        idct32(&temp_in, &mut temp_out);
        for (j, &out) in temp_out.iter().enumerate() {
            let row_offset = j * stride;
            dest[row_offset + i] =
                clip_pixel_add(dest[row_offset + i], round_power_of_two(out as i64, 6));
        }
    }
}

pub fn idct32x32_add(input: &[i32], dest: &mut [u8], stride: usize, eob: i32) {
    if eob == 1 {
        idct32x32_1_add(input, dest, stride);
    } else {
        idct32x32_1024_add(input, dest, stride);
    }
}
