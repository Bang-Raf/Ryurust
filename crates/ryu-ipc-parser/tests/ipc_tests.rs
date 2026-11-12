use ryu_ipc_parser::*;

#[test]
fn test_version() {
    assert_eq!(ryu_ipc_version(), 0x0001_0000);
}

#[test]
fn test_basic_cmif_message_parse() {
    let mut buf = Vec::new();

    // word0: Type=Request(4), ptrBuffCount=1, sendBuffCount=1, recvBuffCount=1, xchgBuffCount=0
    let word0: u32 = 4 | (1 << 16) | (1 << 20) | (1 << 24);
    // word1: rawDataSize=4 words (16 bytes), recvListFlags=2 (count = 2 - 2 = 1), hndDescEnable=1
    let word1: u32 = (4 & 0x3ff) | (2 << 10) | (1 << 31);

    buf.extend_from_slice(&word0.to_le_bytes());
    buf.extend_from_slice(&word1.to_le_bytes());

    // Handle desc: has_pid=1, to_copy=1, to_move=1
    let h_word: u32 = 1 | (1 << 1) | (1 << 5);
    buf.extend_from_slice(&h_word.to_le_bytes());
    buf.extend_from_slice(&123456789u64.to_le_bytes()); // PID
    buf.extend_from_slice(&42i32.to_le_bytes()); // copy handle 0
    buf.extend_from_slice(&99i32.to_le_bytes()); // move handle 0

    // PtrBuff: word0, word1
    let ptr_w0: u32 = (100 << 16) | 5; // size=100, index=5
    let ptr_w1: u32 = 0x8000_1000; // pos
    buf.extend_from_slice(&ptr_w0.to_le_bytes());
    buf.extend_from_slice(&ptr_w1.to_le_bytes());

    // SendBuff: w0 (size), w1 (pos), w2 (flags=1, high bits)
    let send_w0: u32 = 0x2000;
    let send_w1: u32 = 0x9000_0000;
    let send_w2: u32 = 1; // flags=1
    buf.extend_from_slice(&send_w0.to_le_bytes());
    buf.extend_from_slice(&send_w1.to_le_bytes());
    buf.extend_from_slice(&send_w2.to_le_bytes());

    // RecvBuff: w0 (size), w1 (pos), w2 (flags=2)
    let recv_w0: u32 = 0x4000;
    let recv_w1: u32 = 0xA000_0000;
    let recv_w2: u32 = 2; // flags=2
    buf.extend_from_slice(&recv_w0.to_le_bytes());
    buf.extend_from_slice(&recv_w1.to_le_bytes());
    buf.extend_from_slice(&recv_w2.to_le_bytes());

    // Padding pad0 (position in stream)
    // cmdPtr = 0. Current pos = 8 + 4 + 8 + 4 + 4 + 8 + 12 + 12 = 60 bytes.
    // 60 & 0xf = 12 -> pad0 = 16 - 12 = 4 bytes.
    let pad0 = 4usize;
    buf.extend_from_slice(&vec![0u8; pad0]);

    // Raw data: 16 bytes minus pad0 = 12 bytes
    let raw_payload = b"HELLO_SWITCH";
    buf.extend_from_slice(raw_payload);

    // RecvList: packedValue (size in high 16 bits, pos in low 48 bits)
    let recv_list_val: u64 = 0x0050_0000_B000_0000; // size=0x50, pos=0xB0000000
    buf.extend_from_slice(&recv_list_val.to_le_bytes());

    let parsed = parse_ipc_message(&buf, 0).expect("parse should succeed");

    assert_eq!(parsed.msg_type, 4);
    assert_eq!(parsed.handle_desc_enabled, 1);
    assert_eq!(parsed.handle_desc.has_pid, 1);
    assert_eq!(parsed.handle_desc.pid, 123456789);
    assert_eq!(parsed.handle_desc.copy_count, 1);
    assert_eq!(parsed.handle_desc.copy_handles[0], 42);
    assert_eq!(parsed.handle_desc.move_count, 1);
    assert_eq!(parsed.handle_desc.move_handles[0], 99);

    assert_eq!(parsed.ptr_buff_count, 1);
    assert_eq!(parsed.ptr_buffs[0].position, 0x8000_1000);
    assert_eq!(parsed.ptr_buffs[0].size, 100);
    assert_eq!(parsed.ptr_buffs[0].index, 5);

    assert_eq!(parsed.send_buff_count, 1);
    assert_eq!(parsed.send_buffs[0].position, 0x9000_0000);
    assert_eq!(parsed.send_buffs[0].size, 0x2000);
    assert_eq!(parsed.send_buffs[0].flags, 1);

    assert_eq!(parsed.recv_buff_count, 1);
    assert_eq!(parsed.recv_buffs[0].position, 0xA000_0000);
    assert_eq!(parsed.recv_buffs[0].size, 0x4000);
    assert_eq!(parsed.recv_buffs[0].flags, 2);

    assert_eq!(parsed.xchg_buff_count, 0);

    let raw_data_extracted = &buf[parsed.raw_data_offset as usize
        ..(parsed.raw_data_offset + parsed.raw_data_length) as usize];
    assert_eq!(raw_data_extracted, raw_payload);

    assert_eq!(parsed.recv_list_count, 1);
    assert_eq!(parsed.recv_list_buffs[0].position, 0xB000_0000);
    assert_eq!(parsed.recv_list_buffs[0].size, 0x50);
}

#[test]
fn test_tipc_message_parse() {
    let mut buf = Vec::new();

    // TIPC message (Type = 0x10)
    let word0: u32 = 0x10 | (2 << 20); // 2 send buffers
    let word1: u32 = 8; // rawDataSize = 8 words = 32 bytes

    buf.extend_from_slice(&word0.to_le_bytes());
    buf.extend_from_slice(&word1.to_le_bytes());

    // 2 send buffers
    for i in 0..2 {
        let w0: u32 = 0x1000 * (i + 1);
        let w1: u32 = 0x4000_0000 + (i * 0x1000);
        let w2: u32 = 0;
        buf.extend_from_slice(&w0.to_le_bytes());
        buf.extend_from_slice(&w1.to_le_bytes());
        buf.extend_from_slice(&w2.to_le_bytes());
    }

    // TIPC has no padding, raw data immediately follows
    let raw = vec![0xABu8; 32];
    buf.extend_from_slice(&raw);

    let parsed = parse_ipc_message(&buf, 0).expect("TIPC parse should succeed");
    assert_eq!(parsed.msg_type, 0x10);
    assert_eq!(parsed.ptr_buff_count, 0);
    assert_eq!(parsed.send_buff_count, 2);
    assert_eq!(parsed.send_buffs[0].size, 0x1000);
    assert_eq!(parsed.send_buffs[1].size, 0x2000);
    assert_eq!(parsed.raw_data_length, 32);
    assert_eq!(
        &buf[parsed.raw_data_offset as usize..(parsed.raw_data_offset + 32) as usize],
        raw.as_slice()
    );
}

#[test]
fn test_cmif_round_trip_serialization() {
    let mut msg = ParsedIpcMessage {
        msg_type: 4,
        handle_desc_enabled: 1,
        ..Default::default()
    };
    msg.handle_desc.has_pid = 1;
    msg.handle_desc.pid = 987654321;
    msg.handle_desc.copy_count = 1;
    msg.handle_desc.copy_handles[0] = 77;

    msg.ptr_buff_count = 1;
    msg.ptr_buffs[0] = IpcPtrBufferDesc {
        position: 0x5000_0000,
        size: 64,
        index: 3,
        _padding: 0,
    };

    let raw = b"RYUBING_ZERO_ALLOC_IPC";
    let mut out = [0u8; 512];
    let written =
        serialize_cmif(&msg, raw, 0, 0xC000_0000, &mut out).expect("serialize should succeed");
    assert!(written > 0);

    let parsed = parse_ipc_message(&out[..written], 0).expect("re-parse should succeed");
    assert_eq!(parsed.msg_type, 4);
    assert_eq!(parsed.handle_desc.pid, 987654321);
    assert_eq!(parsed.handle_desc.copy_handles[0], 77);
    assert_eq!(parsed.ptr_buffs[0].position, 0x5000_0000);
    assert_eq!(parsed.ptr_buffs[0].size, 64);
    assert_eq!(parsed.ptr_buffs[0].index, 3);
}

#[test]
fn test_ffi_c_abi() {
    let mut msg = ParsedIpcMessage {
        msg_type: 0x10, // TIPC Command
        handle_desc_enabled: 1,
        ..Default::default()
    };
    msg.handle_desc.copy_count = 1;
    msg.handle_desc.copy_handles[0] = 55;

    let raw = b"12345678";
    let mut out_buf = [0u8; 256];
    let mut out_len = 0usize;

    unsafe {
        let ser_res = ryu_ipc_serialize_tipc(
            &msg,
            raw.as_ptr(),
            raw.len(),
            out_buf.as_mut_ptr(),
            out_buf.len(),
            &mut out_len,
        );
        assert_eq!(ser_res, 0);
        assert!(out_len > 0);

        let mut parsed = ParsedIpcMessage::default();
        let parse_res = ryu_ipc_parse(out_buf.as_ptr(), out_len, 0, &mut parsed);
        assert_eq!(parse_res, 0);
        assert_eq!(parsed.msg_type, 0x10);
        assert_eq!(parsed.handle_desc_enabled, 1);
        assert_eq!(parsed.handle_desc.copy_count, 1);
        assert_eq!(parsed.handle_desc.copy_handles[0], 55);
        assert_eq!(parsed.raw_data_length, 8);

        // Test buffer query
        parsed.send_buff_count = 1;
        parsed.send_buffs[0].position = 0x1000;
        parsed.send_buffs[0].size = 0x200;

        let mut pos = 0u64;
        let mut size = 0u64;
        let b_res = ryu_ipc_get_buffer_0x21(&parsed, 0, &mut pos, &mut size);
        assert_eq!(b_res, 0);
        assert_eq!(pos, 0x1000);
        assert_eq!(size, 0x200);
    }
}

#[test]
fn test_truncated_buffer_handling() {
    assert_eq!(
        parse_ipc_message(&[], 0),
        Err(IpcParseError::BufferTooSmall)
    );
    assert_eq!(
        parse_ipc_message(&[0u8; 7], 0),
        Err(IpcParseError::BufferTooSmall)
    );

    // Word0 indicating 1 send buffer (requires at least 8 + 12 = 20 bytes)
    let mut buf = Vec::new();
    let word0: u32 = 4 | (1 << 20);
    let word1: u32 = 0;
    buf.extend_from_slice(&word0.to_le_bytes());
    buf.extend_from_slice(&word1.to_le_bytes());
    buf.extend_from_slice(&[0u8; 8]); // only 8 bytes instead of 12
    assert_eq!(
        parse_ipc_message(&buf, 0),
        Err(IpcParseError::BufferTooSmall)
    );
}

#[test]
fn test_cmif_padding_variations() {
    // Test that pad0 varies properly with (pos + cmd_ptr)
    for cmd_ptr in [0i64, 4, 8, 12, 15, 16, 23] {
        let mut msg = ParsedIpcMessage {
            msg_type: 4,
            ptr_buff_count: 1,
            ..Default::default()
        };
        msg.ptr_buffs[0] = IpcPtrBufferDesc {
            position: 0x1234_5678,
            size: 32,
            index: 1,
            _padding: 0,
        };

        let raw = b"PADDING_TEST_VARIABLE_CMD_PTR";
        let mut out = [0u8; 512];
        let written = serialize_cmif(&msg, raw, cmd_ptr, 0xD000_0000, &mut out)
            .expect("serialization must succeed");

        let parsed = parse_ipc_message(&out[..written], cmd_ptr)
            .expect("deserialization with matching cmd_ptr must succeed");

        assert_eq!(parsed.msg_type, 4);
        assert_eq!(parsed.ptr_buffs[0].position, 0x1234_5678);
        assert_eq!(parsed.ptr_buffs[0].size, 32);

        let extracted_raw = &out[parsed.raw_data_offset as usize
            ..(parsed.raw_data_offset + parsed.raw_data_length) as usize];
        assert!(extracted_raw.starts_with(raw));
    }
}

#[test]
fn test_differential_bitfield_unpacking() {
    // Verify that the 64-bit position and size reconstructed match C#'s formula exactly:
    // Position = word1 | ((word2 << 4) & 0x0f00000000) | ((word2 << 34) & 0x7000000000);
    // Size = word0 | ((word2 << 8) & 0xf00000000);
    let w0: u32 = 0x1234_5678;
    let w1: u32 = 0x8765_4321;
    let w2: u32 = 0b0011_0101_0000_0010; // flags = 2, bits for position and size

    let expected_pos = (w1 as u64)
        | (((w2 as u64) << 4) & 0x0f_0000_0000)
        | (((w2 as u64) << 34) & 0x70_0000_0000);

    let expected_size = (w0 as u64) | (((w2 as u64) << 8) & 0x0f_0000_0000);
    let expected_flags = (w2 & 3) as u8;

    let mut buf = Vec::new();
    let word0: u32 = 4 | (1 << 20); // 1 send buffer
    let word1: u32 = 0;
    buf.extend_from_slice(&word0.to_le_bytes());
    buf.extend_from_slice(&word1.to_le_bytes());
    buf.extend_from_slice(&w0.to_le_bytes());
    buf.extend_from_slice(&w1.to_le_bytes());
    buf.extend_from_slice(&w2.to_le_bytes());

    let parsed = parse_ipc_message(&buf, 0).expect("parse should succeed");
    assert_eq!(parsed.send_buff_count, 1);
    assert_eq!(parsed.send_buffs[0].position, expected_pos);
    assert_eq!(parsed.send_buffs[0].size, expected_size);
    assert_eq!(parsed.send_buffs[0].flags, expected_flags);
}

fn buff(position: u64, size: u64, flags: u8) -> IpcBufferDesc {
    IpcBufferDesc {
        position,
        size,
        flags,
        _padding: [0; 7],
    }
}

fn fill_abw(msg: &mut ParsedIpcMessage) {
    // Exercise the high position bits (32..38) and size bits (32..35).
    msg.send_buff_count = 2;
    msg.send_buffs[0] = buff(0x7f_8765_4321, 0xf_1234_5678, 1);
    msg.send_buffs[1] = buff(0x1000, 0x20, 0);
    msg.recv_buff_count = 3;
    msg.recv_buffs[0] = buff(0x0a_0000_1000, 0x1_0000_0000, 2);
    msg.recv_buffs[1] = buff(0x40_0000_0000, 0x100, 3);
    msg.recv_buffs[2] = buff(0x2000, 0x8_0000_0001, 0);
    msg.xchg_buff_count = 1;
    msg.xchg_buffs[0] = buff(0x35_dead_beef, 0x7_cafe_f00d, 1);
}

fn assert_abw_eq(a: &ParsedIpcMessage, b: &ParsedIpcMessage) {
    assert_eq!(a.send_buff_count, b.send_buff_count);
    assert_eq!(a.recv_buff_count, b.recv_buff_count);
    assert_eq!(a.xchg_buff_count, b.xchg_buff_count);
    assert_eq!(
        a.send_buffs[..a.send_buff_count as usize],
        b.send_buffs[..b.send_buff_count as usize]
    );
    assert_eq!(
        a.recv_buffs[..a.recv_buff_count as usize],
        b.recv_buffs[..b.recv_buff_count as usize]
    );
    assert_eq!(
        a.xchg_buffs[..a.xchg_buff_count as usize],
        b.xchg_buffs[..b.xchg_buff_count as usize]
    );
}

#[test]
fn test_cmif_round_trip_with_abw_descriptors() {
    for cmd_ptr in [0i64, 4, 8, 12, 0x1234_5670 + 0xc] {
        let mut msg = ParsedIpcMessage {
            msg_type: 4,
            handle_desc_enabled: 1,
            ..Default::default()
        };
        msg.handle_desc.has_pid = 1;
        msg.handle_desc.pid = 42;
        msg.handle_desc.move_count = 2;
        msg.handle_desc.move_handles[0] = 9;
        msg.handle_desc.move_handles[1] = -3;
        msg.ptr_buff_count = 1;
        msg.ptr_buffs[0] = IpcPtrBufferDesc {
            position: 0x5000_0000,
            size: 64,
            index: 3,
            _padding: 0,
        };
        fill_abw(&mut msg);

        let raw = b"SFCI\0\0\0\0payload!";
        let mut out = [0u8; 512];
        let written = serialize_cmif(&msg, raw, cmd_ptr, 0xC000_0000, &mut out).expect("serialize");
        let parsed = parse_ipc_message(&out[..written], cmd_ptr).expect("re-parse");

        assert_eq!(parsed.msg_type, 4);
        assert_eq!(parsed.handle_desc.pid, 42);
        assert_eq!(parsed.handle_desc.move_count, 2);
        assert_eq!(parsed.handle_desc.move_handles[..2], [9, -3]);
        assert_eq!(parsed.ptr_buff_count, 1);
        assert_eq!(parsed.ptr_buffs[0], msg.ptr_buffs[0]);
        assert_abw_eq(&parsed, &msg);

        // Raw data lands 16-byte aligned (relative to cmd_ptr) and intact.
        let off = parsed.raw_data_offset as usize;
        assert_eq!((off as i64 + cmd_ptr) & 0xf, 0);
        assert_eq!(&out[off..off + raw.len()], raw);
        assert_eq!(parsed.recv_list_count, 1);
        assert_eq!(parsed.recv_list_buffs[0].position, 0xC000_0000);
    }
}

#[test]
fn test_tipc_round_trip_with_abw_descriptors() {
    let mut msg = ParsedIpcMessage {
        msg_type: 0x10,
        handle_desc_enabled: 1,
        ..Default::default()
    };
    msg.handle_desc.copy_count = 1;
    msg.handle_desc.copy_handles[0] = 55;
    fill_abw(&mut msg);

    // Unaligned raw length: must be zero-padded to the advertised word count.
    let raw = b"12345";
    let mut out = [0xffu8; 256];
    let mut out_len = 0usize;
    // SAFETY: all pointers are valid for the given lengths.
    let res = unsafe {
        ryu_ipc_serialize_tipc(
            &msg,
            raw.as_ptr(),
            raw.len(),
            out.as_mut_ptr(),
            out.len(),
            &mut out_len,
        )
    };
    assert_eq!(res, 0);
    assert_eq!(out_len, 8 + 8 + 6 * 12 + 8);

    let parsed = parse_ipc_message(&out[..out_len], 0).expect("re-parse");
    assert_eq!(parsed.msg_type, 0x10);
    assert_eq!(parsed.handle_desc.copy_handles[0], 55);
    assert_abw_eq(&parsed, &msg);
    let off = parsed.raw_data_offset as usize;
    assert_eq!(parsed.raw_data_length, 8);
    assert_eq!(&out[off..off + 8], b"12345\0\0\0");
}

#[test]
fn test_serialize_rejects_counts_over_header_width() {
    let mut out = [0u8; 4096];
    let msg = ParsedIpcMessage {
        msg_type: 4,
        send_buff_count: 16,
        ..Default::default()
    };
    assert_eq!(
        serialize_cmif(&msg, &[], 0, 0, &mut out),
        Err(IpcSerializeError::InvalidData)
    );
    assert_eq!(
        serialize_tipc(&msg, &[], &mut out),
        Err(IpcSerializeError::InvalidData)
    );

    let mut msg = ParsedIpcMessage {
        handle_desc_enabled: 1,
        ..Default::default()
    };
    msg.handle_desc.copy_count = 16;
    assert_eq!(
        serialize_cmif(&msg, &[], 0, 0, &mut out),
        Err(IpcSerializeError::InvalidData)
    );

    // Too-small output buffer accounts for the descriptor bytes.
    let mut msg = ParsedIpcMessage {
        msg_type: 0x10,
        ..Default::default()
    };
    fill_abw(&mut msg);
    let mut small = [0u8; 8 + 6 * 12 - 1];
    assert_eq!(
        serialize_tipc(&msg, &[], &mut small),
        Err(IpcSerializeError::OutputBufferTooSmall)
    );
}

#[test]
fn test_tipc_rejects_pointer_buffers() {
    // TIPC has no X descriptors; C# asserts there are none. Dropping them silently would
    // send a message the guest did not ask for.
    let msg = ParsedIpcMessage {
        msg_type: 0x10,
        ptr_buff_count: 1,
        ..Default::default()
    };
    let mut out = [0u8; 256];
    assert_eq!(
        serialize_tipc(&msg, &[], &mut out),
        Err(IpcSerializeError::InvalidData)
    );
}
