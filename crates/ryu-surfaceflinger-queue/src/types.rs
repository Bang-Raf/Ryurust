pub const NUM_BUFFER_SLOTS: usize = 64;
pub const MAX_ACQUIRED_BUFFERS: usize = NUM_BUFFER_SLOTS - 2;
pub const INVALID_BUFFER_SLOT: i32 = -1;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferState {
    Free = 0,
    Dequeued = 1,
    Queued = 2,
    Acquired = 3,
}

impl BufferState {
    #[inline(always)]
    pub fn from_u8(val: u8) -> Option<Self> {
        match val {
            0 => Some(BufferState::Free),
            1 => Some(BufferState::Dequeued),
            2 => Some(BufferState::Queued),
            3 => Some(BufferState::Acquired),
            _ => None,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ParcelHeader {
    pub payload_size: u32,
    pub payload_offset: u32,
    pub objects_size: u32,
    pub object_offset: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct QueueItem {
    pub slot: i32,
    pub transform: u32,
    pub scaling_mode: u32,
    pub _pad: u32,
    pub frame_number: u64,
    pub timestamp: i64,
}
