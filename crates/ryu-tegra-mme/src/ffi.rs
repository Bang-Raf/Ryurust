use crate::mme_interpreter::{MmeInterpreter, MmeMethodCall};
use std::slice;

pub const RYU_MME_OK: i32 = 0;
pub const RYU_MME_ERR_NULL_PTR: i32 = -1;

pub const RYU_TEGRA_MME_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_tegra_mme_version() -> u32 {
    RYU_TEGRA_MME_VERSION
}

/// # Safety
/// `code_ptr` must be readable for `code_len` words and `out_calls_count` must be
/// writable. `fifo_ptr`, `regs_ptr` and `out_calls_ptr` may be null; otherwise each
/// must be valid for its length/capacity (`out_calls_ptr` writable and not
/// overlapping the inputs).
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_tegra_mme_version(), RYU_TEGRA_MME_VERSION);
        assert_eq!(RYU_TEGRA_MME_VERSION, 0x0001_0000);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let code = [0i32; 2];
        let mut count = 7usize;
        // SAFETY: every non-null pointer refers to a live local of the stated length.
        unsafe {
            assert_eq!(
                ryu_mme_execute(
                    ptr::null(),
                    2,
                    0,
                    ptr::null(),
                    4,
                    ptr::null(),
                    4,
                    ptr::null_mut(),
                    4,
                    ptr::null_mut()
                ),
                RYU_MME_ERR_NULL_PTR
            );
            assert_eq!(
                ryu_mme_execute(
                    ptr::null(),
                    2,
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
                    code.as_ptr(),
                    2,
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
            assert_eq!(count, 7);
            // Null fifo/register/output arrays are optional, even with non-zero lengths.
            assert_eq!(
                ryu_mme_execute(
                    code.as_ptr(),
                    2,
                    0,
                    ptr::null(),
                    4,
                    ptr::null(),
                    4,
                    ptr::null_mut(),
                    4,
                    &mut count
                ),
                RYU_MME_OK
            );
        }
    }
}
