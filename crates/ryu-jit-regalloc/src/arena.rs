use std::alloc::{alloc, dealloc, handle_alloc_error, Layout};
use std::cell::Cell;
use std::ptr::NonNull;

/// Alignment of the arena's backing allocation.
const ARENA_ALIGN: usize = 64;

/// A simple bump allocator over one fixed block.
///
/// Slices handed out by [`BumpArena::alloc_slice`] borrow the arena, so the borrow checker
/// guarantees they are all gone before [`BumpArena::reset`] (which takes `&mut self`) can
/// recycle their memory, and before the arena is dropped.
pub struct BumpArena {
    ptr: NonNull<u8>,
    capacity: usize,
    offset: Cell<usize>,
}

// SAFETY: the arena exclusively owns its heap block (like a `Box<[u8]>`), so moving it to
// another thread is fine. It is deliberately not `Sync`: `alloc_slice` takes `&self` and
// bumps `offset` through a `Cell`.
unsafe impl Send for BumpArena {}

impl BumpArena {
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(4096);
        let layout = Layout::from_size_align(cap, ARENA_ALIGN).expect("arena capacity overflow");
        // SAFETY: `layout` has non-zero size (`cap >= 4096`).
        let raw = unsafe { alloc(layout) };
        let Some(ptr) = NonNull::new(raw) else {
            handle_alloc_error(layout);
        };
        Self {
            ptr,
            capacity: cap,
            offset: Cell::new(0),
        }
    }

    /// Frees every slice handed out so far. Requires `&mut self`, so no slice from
    /// [`alloc_slice`](Self::alloc_slice) can still be alive.
    #[inline(always)]
    pub fn reset(&mut self) {
        self.offset.set(0);
    }

    /// Allocates `count` elements, each initialised to `T::default()`. Returns `None` when
    /// the arena is exhausted (or `T` needs more than 64-byte alignment). The slice lives as
    /// long as the shared borrow of the arena; several slices can be alive at once.
    #[inline]
    #[allow(clippy::mut_from_ref)] // each call returns a fresh, disjoint region; see SAFETY
    pub fn alloc_slice<T: Copy + Default>(&self, count: usize) -> Option<&mut [T]> {
        let align = std::mem::align_of::<T>();
        if align > ARENA_ALIGN {
            return None;
        }
        let size = std::mem::size_of::<T>().checked_mul(count)?;
        let offset = self.offset.get();
        let aligned_offset = offset.checked_add(align - 1)? & !(align - 1);
        let end = aligned_offset.checked_add(size)?;

        if end > self.capacity {
            return None;
        }
        self.offset.set(end);

        // SAFETY: `aligned_offset + size <= capacity`, so the region is inside the block, and
        // it is aligned for `T` (block is 64-byte aligned, `align <= 64` divides
        // `aligned_offset`). The region `[aligned_offset, end)` lies past every region handed
        // out since the last `reset`, so it does not alias any live slice; `reset` needs
        // `&mut self`, so no returned slice survives it. Every element is written before the
        // slice is created, so no uninitialised memory is exposed.
        unsafe {
            let p = self.ptr.as_ptr().add(aligned_offset).cast::<T>();
            if size != 0 {
                for k in 0..count {
                    p.add(k).write(T::default());
                }
            }
            Some(std::slice::from_raw_parts_mut(p, count))
        }
    }

    #[inline(always)]
    pub fn used_bytes(&self) -> usize {
        self.offset.get()
    }
}

impl Drop for BumpArena {
    fn drop(&mut self) {
        let layout = Layout::from_size_align(self.capacity, ARENA_ALIGN)
            .expect("layout was valid at construction");
        // SAFETY: `ptr` was returned by `alloc(layout)` with this exact layout in
        // `with_capacity` and has not been freed.
        unsafe {
            dealloc(self.ptr.as_ptr(), layout);
        }
    }
}
