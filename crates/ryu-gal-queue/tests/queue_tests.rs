use ryu_gal_queue::ffi::*;
use ryu_gal_queue::*;
use std::thread;

// ----------------------------------------------------------------------------
// Reference C# CircularSpanPool (Bit-for-bit ported from CircularSpanPool.cs)
// ----------------------------------------------------------------------------

struct RefCircularSpanPool {
    pool: Vec<u8>,
    size: usize,
    producer_ptr: usize,
    producer_skip_position: isize,
    consumer_ptr: usize,
}

impl RefCircularSpanPool {
    fn new(size: usize) -> Self {
        Self {
            pool: vec![0u8; size],
            size,
            producer_ptr: 0,
            producer_skip_position: -1,
            consumer_ptr: 0,
        }
    }

    fn insert(&mut self, data: &[u8]) -> Option<usize> {
        let size = data.len();
        if size >= self.size {
            return None;
        }

        let wraparound = self.producer_ptr + size >= self.size;
        let index = if wraparound { 0 } else { self.producer_ptr };

        let consumer = self.consumer_ptr;
        let before_consumer = self.producer_ptr < consumer;

        if size > self.size - 1
            || (wraparound && before_consumer)
            || ((index < consumer || wraparound) && index + size >= consumer)
        {
            return None;
        }

        self.pool[index..index + size].copy_from_slice(data);

        if wraparound {
            self.producer_skip_position = self.producer_ptr as isize;
        }

        self.producer_ptr = index + size;

        Some(index)
    }

    fn get(&mut self, length: usize) -> &[u8] {
        if self.consumer_ptr as isize == self.producer_skip_position {
            self.consumer_ptr = 0;
            self.producer_skip_position = -1;
        }

        let start = self.consumer_ptr;
        &self.pool[start..start + length]
    }

    fn dispose(&mut self, length: usize) {
        self.consumer_ptr += length;
    }
}

// ----------------------------------------------------------------------------
// Step 1: Invariants, Boundary Checks, Null Pointer Safety
// ----------------------------------------------------------------------------

#[test]
fn test_step1_span_pool_null_and_bounds() {
    unsafe {
        assert!(ryu_span_pool_create(0).is_null());
        assert_eq!(
            ryu_span_pool_insert(std::ptr::null_mut(), std::ptr::null(), 10),
            -1
        );
        assert!(ryu_span_pool_get(std::ptr::null_mut(), 10).is_null());
        assert_eq!(ryu_span_pool_advance(std::ptr::null_mut(), 10), -1);
        assert_eq!(ryu_span_pool_reset(std::ptr::null_mut()), -1);

        ryu_span_pool_destroy(std::ptr::null_mut());
    }
}

#[test]
fn test_step1_spsc_null_and_bounds() {
    unsafe {
        assert!(ryu_spsc_create(0, 64).is_null());
        assert!(ryu_spsc_create(100, 0).is_null());

        assert_eq!(ryu_spsc_try_reserve(std::ptr::null_mut()), -1);
        assert_eq!(ryu_spsc_commit(std::ptr::null_mut()), -1);
        assert_eq!(ryu_spsc_try_consume(std::ptr::null_mut()), -1);
        assert_eq!(ryu_spsc_release(std::ptr::null_mut()), -1);
        assert!(ryu_spsc_get_slot_ptr(std::ptr::null_mut(), 0).is_null());
        assert_eq!(ryu_spsc_pending_count(std::ptr::null()), -1);

        ryu_spsc_destroy(std::ptr::null_mut());
    }
}

// ----------------------------------------------------------------------------
// Step 2: Deterministic Dual-Execution Bit-Identical Tests
// ----------------------------------------------------------------------------

#[test]
fn test_step2_span_pool_wraparound_bit_identical() {
    const POOL_SIZE: usize = 1024;
    let mut ref_pool = RefCircularSpanPool::new(POOL_SIZE);
    let rust_pool = GalSpanPool::new(POOL_SIZE);

    let chunks = [
        vec![0xAAu8; 300],
        vec![0xBBu8; 300],
        vec![0xCCu8; 300], // Reaches near end of 1024
        vec![0xDDu8; 250], // Must wraparound to index 0
        vec![0xEEu8; 150],
    ];

    for chunk in &chunks {
        let ref_idx = ref_pool.insert(chunk);
        let rust_idx = rust_pool.try_insert(chunk);

        match (ref_idx, rust_idx) {
            (Some(r), idx) if idx >= 0 => {
                assert_eq!(r as isize, idx, "Insertion offset must match exactly");
            }
            (None, idx) => {
                assert_eq!(idx, -1, "Both must reject insertion when full");
            }
            _ => panic!("Mismatch in pool acceptance"),
        }

        // Consume and check contents
        let ref_data = ref_pool.get(chunk.len());
        let rust_ptr = rust_pool.get_consumer_ptr(chunk.len());
        assert!(!rust_ptr.is_null());
        let rust_slice = unsafe { std::slice::from_raw_parts(rust_ptr, chunk.len()) };

        assert_eq!(ref_data, rust_slice, "Pool contents must be bit-identical");

        ref_pool.dispose(chunk.len());
        rust_pool.advance_consumer(chunk.len());
    }
}

#[test]
fn test_step2_spsc_multithreaded_fifo_order() {
    const CAPACITY: usize = 1024;
    const ELEMENT_SIZE: usize = 8;
    const TOTAL_ITEMS: usize = 50_000;

    let queue = std::sync::Arc::new(GalSpscQueue::new(CAPACITY, ELEMENT_SIZE));
    let q_producer = queue.clone();
    let q_consumer = queue.clone();

    let producer = thread::spawn(move || {
        for i in 0..TOTAL_ITEMS {
            let mut slot;
            loop {
                slot = q_producer.try_reserve();
                if slot >= 0 {
                    break;
                }
                std::hint::spin_loop();
            }

            let ptr = q_producer.get_slot_ptr(slot as usize) as *mut u64;
            unsafe {
                *ptr = i as u64;
            }
            q_producer.commit();
        }
    });

    let consumer = thread::spawn(move || {
        let mut received = Vec::with_capacity(TOTAL_ITEMS);
        for _ in 0..TOTAL_ITEMS {
            let mut slot;
            loop {
                slot = q_consumer.try_consume();
                if slot >= 0 {
                    break;
                }
                std::hint::spin_loop();
            }

            let ptr = q_consumer.get_slot_ptr(slot as usize) as *const u64;
            let val = unsafe { *ptr };
            received.push(val);
            q_consumer.release();
        }
        received
    });

    producer.join().unwrap();
    let received = consumer.join().unwrap();

    assert_eq!(received.len(), TOTAL_ITEMS);
    for (i, &val) in received.iter().enumerate() {
        assert_eq!(
            val, i as u64,
            "Strict FIFO order must be preserved across threads"
        );
    }
}
