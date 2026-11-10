use crate::mme_interpreter::{MmeInterpreter, MmeMethodCall};
use std::slice;

pub const RYU_MME_OK: i32 = 0;
pub const RYU_MME_ERR_NULL_PTR: i32 = -1;

#[no_mangle]
pub unsafe extern "C" fn ryu_mme_execute(
    code_ptr: *const i32,
    code_len: usize,
    arg0: i32,
    fifo_ptr: *const i32,
    fifo_len: usize,
    regs_ptr: *const i32,
    regs_len: usize,
    out_calls_ptr: *mut MmeMethodCall,
    out_calls_capacity: usize,
    out_calls_count: *mut usize,
) -> i32 {
    if code_ptr.is_null() || out_calls_count.is_null() {
        return RYU_MME_ERR_NULL_PTR;
    }
    if code_len == 0 {
        *out_calls_count = 0;
        return RYU_MME_OK;
    }

    let code = slice::from_raw_parts(code_ptr, code_len);
    let fifo = if !fifo_ptr.is_null() && fifo_len > 0 {
        slice::from_raw_parts(fifo_ptr, fifo_len)
    } else {
        &[]
    };
    let registers = if !regs_ptr.is_null() && regs_len > 0 {
        slice::from_raw_parts(regs_ptr, regs_len)
    } else {
        &[]
    };
    let out_calls = if !out_calls_ptr.is_null() && out_calls_capacity > 0 {
        slice::from_raw_parts_mut(out_calls_ptr, out_calls_capacity)
    } else {
        &mut []
    };

    let mut interpreter = MmeInterpreter::new();
    let emitted = interpreter.execute(code, arg0, fifo, registers, out_calls);
    *out_calls_count = emitted;

    RYU_MME_OK
}
