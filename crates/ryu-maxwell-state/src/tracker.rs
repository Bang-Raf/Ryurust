use crate::types::{MAXWELL_NUM_REGISTERS, MAXWELL_STATE_BYTES, REGISTER_SIZE};

pub struct MaxwellStateTracker {
    /// 1-based group mapping for each register (0 means unmapped/no group callback).
    pub register_to_group: [u8; MAXWELL_NUM_REGISTERS],
    /// 64-bit dirty mask representing dirty state of groups 0..63.
    pub dirty_mask: u64,
    /// Mask of all configured/mapped groups.
    pub mapped_mask: u64,
}

impl MaxwellStateTracker {
    pub fn new() -> Self {
        Self {
            register_to_group: [0u8; MAXWELL_NUM_REGISTERS],
            dirty_mask: 0,
            mapped_mask: 0,
        }
    }

    #[inline(always)]
    pub fn map_register(&mut self, reg_offset: usize, group_index: u8) {
        let index = reg_offset / REGISTER_SIZE;
        if index < MAXWELL_NUM_REGISTERS && group_index < 64 {
            self.register_to_group[index] = group_index + 1;
            self.mapped_mask |= 1u64 << group_index;
        }
    }

    pub fn map_register_range(&mut self, start_offset: usize, count_bytes: usize, group_index: u8) {
        let start_idx = start_offset / REGISTER_SIZE;
        let reg_count = count_bytes.div_ceil(REGISTER_SIZE);
        let end_idx = (start_idx + reg_count).min(MAXWELL_NUM_REGISTERS);

        if group_index < 64 {
            for idx in start_idx..end_idx {
                self.register_to_group[idx] = group_index + 1;
            }
            self.mapped_mask |= 1u64 << group_index;
        }
    }

    #[inline(always)]
    pub fn set_dirty(&mut self, reg_offset: usize) {
        let index = reg_offset / REGISTER_SIZE;
        if index < MAXWELL_NUM_REGISTERS {
            let grp = self.register_to_group[index];
            if grp != 0 {
                self.dirty_mask |= 1u64 << (grp - 1);
            }
        }
    }

    #[inline]
    pub fn set_dirty_batch(&mut self, offsets: &[u32]) {
        for &off in offsets {
            self.set_dirty(off as usize);
        }
    }

    #[inline(always)]
    pub fn force_dirty(&mut self, group_index: u8) {
        if group_index < 64 {
            self.dirty_mask |= 1u64 << group_index;
        }
    }

    #[inline(always)]
    pub fn set_all_dirty(&mut self) {
        self.dirty_mask |= self.mapped_mask;
    }

    #[inline(always)]
    pub fn is_dirty(&self, group_index: u8) -> bool {
        if group_index < 64 {
            (self.dirty_mask & (1u64 << group_index)) != 0
        } else {
            false
        }
    }

    #[inline(always)]
    pub fn get_dirty_mask(&self) -> u64 {
        self.dirty_mask
    }

    #[inline(always)]
    pub fn clear_dirty(&mut self, mask: u64) {
        self.dirty_mask &= !mask;
    }

    /// Fast scan comparing current state registers against shadow state.
    /// Uses AVX2 vectorization on supported x86_64 CPUs, otherwise scalar fallback.
    ///
    /// Newly dirty groups are OR-ed into the accumulated dirty mask, which is
    /// returned. The mask is never cleared here; call [`Self::clear_dirty`]
    /// once the dirty groups have been processed.
    pub fn scan_changes(&mut self, current: &[u8], shadow: &[u8]) -> u64 {
        let len = current.len().min(shadow.len()).min(MAXWELL_STATE_BYTES);

        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                // SAFETY: AVX2 was just detected and both slices are exactly `len` bytes.
                return unsafe { self.scan_changes_avx2(&current[..len], &shadow[..len]) };
            }
        }

        self.scan_changes_scalar(&current[..len], &shadow[..len])
    }

    #[inline(always)]
    fn mark_changed_bytes_mask(&mut self, base_reg: usize, mut changed_byte_mask: u32) {
        while changed_byte_mask != 0 {
            let byte_bit = changed_byte_mask.trailing_zeros() as usize;
            let lane = byte_bit >> 2; // 4 bytes per u32 register
            let reg_idx = base_reg + lane;
            if reg_idx < MAXWELL_NUM_REGISTERS {
                let grp = self.register_to_group[reg_idx];
                if grp != 0 {
                    self.dirty_mask |= 1u64 << (grp - 1);
                }
            }
            // Clear all 4 bits of this 32-bit register lane
            changed_byte_mask &= !(0xFu32 << (lane * 4));
        }
    }

    /// AVX2 256-bit SIMD register scanner (4x unrolled: 128 bytes / 32 registers per step):
    /// Processes 4x32-byte blocks per branch using integer-domain `_mm256_cmpeq_epi32` & `_mm256_movemask_epi8`.
    ///
    /// # Safety
    /// The CPU must support AVX2 and `shadow` must be at least as long as `current`.
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn scan_changes_avx2(&mut self, current: &[u8], shadow: &[u8]) -> u64 {
        use std::arch::x86_64::*;

        let len = current.len();
        let c_ptr = current.as_ptr();
        let s_ptr = shadow.as_ptr();
        let mut offset = 0;

        while offset + 128 <= len {
            let c0 = _mm256_loadu_si256(c_ptr.add(offset) as *const __m256i);
            let s0 = _mm256_loadu_si256(s_ptr.add(offset) as *const __m256i);
            let eq0 = _mm256_cmpeq_epi32(c0, s0);

            let c1 = _mm256_loadu_si256(c_ptr.add(offset + 32) as *const __m256i);
            let s1 = _mm256_loadu_si256(s_ptr.add(offset + 32) as *const __m256i);
            let eq1 = _mm256_cmpeq_epi32(c1, s1);

            let c2 = _mm256_loadu_si256(c_ptr.add(offset + 64) as *const __m256i);
            let s2 = _mm256_loadu_si256(s_ptr.add(offset + 64) as *const __m256i);
            let eq2 = _mm256_cmpeq_epi32(c2, s2);

            let c3 = _mm256_loadu_si256(c_ptr.add(offset + 96) as *const __m256i);
            let s3 = _mm256_loadu_si256(s_ptr.add(offset + 96) as *const __m256i);
            let eq3 = _mm256_cmpeq_epi32(c3, s3);

            let all_eq = _mm256_and_si256(_mm256_and_si256(eq0, eq1), _mm256_and_si256(eq2, eq3));
            if _mm256_movemask_epi8(all_eq) != -1 {
                let base_reg = offset / REGISTER_SIZE;
                let m0 = !(_mm256_movemask_epi8(eq0) as u32);
                if m0 != 0 {
                    self.mark_changed_bytes_mask(base_reg, m0);
                }
                let m1 = !(_mm256_movemask_epi8(eq1) as u32);
                if m1 != 0 {
                    self.mark_changed_bytes_mask(base_reg + 8, m1);
                }
                let m2 = !(_mm256_movemask_epi8(eq2) as u32);
                if m2 != 0 {
                    self.mark_changed_bytes_mask(base_reg + 16, m2);
                }
                let m3 = !(_mm256_movemask_epi8(eq3) as u32);
                if m3 != 0 {
                    self.mark_changed_bytes_mask(base_reg + 24, m3);
                }
            }
            offset += 128;
        }

        while offset + 32 <= len {
            let c0 = _mm256_loadu_si256(c_ptr.add(offset) as *const __m256i);
            let s0 = _mm256_loadu_si256(s_ptr.add(offset) as *const __m256i);
            let eq0 = _mm256_cmpeq_epi32(c0, s0);
            let m0 = !(_mm256_movemask_epi8(eq0) as u32);
            if m0 != 0 {
                self.mark_changed_bytes_mask(offset / REGISTER_SIZE, m0);
            }
            offset += 32;
        }

        // Tail bytes (if any remaining < 32 bytes)
        let tail_start = offset;
        if tail_start < current.len() {
            let tail_c = &current[tail_start..];
            let tail_s = &shadow[tail_start..];
            let tail_regs = tail_c.len() / REGISTER_SIZE;
            for i in 0..tail_regs {
                let off = i * REGISTER_SIZE;
                let reg_idx = (tail_start + off) / REGISTER_SIZE;
                if reg_idx < MAXWELL_NUM_REGISTERS {
                    let c_val = u32::from_le_bytes([
                        tail_c[off],
                        tail_c[off + 1],
                        tail_c[off + 2],
                        tail_c[off + 3],
                    ]);
                    let s_val = u32::from_le_bytes([
                        tail_s[off],
                        tail_s[off + 1],
                        tail_s[off + 2],
                        tail_s[off + 3],
                    ]);
                    if c_val != s_val {
                        let grp = self.register_to_group[reg_idx];
                        if grp != 0 {
                            self.dirty_mask |= 1u64 << (grp - 1);
                        }
                    }
                }
            }
        }

        self.dirty_mask
    }

    /// Scalar fallback: scans 8 bytes (2 registers) at a time.
    pub fn scan_changes_scalar(&mut self, current: &[u8], shadow: &[u8]) -> u64 {
        let reg_count = current.len() / REGISTER_SIZE;
        for reg_idx in 0..reg_count {
            let off = reg_idx * REGISTER_SIZE;
            if off + 4 <= current.len() && off + 4 <= shadow.len() {
                let c_val = u32::from_le_bytes([
                    current[off],
                    current[off + 1],
                    current[off + 2],
                    current[off + 3],
                ]);
                let s_val = u32::from_le_bytes([
                    shadow[off],
                    shadow[off + 1],
                    shadow[off + 2],
                    shadow[off + 3],
                ]);
                if c_val != s_val && reg_idx < MAXWELL_NUM_REGISTERS {
                    let grp = self.register_to_group[reg_idx];
                    if grp != 0 {
                        self.dirty_mask |= 1u64 << (grp - 1);
                    }
                }
            }
        }
        self.dirty_mask
    }

    /// Pops the lowest dirty group index under `check_mask` in O(1) time (< 1 ns).
    #[inline(always)]
    pub fn pop_next_dirty(&mut self, check_mask: u64) -> Option<u32> {
        let active = self.dirty_mask & check_mask;
        if active == 0 {
            None
        } else {
            let group_idx = active.trailing_zeros();
            self.dirty_mask &= !(1u64 << group_idx);
            Some(group_idx)
        }
    }

    /// Collects all dirty groups under `check_mask` into a contiguous slice, clearing them.
    pub fn collect_dirty_groups(&mut self, check_mask: u64, out: &mut [u32]) -> usize {
        let mut active = self.dirty_mask & check_mask;
        if active == 0 {
            return 0;
        }

        let mut count = 0;
        let limit = out.len();
        while active != 0 && count < limit {
            let group_idx = active.trailing_zeros();
            out[count] = group_idx;
            count += 1;
            self.dirty_mask &= !(1u64 << group_idx);
            active &= active - 1;
        }

        count
    }
}

impl Default for MaxwellStateTracker {
    fn default() -> Self {
        Self::new()
    }
}
