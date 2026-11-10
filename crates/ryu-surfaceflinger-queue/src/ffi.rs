use crate::parcel::ParcelReader;
use crate::queue::SurfaceFlingerQueue;
use crate::types::{ParcelHeader, QueueItem, INVALID_BUFFER_SLOT};
use std::slice;

pub const RYU_SURFACEFLINGER_VERSION: u32 = 0x00010000;

#[no_mangle]
pub extern "C" fn ryu_surfaceflinger_version() -> u32 {
    RYU_SURFACEFLINGER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_create(max_buffers: u32) -> *mut SurfaceFlingerQueue {
    Box::into_raw(Box::new(SurfaceFlingerQueue::new(max_buffers as usize)))
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_destroy(q: *mut SurfaceFlingerQueue) {
    if !q.is_null() {
        unsafe {
            drop(Box::from_raw(q));
        }
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_set_max_buffers(q: *mut SurfaceFlingerQueue, max_buffers: u32) {
    if !q.is_null() {
        let queue = unsafe { &*q };
        queue.set_max_buffers(max_buffers as usize);
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_find_free_slot(
    q: *const SurfaceFlingerQueue,
    max_buffers: u32,
) -> i32 {
    if q.is_null() {
        return INVALID_BUFFER_SLOT;
    }
    let queue = unsafe { &*q };
    match queue.find_oldest_free_slot(max_buffers as usize) {
        Some(slot) => slot as i32,
        None => INVALID_BUFFER_SLOT,
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_dequeue_slot(
    q: *const SurfaceFlingerQueue,
    max_buffers: u32,
) -> i32 {
    if q.is_null() {
        return INVALID_BUFFER_SLOT;
    }
    let queue = unsafe { &*q };
    match queue.dequeue_slot(max_buffers as usize) {
        Some(slot) => slot as i32,
        None => INVALID_BUFFER_SLOT,
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_queue_buffer(
    q: *const SurfaceFlingerQueue,
    slot: i32,
    frame_number: u64,
    timestamp: i64,
    transform: u32,
    scaling_mode: u32,
) -> i32 {
    if q.is_null() || slot < 0 {
        return 0;
    }
    let queue = unsafe { &*q };
    let item = QueueItem {
        slot,
        transform,
        scaling_mode,
        _pad: 0,
        frame_number,
        timestamp,
    };
    if queue.queue_buffer(item) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_acquire_buffer(
    q: *const SurfaceFlingerQueue,
    out_item: *mut QueueItem,
) -> i32 {
    if q.is_null() || out_item.is_null() {
        return 0;
    }
    let queue = unsafe { &*q };
    match queue.acquire_buffer() {
        Some(item) => {
            unsafe { *out_item = item };
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_release_buffer(q: *const SurfaceFlingerQueue, slot: i32) -> i32 {
    if q.is_null() || slot < 0 {
        return 0;
    }
    let queue = unsafe { &*q };
    if queue.release_buffer(slot as usize) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_cancel_buffer(q: *const SurfaceFlingerQueue, slot: i32) -> i32 {
    if q.is_null() || slot < 0 {
        return 0;
    }
    let queue = unsafe { &*q };
    if queue.cancel_buffer(slot as usize) {
        1
    } else {
        0
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_sync_slot_state(
    q: *const SurfaceFlingerQueue,
    slot: i32,
    state: u8,
    frame_number: u64,
) {
    if !q.is_null() && slot >= 0 {
        let queue = unsafe { &*q };
        queue.sync_slot_state(slot as usize, state, frame_number);
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_get_counts(
    q: *const SurfaceFlingerQueue,
    out_free: *mut u32,
    out_dequeued: *mut u32,
    out_queued: *mut u32,
    out_acquired: *mut u32,
) {
    if q.is_null() {
        return;
    }
    let queue = unsafe { &*q };
    let (f, d, q_cnt, a) = queue.get_counts();
    unsafe {
        if !out_free.is_null() {
            *out_free = f;
        }
        if !out_dequeued.is_null() {
            *out_dequeued = d;
        }
        if !out_queued.is_null() {
            *out_queued = q_cnt;
        }
        if !out_acquired.is_null() {
            *out_acquired = a;
        }
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_parcel_parse_header(
    buf: *const u8,
    len: usize,
    out_header: *mut ParcelHeader,
) -> i32 {
    if buf.is_null() || out_header.is_null() || len < 16 {
        return 0;
    }
    let slice = unsafe { slice::from_raw_parts(buf, len) };
    match ParcelReader::new(slice) {
        Some(reader) => {
            unsafe { *out_header = reader.header };
            1
        }
        None => 0,
    }
}

#[no_mangle]
pub extern "C" fn ryu_sf_parcel_read_i32(
    buf: *const u8,
    len: usize,
    payload_pos: *mut u32,
    out_val: *mut i32,
) -> i32 {
    if buf.is_null() || payload_pos.is_null() || out_val.is_null() {
        return 0;
    }
    let pos = unsafe { *payload_pos } as usize;
    if pos + 4 > len {
        return 0;
    }
    let s = unsafe { slice::from_raw_parts(buf, len) };
    let val = i32::from_le_bytes([s[pos], s[pos + 1], s[pos + 2], s[pos + 3]]);
    unsafe {
        *out_val = val;
        *payload_pos = (pos + 4) as u32;
    }
    1
}

#[no_mangle]
pub extern "C" fn ryu_sf_parcel_write_i32(
    buf: *mut u8,
    capacity: usize,
    payload_pos: *mut u32,
    val: i32,
) -> i32 {
    if buf.is_null() || payload_pos.is_null() {
        return 0;
    }
    let pos = unsafe { *payload_pos } as usize;
    if pos + 4 > capacity {
        return 0;
    }
    let s = unsafe { slice::from_raw_parts_mut(buf, capacity) };
    s[pos..pos + 4].copy_from_slice(&val.to_le_bytes());
    unsafe {
        *payload_pos = (pos + 4) as u32;
    }
    1
}

#[no_mangle]
pub extern "C" fn ryu_sf_parcel_write_header(
    buf: *mut u8,
    capacity: usize,
    payload_size: u32,
    objects_size: u32,
) -> u32 {
    if buf.is_null() || capacity < 16 {
        return 0;
    }
    let s = unsafe { slice::from_raw_parts_mut(buf, capacity) };
    let payload_offset = 16u32;
    let object_offset = payload_offset + payload_size;
    s[0..4].copy_from_slice(&payload_size.to_le_bytes());
    s[4..8].copy_from_slice(&payload_offset.to_le_bytes());
    s[8..12].copy_from_slice(&objects_size.to_le_bytes());
    s[12..16].copy_from_slice(&object_offset.to_le_bytes());
    16 + payload_size + objects_size
}
