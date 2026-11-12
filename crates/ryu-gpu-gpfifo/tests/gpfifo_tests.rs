use ryu_gpu_gpfifo::ffi::*;
use ryu_gpu_gpfifo::parser::*;
use std::ptr;

// --- STEP 1: INVARIANTS & SAFETY TESTS ---

#[test]
fn test_step1_null_safety() {
    // SAFETY: non-null arguments point to live locals; null ones are rejected before use.
    unsafe {
        let mut count = 0usize;
        let mut state = GpfifoState::default();
        assert_eq!(
            ryu_gpfifo_parse_chunk(ptr::null(), 10, &mut state, ptr::null_mut(), 0, &mut count),
            RYU_GPFIFO_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_gpfifo_parse_chunk(
                [0i32].as_ptr(),
                1,
                ptr::null_mut(),
                ptr::null_mut(),
                0,
                &mut count
            ),
            RYU_GPFIFO_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_gpfifo_parse_chunk(
                [0i32].as_ptr(),
                1,
                &mut state,
                ptr::null_mut(),
                0,
                ptr::null_mut()
            ),
            RYU_GPFIFO_ERR_NULL_PTR
        );
    }
}

#[test]
fn test_step1_zero_length_safety() {
    // SAFETY: all pointers refer to live locals and the lengths match them.
    unsafe {
        let mut count = 99usize;
        let mut state = GpfifoState::default();
        let mut out = [GpfifoParsedMethod {
            method_address: 0,
            argument: 0,
            subchannel: 0,
            is_last_call: 0,
        }; 4];
        assert_eq!(
            ryu_gpfifo_parse_chunk(
                [0i32].as_ptr(),
                0,
                &mut state,
                out.as_mut_ptr(),
                4,
                &mut count
            ),
            RYU_GPFIFO_OK
        );
        assert_eq!(count, 0);
    }
}

// --- STEP 2: DUAL-EXECUTION BIT-IDENTICAL TESTS ---

#[test]
fn test_step2_inc_method_bit_identical() {
    let mut state = GpfifoState::default();
    // IncMethod (sec_op = 1): method 0x200, subchannel 1, count 4
    let header = 0x200 | (1 << 13) | (4 << 16) | (1 << 29);
    let cmds = [header, 10, 20, 30, 40];
    let mut out = [GpfifoParsedMethod {
        method_address: 0,
        argument: 0,
        subchannel: 0,
        is_last_call: 0,
    }; 8];

    let count = parse_pushbuffer_chunk(&cmds, &mut state, &mut out);

    assert_eq!(count, 4);
    assert_eq!(
        out[0],
        GpfifoParsedMethod {
            method_address: 0x200,
            argument: 10,
            subchannel: 1,
            is_last_call: 0
        }
    );
    assert_eq!(
        out[1],
        GpfifoParsedMethod {
            method_address: 0x201,
            argument: 20,
            subchannel: 1,
            is_last_call: 0
        }
    );
    assert_eq!(
        out[2],
        GpfifoParsedMethod {
            method_address: 0x202,
            argument: 30,
            subchannel: 1,
            is_last_call: 0
        }
    );
    assert_eq!(
        out[3],
        GpfifoParsedMethod {
            method_address: 0x203,
            argument: 40,
            subchannel: 1,
            is_last_call: 1
        }
    );
    assert_eq!(state.method_count, 0);
}

#[test]
fn test_step2_non_inc_method_bit_identical() {
    let mut state = GpfifoState::default();
    // NonIncMethod (sec_op = 3): method 0x890, subchannel 0, count 3
    let header = 0x890 | (3 << 16) | (3 << 29);
    let cmds = [header, 111, 222, 333];
    let mut out = [GpfifoParsedMethod {
        method_address: 0,
        argument: 0,
        subchannel: 0,
        is_last_call: 0,
    }; 8];

    let count = parse_pushbuffer_chunk(&cmds, &mut state, &mut out);

    assert_eq!(count, 3);
    assert_eq!(
        out[0],
        GpfifoParsedMethod {
            method_address: 0x890,
            argument: 111,
            subchannel: 0,
            is_last_call: 0
        }
    );
    assert_eq!(
        out[1],
        GpfifoParsedMethod {
            method_address: 0x890,
            argument: 222,
            subchannel: 0,
            is_last_call: 0
        }
    );
    assert_eq!(
        out[2],
        GpfifoParsedMethod {
            method_address: 0x890,
            argument: 333,
            subchannel: 0,
            is_last_call: 1
        }
    );
}

#[test]
fn test_step2_one_inc_method_bit_identical() {
    let mut state = GpfifoState::default();
    // OneInc (sec_op = 5): method 0x300, subchannel 2, count 3
    let header = 0x300 | (2 << 13) | (3 << 16) | (5 << 29);
    let cmds = [header, 1, 2, 3];
    let mut out = [GpfifoParsedMethod {
        method_address: 0,
        argument: 0,
        subchannel: 0,
        is_last_call: 0,
    }; 8];

    let count = parse_pushbuffer_chunk(&cmds, &mut state, &mut out);

    assert_eq!(count, 3);
    assert_eq!(
        out[0],
        GpfifoParsedMethod {
            method_address: 0x300,
            argument: 1,
            subchannel: 2,
            is_last_call: 0
        }
    );
    assert_eq!(
        out[1],
        GpfifoParsedMethod {
            method_address: 0x301,
            argument: 2,
            subchannel: 2,
            is_last_call: 0
        }
    );
    assert_eq!(
        out[2],
        GpfifoParsedMethod {
            method_address: 0x301,
            argument: 3,
            subchannel: 2,
            is_last_call: 1
        }
    );
}

#[test]
fn test_step2_immd_data_method_bit_identical() {
    let mut state = GpfifoState::default();
    // ImmdDataMethod (sec_op = 4): method 0x585, subchannel 0, immd_data = 0x123
    let header = 0x585 | (0x123 << 16) | (4 << 29);
    let cmds = [header];
    let mut out = [GpfifoParsedMethod {
        method_address: 0,
        argument: 0,
        subchannel: 0,
        is_last_call: 0,
    }; 4];

    let count = parse_pushbuffer_chunk(&cmds, &mut state, &mut out);

    assert_eq!(count, 1);
    assert_eq!(
        out[0],
        GpfifoParsedMethod {
            method_address: 0x585,
            argument: 0x123,
            subchannel: 0,
            is_last_call: 1
        }
    );
}

#[test]
fn test_step2_interleaved_pushbuffer_stream() {
    let mut state = GpfifoState::default();

    // Stream combining:
    // 1. IncMethod (0x100, count 2)
    // 2. ImmdDataMethod (0x585, val 0x42)
    // 3. NonIncMethod (0x700, count 2)
    let h1 = 0x100 | (2 << 16) | (1 << 29);
    let h2 = 0x585 | (0x42 << 16) | (4 << 29);
    let h3 = 0x700 | (2 << 16) | (3 << 29);

    let stream = [h1, 10, 20, h2, h3, 100, 200];

    let mut out = [GpfifoParsedMethod {
        method_address: 0,
        argument: 0,
        subchannel: 0,
        is_last_call: 0,
    }; 16];
    let count = parse_pushbuffer_chunk(&stream, &mut state, &mut out);

    assert_eq!(count, 5);
    assert_eq!(out[0].method_address, 0x100);
    assert_eq!(out[1].method_address, 0x101);
    assert_eq!(out[2].method_address, 0x585);
    assert_eq!(out[3].method_address, 0x700);
    assert_eq!(out[4].method_address, 0x700);
}
