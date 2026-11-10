use std::alloc::{alloc, dealloc, Layout};

pub struct BumpArena {
    ptr: *mut u8,
    capacity: usize,
    offset: usize,
}

unsafe impl Send for BumpArena {}
unsafe impl Sync for BumpArena {}

impl BumpArena {
    pub fn with_capacity(capacity: usize) -> Self {
        let cap = capacity.max(4096);
        let layout = Layout::from_size_align(cap, 64).unwrap();
        let ptr = unsafe { alloc(layout) };
        Self {
            ptr,
            capacity: cap,
            offset: 0,
        }
    }

    #[inline(always)]
    pub fn reset(&mut self) {
        self.offset = 0;
    }

    #[inline]
    pub fn alloc_slice<T: Copy>(&mut self, count: usize) -> Option<&'static mut [T]> {
        let size = std::mem::size_of::<T>() * count;
        let align = std::mem::align_of::<T>();
        let aligned_offset = (self.offset + align - 1) & !(align - 1);

        if aligned_offset + size > self.capacity {
            return None;
        }

        let p = unsafe { self.ptr.add(aligned_offset) as *mut T };
        self.offset = aligned_offset + size;

        Some(unsafe { std::slice::from_raw_parts_mut(p, count) })
    }

    #[inline(always)]
    pub fn used_bytes(&self) -> usize {
        self.offset
    }
}

impl Drop for BumpArena {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let layout = Layout::from_size_align(self.capacity, 64).unwrap();
            unsafe {
                dealloc(self.ptr, layout);
            }
        }
    }
}
