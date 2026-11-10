#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecOp {
    Grp0UseTert = 0,
    IncMethod = 1,
    Grp2UseTert = 2,
    NonIncMethod = 3,
    ImmdDataMethod = 4,
    OneInc = 5,
    Reserved6 = 6,
    EndPbSegment = 7,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GpfifoState {
    pub method: u32,
    pub subchannel: u32,
    pub method_count: u32,
    pub non_incrementing: bool,
    pub increment_once: bool,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GpfifoParsedMethod {
    pub method_address: u32,
    pub argument: i32,
    pub subchannel: u32,
    pub is_last_call: u32,
}

pub fn parse_pushbuffer_chunk(
    command_buffer: &[i32],
    state: &mut GpfifoState,
    out_methods: &mut [GpfifoParsedMethod],
) -> usize {
    let mut out_idx = 0;
    let mut cmd_idx = 0;
    let cmd_len = command_buffer.len();
    let out_cap = out_methods.len();

    while cmd_idx < cmd_len && out_idx < out_cap {
        let command = command_buffer[cmd_idx];

        if state.method_count != 0 {
            let is_last = if state.method_count <= 1 { 1 } else { 0 };
            out_methods[out_idx] = GpfifoParsedMethod {
                method_address: state.method,
                argument: command,
                subchannel: state.subchannel,
                is_last_call: is_last,
            };
            out_idx += 1;

            if !state.non_incrementing {
                state.method += 1;
            }

            if state.increment_once {
                state.non_incrementing = true;
            }

            state.method_count -= 1;
            cmd_idx += 1;
        } else {
            let cmd_u32 = command as u32;
            let sec_op = (cmd_u32 >> 29) & 0x7;
            let method_addr = cmd_u32 & 0xfff;
            let subchannel = (cmd_u32 >> 13) & 0x7;
            let method_count = (cmd_u32 >> 16) & 0x1fff;
            let immd_data = ((cmd_u32 >> 16) & 0x1fff) as i32;

            match sec_op {
                1 => {
                    // IncMethod
                    state.method = method_addr;
                    state.subchannel = subchannel;
                    state.method_count = method_count;
                    state.increment_once = false;
                    state.non_incrementing = false;
                    cmd_idx += 1;
                }
                3 => {
                    // NonIncMethod
                    state.method = method_addr;
                    state.subchannel = subchannel;
                    state.method_count = method_count;
                    state.increment_once = false;
                    state.non_incrementing = true;
                    cmd_idx += 1;
                }
                5 => {
                    // OneInc
                    state.method = method_addr;
                    state.subchannel = subchannel;
                    state.method_count = method_count;
                    state.increment_once = true;
                    state.non_incrementing = false;
                    cmd_idx += 1;
                }
                4 => {
                    // ImmdDataMethod
                    out_methods[out_idx] = GpfifoParsedMethod {
                        method_address: method_addr,
                        argument: immd_data,
                        subchannel,
                        is_last_call: 1,
                    };
                    out_idx += 1;
                    cmd_idx += 1;
                }
                7 => {
                    // EndPbSegment
                    break;
                }
                _ => {
                    // Other/NOP
                    cmd_idx += 1;
                }
            }
        }
    }

    out_idx
}
