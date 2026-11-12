use crate::arena::ShaderArena;
use crate::transpiler::{transpile_maxwell_to_spirv, TranspileConfig};
use std::cell::RefCell;
use std::slice;

pub const SHADER_STATUS_OK: i32 = 0;
pub const SHADER_STATUS_NULL_POINTER: i32 = -1;
pub const SHADER_STATUS_INVALID_ARGUMENT: i32 = -2;
pub const SHADER_STATUS_BUFFER_TOO_SMALL: i32 = -3;

pub const RYU_SHADER_TRANSPILER_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_shader_transpiler_version() -> u32 {
    RYU_SHADER_TRANSPILER_VERSION
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ShaderTranspileOptions {
    pub stage: u32,
    pub compute_local_size_x: u32,
    pub compute_local_size_y: u32,
    pub compute_local_size_z: u32,
    pub target_spirv_version: u32,
}

impl Default for ShaderTranspileOptions {
    fn default() -> Self {
        Self {
            stage: 4, // Fragment
            compute_local_size_x: 32,
            compute_local_size_y: 1,
            compute_local_size_z: 1,
            target_spirv_version: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct ShaderTranspileResult {
    pub status: i32,
    pub instruction_count: u32,
    pub spirv_word_count: u32,
    pub bytes_emitted: u32,
}

thread_local! {
    static THREAD_ARENA: RefCell<ShaderArena> = RefCell::new(ShaderArena::new());
}

/// # Safety
/// `code_ptr` must be readable for `code_len` bytes and `options` must point to a
/// valid `ShaderTranspileOptions`. `out_spirv` must be writable for
/// `out_spirv_capacity_words` words and `result` for one `ShaderTranspileResult`;
/// neither may overlap the inputs.
#[no_mangle]
pub unsafe extern "C" fn ryu_shader_transpile_maxwell_to_spirv(
    code_ptr: *const u8,
    code_len: usize,
    options: *const ShaderTranspileOptions,
    out_spirv: *mut u32,
    out_spirv_capacity_words: usize,
    result: *mut ShaderTranspileResult,
) -> i32 {
    if code_ptr.is_null() || options.is_null() || out_spirv.is_null() || result.is_null() {
        return SHADER_STATUS_NULL_POINTER;
    }
    if code_len == 0 || !code_len.is_multiple_of(8) {
        (*result).status = SHADER_STATUS_INVALID_ARGUMENT;
        return SHADER_STATUS_INVALID_ARGUMENT;
    }

    let code = slice::from_raw_parts(code_ptr, code_len);
    let opt = &*options;

    let config = TranspileConfig {
        stage: opt.stage,
        compute_local_size_x: opt.compute_local_size_x,
        compute_local_size_y: opt.compute_local_size_y,
        compute_local_size_z: opt.compute_local_size_z,
        spirv_version: opt.target_spirv_version,
    };

    let transpile_res = THREAD_ARENA.with(|arena_cell| {
        let mut arena = arena_cell.borrow_mut();
        transpile_maxwell_to_spirv(code, &config, &mut arena)
    });

    match transpile_res {
        Ok(output) => {
            let words_needed = output.spirv_words.len();
            if words_needed > out_spirv_capacity_words {
                (*result).status = SHADER_STATUS_BUFFER_TOO_SMALL;
                (*result).instruction_count = output.instruction_count as u32;
                (*result).spirv_word_count = words_needed as u32;
                (*result).bytes_emitted = 0;
                return SHADER_STATUS_BUFFER_TOO_SMALL;
            }

            std::ptr::copy_nonoverlapping(output.spirv_words.as_ptr(), out_spirv, words_needed);

            (*result).status = SHADER_STATUS_OK;
            (*result).instruction_count = output.instruction_count as u32;
            (*result).spirv_word_count = words_needed as u32;
            (*result).bytes_emitted = (words_needed * 4) as u32;

            SHADER_STATUS_OK
        }
        Err(err) => {
            (*result).status = err;
            err
        }
    }
}

/// # Safety
/// `out_instruction_count` must be null or writable for one `usize`. `code_ptr` is
/// only null-checked, never read.
#[no_mangle]
pub unsafe extern "C" fn ryu_shader_disassemble_maxwell(
    code_ptr: *const u8,
    code_len: usize,
    out_instruction_count: *mut usize,
) -> i32 {
    if code_ptr.is_null() || out_instruction_count.is_null() {
        return SHADER_STATUS_NULL_POINTER;
    }
    if code_len == 0 || !code_len.is_multiple_of(8) {
        return SHADER_STATUS_INVALID_ARGUMENT;
    }

    *out_instruction_count = code_len / 8;
    SHADER_STATUS_OK
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(
            ryu_shader_transpiler_version(),
            RYU_SHADER_TRANSPILER_VERSION
        );
        assert_eq!(RYU_SHADER_TRANSPILER_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let code = [0u8; 16];
        let opts = ShaderTranspileOptions::default();
        let mut words = [0u32; 64];
        let mut res = ShaderTranspileResult {
            status: 7,
            ..Default::default()
        };
        let mut insts = 7usize;
        // SAFETY: null pointers are rejected before use; every non-null pointer
        // refers to a live local of the stated length.
        unsafe {
            let cp = code.as_ptr();
            let op: *const ShaderTranspileOptions = &opts;
            let wp = words.as_mut_ptr();
            let rp: *mut ShaderTranspileResult = &mut res;
            for (c, o, w, r) in [
                (ptr::null(), op, wp, rp),
                (cp, ptr::null(), wp, rp),
                (cp, op, ptr::null_mut(), rp),
                (cp, op, wp, ptr::null_mut()),
                (ptr::null(), ptr::null(), ptr::null_mut(), ptr::null_mut()),
            ] {
                assert_eq!(
                    ryu_shader_transpile_maxwell_to_spirv(c, 16, o, w, 64, r),
                    SHADER_STATUS_NULL_POINTER
                );
            }
            assert_eq!(res.status, 7);

            assert_eq!(
                ryu_shader_disassemble_maxwell(ptr::null(), 16, &mut insts),
                SHADER_STATUS_NULL_POINTER
            );
            assert_eq!(
                ryu_shader_disassemble_maxwell(cp, 16, ptr::null_mut()),
                SHADER_STATUS_NULL_POINTER
            );
            assert_eq!(insts, 7);
        }
    }
}
