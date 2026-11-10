use ryu_kernel_sync::ffi::*;
use ryu_kernel_sync::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

// ----------------------------------------------------------------------------
// Reference C# KAddressArbiter logic (exact port from KAddressArbiter.cs)
// ----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
struct RefWaiter {
    thread_id: u64,
    priority: i32,
}

#[derive(Default)]
struct RefAddressArbiter {
    arbiter_threads: HashMap<u64, Vec<RefWaiter>>,
}

#[allow(dead_code)]
impl RefAddressArbiter {
    fn insert_waiter(&mut self, address: u64, thread_id: u64, priority: i32) {
        let threads = self.arbiter_threads.entry(address).or_insert_with(Vec::new);
        let waiter = RefWaiter {
            thread_id,
            priority,
        };

        let idx = match threads.binary_search_by(|t| t.priority.cmp(&priority)) {
            Ok(i) => i,
            Err(i) => i,
        };
        threads.insert(idx, waiter);
    }

    fn remove_waiter(&mut self, address: u64, thread_id: u64) -> bool {
        if let Some(threads) = self.arbiter_threads.get_mut(&address) {
            if let Some(pos) = threads.iter().position(|t| t.thread_id == thread_id) {
                threads.remove(pos);
                if threads.is_empty() {
                    self.arbiter_threads.remove(&address);
                }
                return true;
            }
        }
        false
    }

    fn wake_threads(&mut self, address: u64, count: i32) -> Vec<u64> {
        let mut woken = Vec::new();
        if let Some(threads) = self.arbiter_threads.get_mut(&address) {
            let valid_count = if count <= 0 {
                threads.len()
            } else {
                threads.len().min(count as usize)
            };

            for i in 0..valid_count {
                woken.push(threads[i].thread_id);
            }
            threads.drain(0..valid_count);

            if threads.is_empty() {
                self.arbiter_threads.remove(&address);
            }
        }
        woken
    }

    fn compute_signal_modify_addend(&self, address: u64, count: i32) -> i32 {
        let waiting_count = self
            .arbiter_threads
            .get(&address)
            .map(|l| l.len())
            .unwrap_or(0);
        if waiting_count > 0 {
            if count <= 0 {
                -2
            } else if waiting_count < (count as usize) {
                -1
            } else {
                0
            }
        } else {
            1
        }
    }
}

// ----------------------------------------------------------------------------
// Step 1: Invariants, Boundary Checks, Null Safety
// ----------------------------------------------------------------------------

#[test]
fn test_step1_ffi_null_safety() {
    unsafe {
        assert_eq!(
            ryu_arbiter_insert_waiter(std::ptr::null_mut(), 0x1000, 1, 10),
            -1
        );
        assert_eq!(
            ryu_arbiter_remove_waiter(std::ptr::null_mut(), 0x1000, 1),
            -1
        );
        assert_eq!(
            ryu_arbiter_wake_waiters(std::ptr::null_mut(), 0x1000, 1, std::ptr::null_mut(), 0),
            -1
        );
        assert_eq!(ryu_arbiter_get_waiter_count(std::ptr::null(), 0x1000), -1);
        assert_eq!(
            ryu_arbiter_compute_modify_addend(std::ptr::null(), 0x1000, 1),
            0
        );
        assert_eq!(ryu_arbiter_clear(std::ptr::null_mut()), -1);

        ryu_arbiter_destroy(std::ptr::null_mut());
    }
}

// ----------------------------------------------------------------------------
// Step 2: Deterministic Dual-Execution Bit-Identical Tests
// ----------------------------------------------------------------------------

#[test]
fn test_step2_priority_ordering_bit_identical() {
    let mut ref_arbiter = RefAddressArbiter::default();
    let rust_arbiter = ArbiterRegistry::new();

    let address = 0x7100020000;

    // Insert 6 threads with varied priorities (scrambled order)
    let threads_to_insert = [
        (101, 20),
        (102, 10),
        (103, 30),
        (104, 5),  // Highest priority
        (105, 10), // Same priority as 102
        (106, 50), // Lowest priority
    ];

    for &(tid, prio) in &threads_to_insert {
        ref_arbiter.insert_waiter(address, tid, prio);
        rust_arbiter.insert_waiter(address, tid, prio);

        assert_eq!(
            ref_arbiter.arbiter_threads.get(&address).unwrap().len(),
            rust_arbiter.get_waiter_count(address)
        );
    }

    // Wake top 3 highest priority threads
    let ref_woken = ref_arbiter.wake_threads(address, 3);
    let mut rust_woken = [0u64; 3];
    let count = rust_arbiter.wake_waiters(address, 3, &mut rust_woken);

    assert_eq!(count, 3);
    assert_eq!(&rust_woken[..count], &ref_woken[..]);
    assert_eq!(rust_woken[0], 104); // priority 5 woken first!

    // Check remaining count
    assert_eq!(rust_arbiter.get_waiter_count(address), 3);

    // Wake all remaining threads (count <= 0)
    let ref_woken_rem = ref_arbiter.wake_threads(address, 0);
    let mut rust_woken_rem = [0u64; 8];
    let count_rem = rust_arbiter.wake_waiters(address, 0, &mut rust_woken_rem);

    assert_eq!(count_rem, ref_woken_rem.len());
    assert_eq!(&rust_woken_rem[..count_rem], &ref_woken_rem[..]);
    assert_eq!(rust_arbiter.get_waiter_count(address), 0);
}

#[test]
fn test_step2_signal_modify_addend_bit_identical() {
    let mut ref_arbiter = RefAddressArbiter::default();
    let rust_arbiter = ArbiterRegistry::new();

    let addr = 0x7100080000;

    // 0 waiters: addend = 1
    assert_eq!(
        ref_arbiter.compute_signal_modify_addend(addr, 1),
        rust_arbiter.compute_signal_modify_addend(addr, 1)
    );
    assert_eq!(rust_arbiter.compute_signal_modify_addend(addr, 1), 1);

    // Insert 2 waiters
    ref_arbiter.insert_waiter(addr, 1, 10);
    rust_arbiter.insert_waiter(addr, 1, 10);
    ref_arbiter.insert_waiter(addr, 2, 20);
    rust_arbiter.insert_waiter(addr, 2, 20);

    // count <= 0: addend = -2
    assert_eq!(
        ref_arbiter.compute_signal_modify_addend(addr, 0),
        rust_arbiter.compute_signal_modify_addend(addr, 0)
    );
    assert_eq!(rust_arbiter.compute_signal_modify_addend(addr, 0), -2);

    // waitingCount (2) < count (5): addend = -1
    assert_eq!(
        ref_arbiter.compute_signal_modify_addend(addr, 5),
        rust_arbiter.compute_signal_modify_addend(addr, 5)
    );
    assert_eq!(rust_arbiter.compute_signal_modify_addend(addr, 5), -1);

    // waitingCount (2) >= count (2): addend = 0
    assert_eq!(
        ref_arbiter.compute_signal_modify_addend(addr, 2),
        rust_arbiter.compute_signal_modify_addend(addr, 2)
    );
    assert_eq!(rust_arbiter.compute_signal_modify_addend(addr, 2), 0);
}

#[test]
fn test_step2_concurrent_sharded_arbitration() {
    let registry = Arc::new(ArbiterRegistry::new());
    let mut handles = Vec::new();

    // 16 concurrent guest threads operating on different guest mutex addresses
    for t in 0..16 {
        let reg = registry.clone();
        handles.push(thread::spawn(move || {
            let base_addr = 0x7100000000 + (t as u64) * 0x1000;
            for i in 0..1000 {
                let addr = base_addr + ((i % 8) as u64) * 4;
                reg.insert_waiter(addr, t * 1000 + i, (i % 64) as i32);
                if i % 3 == 0 {
                    let mut woken = [0u64; 2];
                    reg.wake_waiters(addr, 2, &mut woken);
                } else if i % 5 == 0 {
                    reg.remove_waiter(addr, t * 1000 + i);
                }
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}
