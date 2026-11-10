//! High-throughput Lock-Free Circular Span Pool for GAL Multithreading.

use std::sync::atomic::{AtomicI32, AtomicIsize, Ordering};

/// Cacheline padding helper to prevent false sharing.
#[repr(align(64))]
struct CachePadded<T>(T);

/// Lock-free circular span pool with wraparound awareness.
/// Replaces `Ryujinx.Graphics.GAL.Multithreading.Model.CircularSpanPool`.
pub struct GalSpanPool {
    pool: Box<[u8]>,
    size: usize,
    producer_ptr: CachePadded<AtomicIsize>,
    consumer_ptr: CachePadded<AtomicIsize>,
    skip_position: AtomicI32,
}

impl GalSpanPool {
    pub fn new(size: usize) -> Self {
        Self {
            pool: vec![0u8; size].into_boxed_slice(),
            size,
            producer_ptr: CachePadded(AtomicIsize::new(0)),
            consumer_ptr: CachePadded(AtomicIsize::new(0)),
            skip_position: AtomicI32::new(-1),
        }
    }

    /// Try to insert a slice into the circular pool.
    /// Returns the offset into the pool on success, or -1 if the pool is full.
    pub fn try_insert(&self, data: &[u8]) -> isize {
        let size = data.len();
        if size == 0 {
            return 0;
        }
        if size >= self.size {
            return -1;
        }

        let cur_prod = self.producer_ptr.0.load(Ordering::Relaxed) as usize;
        let cur_cons = self.consumer_ptr.0.load(Ordering::Acquire) as usize;

        let wraparound = cur_prod + size >= self.size;
        let index = if wraparound { 0 } else { cur_prod };

        let before_consumer = cur_prod < cur_cons;

        if (wraparound && before_consumer)
            || ((index < cur_cons || wraparound) && index + size >= cur_cons)
        {
            return -1;
        }

        // Copy data into pool safely
        let pool_ptr = self.pool.as_ptr() as *mut u8;
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), pool_ptr.add(index), size);
        }

        if wraparound {
            self.skip_position.store(cur_prod as i32, Ordering::Release);
        }

        self.producer_ptr
            .0
            .store((index + size) as isize, Ordering::Release);

        index as isize
    }

    /// Get the read pointer for the consumer.
    /// Returns raw pointer to the start of the data.
    pub fn get_consumer_ptr(&self, size: usize) -> *const u8 {
        let mut cons = self.consumer_ptr.0.load(Ordering::Relaxed) as i32;

        if cons
            == self
                .skip_position
                .compare_exchange(cons, -1, Ordering::SeqCst, Ordering::Relaxed)
                .unwrap_or(cons - 1)
        {
            cons = 0;
            self.consumer_ptr.0.store(0, Ordering::Release);
        }

        let offset = cons as usize;
        if offset + size > self.size {
            return std::ptr::null();
        }

        unsafe { self.pool.as_ptr().add(offset) }
    }

    /// Advance consumer pointer after processing data.
    pub fn advance_consumer(&self, size: usize) {
        self.consumer_ptr
            .0
            .fetch_add(size as isize, Ordering::Release);
    }

    /// Reset pool pointers.
    pub fn reset(&self) {
        self.producer_ptr.0.store(0, Ordering::SeqCst);
        self.consumer_ptr.0.store(0, Ordering::SeqCst);
        self.skip_position.store(-1, Ordering::SeqCst);
    }

    pub fn capacity(&self) -> usize {
        self.size
    }

    pub fn pool_buffer_ptr(&self) -> *const u8 {
        self.pool.as_ptr()
    }
}
