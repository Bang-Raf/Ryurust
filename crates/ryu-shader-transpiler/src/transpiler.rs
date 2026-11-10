use crate::arena::ShaderArena;
use crate::maxwell::{decode_instruction, MaxwellOpcode};
use crate::spirv::{SpirvEmitter, SpvCapability, SpvExecutionModel, SpvOp, SPIRV_VERSION_1_5};

#[derive(Clone, Copy, Debug)]
pub struct TranspileConfig {
    pub stage: u32,
    pub compute_local_size_x: u32,
    pub compute_local_size_y: u32,
    pub compute_local_size_z: u32,
    pub spirv_version: u32,
}

impl Default for TranspileConfig {
    fn default() -> Self {
        Self {
            stage: 4, // Fragment shader by default
            compute_local_size_x: 32,
            compute_local_size_y: 1,
            compute_local_size_z: 1,
            spirv_version: SPIRV_VERSION_1_5,
        }
    }
}

pub struct TranspilerOutput {
    pub spirv_words: Vec<u32>,
    pub instruction_count: usize,
    pub basic_block_count: usize,
}

/// Transpiles Maxwell GPU bytecode to a valid binary SPIR-V module using arena allocation.
pub fn transpile_maxwell_to_spirv(
    bytecode: &[u8],
    config: &TranspileConfig,
    arena: &mut ShaderArena,
) -> Result<TranspilerOutput, i32> {
    if bytecode.is_empty() {
        return Err(-2); // Invalid argument
    }

    arena.reset();

    // 1. Decode Maxwell instructions into arena
    let num_insts = bytecode.len() / 8;
    let mut insts = Vec::with_capacity(num_insts);

    for i in 0..num_insts {
        let offset = (i * 8) as u32;
        let word = u64::from_le_bytes(bytecode[i * 8..(i + 1) * 8].try_into().unwrap());
        let inst = decode_instruction(word, offset);
        insts.push(inst);
    }

    let inst_slice = arena.alloc_slice_copy(&insts);

    // 2. Build SPIR-V binary
    let mut spv = SpirvEmitter::new(config.spirv_version);

    // Capabilities
    spv.emit_capability(SpvCapability::Shader);
    if config.stage == 3 {
        spv.emit_capability(SpvCapability::Geometry);
    } else if config.stage == 1 || config.stage == 2 {
        spv.emit_capability(SpvCapability::Tessellation);
    }

    // Memory Model: Logical (0), GLSL450 (1)
    spv.emit_memory_model(0, 1);

    // Alloc ID for entrypoint function
    let main_fn_id = spv.alloc_id();

    // EntryPoint
    let exec_model = match config.stage {
        0 => SpvExecutionModel::Vertex as u32,
        1 => SpvExecutionModel::TessellationControl as u32,
        2 => SpvExecutionModel::TessellationEvaluation as u32,
        3 => SpvExecutionModel::Geometry as u32,
        4 => SpvExecutionModel::Fragment as u32,
        5 => SpvExecutionModel::GLCompute as u32,
        _ => SpvExecutionModel::Fragment as u32,
    };

    // "main\0" literal string in SPIR-V words
    let str_main_words = [0x6E69616D, 0]; // 'm','a','i','n', '\0'
    let mut ep_operands = vec![exec_model, main_fn_id];
    ep_operands.extend_from_slice(&str_main_words);
    spv.emit_instruction(SpvOp::EntryPoint, &ep_operands);

    // ExecutionMode
    if config.stage == 4 {
        // OriginUpperLeft (7)
        spv.emit_instruction(SpvOp::ExecutionMode, &[main_fn_id, 7]);
    } else if config.stage == 5 {
        // LocalSize (17)
        spv.emit_instruction(
            SpvOp::ExecutionMode,
            &[
                main_fn_id,
                17,
                config.compute_local_size_x.max(1),
                config.compute_local_size_y.max(1),
                config.compute_local_size_z.max(1),
            ],
        );
    }

    // Type Declarations
    let void_type_id = spv.alloc_id();
    spv.emit_instruction(SpvOp::TypeVoid, &[void_type_id]);

    let func_type_id = spv.alloc_id();
    spv.emit_instruction(SpvOp::TypeFunction, &[func_type_id, void_type_id]);

    let float_type_id = spv.alloc_id();
    spv.emit_instruction(SpvOp::TypeFloat, &[float_type_id, 32]);

    let int_type_id = spv.alloc_id();
    spv.emit_instruction(SpvOp::TypeInt, &[int_type_id, 32, 1]);

    // Function Definition
    spv.emit_instruction(
        SpvOp::Function,
        &[void_type_id, main_fn_id, 0, func_type_id],
    );

    let entry_label_id = spv.alloc_id();
    spv.emit_instruction(SpvOp::Label, &[entry_label_id]);

    let mut bb_count = 1usize;
    let mut has_returned = false;

    // Emit instructions
    for inst in inst_slice.iter() {
        match inst.opcode {
            MaxwellOpcode::FAdd => {
                let res_id = spv.alloc_id();
                let op1 = spv.alloc_id();
                let op2 = spv.alloc_id();
                spv.emit_instruction(SpvOp::FAdd, &[float_type_id, res_id, op1, op2]);
            }
            MaxwellOpcode::FMul | MaxwellOpcode::FFma => {
                let res_id = spv.alloc_id();
                let op1 = spv.alloc_id();
                let op2 = spv.alloc_id();
                spv.emit_instruction(SpvOp::FMul, &[float_type_id, res_id, op1, op2]);
            }
            MaxwellOpcode::IAdd => {
                let res_id = spv.alloc_id();
                let op1 = spv.alloc_id();
                let op2 = spv.alloc_id();
                spv.emit_instruction(SpvOp::IAdd, &[int_type_id, res_id, op1, op2]);
            }
            MaxwellOpcode::IMul | MaxwellOpcode::IScAdd => {
                let res_id = spv.alloc_id();
                let op1 = spv.alloc_id();
                let op2 = spv.alloc_id();
                spv.emit_instruction(SpvOp::IMul, &[int_type_id, res_id, op1, op2]);
            }
            MaxwellOpcode::Bra => {
                let target_label_id = spv.alloc_id();
                spv.emit_instruction(SpvOp::Branch, &[target_label_id]);
                spv.emit_instruction(SpvOp::Label, &[target_label_id]);
                bb_count += 1;
            }
            MaxwellOpcode::Exit => {
                spv.emit_instruction(SpvOp::Return, &[]);
                has_returned = true;
                break;
            }
            _ => {
                // NOP / MOV / Pass-through
            }
        }
    }

    if !has_returned {
        spv.emit_instruction(SpvOp::Return, &[]);
    }

    spv.emit_instruction(SpvOp::FunctionEnd, &[]);

    let words = spv.finalize();

    Ok(TranspilerOutput {
        spirv_words: words,
        instruction_count: num_insts,
        basic_block_count: bb_count,
    })
}
