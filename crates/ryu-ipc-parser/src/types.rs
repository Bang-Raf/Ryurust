pub const MAX_BUFFERS: usize = 16;
pub const MAX_HANDLES: usize = 16;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IpcMessageType {
    Invalid = 0,
    LegacyRequest = 1,
    CloseSession = 2,
    LegacyControl = 3,
    Request = 4,
    Control = 5,
    RequestWithContext = 6,
    ControlWithContext = 7,
    TipcCloseSession = 0xF,
    TipcCommandRegister = 0x10,
}

impl Default for IpcMessageType {
    fn default() -> Self {
        IpcMessageType::Invalid
    }
}

impl From<u32> for IpcMessageType {
    fn from(val: u32) -> Self {
        match val {
            0 => IpcMessageType::Invalid,
            1 => IpcMessageType::LegacyRequest,
            2 => IpcMessageType::CloseSession,
            3 => IpcMessageType::LegacyControl,
            4 => IpcMessageType::Request,
            5 => IpcMessageType::Control,
            6 => IpcMessageType::RequestWithContext,
            7 => IpcMessageType::ControlWithContext,
            0xF => IpcMessageType::TipcCloseSession,
            0x10 => IpcMessageType::TipcCommandRegister,
            other => {
                if other >= 0x10 {
                    IpcMessageType::TipcCommandRegister
                } else {
                    IpcMessageType::Invalid
                }
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpcBufferDesc {
    pub position: u64,
    pub size: u64,
    pub flags: u8,
    pub _padding: [u8; 7],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpcPtrBufferDesc {
    pub position: u64,
    pub size: u64,
    pub index: u32,
    pub _padding: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpcRecvListBufferDesc {
    pub position: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IpcHandleDescriptor {
    pub has_pid: u32,
    pub _padding: u32,
    pub pid: u64,
    pub copy_count: u32,
    pub move_count: u32,
    pub copy_handles: [i32; MAX_HANDLES],
    pub move_handles: [i32; MAX_HANDLES],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParsedIpcMessage {
    pub msg_type: u32,
    pub handle_desc_enabled: u32,
    pub handle_desc: IpcHandleDescriptor,

    pub ptr_buff_count: u32,
    pub ptr_buffs: [IpcPtrBufferDesc; MAX_BUFFERS],

    pub send_buff_count: u32,
    pub send_buffs: [IpcBufferDesc; MAX_BUFFERS],

    pub recv_buff_count: u32,
    pub recv_buffs: [IpcBufferDesc; MAX_BUFFERS],

    pub xchg_buff_count: u32,
    pub xchg_buffs: [IpcBufferDesc; MAX_BUFFERS],

    pub raw_data_offset: u32,
    pub raw_data_length: u32,

    pub recv_list_count: u32,
    pub _reserved: u32,
    pub recv_list_buffs: [IpcRecvListBufferDesc; MAX_BUFFERS],
}

impl Default for ParsedIpcMessage {
    fn default() -> Self {
        Self {
            msg_type: 0,
            handle_desc_enabled: 0,
            handle_desc: IpcHandleDescriptor::default(),
            ptr_buff_count: 0,
            ptr_buffs: [IpcPtrBufferDesc::default(); MAX_BUFFERS],
            send_buff_count: 0,
            send_buffs: [IpcBufferDesc::default(); MAX_BUFFERS],
            recv_buff_count: 0,
            recv_buffs: [IpcBufferDesc::default(); MAX_BUFFERS],
            xchg_buff_count: 0,
            xchg_buffs: [IpcBufferDesc::default(); MAX_BUFFERS],
            raw_data_offset: 0,
            raw_data_length: 0,
            recv_list_count: 0,
            _reserved: 0,
            recv_list_buffs: [IpcRecvListBufferDesc::default(); MAX_BUFFERS],
        }
    }
}
