use crate::parcel::ParcelReader;
use crate::queue::SurfaceFlingerQueue;
use crate::types::{ParcelHeader, QueueItem, INVALID_BUFFER_SLOT};
use std::slice;

pub const RYU_SURFACEFLINGER_VERSION: u32 = 0x00010000;

/// Kept for compatibility with existing bindings; same value as
/// `ryu_surfaceflinger_queue_version`.
#[no_mangle]
pub extern "C" fn ryu_surfaceflinger_version() -> u32 {
    RYU_SURFACEFLINGER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_surfaceflinger_queue_version() -> u32 {
    RYU_SURFACEFLINGER_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_sf_queue_create(max_buffers: u32) -> *mut SurfaceFlingerQueue {
    Box::into_raw(Box::new(SurfaceFlingerQueue::new(max_buffers as usize)))
}

/// # Safety
/// `q` must be null or a handle from `ryu_sf_queue_create` that has not been destroyed
/// yet; it must not be used (by any thread) after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_destroy(q: *mut SurfaceFlingerQueue) {
    if !q.is_null() {
        // SAFETY: a non-null `q` came from `Box::into_raw` in `ryu_sf_queue_create` and is
        // not used again (caller contract).
        unsafe {
            drop(Box::from_raw(q));
        }
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_set_max_buffers(
    q: *mut SurfaceFlingerQueue,
    max_buffers: u32,
) {
    if !q.is_null() {
        // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
        let queue = unsafe { &*q };
        queue.set_max_buffers(max_buffers as usize);
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_find_free_slot(
    q: *const SurfaceFlingerQueue,
    max_buffers: u32,
) -> i32 {
    if q.is_null() {
        return INVALID_BUFFER_SLOT;
    }
    // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
    let queue = unsafe { &*q };
    match queue.find_oldest_free_slot(max_buffers as usize) {
        Some(slot) => slot as i32,
        None => INVALID_BUFFER_SLOT,
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_dequeue_slot(
    q: *const SurfaceFlingerQueue,
    max_buffers: u32,
) -> i32 {
    if q.is_null() {
        return INVALID_BUFFER_SLOT;
    }
    // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
    let queue = unsafe { &*q };
    match queue.dequeue_slot(max_buffers as usize) {
        Some(slot) => slot as i32,
        None => INVALID_BUFFER_SLOT,
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`. Only one thread at
/// a time may queue buffers (single producer).
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_queue_buffer(
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
    // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
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

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`. `out_item` must be
/// null or valid for writing one `QueueItem`. Only one thread at a time may acquire
/// buffers (single consumer).
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_acquire_buffer(
    q: *const SurfaceFlingerQueue,
    out_item: *mut QueueItem,
) -> i32 {
    if q.is_null() || out_item.is_null() {
        return 0;
    }
    // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
    let queue = unsafe { &*q };
    match queue.acquire_buffer() {
        Some(item) => {
            // SAFETY: `out_item` was checked non-null above and is valid for one write.
            unsafe { *out_item = item };
            1
        }
        None => 0,
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_release_buffer(
    q: *const SurfaceFlingerQueue,
    slot: i32,
) -> i32 {
    if q.is_null() || slot < 0 {
        return 0;
    }
    // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
    let queue = unsafe { &*q };
    if queue.release_buffer(slot as usize) {
        1
    } else {
        0
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_cancel_buffer(
    q: *const SurfaceFlingerQueue,
    slot: i32,
) -> i32 {
    if q.is_null() || slot < 0 {
        return 0;
    }
    // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
    let queue = unsafe { &*q };
    if queue.cancel_buffer(slot as usize) {
        1
    } else {
        0
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_sync_slot_state(
    q: *const SurfaceFlingerQueue,
    slot: i32,
    state: u8,
    frame_number: u64,
) {
    if !q.is_null() && slot >= 0 {
        // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
        let queue = unsafe { &*q };
        queue.sync_slot_state(slot as usize, state, frame_number);
    }
}

/// # Safety
/// `q` must be null or a live handle from `ryu_sf_queue_create`. Each out-pointer
/// must be null (skipped) or valid for writing one `u32`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_queue_get_counts(
    q: *const SurfaceFlingerQueue,
    out_free: *mut u32,
    out_dequeued: *mut u32,
    out_queued: *mut u32,
    out_acquired: *mut u32,
) {
    if q.is_null() {
        return;
    }
    // SAFETY: `q` was checked non-null above and is a live handle (caller contract).
    let queue = unsafe { &*q };
    let (f, d, q_cnt, a) = queue.get_counts();
    // SAFETY: each out-pointer is written only if non-null, in which case the caller
    // guarantees it is valid for one `u32`.
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

/// # Safety
/// `buf` must be null or valid for reads of `len` bytes; `out_header` must be null or
/// valid for writing one `ParcelHeader`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_parcel_parse_header(
    buf: *const u8,
    len: usize,
    out_header: *mut ParcelHeader,
) -> i32 {
    if buf.is_null() || out_header.is_null() || len < 16 {
        return 0;
    }
    // SAFETY: `buf` is non-null and valid for `len` bytes (caller contract).
    let slice = unsafe { slice::from_raw_parts(buf, len) };
    match ParcelReader::new(slice) {
        Some(reader) => {
            // SAFETY: `out_header` was checked non-null above and is valid for one write.
            unsafe { *out_header = reader.header };
            1
        }
        None => 0,
    }
}

/// # Safety
/// `buf` must be null or valid for reads of `len` bytes; `payload_pos` must be null or
/// valid for reading and writing one `u32`; `out_val` null or valid for writing one `i32`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_parcel_read_i32(
    buf: *const u8,
    len: usize,
    payload_pos: *mut u32,
    out_val: *mut i32,
) -> i32 {
    if buf.is_null() || payload_pos.is_null() || out_val.is_null() {
        return 0;
    }
    // SAFETY: `payload_pos` was checked non-null above and is valid for one read.
    let pos = unsafe { *payload_pos } as usize;
    if pos + 4 > len {
        return 0;
    }
    // SAFETY: `buf` is non-null and valid for `len` bytes (caller contract).
    let s = unsafe { slice::from_raw_parts(buf, len) };
    let val = i32::from_le_bytes([s[pos], s[pos + 1], s[pos + 2], s[pos + 3]]);
    // SAFETY: the out-pointers were checked non-null above and are valid for one write.
    unsafe {
        *out_val = val;
        *payload_pos = (pos + 4) as u32;
    }
    1
}

/// # Safety
/// `buf` must be null or valid for writes of `capacity` bytes; `payload_pos` must be
/// null or valid for reading and writing one `u32`.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_parcel_write_i32(
    buf: *mut u8,
    capacity: usize,
    payload_pos: *mut u32,
    val: i32,
) -> i32 {
    if buf.is_null() || payload_pos.is_null() {
        return 0;
    }
    // SAFETY: `payload_pos` was checked non-null above and is valid for one read.
    let pos = unsafe { *payload_pos } as usize;
    if pos + 4 > capacity {
        return 0;
    }
    // SAFETY: `buf` is non-null and valid for `capacity` bytes (caller contract).
    let s = unsafe { slice::from_raw_parts_mut(buf, capacity) };
    s[pos..pos + 4].copy_from_slice(&val.to_le_bytes());
    // SAFETY: the out-pointers were checked non-null above and are valid for one write.
    unsafe {
        *payload_pos = (pos + 4) as u32;
    }
    1
}

/// Returns the total parcel size, or 0 if `buf` is null, `capacity` is under 16
/// bytes, or the total does not fit in a `u32`.
///
/// # Safety
/// `buf` must be null or valid for writes of `capacity` bytes.
#[no_mangle]
pub unsafe extern "C" fn ryu_sf_parcel_write_header(
    buf: *mut u8,
    capacity: usize,
    payload_size: u32,
    objects_size: u32,
) -> u32 {
    if buf.is_null() || capacity < 16 {
        return 0;
    }
    let payload_offset = 16u32;
    let Some(object_offset) = payload_offset.checked_add(payload_size) else {
        return 0;
    };
    let Some(total) = object_offset.checked_add(objects_size) else {
        return 0;
    };
    // SAFETY: `buf` is non-null and valid for `capacity` bytes (caller contract).
    let s = unsafe { slice::from_raw_parts_mut(buf, capacity) };
    s[0..4].copy_from_slice(&payload_size.to_le_bytes());
    s[4..8].copy_from_slice(&payload_offset.to_le_bytes());
    s[8..12].copy_from_slice(&objects_size.to_le_bytes());
    s[12..16].copy_from_slice(&object_offset.to_le_bytes());
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(
            ryu_surfaceflinger_queue_version(),
            RYU_SURFACEFLINGER_VERSION
        );
        assert_eq!(ryu_surfaceflinger_version(), RYU_SURFACEFLINGER_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        let null = ptr::null::<SurfaceFlingerQueue>();
        let mut item = QueueItem {
            slot: 0,
            transform: 0,
            scaling_mode: 0,
            _pad: 0,
            frame_number: 0,
            timestamp: 0,
        };
        let mut buf = [0u8; 32];
        let mut pos = 0u32;
        let mut val = 0i32;
        unsafe {
            ryu_sf_queue_destroy(ptr::null_mut());
            ryu_sf_queue_set_max_buffers(ptr::null_mut(), 4);
            assert_eq!(ryu_sf_queue_find_free_slot(null, 4), INVALID_BUFFER_SLOT);
            assert_eq!(ryu_sf_queue_dequeue_slot(null, 4), INVALID_BUFFER_SLOT);
            assert_eq!(ryu_sf_queue_queue_buffer(null, 0, 1, 1, 0, 0), 0);
            assert_eq!(ryu_sf_queue_acquire_buffer(null, &mut item), 0);
            assert_eq!(ryu_sf_queue_release_buffer(null, 0), 0);
            assert_eq!(ryu_sf_queue_cancel_buffer(null, 0), 0);
            ryu_sf_queue_sync_slot_state(null, 0, 0, 0);
            let mut free = 77u32;
            ryu_sf_queue_get_counts(
                null,
                &mut free,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            assert_eq!(free, 77);

            let q = ryu_sf_queue_create(4);
            assert_eq!(ryu_sf_queue_acquire_buffer(q, ptr::null_mut()), 0);
            ryu_sf_queue_get_counts(
                q,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            );
            ryu_sf_queue_destroy(q);

            let mut header = std::mem::zeroed::<ParcelHeader>();
            assert_eq!(ryu_sf_parcel_parse_header(ptr::null(), 32, &mut header), 0);
            assert_eq!(
                ryu_sf_parcel_parse_header(buf.as_ptr(), 32, ptr::null_mut()),
                0
            );
            assert_eq!(
                ryu_sf_parcel_read_i32(ptr::null(), 32, &mut pos, &mut val),
                0
            );
            assert_eq!(
                ryu_sf_parcel_read_i32(buf.as_ptr(), 32, ptr::null_mut(), &mut val),
                0
            );
            assert_eq!(
                ryu_sf_parcel_read_i32(buf.as_ptr(), 32, &mut pos, ptr::null_mut()),
                0
            );
            assert_eq!(ryu_sf_parcel_write_i32(ptr::null_mut(), 32, &mut pos, 5), 0);
            assert_eq!(
                ryu_sf_parcel_write_i32(buf.as_mut_ptr(), 32, ptr::null_mut(), 5),
                0
            );
            assert_eq!(pos, 0);
            assert_eq!(ryu_sf_parcel_write_header(ptr::null_mut(), 32, 8, 0), 0);
        }
    }
}
