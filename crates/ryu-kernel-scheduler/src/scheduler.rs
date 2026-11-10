use crate::queue::ThreadQueue;
use std::array;

pub const PRIORITIES_COUNT: usize = 64;
pub const CPU_CORES_COUNT: usize = 4;

pub struct RyuScheduler {
    scheduled: [[ThreadQueue; PRIORITIES_COUNT]; CPU_CORES_COUNT],
    suggested: [[ThreadQueue; PRIORITIES_COUNT]; CPU_CORES_COUNT],
    scheduled_mask: [u64; CPU_CORES_COUNT],
    suggested_mask: [u64; CPU_CORES_COUNT],
}

impl Default for RyuScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl RyuScheduler {
    pub fn new() -> Self {
        Self {
            scheduled: array::from_fn(|_| array::from_fn(|_| ThreadQueue::new())),
            suggested: array::from_fn(|_| array::from_fn(|_| ThreadQueue::new())),
            scheduled_mask: [0; CPU_CORES_COUNT],
            suggested_mask: [0; CPU_CORES_COUNT],
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    #[inline]
    pub fn schedule(&mut self, core: usize, prio: usize, thread_id: u64) {
        if core >= CPU_CORES_COUNT || prio >= PRIORITIES_COUNT || thread_id == 0 {
            return;
        }
        self.scheduled[core][prio].push_back(thread_id);
        self.scheduled_mask[core] |= 1u64 << prio;
    }

    #[inline]
    pub fn schedule_prepend(&mut self, core: usize, prio: usize, thread_id: u64) {
        if core >= CPU_CORES_COUNT || prio >= PRIORITIES_COUNT || thread_id == 0 {
            return;
        }
        self.scheduled[core][prio].push_front(thread_id);
        self.scheduled_mask[core] |= 1u64 << prio;
    }

    #[inline]
    pub fn unschedule(&mut self, core: usize, prio: usize, thread_id: u64) {
        if core >= CPU_CORES_COUNT || prio >= PRIORITIES_COUNT || thread_id == 0 {
            return;
        }
        let queue = &mut self.scheduled[core][prio];
        if queue.remove(thread_id) && queue.is_empty() {
            self.scheduled_mask[core] &= !(1u64 << prio);
        }
    }

    #[inline]
    pub fn reschedule(&mut self, core: usize, prio: usize, thread_id: u64) -> u64 {
        if core >= CPU_CORES_COUNT || prio >= PRIORITIES_COUNT || thread_id == 0 {
            return 0;
        }
        self.scheduled[core][prio]
            .reschedule(thread_id)
            .unwrap_or(0)
    }

    #[inline]
    pub fn suggest(&mut self, core: usize, prio: usize, thread_id: u64) {
        if core >= CPU_CORES_COUNT || prio >= PRIORITIES_COUNT || thread_id == 0 {
            return;
        }
        self.suggested[core][prio].push_front(thread_id);
        self.suggested_mask[core] |= 1u64 << prio;
    }

    #[inline]
    pub fn unsuggest(&mut self, core: usize, prio: usize, thread_id: u64) {
        if core >= CPU_CORES_COUNT || prio >= PRIORITIES_COUNT || thread_id == 0 {
            return;
        }
        let queue = &mut self.suggested[core][prio];
        if queue.remove(thread_id) && queue.is_empty() {
            self.suggested_mask[core] &= !(1u64 << prio);
        }
    }

    #[inline]
    pub fn transfer_to_core(&mut self, prio: usize, src_core: i32, dst_core: i32, thread_id: u64) {
        if prio >= PRIORITIES_COUNT || src_core == dst_core || thread_id == 0 {
            return;
        }

        if src_core >= 0 && (src_core as usize) < CPU_CORES_COUNT {
            self.unschedule(src_core as usize, prio, thread_id);
        }

        if dst_core >= 0 && (dst_core as usize) < CPU_CORES_COUNT {
            self.unsuggest(dst_core as usize, prio, thread_id);
            self.schedule(dst_core as usize, prio, thread_id);
        }

        if src_core >= 0 && (src_core as usize) < CPU_CORES_COUNT {
            self.suggest(src_core as usize, prio, thread_id);
        }
    }

    #[inline(always)]
    pub fn scheduled_first(&self, core: usize) -> u64 {
        if core >= CPU_CORES_COUNT {
            return 0;
        }
        let mask = self.scheduled_mask[core];
        if mask == 0 {
            return 0;
        }
        let prio = mask.trailing_zeros() as usize;
        self.scheduled[core][prio].first().unwrap_or(0)
    }

    #[inline(always)]
    pub fn suggested_first(&self, core: usize) -> u64 {
        if core >= CPU_CORES_COUNT {
            return 0;
        }
        let mask = self.suggested_mask[core];
        if mask == 0 {
            return 0;
        }
        let prio = mask.trailing_zeros() as usize;
        self.suggested[core][prio].first().unwrap_or(0)
    }

    #[inline]
    pub fn scheduled_element_at(&self, core: usize, index: usize) -> u64 {
        if core >= CPU_CORES_COUNT {
            return 0;
        }
        let mut mask = self.scheduled_mask[core];
        let mut curr_idx = 0;
        while mask != 0 {
            let prio = mask.trailing_zeros() as usize;
            mask &= mask - 1; // Clear lowest set bit via BLSR
            let q = &self.scheduled[core][prio];
            let len = q.len();
            if curr_idx + len > index {
                return q.get(index - curr_idx).unwrap_or(0);
            }
            curr_idx += len;
        }
        0
    }

    #[inline]
    pub fn scheduled_dynamic_prio_first(&self, core: usize, prio: usize) -> u64 {
        if core >= CPU_CORES_COUNT || prio >= PRIORITIES_COUNT {
            return 0;
        }
        if (self.scheduled_mask[core] & (1u64 << prio)) != 0 {
            self.scheduled[core][prio].first().unwrap_or(0)
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn has_scheduled(&self, core: usize) -> bool {
        if core >= CPU_CORES_COUNT {
            return false;
        }
        self.scheduled_mask[core] != 0
    }

    #[inline(always)]
    pub fn has_suggested(&self, core: usize) -> bool {
        if core >= CPU_CORES_COUNT {
            return false;
        }
        self.suggested_mask[core] != 0
    }

    #[inline(always)]
    pub fn get_scheduled_mask(&self, core: usize) -> u64 {
        if core < CPU_CORES_COUNT {
            self.scheduled_mask[core]
        } else {
            0
        }
    }

    #[inline(always)]
    pub fn get_suggested_mask(&self, core: usize) -> u64 {
        if core < CPU_CORES_COUNT {
            self.suggested_mask[core]
        } else {
            0
        }
    }

    pub fn export_scheduled(&self, core: usize, out_buf: &mut [u64]) -> usize {
        if core >= CPU_CORES_COUNT || out_buf.is_empty() {
            return 0;
        }
        let mut mask = self.scheduled_mask[core];
        let mut written = 0;
        while mask != 0 && written < out_buf.len() {
            let prio = mask.trailing_zeros() as usize;
            mask &= mask - 1;
            let q = &self.scheduled[core][prio];
            let n = q.export(&mut out_buf[written..]);
            written += n;
        }
        written
    }

    pub fn export_suggested(&self, core: usize, out_buf: &mut [u64]) -> usize {
        if core >= CPU_CORES_COUNT || out_buf.is_empty() {
            return 0;
        }
        let mut mask = self.suggested_mask[core];
        let mut written = 0;
        while mask != 0 && written < out_buf.len() {
            let prio = mask.trailing_zeros() as usize;
            mask &= mask - 1;
            let q = &self.suggested[core][prio];
            let n = q.export(&mut out_buf[written..]);
            written += n;
        }
        written
    }
}
