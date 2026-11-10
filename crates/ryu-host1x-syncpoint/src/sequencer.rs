//! Tegra Host1x Syncpoint and Hardware Fence Sequencer implementation.

use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncptIncr {
    pub id: u32,
    pub class_id: u32,
    pub syncpt_id: u32,
    pub done: bool,
}

#[derive(Debug, Default)]
struct SequencerInner {
    queue: VecDeque<SyncptIncr>,
    flushed: VecDeque<u32>,
    current_id: u32,
}

impl SequencerInner {
    fn new() -> Self {
        Self {
            queue: VecDeque::with_capacity(128),
            flushed: VecDeque::with_capacity(128),
            current_id: 0,
        }
    }

    fn increment(&mut self, syncpt_id: u32) {
        self.queue.push_back(SyncptIncr {
            id: 0,
            class_id: 0,
            syncpt_id,
            done: true,
        });
        self.flush_sequential_done();
    }

    fn increment_when_done(&mut self, class_id: u32, syncpt_id: u32) -> u32 {
        let handle = self.current_id;
        self.current_id = self.current_id.wrapping_add(1);

        self.queue.push_back(SyncptIncr {
            id: handle,
            class_id,
            syncpt_id,
            done: false,
        });

        handle
    }

    fn signal_done(&mut self, handle: u32) {
        for incr in self.queue.iter_mut() {
            if incr.id == handle {
                incr.done = true;
                break;
            }
        }
        self.flush_sequential_done();
    }

    fn flush_sequential_done(&mut self) {
        while let Some(front) = self.queue.front() {
            if front.done {
                let item = self.queue.pop_front().unwrap();
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
