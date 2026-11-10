use ryu_shader_transpiler::arena::ShaderArena;
use ryu_shader_transpiler::ffi::*;
use ryu_shader_transpiler::maxwell::*;
use ryu_shader_transpiler::spirv::*;
use ryu_shader_transpiler::transpiler::*;
use std::mem::size_of;

// =========================================================================
// STEP 1: SAFETY, INVARIANTS, NULL POINTER & BOUNDARY VERIFICATION
// =========================================================================

#[test]
fn test_step1_struct_sizes() {
    assert_eq!(size_of::<ShaderTranspileOptions>(), 20);
    assert_eq!(size_of::<ShaderTranspileResult>(), 16);
}

#[test]
fn test_step1_null_pointer_safety() {
    let opts = ShaderTranspileOptions::default();
    let mut res = ShaderTranspileResult::default();
    let mut out_words = [0u32; 128];
    let mut inst_cnt = 0usize;

    unsafe {
        assert_eq!(
            ryu_shader_transpile_maxwell_to_spirv(
                std::ptr::null(),
                0,
                &opts,
                out_words.as_mut_ptr(),
                128,
                &mut res,
            ),
            SHADER_STATUS_NULL_POINTER
        );

        assert_eq!(
            ryu_shader_disassemble_maxwell(std::ptr::null(), 0, &mut inst_cnt),
            SHADER_STATUS_NULL_POINTER
        );
    }
}

#[test]
fn test_step1_invalid_bytecode_alignment() {
    let opts = ShaderTranspileOptions::default();
    let mut res = ShaderTranspileResult::default();
    let mut out_words = [0u32; 128];
    let bad_code = [0u8; 7]; // Not a multiple of 8 bytes!

    unsafe {
        assert_eq!(
            ryu_shader_transpile_maxwell_to_spirv(
                bad_code.as_ptr(),
                bad_code.len(),
                &opts,
                out_words.as_mut_ptr(),
                128,
                &mut res,
            ),
            SHADER_STATUS_INVALID_ARGUMENT
        );
    }
}

#[test]
fn test_step1_insufficient_output_buffer() {
    let opts = ShaderTranspileOptions::default();
    let mut res = ShaderTranspileResult::default();
    let mut out_words = [0u32; 2]; // Way too small!
    let code = [0u8; 16];

    unsafe {
        assert_eq!(
            ryu_shader_transpile_maxwell_to_spirv(
                code.as_ptr(),
                code.len(),
                &opts,
                out_words.as_mut_ptr(),
                2,
                &mut res,
            ),
            SHADER_STATUS_BUFFER_TOO_SMALL
        );
    }
}

// =========================================================================
// STEP 2: DETERMINISTIC DUAL-EXECUTION & BIT-IDENTICAL VERIFICATION
// =========================================================================

#[test]
fn test_step2_maxwell_decoder_deterministic() {
    // FADD R0, R1, R2
    let fadd_word: u64 = 0x5800000002010007;
    let inst = decode_instruction(fadd_word, 0);
    assert_eq!(inst.opcode, MaxwellOpcode::FAdd);
    assert_eq!(inst.dst_reg, 0);
    assert_eq!(inst.src_a, 1);
    assert_eq!(inst.src_b, 2);

    // FMUL R3, R4, R5
    let fmul_word: u64 = 0x5900000005040307;
    let inst2 = decode_instruction(fmul_word, 8);
    assert_eq!(inst2.opcode, MaxwellOpcode::FMul);
    assert_eq!(inst2.dst_reg, 3);

    // EXIT
    let exit_word: u64 = 0xE400000000000007;
    let inst3 = decode_instruction(exit_word, 16);
    assert_eq!(inst3.opcode, MaxwellOpcode::Exit);
    assert!(inst3.is_last);
}

#[test]
fn test_step2_spirv_binary_header_and_opcodes_valid() {
    let mut bytecode = Vec::new();
    // 1. FADD R0, R1, R2
    bytecode.extend_from_slice(&0x5800000002010007u64.to_le_bytes());
    // 2. FMUL R3, R0, R4
    bytecode.extend_from_slice(&0x5900000004000307u64.to_le_bytes());
    // 3. EXIT
    bytecode.extend_from_slice(&0xE400000000000007u64.to_le_bytes());

    let config = TranspileConfig::default();
    let mut arena = ShaderArena::new();

    let output =
        transpile_maxwell_to_spirv(&bytecode, &config, &mut arena).expect("Transpilation failed");

    let words = output.spirv_words;

    // Verify SPIR-V Header
    assert_eq!(words[0], SPIRV_MAGIC, "Invalid SPIR-V Magic number");
    assert_eq!(words[1], SPIRV_VERSION_1_5, "Invalid SPIR-V Version");
    assert_eq!(words[2], RYUBING_GENERATOR_ID, "Invalid Generator ID");
    assert!(words[3] > 0, "Bound ID must be non-zero");
    assert_eq!(words[4], 0, "Reserved word 4 must be 0");

    // Verify that binary contains OpCapability (17), OpMemoryModel (14), OpEntryPoint (15), OpReturn (253)
    let mut has_cap = false;
    let mut has_mem = false;
    let mut has_ep = false;
    let mut has_ret = false;

    let mut i = 5;
    while i < words.len() {
        let first_word = words[i];
        let word_count = (first_word >> 16) as usize;
        let opcode = first_word & 0xFFFF;

        match opcode {
            17 => has_cap = true,
            14 => has_mem = true,
            15 => has_ep = true,
            253 => has_ret = true,
            _ => {}
        }

        assert!(word_count > 0, "Instruction word count must be positive");
        i += word_count;
    }

    assert!(has_cap, "Missing OpCapability");
    assert!(has_mem, "Missing OpMemoryModel");
    assert!(has_ep, "Missing OpEntryPoint");
    assert!(has_ret, "Missing OpReturn");
}

#[test]
fn test_step2_repeatability_100pct_bit_identical() {
    let mut bytecode = Vec::new();
    for i in 0..64 {
        let word: u64 = if i % 4 == 0 {
            0x5800000002010007 // FAdd
        } else if i % 4 == 1 {
            0x5900000004000307 // FMul
        } else if i % 4 == 2 {
            0x1C00000001020307 // IAdd
        } else {
            0x5C00000001000007 // Mov
        };
        bytecode.extend_from_slice(&word.to_le_bytes());
    }
    bytecode.extend_from_slice(&0xE400000000000007u64.to_le_bytes()); // Exit

    let config = TranspileConfig::default();
    let mut arena = ShaderArena::new();

    let out1 = transpile_maxwell_to_spirv(&bytecode, &config, &mut arena).unwrap();
    let out2 = transpile_maxwell_to_spirv(&bytecode, &config, &mut arena).unwrap();

    assert_eq!(
        out1.spirv_words, out2.spirv_words,
        "Transpiler outputs must be 100% bit-identical across successive runs"
    );
}

#[test]
fn test_step2_stage_variations_execution_models() {
    let bytecode = 0xE400000000000007u64.to_le_bytes(); // Exit
    let mut arena = ShaderArena::new();

    for stage_id in [0, 3, 4, 5] {
        let config = TranspileConfig {
            stage: stage_id,
            compute_local_size_x: 64,
            compute_local_size_y: 1,
            compute_local_size_z: 1,
            spirv_version: SPIRV_VERSION_1_5,
        };

        let out = transpile_maxwell_to_spirv(&bytecode, &config, &mut arena).unwrap();
        assert_eq!(out.spirv_words[0], SPIRV_MAGIC);
    }
}
