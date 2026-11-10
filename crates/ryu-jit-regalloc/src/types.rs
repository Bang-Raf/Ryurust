pub const NOT_FOUND: i32 = -1;
pub const MAX_REGISTERS: usize = 64;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegisterType {
    Integer = 0,
    Vector = 1,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub id: u32,
    pub start: i32,
    pub end: i32,
    pub reg_type: u8, // 0 = Integer, 1 = Vector
    pub is_fixed: bool,
    pub fixed_reg: i16,
    pub assigned_reg: i16,
    pub spill_offset: i32,
    pub first_use: i32,
    pub next_use: i32,
    pub weight: f32,
}

impl Default for Interval {
    fn default() -> Self {
        Self {
            id: 0,
            start: 0,
            end: 0,
            reg_type: 0,
            is_fixed: false,
            fixed_reg: -1,
            assigned_reg: -1,
            spill_offset: -1,
            first_use: 0,
            next_use: i32::MAX,
            weight: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegisterMasksNative {
    pub int_available: u32,
    pub vec_available: u32,
    pub registers_count: u32,
}
