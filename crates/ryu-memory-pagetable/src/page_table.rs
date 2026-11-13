use std::ptr;

pub const PAGE_BITS: usize = 12;
pub const PAGE_SIZE: usize = 1 << PAGE_BITS;
pub const PAGE_MASK: u64 = (PAGE_SIZE as u64) - 1;

pub const LEVEL_BITS: usize = 9;
pub const LEVEL_SIZE: usize = 1 << LEVEL_BITS;
pub const LEVEL_MASK: u64 = (LEVEL_SIZE as u64) - 1;

/// One past the highest page index the four levels can address (2^36 pages = 48-bit VA).
pub const PAGE_INDEX_LIMIT: u64 = 1 << (LEVEL_BITS * 4);

pub struct L3Node {
    pub entries: [u64; LEVEL_SIZE],
}

pub struct L2Node {
    pub entries: [*mut L3Node; LEVEL_SIZE],
}

pub struct L1Node {
    pub entries: [*mut L2Node; LEVEL_SIZE],
}

pub struct Radix48PageTable {
    l0: [*mut L1Node; LEVEL_SIZE],
    last_l3_base_va: u64,
    last_l3: *mut L3Node,
}

// SAFETY: the table exclusively owns every node reachable from `l0` (allocated with
// `Box` and freed only by `unmap`/`Drop`), so moving it to another thread is sound.
unsafe impl Send for Radix48PageTable {}
// SAFETY: `&self` methods only read nodes; all mutation requires `&mut self`.
unsafe impl Sync for Radix48PageTable {}

impl Default for Radix48PageTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Radix48PageTable {
    pub fn new() -> Self {
        Self {
            l0: [ptr::null_mut(); LEVEL_SIZE],
            last_l3_base_va: u64::MAX,
            last_l3: ptr::null_mut(),
        }
    }

    #[inline(always)]
    pub fn read(&self, va: u64) -> u64 {
        let l3_idx = ((va >> PAGE_BITS) & LEVEL_MASK) as usize;
        let base_va = va >> (PAGE_BITS + LEVEL_BITS);

        if base_va == self.last_l3_base_va && !self.last_l3.is_null() {
            // SAFETY: `last_l3` is non-null and points at a live L3 node: `unmap` clears
            // the cache before freeing the node it refers to.
            unsafe {
                return (*self.last_l3).entries[l3_idx];
            }
        }

        let l2_idx = ((va >> (PAGE_BITS + LEVEL_BITS)) & LEVEL_MASK) as usize;
        let l1_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 2)) & LEVEL_MASK) as usize;
        let l0_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 3)) & LEVEL_MASK) as usize;

        // SAFETY: every non-null entry points at a live node owned by this table; each
        // level is null-checked before it is dereferenced.
        unsafe {
            let l1 = self.l0[l0_idx];
            if l1.is_null() {
                return 0;
            }
            let l2 = (*l1).entries[l1_idx];
            if l2.is_null() {
                return 0;
            }
            let l3 = (*l2).entries[l2_idx];
            if l3.is_null() {
                return 0;
            }
            (*l3).entries[l3_idx]
        }
    }

    #[inline(always)]
    pub fn read_mut(&mut self, va: u64) -> u64 {
        let l3_idx = ((va >> PAGE_BITS) & LEVEL_MASK) as usize;
        let base_va = va >> (PAGE_BITS + LEVEL_BITS);

        if base_va == self.last_l3_base_va && !self.last_l3.is_null() {
            // SAFETY: as in `read`, a non-null `last_l3` always refers to a live node.
            unsafe {
                return (*self.last_l3).entries[l3_idx];
            }
        }

        let l2_idx = ((va >> (PAGE_BITS + LEVEL_BITS)) & LEVEL_MASK) as usize;
        let l1_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 2)) & LEVEL_MASK) as usize;
        let l0_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 3)) & LEVEL_MASK) as usize;

        // SAFETY: every non-null entry points at a live node owned by this table; each
        // level is null-checked before it is dereferenced.
        unsafe {
            let l1 = self.l0[l0_idx];
            if l1.is_null() {
                return 0;
            }
            let l2 = (*l1).entries[l1_idx];
            if l2.is_null() {
                return 0;
            }
            let l3 = (*l2).entries[l2_idx];
            if l3.is_null() {
                return 0;
            }
            self.last_l3_base_va = base_va;
            self.last_l3 = l3;
            (*l3).entries[l3_idx]
        }
    }

    pub fn map(&mut self, va: u64, value: u64) {
        let l3_idx = ((va >> PAGE_BITS) & LEVEL_MASK) as usize;
        let l2_idx = ((va >> (PAGE_BITS + LEVEL_BITS)) & LEVEL_MASK) as usize;
        let l1_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 2)) & LEVEL_MASK) as usize;
        let l0_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 3)) & LEVEL_MASK) as usize;

        // SAFETY: missing levels are allocated before use, so each pointer dereferenced
        // below is a live node owned by this table, and `&mut self` gives exclusive access.
        unsafe {
            if self.l0[l0_idx].is_null() {
                self.l0[l0_idx] = Box::into_raw(Box::new(L1Node {
                    entries: [ptr::null_mut(); LEVEL_SIZE],
                }));
            }
            let l1 = self.l0[l0_idx];

            if (*l1).entries[l1_idx].is_null() {
                (*l1).entries[l1_idx] = Box::into_raw(Box::new(L2Node {
                    entries: [ptr::null_mut(); LEVEL_SIZE],
                }));
            }
            let l2 = (*l1).entries[l1_idx];

            if (*l2).entries[l2_idx].is_null() {
                (*l2).entries[l2_idx] = Box::into_raw(Box::new(L3Node {
                    entries: [0; LEVEL_SIZE],
                }));
            }
            let l3 = (*l2).entries[l2_idx];

            (*l3).entries[l3_idx] = value;

            self.last_l3_base_va = va >> (PAGE_BITS + LEVEL_BITS);
            self.last_l3 = l3;
        }
    }

    /// Pages covered by `size` bytes, rounded up. Doesn't overflow for `size` near
    /// `u64::MAX`.
    #[inline]
    fn page_count(size: u64) -> u64 {
        size / (PAGE_SIZE as u64) + u64::from(size & PAGE_MASK != 0)
    }

    /// The pages `[first, end)` a range starting at `va` covers, clipped to the 48-bit
    /// address space the table indexes. `end <= first` (empty) if `va` is outside it.
    #[inline]
    fn page_span(va: u64, size: u64) -> (u64, u64) {
        let first = va >> PAGE_BITS;
        let end = first
            .saturating_add(Self::page_count(size))
            .min(PAGE_INDEX_LIMIT);
        (first, end)
    }

    /// Maps every page of the range. The range is clipped to the 48-bit address space
    /// (it used to wrap round to low addresses); each page still gets its own entry, so
    /// a range of terabytes allocates accordingly.
    pub fn map_range(&mut self, va: u64, size: u64, host_ptr_base: u64) {
        let (first, end) = Self::page_span(va, size);
        for i in 0..end.saturating_sub(first) {
            let offset = i << PAGE_BITS;
            self.map(va.wrapping_add(offset), host_ptr_base.wrapping_add(offset));
        }
    }

    pub fn unmap(&mut self, va: u64) {
        let l3_idx = ((va >> PAGE_BITS) & LEVEL_MASK) as usize;
        let l2_idx = ((va >> (PAGE_BITS + LEVEL_BITS)) & LEVEL_MASK) as usize;
        let l1_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 2)) & LEVEL_MASK) as usize;
        let l0_idx = ((va >> (PAGE_BITS + LEVEL_BITS * 3)) & LEVEL_MASK) as usize;

        // SAFETY: each level is null-checked before use and `&mut self` gives exclusive
        // access. A node is freed with `Box::from_raw` exactly once, after which its parent
        // slot (and the `last_l3` cache, reset above) no longer refers to it.
        unsafe {
            let l1 = self.l0[l0_idx];
            if l1.is_null() {
                return;
            }
            let l2 = (*l1).entries[l1_idx];
            if l2.is_null() {
                return;
            }
            let l3 = (*l2).entries[l2_idx];
            if l3.is_null() {
                return;
            }

            (*l3).entries[l3_idx] = 0;

            if self.last_l3 == l3 {
                self.last_l3 = ptr::null_mut();
                self.last_l3_base_va = u64::MAX;
            }

            // Check if L3 is completely empty
            let is_l3_empty = (*l3).entries.iter().all(|&val| val == 0);
            if is_l3_empty {
                drop(Box::from_raw(l3));
                (*l2).entries[l2_idx] = ptr::null_mut();

                let is_l2_empty = (*l2).entries.iter().all(|&p| p.is_null());
                if is_l2_empty {
                    drop(Box::from_raw(l2));
                    (*l1).entries[l1_idx] = ptr::null_mut();

                    let is_l1_empty = (*l1).entries.iter().all(|&p| p.is_null());
                    if is_l1_empty {
                        drop(Box::from_raw(l1));
                        self.l0[l0_idx] = ptr::null_mut();
                    }
                }
            }
        }
    }

    /// Unmaps every page of the range, clipped to the 48-bit address space. Subtrees
    /// with nothing mapped are skipped whole, so the cost follows what is mapped rather
    /// than `size`.
    pub fn unmap_range(&mut self, va: u64, size: u64) {
        let (mut page, end) = Self::page_span(va, size);
        let offset_in_page = va & PAGE_MASK;

        while page < end {
            let l2_idx = ((page >> LEVEL_BITS) & LEVEL_MASK) as usize;
            let l1_idx = ((page >> (LEVEL_BITS * 2)) & LEVEL_MASK) as usize;
            let l0_idx = ((page >> (LEVEL_BITS * 3)) & LEVEL_MASK) as usize;

            // Pages covered by one L1, L2 and L3 node; jump to the next node boundary
            // when a level is missing.
            let skip_to = |span_bits: usize| ((page >> span_bits) + 1) << span_bits;

            let l1 = self.l0[l0_idx];
            if l1.is_null() {
                page = skip_to(LEVEL_BITS * 3);
                continue;
            }
            // SAFETY: `l1` is a non-null node owned by this table, read only here.
            let l2 = unsafe { (*l1).entries[l1_idx] };
            if l2.is_null() {
                page = skip_to(LEVEL_BITS * 2);
                continue;
            }
            // SAFETY: `l2` is a non-null node owned by this table, read only here.
            let l3 = unsafe { (*l2).entries[l2_idx] };
            if l3.is_null() {
                page = skip_to(LEVEL_BITS);
                continue;
            }

            self.unmap((page << PAGE_BITS) | offset_in_page);
            page += 1;
        }
    }
}

impl Drop for Radix48PageTable {
    fn drop(&mut self) {
        for slot in self.l0.iter_mut() {
            let l1 = *slot;
            if !l1.is_null() {
                // SAFETY: all non-null entries are live `Box` allocations owned solely by
                // this table; each is freed exactly once, children before their parent.
                unsafe {
                    for l1_idx in 0..LEVEL_SIZE {
                        let l2 = (*l1).entries[l1_idx];
                        if !l2.is_null() {
                            for l2_idx in 0..LEVEL_SIZE {
                                let l3 = (*l2).entries[l2_idx];
                                if !l3.is_null() {
                                    drop(Box::from_raw(l3));
                                }
                            }
                            drop(Box::from_raw(l2));
                        }
                    }
                    drop(Box::from_raw(l1));
                }
                *slot = ptr::null_mut();
            }
        }
    }
}
