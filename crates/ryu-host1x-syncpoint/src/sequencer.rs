//! Tegra Host1x Syncpoint and Hardware Fence Sequencer implementation.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncptIncr {
    pub id: u32,
    pub class_id: u32,
    pub syncpt_id: u32,
    pub done: bool,
}

/// Id stored on plain `increment()` entries. Never handed out as a real handle,
/// so `signal_done` can never match (and complete) an untracked entry.
pub const UNTRACKED_INCR_ID: u32 = u32::MAX;

#[derive(Debug, Default)]
struct SequencerInner {
    queue: VecDeque<SyncptIncr>,
    flushed: VecDeque<u32>,
    current_id: u32,
    /// Monotonic sequence number of `queue.front()`; entry `i` has `base_seq + i`.
    base_seq: u64,
    /// Pending tracked handle -> sequence number of its queue entry.
    pending: HashMap<u32, u64>,
}

impl SequencerInner {
    fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(128),
            flushed: VecDeque::with_capacity(128),
            current_id: 0,
            base_seq: 0,
            pending: HashMap::with_capacity(128),
        }
    }

    fn increment(&mut self, syncpt_id: u32) {
        self.queue.push_back(SyncptIncr {
            id: UNTRACKED_INCR_ID,
            class_id: 0,
            syncpt_id,
            done: true,
        });
        self.flush_sequential_done();
    }

    fn increment_when_done(&mut self, class_id: u32, syncpt_id: u32) -> u32 {
        let handle = self.current_id;
        self.current_id = self.current_id.wrapping_add(1);
        if self.current_id == UNTRACKED_INCR_ID {
            // Skip the reserved sentinel when the counter wraps.
            self.current_id = 0;
        }

        let seq = self.base_seq + self.queue.len() as u64;
        // If a handle is still pending after a full 2^32 wrap, keep the oldest
        // entry (matches the previous first-match linear search).
        self.pending.entry(handle).or_insert(seq);

        self.queue.push_back(SyncptIncr {
            id: handle,
            class_id,
            syncpt_id,
            done: false,
        });

        handle
    }

    fn signal_done(&mut self, handle: u32) {
        if let Some(&seq) = self.pending.get(&handle) {
            let idx = (seq - self.base_seq) as usize;
            self.queue[idx].done = true;
        }
        self.flush_sequential_done();
    }

    fn flush_sequential_done(&mut self) {
        while let Some(front) = self.queue.front() {
            if front.done {
                let item = self.queue.pop_front().unwrap();
                if item.id != UNTRACKED_INCR_ID
                    && self.pending.get(&item.id) == Some(&self.base_seq)
                {
                    self.pending.remove(&item.id);
                }
                self.base_seq += 1;
                self.flushed.push_back(item.syncpt_id);
            } else {
                break;
            }
        }
    }

    fn drain_flushed(&mut self, dst: &mut [u32]) -> usize {
        let to_drain = self.flushed.len().min(dst.len());
        for i in 0..to_drain {
            dst[i] = self.flushed.pop_front().unwrap();
        }
        to_drain
    }

    fn reset(&mut self) {
        self.queue.clear();
        self.flushed.clear();
        self.pending.clear();
        self.base_seq = 0;
        self.current_id = 0;
    }

    fn pending_count(&self) -> usize {
        self.queue.len()
    }

    fn flushed_count(&self) -> usize {
        self.flushed.len()
    }
}

/// Thread-safe Tegra Host1x Syncpoint Increment Sequencer.
#[derive(Debug)]
pub struct SyncptSequencer {
    inner: Mutex<SequencerInner>,
}

impl SyncptSequencer {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(SequencerInner::new()),
        }
    }

    pub fn increment(&self, syncpt_id: u32) {
        self.inner.lock().unwrap().increment(syncpt_id);
    }

    pub fn increment_when_done(&self, class_id: u32, syncpt_id: u32) -> u32 {
        self.inner
            .lock()
            .unwrap()
            .increment_when_done(class_id, syncpt_id)
    }

    pub fn signal_done(&self, handle: u32) {
        self.inner.lock().unwrap().signal_done(handle);
    }

    pub fn drain_flushed(&self, dst: &mut [u32]) -> usize {
        self.inner.lock().unwrap().drain_flushed(dst)
    }

    pub fn reset(&self) {
        self.inner.lock().unwrap().reset();
    }

    pub fn pending_count(&self) -> usize {
        self.inner.lock().unwrap().pending_count()
    }

    pub fn flushed_count(&self) -> usize {
        self.inner.lock().unwrap().flushed_count()
    }
}

impl Default for SyncptSequencer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_counter_wrap_skips_sentinel() {
        let mut inner = SequencerInner::new();
        inner.current_id = UNTRACKED_INCR_ID - 1;
        let a = inner.increment_when_done(0, 1);
        assert_eq!(a, UNTRACKED_INCR_ID - 1);
        // A plain increment queued between `a` and the wrapped handle 0. With a
        // plain-increment id of 0 it would shadow handle 0 in `signal_done`.
        inner.increment(2);
        let b = inner.increment_when_done(0, 3);
        assert_eq!(b, 0, "counter must wrap past the sentinel to 0");

        // Signalling the sentinel is a no-op.
        inner.signal_done(UNTRACKED_INCR_ID);
        assert_eq!(inner.pending_count(), 3);
        assert_eq!(inner.flushed_count(), 0);

        // Completing handle 0 must mark the real handle-0 entry (syncpt 3).
        inner.signal_done(b);
        assert_eq!(inner.flushed_count(), 0);
        assert!(inner.queue[2].done && inner.queue[2].syncpt_id == 3);

        inner.signal_done(a);
        let mut out = [0u32; 4];
        assert_eq!(inner.drain_flushed(&mut out), 3);
        assert_eq!(&out[..3], &[1, 2, 3]);
        assert!(inner.pending.is_empty());
    }
}
