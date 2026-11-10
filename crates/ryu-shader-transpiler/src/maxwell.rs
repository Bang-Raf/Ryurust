#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaxwellOpcode {
    Nop = 0,
    Mov = 1,
    Sel = 2,
    FAdd = 3,
    FMul = 4,
    FFma = 5,
    IAdd = 6,
    IMul = 7,
    IScAdd = 8,
    Lop = 9,
    Lop3 = 10,
    Shl = 11,
    Shr = 12,
    Ldc = 13,
    Ldg = 14,
    Stg = 15,
    Tex = 16,
    Bra = 17,
    Exit = 18,
    Unknown = 255,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaxwellInstruction {
    pub offset: u32,
    pub opcode: MaxwellOpcode,
    pub dst_reg: u8,
    pub src_a: u8,
    pub src_b: u8,
    pub immediate: u32,
    pub is_predicated: bool,
    pub pred_reg: u8,
    pub is_last: bool,
}

/// Decodes a 64-bit raw Maxwell instruction word.
pub fn decode_instruction(word: u64, offset: u32) -> MaxwellInstruction {
    let op_bits = ((word >> 56) & 0xFF) as u8;
    let dst_reg = ((word >> 8) & 0xFF) as u8;
    let src_a = ((word >> 16) & 0xFF) as u8;
    let src_b = ((word >> 24) & 0xFF) as u8;
    let imm = ((word >> 32) & 0xFFFFFFFF) as u32;

    let pred_bits = (word & 0x07) as u8;
    let is_pred = pred_bits != 7;

    // Maxwell primary opcode mapping
    let opcode = match op_bits {
        0x50 | 0x51 => MaxwellOpcode::Nop,
        0x5C => MaxwellOpcode::Mov,
        0x5D => MaxwellOpcode::Sel,
        0x58 => MaxwellOpcode::FAdd,
        0x59 => MaxwellOpcode::FMul,
        0x5A => MaxwellOpcode::FFma,
        0x1C => MaxwellOpcode::IAdd,
        0x1D => MaxwellOpcode::IMul,
        0x1E => MaxwellOpcode::IScAdd,
        0x04 | 0x05 => MaxwellOpcode::Lop,
        0x21 => MaxwellOpcode::Lop3,
        0x28 => MaxwellOpcode::Shl,
        0x29 => MaxwellOpcode::Shr,
        0xEF => MaxwellOpcode::Ldc,
        0x38 => MaxwellOpcode::Ldg,
        0x39 => MaxwellOpcode::Stg,
        0xC0 | 0xC1 => MaxwellOpcode::Tex,
        0xE2 | 0xE3 => MaxwellOpcode::Bra,
        0xE4 | 0xE5 => MaxwellOpcode::Exit,
        _ => {
            // Heuristic fallback based on lower byte signature
            match (word >> 20) & 0xF {
                0x0 => MaxwellOpcode::Mov,
                0x1 => MaxwellOpcode::IAdd,
                0x2 => MaxwellOpcode::FAdd,
                0x3 => MaxwellOpcode::FMul,
                0x4 => MaxwellOpcode::Ldc,
                0x5 => MaxwellOpcode::Exit,
                _ => MaxwellOpcode::Unknown,
            }
        }
    };

    let is_last = opcode == MaxwellOpcode::Exit;

    MaxwellInstruction {
        offset,
        opcode,
        dst_reg,
        src_a,
        src_b,
        immediate: imm,
        is_predicated: is_pred,
        pred_reg: pred_bits,
        is_last,
    }
}
