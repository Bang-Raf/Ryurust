use crate::arena::BumpArena;
use crate::types::{Interval, RegisterMasksNative, MAX_REGISTERS};

pub struct LinearScanEngine {
    pub arena: BumpArena,
    pub intervals: Vec<Interval>,
    pub next_spill_offset: i32,
}

impl LinearScanEngine {
    pub fn new() -> Self {
        Self {
            arena: BumpArena::with_capacity(256 * 1024), // 256 KB pre-allocated per JIT thread
            intervals: Vec::with_capacity(1024),
            next_spill_offset: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.arena.reset();
        self.intervals.clear();
        self.next_spill_offset = 0;
    }

    #[inline]
    pub fn add_interval(&mut self, interval: Interval) {
        self.intervals.push(interval);
    }

    /// Fast scan for the index with highest value in `span`.
    #[inline]
    pub fn get_highest_value_index(span: &[i32]) -> usize {
        if span.is_empty() {
            return 0;
        }

        let mut highest = i32::MIN;
        let mut selected = 0;

        for (i, &val) in span.iter().enumerate() {
            if val > highest {
                highest = val;
                selected = i;
                if val == i32::MAX {
                    break;
                }
            }
        }

        selected
    }

    /// Runs zero-allocation O(N) bitmask-accelerated linear scan register allocation.
    pub fn run_pass(&mut self, masks: RegisterMasksNative) -> usize {
        let count = self.intervals.len();
        if count == 0 {
            return 0;
        }

        // Check if already sorted (common in SSA IR) before sorting
        let mut is_sorted = true;
        for i in 1..count {
            if self.intervals[i].start < self.intervals[i - 1].start {
                is_sorted = false;
                break;
            }
        }
        if !is_sorted {
            self.intervals.sort_by_key(|iv| iv.start);
        }

        let reg_count = (masks.registers_count as usize).min(MAX_REGISTERS);
        let reg_mask_limit = if reg_count >= 32 {
            u32::MAX
        } else {
            (1u32 << reg_count) - 1
        };

        let int_avail = masks.int_available & reg_mask_limit;
        let vec_avail = masks.vec_available & reg_mask_limit;

        let mut int_active_mask: u32 = 0;
        let mut vec_active_mask: u32 = 0;
        let mut int_end_pos = [0i32; 32];
        let mut vec_end_pos = [0i32; 32];

        let mut allocated_count = 0;

        for i in 0..count {
            let cur_start = self.intervals[i].start;
            let cur_end = self.intervals[i].end;
            let cur_reg_type = self.intervals[i].reg_type;

            if cur_reg_type == 0 {
                // Expire active integer registers whose interval ended at or before cur_start
                let mut active = int_active_mask;
                while active != 0 {
                    let r = active.trailing_zeros() as usize;
                    if int_end_pos[r] <= cur_start {
                        int_active_mask &= !(1u32 << r);
                    }
                    active &= active - 1;
                }

                if self.intervals[i].is_fixed {
                    let r = self.intervals[i].fixed_reg;
                    self.intervals[i].assigned_reg = r;
                    if r >= 0 && (r as usize) < 32 {
                        int_active_mask |= 1u32 << (r as usize);
                        int_end_pos[r as usize] = cur_end;
                    }
                    allocated_count += 1;
                } else {
                    let free_mask = int_avail & !int_active_mask;
                    if free_mask != 0 {
                        let r = free_mask.trailing_zeros() as usize;
                        self.intervals[i].assigned_reg = r as i16;
                        int_active_mask |= 1u32 << r;
                        int_end_pos[r] = cur_end;
                        allocated_count += 1;
                    } else {
                        self.intervals[i].assigned_reg = -1;
                        self.intervals[i].spill_offset = self.next_spill_offset;
                        self.next_spill_offset += 8;
                    }
                }
            } else {
                // Expire active vector registers whose interval ended at or before cur_start
                let mut active = vec_active_mask;
                while active != 0 {
                    let r = active.trailing_zeros() as usize;
                    if vec_end_pos[r] <= cur_start {
                        vec_active_mask &= !(1u32 << r);
                    }
                    active &= active - 1;
                }

                if self.intervals[i].is_fixed {
                    let r = self.intervals[i].fixed_reg;
                    self.intervals[i].assigned_reg = r;
                    if r >= 0 && (r as usize) < 32 {
                        vec_active_mask |= 1u32 << (r as usize);
                        vec_end_pos[r as usize] = cur_end;
                    }
                    allocated_count += 1;
                } else {
                    let free_mask = vec_avail & !vec_active_mask;
                    if free_mask != 0 {
                        let r = free_mask.trailing_zeros() as usize;
                        self.intervals[i].assigned_reg = r as i16;
                        vec_active_mask |= 1u32 << r;
                        vec_end_pos[r] = cur_end;
                        allocated_count += 1;
                    } else {
                        self.intervals[i].assigned_reg = -1;
                        self.intervals[i].spill_offset = self.next_spill_offset;
                        self.next_spill_offset += 8;
                    }
                }
            }
        }

        allocated_count
    }
}

/// Vectorized bitset OR using AVX2 when available. Returns true if `dst` was modified.
#[inline]
pub fn bitset_or_avx2(dst: &mut [u64], src: &[u64]) -> bool {
    let len = dst.len().min(src.len());
    let mut modified = false;

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && len >= 4 {
            unsafe {
                return bitset_or_avx2_inner(dst, src, len);
            }
        }
    }

    for i in 0..len {
        let old = dst[i];
        let new_val = old | src[i];
        if old != new_val {
            dst[i] = new_val;
            modified = true;
        }
    }
    modified
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn bitset_or_avx2_inner(dst: &mut [u64], src: &[u64], len: usize) -> bool {
    use std::arch::x86_64::*;
    let mut i = 0;
    let d_ptr = dst.as_mut_ptr() as *mut __m256i;
    let s_ptr = src.as_ptr() as *const __m256i;
    let mut acc = _mm256_setzero_si256();

    while i + 4 <= len {
        let vd = _mm256_loadu_si256(d_ptr.add(i / 4));
        let vs = _mm256_loadu_si256(s_ptr.add(i / 4));
        let diff = _mm256_andnot_si256(vd, vs);
        let r = _mm256_or_si256(vd, vs);
        acc = _mm256_or_si256(acc, diff);
        _mm256_storeu_si256(d_ptr.add(i / 4), r);
        i += 4;
    }

    let mut modified = _mm256_testz_si256(acc, acc) == 0;

    while i < len {
        let old = *dst.get_unchecked(i);
        let new_val = old | *src.get_unchecked(i);
        if old != new_val {
            *dst.get_unchecked_mut(i) = new_val;
            modified = true;
        }
        i += 1;
    }
    modified
}

/// Vectorized bitset AND-NOT (dst &= ~src). Returns true if `dst` was modified.
#[inline]
pub fn bitset_and_not(dst: &mut [u64], src: &[u64]) -> bool {
    let len = dst.len().min(src.len());
    let mut modified = false;

    #[cfg(target_arch = "x86_64")]
    {
        if is_x86_feature_detected!("avx2") && len >= 4 {
            unsafe {
                return bitset_and_not_avx2_inner(dst, src, len);
            }
        }
    }

    for i in 0..len {
        let old = dst[i];
        let new_val = old & !src[i];
        if old != new_val {
            dst[i] = new_val;
            modified = true;
        }
    }
    modified
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn bitset_and_not_avx2_inner(dst: &mut [u64], src: &[u64], len: usize) -> bool {
    use std::arch::x86_64::*;
    let mut i = 0;
    let d_ptr = dst.as_mut_ptr() as *mut __m256i;
    let s_ptr = src.as_ptr() as *const __m256i;
    let mut acc = _mm256_setzero_si256();

    while i + 4 <= len {
        let vd = _mm256_loadu_si256(d_ptr.add(i / 4));
        let vs = _mm256_loadu_si256(s_ptr.add(i / 4));
        let diff = _mm256_and_si256(vd, vs);
        let r = _mm256_andnot_si256(vs, vd);
        acc = _mm256_or_si256(acc, diff);
        _mm256_storeu_si256(d_ptr.add(i / 4), r);
        i += 4;
    }

    let mut modified = _mm256_testz_si256(acc, acc) == 0;

    while i < len {
        let old = *dst.get_unchecked(i);
        let new_val = old & !*src.get_unchecked(i);
        if old != new_val {
            *dst.get_unchecked_mut(i) = new_val;
            modified = true;
        }
        i += 1;
    }
    modified
}
