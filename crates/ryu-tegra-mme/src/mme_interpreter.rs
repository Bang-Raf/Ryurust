#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MmeMethodCall {
    pub meth_addr: u32,
    pub value: i32,
}

pub struct MmeInterpreter {
    gprs: [i32; 8],
    meth_addr: i32,
    meth_incr: i32,
    carry: bool,
    op_code: i32,
    pipe_op: i32,
    ignore_exit_flag: bool,
    pc: usize,
    last_run_hit_step_limit: bool,
}

/// Maximum number of instructions one macro invocation may execute before it is treated as
/// runaway and stopped (16,777,216). Real macros run a handful of instructions per FIFO
/// parameter; even a macro consuming hundreds of thousands of parameters stays far below
/// this, while an infinite loop is cut off after a bounded amount of work (tens of
/// milliseconds in a release build).
pub const MME_MAX_STEPS: u64 = 1 << 24;

impl Default for MmeInterpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl MmeInterpreter {
    pub fn new() -> Self {
        Self {
            gprs: [0; 8],
            meth_addr: 0,
            meth_incr: 0,
            carry: false,
            op_code: 0,
            pipe_op: 0,
            ignore_exit_flag: false,
            pc: 0,
            last_run_hit_step_limit: false,
        }
    }

    pub fn reset(&mut self) {
        self.gprs = [0; 8];
        self.meth_addr = 0;
        self.meth_incr = 0;
        self.carry = false;
        self.op_code = 0;
        self.pipe_op = 0;
        self.ignore_exit_flag = false;
        self.pc = 0;
        self.last_run_hit_step_limit = false;
    }

    #[inline(always)]
    fn fetch_op_code(&mut self, code: &[i32]) {
        self.op_code = self.pipe_op;
        if self.pc < code.len() {
            self.pipe_op = code[self.pc];
            self.pc += 1;
        } else {
            self.pipe_op = 0;
            self.pc += 1;
        }
    }

    #[inline(always)]
    fn get_imm(&self) -> i32 {
        self.op_code >> 14
    }

    #[inline(always)]
    fn get_gpr_value(&self, index: usize) -> i32 {
        if index != 0 && index < 8 {
            self.gprs[index]
        } else {
            0
        }
    }

    #[inline(always)]
    fn get_gpr_a(&self) -> i32 {
        self.get_gpr_value(((self.op_code >> 11) & 7) as usize)
    }

    #[inline(always)]
    fn get_gpr_b(&self) -> i32 {
        self.get_gpr_value(((self.op_code >> 14) & 7) as usize)
    }

    #[inline(always)]
    fn set_dst_gpr(&mut self, value: i32) {
        let index = ((self.op_code >> 8) & 7) as usize;
        if index < 8 {
            self.gprs[index] = value;
        }
    }

    #[inline(always)]
    fn set_meth_addr(&mut self, value: i32) {
        self.meth_addr = value & 0xfff;
        self.meth_incr = (value >> 12) & 0x3f;
    }

    #[inline(always)]
    fn send(&mut self, out_calls: &mut [MmeMethodCall], out_pos: &mut usize, value: i32) {
        if *out_pos < out_calls.len() {
            out_calls[*out_pos] = MmeMethodCall {
                meth_addr: self.meth_addr as u32,
                value,
            };
            *out_pos += 1;
        }
        self.meth_addr = self.meth_addr.wrapping_add(self.meth_incr);
    }

    #[inline(always)]
    fn fetch_param(fifo: &[i32], fifo_pos: &mut usize) -> i32 {
        if *fifo_pos < fifo.len() {
            let val = fifo[*fifo_pos];
            *fifo_pos += 1;
            val
        } else {
            0
        }
    }

    #[inline(always)]
    fn read_gpu_reg(registers: &[i32], reg: i32) -> i32 {
        if reg >= 0 && (reg as usize) < registers.len() {
            registers[reg as usize]
        } else {
            0
        }
    }

    fn get_alu_result(&mut self, registers: &[i32]) -> i32 {
        let op = self.op_code & 7;
        match op {
            0 => {
                // AluReg
                let alu_op = (self.op_code >> 17) & 0x1f;
                let a = self.get_gpr_a();
                let b = self.get_gpr_b();
                self.get_alu_reg_result(alu_op, a, b)
            }
            1 => {
                // AddImmediate
                self.get_gpr_a().wrapping_add(self.get_imm())
            }
            2..=4 => {
                // Bitfield operations
                let bf_src_bit = ((self.op_code >> 17) & 0x1f) as u32;
                let bf_size = ((self.op_code >> 22) & 0x1f) as u32;
                let bf_dst_bit = ((self.op_code >> 27) & 0x1f) as u32;

                let bf_mask = if bf_size >= 32 {
                    !0i32
                } else {
                    (1i32 << bf_size).wrapping_sub(1)
                };

                let mut dst = self.get_gpr_a();
                let mut src = self.get_gpr_b();

                match op {
                    2 => {
                        // BitfieldReplace
                        src = (((src as u32).wrapping_shr(bf_src_bit)) as i32) & bf_mask;
                        dst &= !(bf_mask.wrapping_shl(bf_dst_bit));
                        dst |= src.wrapping_shl(bf_dst_bit);
                        dst
                    }
                    3 => {
                        // BitfieldExtractLslImm
                        src = (((src as u32).wrapping_shr(dst as u32)) as i32) & bf_mask;
                        src.wrapping_shl(bf_dst_bit)
                    }
                    4 => {
                        // BitfieldExtractLslReg
                        src = (((src as u32).wrapping_shr(bf_src_bit)) as i32) & bf_mask;
                        src.wrapping_shl(dst as u32)
                    }
                    _ => 0,
                }
            }
            5 => {
                // ReadImmediate
                Self::read_gpu_reg(registers, self.get_gpr_a().wrapping_add(self.get_imm()))
            }
            _ => 0,
        }
    }

    #[inline(always)]
    fn get_alu_reg_result(&mut self, alu_op: i32, a: i32, b: i32) -> i32 {
        let a_u64 = (a as i64) as u64;
        let b_u64 = (b as i64) as u64;

        match alu_op {
            0 => {
                // Add
                let result = a_u64.wrapping_add(b_u64);
                self.carry = result > 0xffff_ffff;
                result as i32
            }
            1 => {
                // AddWithCarry
                let carry_val = if self.carry { 1u64 } else { 0u64 };
                let result = a_u64.wrapping_add(b_u64).wrapping_add(carry_val);
                self.carry = result > 0xffff_ffff;
                result as i32
            }
            2 => {
                // Subtract
                let result = a_u64.wrapping_sub(b_u64);
                self.carry = result < 0x1_0000_0000;
                result as i32
            }
            3 => {
                // SubtractWithBorrow
                let borrow_val = if self.carry { 0u64 } else { 1u64 };
                let result = a_u64.wrapping_sub(b_u64).wrapping_sub(borrow_val);
                self.carry = result < 0x1_0000_0000;
                result as i32
            }
            8 => a ^ b,
            9 => a | b,
            10 => a & b,
            11 => a & !b,
            12 => !(a & b),
            _ => 0,
        }
    }

    pub fn step(
        &mut self,
        code: &[i32],
        registers: &[i32],
        fifo: &[i32],
        fifo_pos: &mut usize,
        out_calls: &mut [MmeMethodCall],
        out_pos: &mut usize,
    ) -> bool {
        let base_addr = (self.pc as i32).wrapping_sub(1);

        self.fetch_op_code(code);

        if (self.op_code & 7) < 7 {
            let asg_op = (self.op_code >> 4) & 7;
            let result = self.get_alu_result(registers);

            match asg_op {
                0 => {
                    // IgnoreAndFetch
                    let p = Self::fetch_param(fifo, fifo_pos);
                    self.set_dst_gpr(p);
                }
                1 => {
                    // Move
                    self.set_dst_gpr(result);
                }
                2 => {
                    // MoveAndSetMaddr
                    self.set_dst_gpr(result);
                    self.set_meth_addr(result);
                }
                3 => {
                    // FetchAndSend
                    let p = Self::fetch_param(fifo, fifo_pos);
                    self.set_dst_gpr(p);
                    self.send(out_calls, out_pos, result);
                }
                4 => {
                    // MoveAndSend
                    self.set_dst_gpr(result);
                    self.send(out_calls, out_pos, result);
                }
                5 => {
                    // FetchAndSetMaddr
                    let p = Self::fetch_param(fifo, fifo_pos);
                    self.set_dst_gpr(p);
                    self.set_meth_addr(result);
                }
                6 => {
                    // MoveAndSetMaddrThenFetchAndSend
                    self.set_dst_gpr(result);
                    self.set_meth_addr(result);
                    let p = Self::fetch_param(fifo, fifo_pos);
                    self.send(out_calls, out_pos, p);
                }
                7 => {
                    // MoveAndSetMaddrThenSendHigh
                    self.set_dst_gpr(result);
                    self.set_meth_addr(result);
                    let high = (result >> 12) & 0x3f;
                    self.send(out_calls, out_pos, high);
                }
                _ => {}
            }
        } else {
            // Branch
            let on_not_zero = ((self.op_code >> 4) & 1) != 0;
            let taken = if on_not_zero {
                self.get_gpr_a() != 0
            } else {
                self.get_gpr_a() == 0
            };

            if taken {
                let target = base_addr.wrapping_add(self.get_imm());
                self.pc = target.max(0) as usize;

                let no_delays = (self.op_code & 0x20) != 0;
                if no_delays {
                    self.fetch_op_code(code);
                } else {
                    self.ignore_exit_flag = true;
                }

                return true;
            }
        }

        let exit = (self.op_code & 0x80) != 0 && !self.ignore_exit_flag;
        self.ignore_exit_flag = false;
        !exit
    }

    /// Runs a macro and returns the number of method calls written to `out_calls`.
    ///
    /// Execution stops when the macro exits, or after [`MME_MAX_STEPS`] executed
    /// instructions (a runaway / infinitely looping macro), whichever comes first.
    pub fn execute(
        &mut self,
        code: &[i32],
        arg0: i32,
        fifo: &[i32],
        registers: &[i32],
        out_calls: &mut [MmeMethodCall],
    ) -> usize {
        self.execute_with_step_limit(code, arg0, fifo, registers, out_calls, MME_MAX_STEPS)
    }

    /// [`execute`](Self::execute) with an explicit instruction budget. When the budget is
    /// used up the macro is abandoned (the pending delay-slot instruction is not run).
    pub fn execute_with_step_limit(
        &mut self,
        code: &[i32],
        arg0: i32,
        fifo: &[i32],
        registers: &[i32],
        out_calls: &mut [MmeMethodCall],
        max_steps: u64,
    ) -> usize {
        if code.is_empty() {
            return 0;
        }

        self.reset();
        self.gprs[1] = arg0;
        self.pc = 0;

        let mut fifo_pos = 0;
        let mut out_pos = 0;

        self.fetch_op_code(code);

        let mut steps: u64 = 0;
        loop {
            if steps >= max_steps {
                // Runaway macro: stop without running the exit delay slot.
                self.last_run_hit_step_limit = true;
                return out_pos;
            }
            steps += 1;
            if !self.step(
                code,
                registers,
                fifo,
                &mut fifo_pos,
                out_calls,
                &mut out_pos,
            ) {
                break;
            }
            if self.pc > code.len() + 1000 {
                // Ran far past the end of the code: only zero opcodes (no exit, no branch)
                // can follow, so stop early instead of burning the whole step budget.
                break;
            }
        }

        // Delay slot final instruction execution
        self.step(
            code,
            registers,
            fifo,
            &mut fifo_pos,
            out_calls,
            &mut out_pos,
        );

        out_pos
    }

    /// True if the most recent `execute*` call was stopped by the step budget rather than by
    /// the macro exiting.
    pub fn last_run_hit_step_limit(&self) -> bool {
        self.last_run_hit_step_limit
    }
}
