pub const NOT_FOUND: i32 = -1;
pub const MAX_REGISTERS: usize = 64;

/// Value returned by [`LinearScanEngine::run_pass`](crate::LinearScanEngine::run_pass) (and
/// by `ryu_regalloc_run_pass`) when the input cannot be allocated because two fixed
/// intervals that are live at the same time demand the same register. Read as a signed
/// integer of any width this is `-1`.
pub const RUN_PASS_FIXED_CONFLICT: usize = usize::MAX;

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

/// Two fixed intervals of the same register class are live at the same time and demand the
/// same register, so no valid allocation exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixedRegisterConflict {
    /// `id` of the conflicting interval that starts first.
    pub first_id: u32,
    /// `id` of the interval that starts while `first_id` still holds the register.
    pub second_id: u32,
    /// Register class as stored in [`Interval::reg_type`] (0 = Integer, otherwise Vector).
    pub reg_type: u8,
    /// The register both intervals are fixed to.
    pub reg: i16,
}

impl std::fmt::Display for FixedRegisterConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "fixed intervals {} and {} overlap and both require {} register {}",
            self.first_id,
            self.second_id,
            if self.reg_type == 0 {
                "integer"
            } else {
                "vector"
            },
            self.reg
        )
    }
}

impl std::error::Error for FixedRegisterConflict {}
