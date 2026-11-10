/// Total number of Maxwell 3D configuration registers (0x000 to 0xE00 = 3584 registers).
pub const MAXWELL_NUM_REGISTERS: usize = 0x0E00;

/// Register size in bytes (each register is a 32-bit uint).
pub const REGISTER_SIZE: usize = 4;

/// Total size of the Maxwell 3D state block in bytes (3584 * 4 = 14336 bytes).
pub const MAXWELL_STATE_BYTES: usize = MAXWELL_NUM_REGISTERS * REGISTER_SIZE;

/// Maximum number of distinct register callback groups supported by a 64-bit mask.
pub const MAX_STATE_GROUPS: usize = 64;

/// Standard known group indices in Maxwell 3D engine.
pub const GROUP_VERTEX_BUFFER: u8 = 0;
pub const GROUP_VERTEX_ATTRIB: u8 = 1;
pub const GROUP_BLEND_STATE: u8 = 2;
pub const GROUP_PRIMITIVE_RESTART: u8 = 12;
pub const GROUP_RT_COLOR_MASK: u8 = 14;
pub const GROUP_RASTERIZER: u8 = 15;
pub const GROUP_SCISSOR: u8 = 16;
pub const GROUP_INDEX_BUFFER: u8 = 23;
pub const GROUP_SHADER_STATE: u8 = 26;
pub const GROUP_RENDER_TARGET: u8 = 27;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaxwellStateStats {
    pub dirty_mask: u64,
    pub scanned_bytes: usize,
    pub dirty_groups_count: u32,
}
