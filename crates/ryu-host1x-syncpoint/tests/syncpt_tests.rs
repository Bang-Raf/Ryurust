use ryu_host1x_syncpoint::ffi::*;
use ryu_host1x_syncpoint::*;

// ----------------------------------------------------------------------------
// Reference C# SyncptIncrManager (Bit-for-bit ported from SyncptIncrManager.cs)
// ----------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct RefSyncptIncr {
    id: u32,
    class_id: u32,
    syncpt_id: u32,
    done: bool,
}

#[derive(Default)]
struct RefSyncptIncrManager {
    incrs: Vec<RefSyncptIncr>,
    current_id: u32,
    flushed: Vec<u32>,
}

impl RefSyncptIncrManager {
    fn increment(&mut self, id: u32) {
        // Rust uses a reserved sentinel id for untracked increments instead of
        // C#'s 0 (which collides with real handle 0); it never matches a handle.
        self.incrs.push(RefSyncptIncr {
            id: u32::MAX,
            class_id: 0,
            syncpt_id: id,
            done: true,
        });
        self.increment_all_done();
    }

    fn increment_when_done(&mut self, class_id: u32, id: u32) -> u32 {
        let handle = self.current_id;
        self.current_id = self.current_id.wrapping_add(1);

        self.incrs.push(RefSyncptIncr {
            id: handle,
            class_id,
            syncpt_id: id,
            done: false,
        });

        handle
    }

    fn signal_done(&mut self, handle: u32) {
        for incr in self.incrs.iter_mut() {
            if incr.id == handle {
                incr.done = true;
                break;
            }
        }
        self.increment_all_done();
    }

    fn increment_all_done(&mut self) {
        let mut done_count = 0;
        for incr in &self.incrs {
            if !incr.done {
                break;
            }
            self.flushed.push(incr.syncpt_id);
            done_count += 1;
        }
        self.incrs.drain(0..done_count);
    }
}

// ----------------------------------------------------------------------------
// Step 1: Invariants, Boundary Checks, Null Pointer Safety
// ----------------------------------------------------------------------------

#[test]
fn test_step1_ffi_null_safety() {
    unsafe {
        assert_eq!(ryu_syncpt_increment(std::ptr::null_mut(), 1), -1);
        assert_eq!(
            ryu_syncpt_increment_when_done(std::ptr::null_mut(), 0, 1, std::ptr::null_mut()),
            -1
        );
        assert_eq!(ryu_syncpt_signal_done(std::ptr::null_mut(), 0), -1);
        assert_eq!(
            ryu_syncpt_drain_flushed(std::ptr::null_mut(), std::ptr::null_mut(), 10),
            -1
        );
        assert_eq!(ryu_syncpt_reset(std::ptr::null_mut()), -1);
        assert_eq!(ryu_syncpt_pending_count(std::ptr::null()), -1);
        assert_eq!(ryu_syncpt_flushed_count(std::ptr::null()), -1);

        // Safe destruction of null handle
        ryu_syncpt_destroy(std::ptr::null_mut());
    }
}

#[test]
fn test_step1_lifecycle_and_drain_bounds() {
    unsafe {
        let seq = ryu_syncpt_create();
        assert!(!seq.is_null());

        assert_eq!(ryu_syncpt_pending_count(seq), 0);
        assert_eq!(ryu_syncpt_flushed_count(seq), 0);

        // Drain with zero capacity or null dst
        assert_eq!(ryu_syncpt_drain_flushed(seq, std::ptr::null_mut(), 0), 0);

        let mut buf = [0u32; 8];
        assert_eq!(ryu_syncpt_drain_flushed(seq, buf.as_mut_ptr(), 8), 0);

        ryu_syncpt_destroy(seq);
    }
}

// ----------------------------------------------------------------------------
// Step 2: Deterministic Dual-Execution Bit-Identical Tests
// ----------------------------------------------------------------------------

#[test]
fn test_step2_direct_increments_bit_identical() {
    let mut ref_mgr = RefSyncptIncrManager::default();
    let rust_mgr = SyncptSequencer::new();

    for id in 1..=50 {
        ref_mgr.increment(id);
        rust_mgr.increment(id);
    }

    assert_eq!(ref_mgr.incrs.len(), rust_mgr.pending_count());
    assert_eq!(ref_mgr.flushed.len(), rust_mgr.flushed_count());

    let mut rust_flushed = vec![0u32; ref_mgr.flushed.len()];
    let drained = rust_mgr.drain_flushed(&mut rust_flushed);
    assert_eq!(drained, ref_mgr.flushed.len());
    assert_eq!(rust_flushed, ref_mgr.flushed);
}

#[test]
fn test_step2_out_of_order_signal_done_bit_identical() {
    let mut ref_mgr = RefSyncptIncrManager::default();
    let rust_mgr = SyncptSequencer::new();

    // Register 5 deferred operations
    let h0 = rust_mgr.increment_when_done(0xB0, 10);
    let rh0 = ref_mgr.increment_when_done(0xB0, 10);
    assert_eq!(h0, rh0);

    let h1 = rust_mgr.increment_when_done(0xB0, 11);
    let rh1 = ref_mgr.increment_when_done(0xB0, 11);
    assert_eq!(h1, rh1);

    let h2 = rust_mgr.increment_when_done(0xB0, 12);
    let rh2 = ref_mgr.increment_when_done(0xB0, 12);
    assert_eq!(h2, rh2);

    let h3 = rust_mgr.increment_when_done(0xB0, 13);
    let rh3 = ref_mgr.increment_when_done(0xB0, 13);
    assert_eq!(h3, rh3);

    // Signal h2, h3 first (out of order). None should flush yet because h0 and h1 are not done!
    rust_mgr.signal_done(h2);
    ref_mgr.signal_done(rh2);
    assert_eq!(ref_mgr.flushed.len(), 0);
    assert_eq!(rust_mgr.flushed_count(), 0);
    assert_eq!(rust_mgr.pending_count(), 4);

    rust_mgr.signal_done(h3);
    ref_mgr.signal_done(rh3);
    assert_eq!(ref_mgr.flushed.len(), 0);
    assert_eq!(rust_mgr.flushed_count(), 0);

    // Signal h1. Still nothing should flush because h0 is at the head!
    rust_mgr.signal_done(h1);
    ref_mgr.signal_done(rh1);
    assert_eq!(ref_mgr.flushed.len(), 0);
    assert_eq!(rust_mgr.flushed_count(), 0);

    // Finally signal h0! All 4 (10, 11, 12, 13) should cascade-flush in order!
    rust_mgr.signal_done(h0);
    ref_mgr.signal_done(rh0);
    assert_eq!(ref_mgr.flushed.len(), 4);
    assert_eq!(rust_mgr.flushed_count(), 4);
    assert_eq!(rust_mgr.pending_count(), 0);

    let mut rust_flushed = vec![0u32; 4];
    rust_mgr.drain_flushed(&mut rust_flushed);
    assert_eq!(rust_flushed, ref_mgr.flushed);
    assert_eq!(rust_flushed, vec![10, 11, 12, 13]);
}

#[test]
fn test_step2_massive_random_interleaved_simulation() {
    let mut ref_mgr = RefSyncptIncrManager::default();
    let rust_mgr = SyncptSequencer::new();

    let mut pending_handles = Vec::new();

    // Pseudo-random deterministic PRNG (LCG)
    let mut lcg_state: u64 = 0xCAFEBABE;
    let mut lcg = || -> u32 {
        lcg_state = lcg_state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (lcg_state >> 32) as u32
    };

    for _ in 0..10_000 {
        let op = lcg() % 3;
        match op {
            0 => {
                // Direct increment
                let syncpt_id = (lcg() % 64) + 1;
                ref_mgr.increment(syncpt_id);
                rust_mgr.increment(syncpt_id);
            }
            1 => {
                // Increment when done
                let class_id = lcg() % 16;
                let syncpt_id = (lcg() % 64) + 1;
                let rh = ref_mgr.increment_when_done(class_id, syncpt_id);
                let h = rust_mgr.increment_when_done(class_id, syncpt_id);
                assert_eq!(h, rh);
                pending_handles.push(h);
            }
            _ => {
                // Signal done
                if !pending_handles.is_empty() {
                    let idx = (lcg() as usize) % pending_handles.len();
                    let handle = pending_handles.swap_remove(idx);
                    ref_mgr.signal_done(handle);
                    rust_mgr.signal_done(handle);
                }
            }
        }

        assert_eq!(ref_mgr.incrs.len(), rust_mgr.pending_count());
        assert_eq!(ref_mgr.flushed.len(), rust_mgr.flushed_count());
    }

    // Drain and verify 100% bit-identical stream
    let total_flushed = ref_mgr.flushed.len();
    let mut rust_flushed = vec![0u32; total_flushed];
    let drained = rust_mgr.drain_flushed(&mut rust_flushed);
    assert_eq!(drained, total_flushed);
    assert_eq!(rust_flushed, ref_mgr.flushed);
}

#[test]
fn test_plain_increment_not_completed_by_handle_zero() {
    // With id 0 on plain increments, signal_done(0) could match the wrong
    // entry. Here handle 0 is pending behind nothing; a plain increment queued
    // after it must flush only once handle 0 completes.
    let seq = SyncptSequencer::new();
    let h0 = seq.increment_when_done(0xB0, 7);
    assert_eq!(h0, 0);
    seq.increment(8);
    assert_eq!(seq.pending_count(), 2);
    seq.signal_done(h0);
    let mut out = [0u32; 2];
    assert_eq!(seq.drain_flushed(&mut out), 2);
    assert_eq!(out, [7, 8]);
}

#[test]
fn test_reverse_order_completion_large() {
    const N: u32 = 20_000;
    let mut ref_mgr = RefSyncptIncrManager::default();
    let seq = SyncptSequencer::new();
    let handles: Vec<u32> = (0..N)
        .map(|i| {
            let r = ref_mgr.increment_when_done(1, i);
            let h = seq.increment_when_done(1, i);
            assert_eq!(h, r);
            if i % 3 == 0 {
                ref_mgr.increment(i + 100_000);
                seq.increment(i + 100_000);
            }
            h
        })
        .collect();
    for &h in handles.iter().rev() {
        ref_mgr.signal_done(h);
        seq.signal_done(h);
        // Re-signalling an already-done handle is a no-op.
        seq.signal_done(h);
    }
    assert_eq!(seq.pending_count(), 0);
    let mut out = vec![0u32; ref_mgr.flushed.len()];
    assert_eq!(seq.drain_flushed(&mut out), ref_mgr.flushed.len());
    assert_eq!(out, ref_mgr.flushed);
}
