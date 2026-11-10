use crate::types::{QueueItem, NUM_BUFFER_SLOTS};
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering};

const RING_CAPACITY: usize = NUM_BUFFER_SLOTS;

pub struct SurfaceFlingerQueue {
    pub max_buffers: AtomicU32,
    pub free_mask: AtomicU64,
    pub dequeued_mask: AtomicU64,
    pub queued_mask: AtomicU64,
    pub acquired_mask: AtomicU64,

    pub frame_numbers: [AtomicU64; NUM_BUFFER_SLOTS],
    pub timestamps: [AtomicI64; NUM_BUFFER_SLOTS],

    // Lock-free SPSC Ring Buffer for queued frames
    head: AtomicUsize,
    tail: AtomicUsize,
    ring: [UnsafeCell<QueueItem>; RING_CAPACITY],
}

// SAFETY: all slot bookkeeping is atomic; ring entries are accessed only under
// the SPSC head/tail protocol (one producer writes at `head`, one consumer reads
// at `tail`, published with Release/Acquire), so no entry is accessed
// concurrently.
unsafe impl Send for SurfaceFlingerQueue {}
// SAFETY: see above.
unsafe impl Sync for SurfaceFlingerQueue {}

impl SurfaceFlingerQueue {
    pub fn new(max_buffers: usize) -> Self {
        let max_b = max_buffers.min(NUM_BUFFER_SLOTS).max(1);
        let init_free = if max_b == 64 {
            u64::MAX
        } else {
            (1u64 << max_b) - 1
        };

        // Construct arrays
        let frame_numbers = std::array::from_fn(|_| AtomicU64::new(0));
        let timestamps = std::array::from_fn(|_| AtomicI64::new(0));
        let ring = std::array::from_fn(|_| UnsafeCell::new(QueueItem::default()));

        Self {
            max_buffers: AtomicU32::new(max_b as u32),
            free_mask: AtomicU64::new(init_free),
            dequeued_mask: AtomicU64::new(0),
            queued_mask: AtomicU64::new(0),
            acquired_mask: AtomicU64::new(0),
            frame_numbers,
            timestamps,
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            ring,
        }
    }

    pub fn set_max_buffers(&self, count: usize) {
        let count = count.min(NUM_BUFFER_SLOTS).max(1);
        self.max_buffers.store(count as u32, Ordering::Release);
    }

    #[inline]
    fn valid_mask(&self, max_count: usize) -> u64 {
        if max_count >= 64 {
            u64::MAX
        } else {
            (1u64 << max_count) - 1
        }
    }

    /// Finds the oldest free slot (lowest frame number).
    ///
    /// Scans every set bit of the free mask, so this is O(k) in the number of
    /// free slots (bounded by `NUM_BUFFER_SLOTS` = 64), not O(1).
    pub fn find_oldest_free_slot(&self, max_buffer_count: usize) -> Option<usize> {
        let max_b = if max_buffer_count > 0 {
            max_buffer_count
        } else {
            self.max_buffers.load(Ordering::Acquire) as usize
        };
        let valid = self.valid_mask(max_b);
        let mut free = self.free_mask.load(Ordering::Acquire) & valid;

        if free == 0 {
            return None;
        }

        let mut oldest_slot = None;
        let mut min_frame = u64::MAX;

        while free != 0 {
            let slot = free.trailing_zeros() as usize;
            let frame = self.frame_numbers[slot].load(Ordering::Acquire);
            if frame < min_frame {
                min_frame = frame;
                oldest_slot = Some(slot);
            }
            free &= free - 1; // Clear lowest bit
        }

        oldest_slot
    }

    /// Dequeues an available free slot atomically, transitioning it Free -> Dequeued.
    pub fn dequeue_slot(&self, max_buffer_count: usize) -> Option<usize> {
        let max_b = if max_buffer_count > 0 {
            max_buffer_count
        } else {
            self.max_buffers.load(Ordering::Acquire) as usize
        };

        // Try to find the oldest free slot
        if let Some(slot) = self.find_oldest_free_slot(max_b) {
            let slot_bit = 1u64 << slot;
            // Atomically clear from free_mask
            let prev = self.free_mask.fetch_and(!slot_bit, Ordering::AcqRel);
            if (prev & slot_bit) != 0 {
                // Successfully took from free
                self.dequeued_mask.fetch_or(slot_bit, Ordering::Release);
                return Some(slot);
            }
        }

        // Fast fallback: try taking any free slot via CAS
        let valid = self.valid_mask(max_b);
        let mut current_free = self.free_mask.load(Ordering::Acquire) & valid;
        while current_free != 0 {
            let slot = current_free.trailing_zeros() as usize;
            let slot_bit = 1u64 << slot;
            match self.free_mask.compare_exchange_weak(
                current_free,
                current_free & !slot_bit,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.dequeued_mask.fetch_or(slot_bit, Ordering::Release);
                    return Some(slot);
                }
                Err(actual) => {
                    current_free = actual & valid;
                }
            }
        }

        None
    }

    /// Queues a buffer into the ring buffer, transitioning Dequeued -> Queued.
    ///
    /// Must only be called from the single producer thread. Returns `false`
    /// (and leaves all state unchanged) if the slot is out of range, the slot is
    /// not currently Dequeued, or the ring is full. The producer never touches
    /// consumer-owned state (`tail`), so a full ring is rejected rather than
    /// silently dropping the oldest frame.
    pub fn queue_buffer(&self, item: QueueItem) -> bool {
        if item.slot < 0 || item.slot as usize >= NUM_BUFFER_SLOTS {
            return false;
        }
        let slot = item.slot as usize;
        let slot_bit = 1u64 << slot;

        // Claim the Dequeued -> Queued transition; reject if not dequeued.
        let prev = self.dequeued_mask.fetch_and(!slot_bit, Ordering::AcqRel);
        if prev & slot_bit == 0 {
            return false;
        }

        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head.wrapping_sub(tail) >= RING_CAPACITY {
            // Ring full (only reachable if external `sync_slot_state` calls put
            // the same slot in the ring twice). Undo the claim and reject; the
            // slot stays Dequeued so the caller can cancel or retry it.
            self.dequeued_mask.fetch_or(slot_bit, Ordering::Release);
            return false;
        }

        self.frame_numbers[slot].store(item.frame_number, Ordering::Release);
        self.timestamps[slot].store(item.timestamp, Ordering::Release);

        // SAFETY: only the single producer writes ring entries, and the entry at
        // `head` is not visible to the consumer until `head` is advanced below
        // (the full check above guarantees the consumer has finished with it).
        unsafe {
            let ptr = self.ring[head % RING_CAPACITY].get();
            *ptr = item;
        }
        self.queued_mask.fetch_or(slot_bit, Ordering::Release);
        self.head.store(head.wrapping_add(1), Ordering::Release);

        true
    }

    /// Pops the next queued item from the ring buffer, transitioning Queued -> Acquired.
    pub fn acquire_buffer(&self) -> Option<QueueItem> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail == head {
            return None; // Empty
        }

        // SAFETY: `tail != head`, so the entry at `tail` was fully written and
        // published by the producer's Release store of `head`; the producer will
        // not overwrite it until `tail` is advanced below.
        let item = unsafe { *self.ring[tail % RING_CAPACITY].get() };
        self.tail.store(tail.wrapping_add(1), Ordering::Release);

        let slot = item.slot as usize;
        if slot < NUM_BUFFER_SLOTS {
            let slot_bit = 1u64 << slot;
            self.queued_mask.fetch_and(!slot_bit, Ordering::AcqRel);
            self.acquired_mask.fetch_or(slot_bit, Ordering::Release);
        }

        Some(item)
    }

    /// Releases an acquired buffer back to Free state.
    /// Returns `false` (no state change) if the slot is not currently Acquired.
    pub fn release_buffer(&self, slot: usize) -> bool {
        if slot >= NUM_BUFFER_SLOTS {
            return false;
        }
        let slot_bit = 1u64 << slot;
        let prev = self.acquired_mask.fetch_and(!slot_bit, Ordering::AcqRel);
        if prev & slot_bit == 0 {
            return false;
        }
        self.free_mask.fetch_or(slot_bit, Ordering::Release);
        true
    }

    /// Cancels a dequeued buffer without presenting, transitioning Dequeued -> Free.
    /// Returns `false` (no state change) if the slot is not currently Dequeued.
    pub fn cancel_buffer(&self, slot: usize) -> bool {
        if slot >= NUM_BUFFER_SLOTS {
            return false;
        }
        let slot_bit = 1u64 << slot;
        let prev = self.dequeued_mask.fetch_and(!slot_bit, Ordering::AcqRel);
        if prev & slot_bit == 0 {
            return false;
        }
        self.free_mask.fetch_or(slot_bit, Ordering::Release);
        true
    }

    /// Synchronizes external slot state directly from C# (e.g. FreeBufferLocked).
    pub fn sync_slot_state(&self, slot: usize, state: u8, frame_number: u64) {
        if slot >= NUM_BUFFER_SLOTS {
            return;
        }
        let slot_bit = 1u64 << slot;
        self.free_mask.fetch_and(!slot_bit, Ordering::AcqRel);
        self.dequeued_mask.fetch_and(!slot_bit, Ordering::AcqRel);
        self.queued_mask.fetch_and(!slot_bit, Ordering::AcqRel);
        self.acquired_mask.fetch_and(!slot_bit, Ordering::AcqRel);

        match state {
            0 => {
                self.free_mask.fetch_or(slot_bit, Ordering::Release);
            }
            1 => {
                self.dequeued_mask.fetch_or(slot_bit, Ordering::Release);
            }
            2 => {
                self.queued_mask.fetch_or(slot_bit, Ordering::Release);
            }
            3 => {
                self.acquired_mask.fetch_or(slot_bit, Ordering::Release);
            }
            _ => {
                self.free_mask.fetch_or(slot_bit, Ordering::Release);
            }
        }

        self.frame_numbers[slot].store(frame_number, Ordering::Release);
    }

    /// Returns the counts of (Free, Dequeued, Queued, Acquired) slots in 1 CPU cycle.
    pub fn get_counts(&self) -> (u32, u32, u32, u32) {
        (
            self.free_mask.load(Ordering::Acquire).count_ones(),
            self.dequeued_mask.load(Ordering::Acquire).count_ones(),
            self.queued_mask.load(Ordering::Acquire).count_ones(),
            self.acquired_mask.load(Ordering::Acquire).count_ones(),
        )
    }
}
