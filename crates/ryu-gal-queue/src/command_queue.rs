//! High-performance Lock-Free SPSC command queue for ThreadedRenderer.

use std::sync::atomic::{AtomicUsize, Ordering};

#[repr(align(64))]
struct CachePadded<T>(T);

/// Cache-aligned lock-free Single-Producer Single-Consumer (SPSC) command ring buffer.
pub struct GalSpscQueue {
    buffer: Box<[u8]>,
    capacity: usize,
    element_size: usize,
    head: CachePadded<AtomicUsize>, // Read/consumer position
    tail: CachePadded<AtomicUsize>, // Write/producer position
}

impl GalSpscQueue {
    pub fn new(capacity: usize, element_size: usize) -> Self {
        let total_bytes = capacity * element_size;
        Self {
            buffer: vec![0u8; total_bytes].into_boxed_slice(),
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
        unsafe { (self.buffer.as_ptr() as *mut u8).add(offset) }
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
