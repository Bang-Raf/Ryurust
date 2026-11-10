//! High-performance Lock-Free SPSC command queue for ThreadedRenderer.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(align(64))]
struct CachePadded<T>(T);

/// Cache-aligned lock-free Single-Producer Single-Consumer (SPSC) command ring buffer.
pub struct GalSpscQueue {
    /// Slot storage. Wrapped in `UnsafeCell` because producer and consumer write
    /// and read slots through `&self` (via `get_slot_ptr`).
    buffer: Box<[UnsafeCell<u8>]>,
    capacity: usize,
    element_size: usize,
    head: CachePadded<AtomicUsize>, // Read/consumer position
    tail: CachePadded<AtomicUsize>, // Write/producer position
}

// SAFETY: the queue owns its buffer; all shared state other than the slot bytes
// is atomic. Slot bytes are only accessed through raw pointers handed out by
// `get_slot_ptr`, and the SPSC protocol (Acquire/Release on head/tail) ensures a
// slot is accessed by at most one side at a time. Callers of the raw pointers are
// responsible for upholding that protocol, exactly as with the C# ring.
unsafe impl Sync for GalSpscQueue {}
// SAFETY: no thread-affine state; the buffer is a plain heap allocation.
unsafe impl Send for GalSpscQueue {}

/// Allocate a zeroed byte buffer whose bytes are interior-mutable.
pub(crate) fn zeroed_cell_buffer(len: usize) -> Box<[UnsafeCell<u8>]> {
    let raw = Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut [UnsafeCell<u8>];
    // SAFETY: `UnsafeCell<u8>` is `repr(transparent)` over `u8`, so the slice
    // layout is identical and the allocation can be reinterpreted in place.
    unsafe { Box::from_raw(raw) }
}

impl GalSpscQueue {
    pub fn new(capacity: usize, element_size: usize) -> Self {
        let total_bytes = capacity * element_size;
        Self {
            buffer: zeroed_cell_buffer(total_bytes),
            capacity,
            element_size,
            head: CachePadded(AtomicUsize::new(0)),
            tail: CachePadded(AtomicUsize::new(0)),
        }
    }

    /// Try to reserve a slot for the producer.
    /// Returns the slot index (0..capacity) on success, or -1 if full.
    #[inline(always)]
    pub fn try_reserve(&self) -> isize {
        let tail = self.tail.0.load(Ordering::Relaxed);
        let head = self.head.0.load(Ordering::Acquire);

        // Queue is full when tail is capacity slots ahead of head
        if tail.wrapping_sub(head) >= self.capacity {
            return -1;
        }

        let slot = tail % self.capacity;
        slot as isize
    }

    /// Commit the reserved slot, making it available to consumer.
    #[inline(always)]
    pub fn commit(&self) {
        self.tail.0.fetch_add(1, Ordering::Release);
    }

    /// Try to acquire the next available slot for the consumer.
    /// Returns the slot index (0..capacity) on success, or -1 if empty.
    #[inline(always)]
    pub fn try_consume(&self) -> isize {
        let head = self.head.0.load(Ordering::Relaxed);
        let tail = self.tail.0.load(Ordering::Acquire);

        if head == tail {
            return -1; // Empty
        }

        let slot = head % self.capacity;
        slot as isize
    }

    /// Release the consumed slot, freeing it for producer.
    #[inline(always)]
    pub fn release(&self) {
        self.head.0.fetch_add(1, Ordering::Release);
    }

    /// Get raw pointer to element slot in buffer.
    #[inline(always)]
    pub fn get_slot_ptr(&self, slot: usize) -> *mut u8 {
        if slot >= self.capacity {
            return std::ptr::null_mut();
        }
        let offset = slot * self.element_size;
        // SAFETY: `slot < capacity`, so `offset + element_size <= buffer.len()`.
        // The pointer is derived from the `UnsafeCell` slice, so writing through
        // it from `&self` is permitted.
        unsafe { UnsafeCell::raw_get(self.buffer.as_ptr()).add(offset) }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn element_size(&self) -> usize {
        self.element_size
    }

    pub fn pending_count(&self) -> usize {
        let tail = self.tail.0.load(Ordering::Acquire);
        let head = self.head.0.load(Ordering::Acquire);
        tail.wrapping_sub(head)
    }

    pub fn reset(&self) {
        self.head.0.store(0, Ordering::SeqCst);
        self.tail.0.store(0, Ordering::SeqCst);
    }
}
