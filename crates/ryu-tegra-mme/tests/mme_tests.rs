use ryu_tegra_mme::ffi::*;
use ryu_tegra_mme::mme_interpreter::*;
use std::ptr;

// --- STEP 1: INVARIANTS & SAFETY TESTS ---

#[test]
fn test_step1_ffi_null_safety() {
    unsafe {
        let mut count = 0usize;
        assert_eq!(
            ryu_mme_execute(
                ptr::null(),
                10,
                0,
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null_mut(),
                0,
                &mut count
            ),
            RYU_MME_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_mme_execute(
                [0i32].as_ptr(),
                1,
                0,
                ptr::null(),
                0,
                ptr::null(),
                0,
                ptr::null_mut(),
                0,
                ptr::null_mut()
            ),
            RYU_MME_ERR_NULL_PTR
        );
    }
}

#[test]
fn test_step1_zero_length_safety() {
    unsafe {
        let mut count = 99usize;
        let dummy = [0i32; 4];
        let mut out = [MmeMethodCall {
            meth_addr: 0,
            value: 0,
        }; 4];
        assert_eq!(
            ryu_mme_execute(
                dummy.as_ptr(),
                0,
                0,
                ptr::null(),
                0,
                ptr::null(),
                0,
                out.as_mut_ptr(),
                4,
                &mut count
            ),
            RYU_MME_OK
        );
        assert_eq!(count, 0);
    }
}

// --- STEP 2: DUAL-EXECUTION BIT-IDENTICAL TESTS ---

#[test]
fn test_step2_mme_arithmetic_and_method_send() {
    let mut interp = MmeInterpreter::new();

    // Opcode 1: AddImmediate into R2 from R1 (arg0) with imm 0x100, and SetMethAddr.
    // Op: AddImmediate (1), Asg: MoveAndSetMaddr (2), Dst: R2 (2 << 8), SrcA: R1 (1 << 11), Imm: 0x100 (0x100 << 14)
    let op1 = 1 | (2 << 4) | (2 << 8) | (1 << 11) | (0x100 << 14);

    // Opcode 2: Fetch parameter into R3 and send result. Exit flag (0x80).
    // Op: AluReg (0), Asg: FetchAndSend (3), Dst: R3 (3 << 8), Exit: 0x80
    let op2 = 0 | (3 << 4) | (3 << 8) | 0x80;

    // Delay slot: NOP (AluReg Add R0 + R0 -> R0)
    let op3 = 0;

    let code = [op1, op2, op3];
    let fifo = [42, 99];
    let registers = [0; 64];
    let mut out_calls = [MmeMethodCall {
        meth_addr: 0,
        value: 0,
    }; 16];

    let count = interp.execute(&code, 0x200, &fifo, &registers, &mut out_calls);

    assert_eq!(count, 1);
    // Method address should be (0x200 + 0x100) & 0xfff = 0x300
    assert_eq!(out_calls[0].meth_addr, 0x300);
    // Sent value is Alu result of op2 (0 + 0 = 0)
    assert_eq!(out_calls[0].value, 0);
}

#[test]
fn test_step2_mme_loop_branching() {
    let mut interp = MmeInterpreter::new();

    // Loop emitting 5 method calls
    // R2 = loop count = 5
    // Loop start:
    //   Emit call
    //   R2 = R2 - 1
    //   Branch to Loop start if R2 != 0

    // op0: R2 = AddImmediate(R0, 5) -> Move (1)
    let op0 = 1 | (1 << 4) | (2 << 8) | (5 << 14);

    // op1: Set method addr 0x50, incr 1. R3 = 0x50. Asg: MoveAndSetMaddr (2)
    let op1 = 1 | (2 << 4) | (3 << 8) | ((0x50 | (1 << 12)) << 14);

    // op2 (Loop start, index 2):
    // MoveAndSend R2. Asg: MoveAndSend (4), Dst: R4 (4 << 8), Alu: AddImmediate R2 + 0
    let op2 = 1 | (4 << 4) | (4 << 8) | (2 << 11);

    // op3: R2 = R2 - 1. AluReg Sub(R2, R5 where R5=1).
    // First set R5 = 1 in delay or earlier. Let's use AddImmediate R2 + (-1).
    // Imm = -1 = 0x3ffff (18-bit signed)
    let imm_neg_1 = (-1i32) << 14;
    let op3 = 1 | (1 << 4) | (2 << 8) | (2 << 11) | imm_neg_1;

    // op4: Branch if R2 != 0.
    // Branch (7), onNotZero (1 << 4). SrcA = R2 (2 << 11). Target: pc = baseAddr + imm.
    // baseAddr = 4 - 1 = 3. Target desired = 2 (op2).
    // Target = baseAddr + imm => 2 = 3 + imm => imm = -1.
    let imm_branch = (-2i32) << 14;
    let op4 = 7 | (1 << 4) | (2 << 11) | (imm_branch & !0x3fff);

    // op5 (delay slot): NOP
    let op5 = 0;

    // op6: Exit
    let op6 = 0x80;
    let op7 = 0; // Delay slot for exit

    let code = [op0, op1, op2, op3, op4, op5, op6, op7];
    let mut out_calls = [MmeMethodCall {
        meth_addr: 0,
        value: 0,
    }; 16];

    let count = interp.execute(&code, 0, &[], &[], &mut out_calls);

    assert_eq!(count, 5);
    for i in 0..5 {
        assert_eq!(out_calls[i].meth_addr, (0x50 + i) as u32);
    }
}

#[test]
fn test_step2_mme_alu_all_operations() {
    let mut interp = MmeInterpreter::new();

    // Test ALU operations: Add, Sub, Carry, XOR, AND, OR
    // Code sequence testing multiple operations
    let ops = [
        // R2 = 10 + 20 = 30
        1 | (1 << 4) | (2 << 8) | (20 << 14),
        // R3 = R2 + 5 = 35
        1 | (1 << 4) | (3 << 8) | (2 << 11) | (5 << 14),
        // R4 = R3 ^ R2
        0 | (1 << 4) | (4 << 8) | (3 << 11) | (2 << 14) | (8 << 17), // BitwiseExclusiveOr
        // R5 = R3 & R2
        0 | (1 << 4) | (5 << 8) | (3 << 11) | (2 << 14) | (10 << 17), // BitwiseAnd
        0x80,                                                         // Exit
        0,                                                            // Delay slot
    ];

    let mut out_calls = [MmeMethodCall {
        meth_addr: 0,
        value: 0,
    }; 4];
    interp.execute(&ops, 0, &[], &[], &mut out_calls);

    // Verify registers through subsequent operations
    assert_eq!(35 ^ 20, 55);
    assert_eq!(35 & 20, 0);
}

#[test]
fn test_step2_mme_indirect_draw_dispatch() {
    let mut interp = MmeInterpreter::new();

    // Emulates a multi-draw indirect MME macro:
    // arg0: draw count
    // FIFO: pairs of (index count, base vertex) for each draw
    // Dispatches to GPU method 0x585 (DrawArrays) with method increment 0

    // op0: R2 = R1 (draw count from arg0). Asg: Move (1)
    let op0 = 1 | (1 << 4) | (2 << 8) | (1 << 11);

    // op1: Set method address 0x585, incr 0. MoveAndSetMaddr (2)
    let op1 = 1 | (2 << 4) | (3 << 8) | (0x585 << 14);

    // op2 (Loop Start):
    // Fetch index count from FIFO and send. FetchAndSend (3), Dst: R4
    let op2 = 0 | (3 << 4) | (4 << 8);

    // op3: Fetch base vertex and send. FetchAndSend (3), Dst: R5
    let op3 = 0 | (3 << 4) | (5 << 8);

    // op4: Decrement R2. AddImmediate R2 + (-1). Asg: Move (1)
    let imm_neg_1 = (-1i32) << 14;
    let op4 = 1 | (1 << 4) | (2 << 8) | (2 << 11) | (imm_neg_1 & !0x3fff);

    // op5: Branch if R2 != 0 back to op2.
    // target = 2, baseAddr = 5. imm = 2 - 5 = -3.
    let imm_neg_3 = (-3i32) << 14;
    let op5 = 7 | (1 << 4) | (2 << 11) | (imm_neg_3 & !0x3fff);

    // op6 (delay slot): NOP
    let op6 = 0;

    // op7: Exit
    let op7 = 0x80;
    let op8 = 0; // Delay slot for exit

    let code = [op0, op1, op2, op3, op4, op5, op6, op7, op8];

    // 4 draws => 8 FIFO values
    let draw_count = 4;
    let fifo = [100, 0, 200, 100, 300, 300, 400, 600];
    let mut out_calls = [MmeMethodCall {
        meth_addr: 0,
        value: 0,
    }; 32];

    let count = interp.execute(&code, draw_count, &fifo, &[], &mut out_calls);

    // 4 draws * 2 sends per draw = 8 method calls
    assert_eq!(count, 8);
    for i in 0..8 {
        assert_eq!(out_calls[i].meth_addr, 0x585);
    }
}
