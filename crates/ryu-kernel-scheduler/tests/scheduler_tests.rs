use ryu_kernel_scheduler::*;

#[test]
fn test_empty_scheduler() {
    let sched = RyuScheduler::new();
    for c in 0..CPU_CORES_COUNT {
        assert!(!sched.has_scheduled(c));
        assert!(!sched.has_suggested(c));
        assert_eq!(sched.scheduled_first(c), 0);
        assert_eq!(sched.suggested_first(c), 0);
        assert_eq!(sched.get_scheduled_mask(c), 0);
        assert_eq!(sched.get_suggested_mask(c), 0);
    }
}

#[test]
fn test_schedule_and_priority_order() {
    let mut sched = RyuScheduler::new();
    let core = 1;

    // Schedule thread 100 at prio 20, 200 at prio 5, 300 at prio 10
    sched.schedule(core, 20, 100);
    sched.schedule(core, 5, 200);
    sched.schedule(core, 10, 300);

    assert!(sched.has_scheduled(core));
    let mask = sched.get_scheduled_mask(core);
    assert_eq!(mask, (1u64 << 5) | (1u64 << 10) | (1u64 << 20));

    // Highest priority (lowest numerical index) is 5 -> thread 200
    assert_eq!(sched.scheduled_first(core), 200);
    assert_eq!(sched.scheduled_element_at(core, 0), 200);
    assert_eq!(sched.scheduled_element_at(core, 1), 300);
    assert_eq!(sched.scheduled_element_at(core, 2), 100);
    assert_eq!(sched.scheduled_element_at(core, 3), 0); // Out of bounds

    assert_eq!(sched.scheduled_dynamic_prio_first(core, 5), 200);
    assert_eq!(sched.scheduled_dynamic_prio_first(core, 10), 300);
    assert_eq!(sched.scheduled_dynamic_prio_first(core, 20), 100);
    assert_eq!(sched.scheduled_dynamic_prio_first(core, 15), 0);
}

#[test]
fn test_fifo_within_same_priority_and_reschedule() {
    let mut sched = RyuScheduler::new();
    let core = 0;
    let prio = 12;

    // FIFO: A=10, B=20, C=30
    sched.schedule(core, prio, 10);
    sched.schedule(core, prio, 20);
    sched.schedule(core, prio, 30);

    assert_eq!(sched.scheduled_first(core), 10);
    assert_eq!(sched.scheduled_element_at(core, 0), 10);
    assert_eq!(sched.scheduled_element_at(core, 1), 20);
    assert_eq!(sched.scheduled_element_at(core, 2), 30);

    // Reschedule 10 -> rotated to back, 20 is new head
    let next = sched.reschedule(core, prio, 10);
    assert_eq!(next, 20);
    assert_eq!(sched.scheduled_first(core), 20);
    assert_eq!(sched.scheduled_element_at(core, 0), 20);
    assert_eq!(sched.scheduled_element_at(core, 1), 30);
    assert_eq!(sched.scheduled_element_at(core, 2), 10);
}

#[test]
fn test_unschedule_and_mask_clear() {
    let mut sched = RyuScheduler::new();
    let core = 2;

    sched.schedule(core, 16, 501);
    sched.schedule(core, 16, 502);
    assert_eq!(sched.get_scheduled_mask(core), 1u64 << 16);

    // Remove first
    sched.unschedule(core, 16, 501);
    assert_eq!(sched.get_scheduled_mask(core), 1u64 << 16);
    assert_eq!(sched.scheduled_first(core), 502);

    // Remove second -> queue becomes empty, mask bit cleared
    sched.unschedule(core, 16, 502);
    assert_eq!(sched.get_scheduled_mask(core), 0);
    assert!(!sched.has_scheduled(core));
    assert_eq!(sched.scheduled_first(core), 0);
}

#[test]
fn test_suggest_and_transfer() {
    let mut sched = RyuScheduler::new();
    let prio = 8;
    let thread = 777;

    // Initially thread is on core 0
    sched.schedule(0, prio, thread);
    assert!(sched.has_scheduled(0));
    assert!(!sched.has_scheduled(1));

    // Transfer from core 0 to core 1
    sched.transfer_to_core(prio, 0, 1, thread);

    assert!(!sched.has_scheduled(0));
    assert!(sched.has_suggested(0)); // Suggested on old core
    assert!(sched.has_scheduled(1)); // Scheduled on new core
    assert_eq!(sched.scheduled_first(1), thread);
    assert_eq!(sched.suggested_first(0), thread);
}

#[test]
fn test_schedule_prepend() {
    let mut sched = RyuScheduler::new();
    let core = 3;
    let prio = 25;

    sched.schedule(core, prio, 1);
    sched.schedule(core, prio, 2);
    assert_eq!(sched.scheduled_first(core), 1);

    // Prepend 999
    sched.schedule_prepend(core, prio, 999);
    assert_eq!(sched.scheduled_first(core), 999);
    assert_eq!(sched.scheduled_element_at(core, 0), 999);
    assert_eq!(sched.scheduled_element_at(core, 1), 1);
    assert_eq!(sched.scheduled_element_at(core, 2), 2);
}

#[test]
fn test_high_thread_count_overflow() {
    let mut sched = RyuScheduler::new();
    let core = 0;
    let prio = 30;

    for id in 1..=16 {
        sched.schedule(core, prio, id);
    }

    assert_eq!(sched.scheduled_first(core), 1);
    for id in 1..=16 {
        assert_eq!(sched.scheduled_element_at(core, (id - 1) as usize), id);
    }

    // Remove from middle (element 5)
    sched.unschedule(core, prio, 5);
    assert_eq!(sched.scheduled_element_at(core, 4), 6);

    // Remove from overflow (element 14)
    sched.unschedule(core, prio, 14);

    let mut buf = [0u64; 32];
    let count = sched.export_scheduled(core, &mut buf);
    assert_eq!(count, 14);
}

#[test]
fn test_c_abi_lifecycle() {
    let s = ryu_scheduler_create();
    assert!(!s.is_null());

    assert_eq!(ryu_scheduler_version(), 0x00010000);
    assert_eq!(ryu_scheduler_has_scheduled(s, 0), 0);

    ryu_scheduler_schedule(s, 4, 0, 888);
    ryu_scheduler_schedule(s, 2, 0, 999);

    assert_eq!(ryu_scheduler_has_scheduled(s, 0), 1);
    assert_eq!(ryu_scheduler_first_scheduled(s, 0), 999); // Prio 2 is higher than 4
    assert_eq!(ryu_scheduler_element_at_scheduled(s, 0, 1), 888);

    let mut buf = [0u64; 4];
    let exported = ryu_scheduler_export_scheduled(s, 0, buf.as_mut_ptr(), 4);
    assert_eq!(exported, 2);
    assert_eq!(buf[0], 999);
    assert_eq!(buf[1], 888);

    ryu_scheduler_destroy(s);
}
