//! High-throughput Lock-Free Circular Span Pool for GAL Multithreading.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicI32, AtomicIsize, Ordering};

use crate::command_queue::zeroed_cell_buffer;

/// Cacheline padding helper to prevent false sharing.
#[repr(align(64))]
struct CachePadded<T>(T);

/// Lock-free circular span pool with wraparound awareness.
/// Replaces `Ryujinx.Graphics.GAL.Multithreading.Model.CircularSpanPool`.
pub struct GalSpanPool {
    /// Pool storage; interior-mutable because the producer writes via `&self`.
    pool: Box<[UnsafeCell<u8>]>,
    size: usize,
    producer_ptr: CachePadded<AtomicIsize>,
    consumer_ptr: CachePadded<AtomicIsize>,
    skip_position: AtomicI32,
}

// SAFETY: all bookkeeping is atomic. Pool bytes are written only by the single
// producer in regions the consumer has released (checked against the consumer
// pointer with Acquire) and published with a Release store of `producer_ptr`,
// so producer and consumer never access the same bytes concurrently.
unsafe impl Sync for GalSpanPool {}
// SAFETY: no thread-affine state; the pool is a plain heap allocation.
unsafe impl Send for GalSpanPool {}

impl GalSpanPool {
    pub fn new(size: usize) -> Self {
        Self {
            pool: zeroed_cell_buffer(size),
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

        // A payload that ends exactly at the end of the pool still fits; only
        // wrap when it would run past the end. (C# uses `>=`, which needlessly
        // moves an exactly-fitting payload to offset 0.) `producer_ptr` may then
        // equal `size`; the next insert wraps and records `skip_position == size`,
        // which the consumer handles the same way as any other skip point.
        let wraparound = cur_prod + size > self.size;
        let index = if wraparound { 0 } else { cur_prod };

        let before_consumer = cur_prod < cur_cons;

        if (wraparound && before_consumer)
            || ((index < cur_cons || wraparound) && index + size >= cur_cons)
        {
            return -1;
        }

        let pool_ptr = UnsafeCell::raw_get(self.pool.as_ptr());
        // SAFETY: `index + size <= self.size` (either `cur_prod + size <= size`
        // or `index == 0` with `size < self.size`), and the checks above ensure
        // the region does not overlap data the consumer has not released yet.
        // The pointer is derived from the `UnsafeCell` slice, so writes via
        // `&self` are allowed.
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
        // `checked_add`: a wrapped sum would pass the bound check with `offset`
        // possibly outside the pool.
        if offset.checked_add(size).is_none_or(|end| end > self.size) {
            return std::ptr::null();
        }

        // SAFETY: `offset + size <= self.size` was checked above (without overflow).
        unsafe { UnsafeCell::raw_get(self.pool.as_ptr()).add(offset) as *const u8 }
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
        UnsafeCell::raw_get(self.pool.as_ptr()) as *const u8
    }
}
