use ryu_surfaceflinger_queue::*;
use std::sync::Arc;
use std::thread;

#[test]
fn test_slot_lifecycle() {
    let q = SurfaceFlingerQueue::new(4);
    assert_eq!(q.get_counts(), (4, 0, 0, 0));

    // Dequeue slot
    let s1 = q.dequeue_slot(4).expect("Should get slot 0");
    assert_eq!(s1, 0);
    assert_eq!(q.get_counts(), (3, 1, 0, 0));

    // Queue slot
    assert!(q.queue_buffer(QueueItem {
        slot: s1 as i32,
        frame_number: 1,
        timestamp: 1000,
        transform: 0,
        scaling_mode: 0,
        _pad: 0,
    }));
    assert_eq!(q.get_counts(), (3, 0, 1, 0));

    // Acquire slot
    let item = q.acquire_buffer().expect("Should acquire item");
    assert_eq!(item.slot, 0);
    assert_eq!(item.frame_number, 1);
    assert_eq!(q.get_counts(), (3, 0, 0, 1));

    // Release slot
    assert!(q.release_buffer(0));
    assert_eq!(q.get_counts(), (4, 0, 0, 0));
}

#[test]
fn test_oldest_free_slot_selection() {
    let q = SurfaceFlingerQueue::new(4);
    q.sync_slot_state(0, BufferState::Free as u8, 100);
    q.sync_slot_state(1, BufferState::Free as u8, 50);
    q.sync_slot_state(2, BufferState::Free as u8, 200);
    q.sync_slot_state(3, BufferState::Acquired as u8, 30);

    let oldest = q.find_oldest_free_slot(4).unwrap();
    assert_eq!(oldest, 1); // Frame 50 is lowest among Free slots
}

#[test]
fn test_lock_free_spsc_multithread() {
    let q = Arc::new(SurfaceFlingerQueue::new(8));
    let q_prod = Arc::clone(&q);
    let q_cons = Arc::clone(&q);

    const TOTAL_FRAMES: u64 = 5_000;

    let prod = thread::spawn(move || {
        for frame in 1..=TOTAL_FRAMES {
            loop {
                if let Some(slot) = q_prod.dequeue_slot(8) {
                    q_prod.queue_buffer(QueueItem {
                        slot: slot as i32,
                        frame_number: frame,
                        timestamp: (frame * 16) as i64,
                        transform: 0,
                        scaling_mode: 0,
                        _pad: 0,
                    });
                    break;
                }
                std::hint::spin_loop();
            }
        }
    });

    let cons = thread::spawn(move || {
        let mut received = 0u64;
        while received < TOTAL_FRAMES {
            if let Some(item) = q_cons.acquire_buffer() {
                received += 1;
                assert_eq!(item.frame_number, received);
                q_cons.release_buffer(item.slot as usize);
            } else {
                std::hint::spin_loop();
            }
        }
    });

    prod.join().unwrap();
    cons.join().unwrap();
    assert_eq!(q.get_counts(), (8, 0, 0, 0));
}

#[test]
fn test_zero_copy_parcel() {
    let mut buf = [0u8; 128];
    let total_len = {
        let mut writer = ParcelWriter::new(&mut buf).unwrap();
        assert!(writer.write_i32(42));
        assert!(writer.write_u64(0x1234_5678_9ABC_DEF0));
        assert!(writer.write_bytes_inplace(&[1, 2, 3, 4, 5]));
        writer.finish()
    };

    assert!(total_len > 16);

    let mut reader = ParcelReader::new(&buf[..total_len]).unwrap();
    assert_eq!(reader.read_i32(), Some(42));
    assert_eq!(reader.read_u64(), Some(0x1234_5678_9ABC_DEF0));
    let bytes = reader.read_bytes_inplace(5).unwrap();
    assert_eq!(bytes, &[1, 2, 3, 4, 5]);
}

#[test]
fn test_c_abi_lifecycle() {
    assert_eq!(ryu_surfaceflinger_version(), 0x00010000);
    let q = ryu_sf_queue_create(3);
    assert!(!q.is_null());

    let slot = ryu_sf_queue_dequeue_slot(q, 3);
    assert!(slot >= 0 && slot < 3);

    assert_eq!(ryu_sf_queue_queue_buffer(q, slot, 10, 9999, 0, 0), 1);

    let mut item = QueueItem::default();
    assert_eq!(ryu_sf_queue_acquire_buffer(q, &mut item), 1);
    assert_eq!(item.slot, slot);
    assert_eq!(item.frame_number, 10);

    assert_eq!(ryu_sf_queue_release_buffer(q, slot), 1);

    let (mut f, mut d, mut qu, mut a) = (0, 0, 0, 0);
    ryu_sf_queue_get_counts(q, &mut f, &mut d, &mut qu, &mut a);
    assert_eq!((f, d, qu, a), (3, 0, 0, 0));

    ryu_sf_queue_destroy(q);
}
