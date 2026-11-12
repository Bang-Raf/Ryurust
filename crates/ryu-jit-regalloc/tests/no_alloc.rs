//! `run_pass` must not touch the heap once the engine's buffers are warm, including when
//! the input is unsorted. Kept in its own test binary because it replaces the global
//! allocator.

use ryu_jit_regalloc::{Interval, LinearScanEngine, RegisterMasksNative};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;

// SAFETY: forwards everything to `System`; the counter is a const-initialised thread-local
// `Cell` without a destructor, so touching it cannot allocate or re-enter.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.with(|c| c.set(c.get() + 1));
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout)
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATIONS.with(|c| c.set(c.get() + 1));
        System.realloc(ptr, layout, new_size)
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

#[test]
fn test_run_pass_on_unsorted_input_does_not_allocate() {
    const N: usize = 4096;
    // Reverse start order, a few equal starts, some long intervals to force spills.
    let intervals: Vec<Interval> = (0..N)
        .map(|i| {
            let start = ((N - i) / 2 * 2) as i32;
            Interval {
                id: i as u32,
                start,
                end: start + if i % 7 == 0 { 400 } else { 6 },
                reg_type: (i % 3 == 0) as u8,
                ..Default::default()
            }
        })
        .collect();
    let masks = RegisterMasksNative {
        int_available: 0xFF,
        vec_available: 0xFF,
        registers_count: 8,
    };

    let mut engine = LinearScanEngine::new();
    let run = |engine: &mut LinearScanEngine| {
        engine.reset();
        for &iv in &intervals {
            engine.add_interval(iv);
        }
        let before = ALLOCATIONS.with(|c| c.get());
        let allocated = engine.run_pass(masks);
        (allocated, ALLOCATIONS.with(|c| c.get()) - before)
    };

    // First pass may grow the engine's reusable scratch buffer.
    let (first, _) = run(&mut engine);
    let (second, allocations) = run(&mut engine);
    assert_eq!(first, second);
    assert!(first > 0 && first < N, "the input should force some spills");
    assert_eq!(allocations, 0, "run_pass allocated on unsorted input");
}
