#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VicPixel {
    pub r: u16,
    pub g: u16,
    pub b: u16,
    pub a: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VicColorMatrix {
    pub matrix_enable: u32,
    pub r_shift: i32,
    pub coeff00: i32,
    pub coeff01: i32,
    pub coeff02: i32,
    pub coeff03: i32,
    pub coeff10: i32,
    pub coeff11: i32,
    pub coeff12: i32,
    pub coeff13: i32,
    pub coeff20: i32,
    pub coeff21: i32,
    pub coeff22: i32,
    pub coeff23: i32,
    pub clamp_low: i32,
    pub clamp_high: i32,
}

impl Default for VicColorMatrix {
    fn default() -> Self {
        Self {
            matrix_enable: 0,
            r_shift: 0,
            coeff00: 1,
            coeff01: 0,
            coeff02: 0,
            coeff03: 0,
            coeff10: 0,
            coeff11: 1,
            coeff12: 0,
            coeff13: 0,
            coeff20: 0,
            coeff21: 0,
            coeff22: 1,
            coeff23: 0,
            clamp_low: 0,
            clamp_high: 1023,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VicRectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
