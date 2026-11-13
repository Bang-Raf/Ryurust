use crate::arena::BumpArena;
use crate::types::{
    FixedRegisterConflict, Interval, RegisterMasksNative, MAX_REGISTERS, RUN_PASS_FIXED_CONFLICT,
};

/// Sentinel for "no interval" in the per-register index tables.
const NONE: u32 = u32::MAX;

pub struct LinearScanEngine {
    pub arena: BumpArena,
    pub intervals: Vec<Interval>,
    pub next_spill_offset: i32,
    /// Scratch for `run_pass`: for each fixed interval, the index of the next fixed interval
    /// on the same (register class, register). Also used as the permutation buffer when the
    /// input has to be sorted. Kept across passes so its capacity is reused.
    fixed_links: Vec<u32>,
    /// Conflict found by the most recent pass, if any.
    last_conflict: Option<FixedRegisterConflict>,
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
            last_conflict: None,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.arena.reset();
        self.intervals.clear();
        self.next_spill_offset = 0;
        self.last_conflict = None;
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

    /// The fixed-register conflict that made the most recent pass fail, if it did.
    /// Cleared by [`reset`](Self::reset) and at the start of every pass.
    #[inline]
    pub fn fixed_conflict(&self) -> Option<FixedRegisterConflict> {
        self.last_conflict
    }

    /// Same as [`try_run_pass`](Self::try_run_pass), with the error folded into the return
    /// value: returns the number of intervals that ended up in a register, or
    /// [`RUN_PASS_FIXED_CONFLICT`] (`usize::MAX`) if two overlapping fixed intervals demand
    /// the same register. The details are then available from
    /// [`fixed_conflict`](Self::fixed_conflict).
    pub fn run_pass(&mut self, masks: RegisterMasksNative) -> usize {
        self.try_run_pass(masks).unwrap_or(RUN_PASS_FIXED_CONFLICT)
    }

    /// Linear-scan register allocation over `self.intervals` (half-open `[start, end)`).
    ///
    /// The intervals are first ordered by `start` (stable: equal starts keep insertion
    /// order; nothing is allocated for this). Each interval then either gets a register
    /// (`assigned_reg >= 0`) for its whole lifetime or is spilled (`assigned_reg == -1`,
    /// `spill_offset` set); intervals are not split. Returns the number of intervals that
    /// ended up in a register.
    ///
    /// Fixed intervals always get `fixed_reg` and are never spilled. A non-fixed interval is
    /// only given a register that is free now *and* stays free until it ends, i.e. no fixed
    /// interval on that register starts before its end (looked up through a per-register
    /// chain of upcoming fixed intervals). Hence no two intervals live at the same time
    /// share a register.
    ///
    /// # Spilling
    ///
    /// When no register is free for a non-fixed interval, the candidates are that interval
    /// and the non-fixed intervals currently holding a register of its class; the one that
    /// ends last is spilled (ties: the current interval, then the lowest register). If that
    /// is an active interval, it loses its register for its whole lifetime and the current
    /// interval takes the register over.
    ///
    /// This is the furthest-end-point heuristic of Poletto & Sarkar. The C#
    /// `LinearScanAllocator` instead picks the register whose next use is furthest away
    /// (`NextUseAfter`) and splits the evicted interval there; that needs per-interval use
    /// lists and interval splitting, which this data model does not have
    /// ([`Interval::next_use`] is a single value that never advances, and intervals stay
    /// whole), so the end point stands in for the next use. [`Interval::weight`] is not
    /// consulted either; the C# allocator has no spill weights.
    ///
    /// # Errors
    ///
    /// If two non-empty fixed intervals of the same register class and the same `fixed_reg`
    /// overlap, or a fixed interval's `fixed_reg` is outside `0..32` (the register masks
    /// are 32 bits wide), the input is unsatisfiable. The pass then allocates nothing:
    /// the intervals are left sorted, every `assigned_reg` and `spill_offset` is set to
    /// `-1`, and the first such pair (in start order) is returned and kept for
    /// [`fixed_conflict`](Self::fixed_conflict). An out-of-range register is reported first,
    /// as a "pair" of that one interval (`first_id == second_id`).
    pub fn try_run_pass(
        &mut self,
        masks: RegisterMasksNative,
    ) -> Result<usize, FixedRegisterConflict> {
        self.last_conflict = None;

        let count = self.intervals.len();
        if count == 0 {
            return Ok(0);
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
            // Stable sort by start without the temporary buffer `sort_by_key` allocates:
            // unstable-sort the indices on (start, original index), which is a total order
            // and so gives exactly the stable result, then permute the intervals in place.
            let order = &mut self.fixed_links;
            let ivs = &mut self.intervals;
            order.clear();
            order.extend(0..count as u32);
            order.sort_unstable_by_key(|&k| (ivs[k as usize].start, k));

            // `order[j]` is the old index of the interval that belongs at `j`. Follow each
            // cycle once, marking visited slots with `NONE`.
            for i in 0..count {
                if order[i] == NONE {
                    continue;
                }
                let first = ivs[i];
                let mut j = i;
                loop {
                    let k = order[j] as usize;
                    order[j] = NONE;
                    if k == i {
                        ivs[j] = first;
                        break;
                    }
                    ivs[j] = ivs[k];
                    j = k;
                }
            }
        }

        // A fixed interval must name a register the 32-bit masks can describe.
        if let Some(bad) = self
            .intervals
            .iter()
            .find(|iv| iv.is_fixed && !(0..32).contains(&iv.fixed_reg))
        {
            let err = FixedRegisterConflict {
                first_id: bad.id,
                second_id: bad.id,
                reg_type: bad.reg_type,
                reg: bad.fixed_reg,
            };
            for iv in &mut self.intervals {
                iv.assigned_reg = -1;
                iv.spill_offset = -1;
            }
            self.last_conflict = Some(err);
            return Err(err);
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
        for (mask, heads) in pending_mask.iter_mut().zip(&next_fixed) {
            for (r, &head) in heads.iter().enumerate() {
                if head != NONE {
                    *mask |= 1 << r;
                }
            }
        }

        // Two fixed intervals live at the same time on one register cannot both be honoured.
        // Walk each chain, tracking the fixed interval seen so far that ends last. Empty
        // intervals are live nowhere and are ignored.
        let mut conflict: Option<(usize, usize)> = None;
        for c in 0..2 {
            let mut regs = pending_mask[c];
            while regs != 0 {
                let r = regs.trailing_zeros() as usize;
                let mut last = NONE;
                let mut j = next_fixed[c][r];
                while j != NONE {
                    let iv = &self.intervals[j as usize];
                    if iv.end > iv.start {
                        if last == NONE {
                            last = j;
                        } else {
                            let prev = &self.intervals[last as usize];
                            if prev.end > iv.start {
                                // Report the pair whose second interval starts first.
                                if conflict.is_none_or(|(_, second)| (j as usize) < second) {
                                    conflict = Some((last as usize, j as usize));
                                }
                                break;
                            }
                            if iv.end > prev.end {
                                last = j;
                            }
                        }
                    }
                    j = self.fixed_links[j as usize];
                }
                regs &= regs - 1;
            }
        }
        if let Some((first, second)) = conflict {
            let err = FixedRegisterConflict {
                first_id: self.intervals[first].id,
                second_id: self.intervals[second].id,
                reg_type: self.intervals[second].reg_type,
                reg: self.intervals[second].fixed_reg,
            };
            for iv in &mut self.intervals {
                iv.assigned_reg = -1;
                iv.spill_offset = -1;
            }
            self.last_conflict = Some(err);
            return Err(err);
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
                            // Only reachable when this interval is empty (overlapping
                            // non-empty fixed intervals were rejected above): keep the
                            // register reserved until the holder ends.
                            end = end.max(end_pos[c][r]);
                        } else {
                            // Evict the non-fixed holder. Unreachable with the free-until
                            // checks below, kept as a safety net.
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
                    // Nothing is free: spill whichever of the current interval and the
                    // non-fixed active intervals ends last (see "Spilling" above).
                    let mut victim = None;
                    let mut victim_end = cur_end;
                    let mut occupied = avail[c] & active_mask[c];
                    while occupied != 0 {
                        let r = occupied.trailing_zeros() as usize;
                        let bit = 1u32 << r;
                        // The holder was only given `r` because it stays free of fixed
                        // intervals until the holder ends, so this also holds up to the
                        // earlier `cur_end`; checked again to keep that local.
                        if end_pos[c][r] > victim_end
                            && !self.intervals[holder[c][r] as usize].is_fixed
                            && (pending_mask[c] & bit == 0
                                || self.intervals[next_fixed[c][r] as usize].start >= cur_end)
                        {
                            victim = Some(r);
                            victim_end = end_pos[c][r];
                        }
                        occupied &= occupied - 1;
                    }

                    let spilled = if let Some(r) = victim {
                        // The evicted interval and the current one swap places, so the
                        // number of intervals in a register does not change.
                        let h = holder[c][r] as usize;
                        self.intervals[i].assigned_reg = r as i16;
                        end_pos[c][r] = cur_end;
                        holder[c][r] = i as u32;
                        h
                    } else {
                        i
                    };
                    self.intervals[spilled].assigned_reg = -1;
                    self.intervals[spilled].spill_offset = self.next_spill_offset;
                    self.next_spill_offset += 8;
                }
            }
        }

        Ok(allocated_count)
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
            // SAFETY: AVX2 support was just checked at runtime, and `len` is at most the
            // length of both slices.
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

/// # Safety
/// The CPU must support AVX2, and `len <= dst.len().min(src.len())`.
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
            // SAFETY: AVX2 support was just checked at runtime, and `len` is at most the
            // length of both slices.
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

/// # Safety
/// The CPU must support AVX2, and `len <= dst.len().min(src.len())`.
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
