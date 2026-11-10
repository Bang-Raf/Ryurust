use crate::arena::BumpArena;
use crate::types::{Interval, RegisterMasksNative, MAX_REGISTERS};

/// Sentinel for "no interval" in the per-register index tables.
const NONE: u32 = u32::MAX;

pub struct LinearScanEngine {
    pub arena: BumpArena,
    pub intervals: Vec<Interval>,
    pub next_spill_offset: i32,
    /// Scratch for `run_pass`: for each fixed interval, the index of the next fixed interval
    /// on the same (register class, register). Kept across passes so its capacity is reused.
    fixed_links: Vec<u32>,
}

impl Default for LinearScanEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl LinearScanEngine {
    pub fn new() -> Self {
        Self {
            arena: BumpArena::with_capacity(256 * 1024), // 256 KB pre-allocated per JIT thread
            intervals: Vec::with_capacity(1024),
            next_spill_offset: 0,
            fixed_links: Vec::with_capacity(1024),
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

    /// Linear-scan register allocation over `self.intervals` (half-open `[start, end)`).
    ///
    /// Each interval either gets a register (`assigned_reg >= 0`) for its whole lifetime or
    /// is spilled (`assigned_reg == -1`, `spill_offset` set); intervals are not split.
    /// Returns the number of intervals that ended up in a register.
    ///
    /// Fixed intervals always get `fixed_reg`. A non-fixed interval is only given a register
    /// that is free now *and* stays free until it ends, i.e. no fixed interval on that
    /// register starts before its end (looked up through a per-register chain of upcoming
    /// fixed intervals). If, when a fixed interval starts, its register is still held by a
    /// non-fixed interval, that interval is evicted and spilled. Hence no two intervals live
    /// at the same time share a register, provided fixed intervals that demand the same
    /// register do not overlap each other (that would be an unsatisfiable input; both keep
    /// their fixed register).
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

        // Index 0 = integer class (`reg_type == 0`), 1 = vector class (anything else).
        let avail = [
            masks.int_available & reg_mask_limit,
            masks.vec_available & reg_mask_limit,
        ];
        let mut active_mask = [0u32; 2];
        let mut end_pos = [[0i32; 32]; 2];
        let mut holder = [[NONE; 32]; 2];

        // Chain the fixed intervals per (class, register) in start order. `next_fixed[c][r]`
        // is the next not-yet-processed fixed interval on that register; `pending_mask[c]`
        // has bit `r` set while there is one.
        let mut next_fixed = [[NONE; 32]; 2];
        self.fixed_links.clear();
        self.fixed_links.resize(count, NONE);
        for j in (0..count).rev() {
            let iv = &self.intervals[j];
            if iv.is_fixed && (0..32).contains(&iv.fixed_reg) {
                let c = (iv.reg_type != 0) as usize;
                let r = iv.fixed_reg as usize;
                self.fixed_links[j] = next_fixed[c][r];
                next_fixed[c][r] = j as u32;
            }
        }
        let mut pending_mask = [0u32; 2];
        for c in 0..2 {
            for r in 0..32 {
                if next_fixed[c][r] != NONE {
                    pending_mask[c] |= 1 << r;
                }
            }
        }

        let mut allocated_count = 0;

        for i in 0..count {
            let cur_start = self.intervals[i].start;
            let cur_end = self.intervals[i].end;
            let c = (self.intervals[i].reg_type != 0) as usize;

            // Expire active registers whose interval ended at or before cur_start
            let mut active = active_mask[c];
            while active != 0 {
                let r = active.trailing_zeros() as usize;
                if end_pos[c][r] <= cur_start {
                    active_mask[c] &= !(1u32 << r);
                }
                active &= active - 1;
            }

            if self.intervals[i].is_fixed {
                let fr = self.intervals[i].fixed_reg;
                self.intervals[i].assigned_reg = fr;
                if (0..32).contains(&fr) {
                    let r = fr as usize;

                    // This fixed interval is now being processed: move the chain on.
                    debug_assert_eq!(next_fixed[c][r], i as u32);
                    next_fixed[c][r] = self.fixed_links[i];
                    if next_fixed[c][r] == NONE {
                        pending_mask[c] &= !(1u32 << r);
                    }

                    let bit = 1u32 << r;
                    let mut end = cur_end;
                    if active_mask[c] & bit != 0 {
                        let h = holder[c][r] as usize;
                        if self.intervals[h].is_fixed {
                            // Overlapping fixed intervals on one register: keep the register
                            // reserved until the later of the two ends.
                            end = end.max(end_pos[c][r]);
                        } else {
                            // Evict the non-fixed holder. Unreachable with the free-until
                            // check below, kept as a safety net.
                            self.intervals[h].assigned_reg = -1;
                            self.intervals[h].spill_offset = self.next_spill_offset;
                            self.next_spill_offset += 8;
                            allocated_count -= 1;
                        }
                    }
                    active_mask[c] |= bit;
                    end_pos[c][r] = end;
                    holder[c][r] = i as u32;
                }
                allocated_count += 1;
            } else {
                let free_mask = avail[c] & !active_mask[c];

                // Registers with no upcoming fixed interval are free forever; otherwise the
                // register must stay free until this interval ends.
                let mut chosen = None;
                let unconstrained = free_mask & !pending_mask[c];
                if unconstrained != 0 {
                    chosen = Some(unconstrained.trailing_zeros() as usize);
                } else {
                    let mut candidates = free_mask & pending_mask[c];
                    while candidates != 0 {
                        let r = candidates.trailing_zeros() as usize;
                        let fixed_start = self.intervals[next_fixed[c][r] as usize].start;
                        if fixed_start >= cur_end {
                            chosen = Some(r);
                            break;
                        }
                        candidates &= candidates - 1;
                    }
                }

                if let Some(r) = chosen {
                    self.intervals[i].assigned_reg = r as i16;
                    active_mask[c] |= 1u32 << r;
                    end_pos[c][r] = cur_end;
                    holder[c][r] = i as u32;
                    allocated_count += 1;
                } else {
                    self.intervals[i].assigned_reg = -1;
                    self.intervals[i].spill_offset = self.next_spill_offset;
                    self.next_spill_offset += 8;
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
