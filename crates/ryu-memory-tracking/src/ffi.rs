use crate::bitmap::AtomicPageBitmap;

pub const RYU_BITMAP_OK: i32 = 0;
pub const RYU_BITMAP_NOT_FOUND: i32 = -1;

pub const RYU_MEMORY_TRACKING_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_memory_tracking_version() -> u32 {
    RYU_MEMORY_TRACKING_VERSION
}

/// # Safety
/// Always sound to call; release the returned handle with `ryu_bitmap_destroy`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_create(
    bit_count: usize,
    initial_set: i32,
) -> *mut AtomicPageBitmap {
    Box::into_raw(Box::new(AtomicPageBitmap::new(bit_count, initial_set != 0)))
}

/// # Safety
/// `bm` must be null or a handle from `ryu_bitmap_create` that has not been destroyed
/// yet; it must not be used (by any thread) after this call.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_destroy(bm: *mut AtomicPageBitmap) {
    if !bm.is_null() {
        drop(Box::from_raw(bm));
    }
}

/// # Safety
/// `bm` must be null (returns 0) or a live handle from `ryu_bitmap_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_is_set(bm: *const AtomicPageBitmap, bit: usize) -> i32 {
    if bm.is_null() {
        return 0;
    }
    if (*bm).is_set(bit) {
        1
    } else {
        0
    }
}

/// # Safety
/// `bm` must be null (returns 0) or a live handle from `ryu_bitmap_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_is_set_range(
    bm: *const AtomicPageBitmap,
    start: usize,
    end: usize,
) -> i32 {
    if bm.is_null() {
        return 0;
    }
    if (*bm).is_set_range(start, end) {
        1
    } else {
        0
    }
}

/// # Safety
/// `bm` must be null (no-op) or a live handle from `ryu_bitmap_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_set(bm: *mut AtomicPageBitmap, bit: usize, value: i32) {
    if !bm.is_null() {
        (*bm).set(bit, value != 0);
    }
}

/// # Safety
/// `bm` must be null (no-op) or a live handle from `ryu_bitmap_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_set_range(
    bm: *mut AtomicPageBitmap,
    start: usize,
    end: usize,
    value: i32,
) {
    if !bm.is_null() {
        (*bm).set_range(start, end, value != 0);
    }
}

/// # Safety
/// `bm` must be null (returns 0) or a live handle from `ryu_bitmap_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_any_set(bm: *const AtomicPageBitmap) -> i32 {
    if bm.is_null() {
        return 0;
    }
    if (*bm).any_set() {
        1
    } else {
        0
    }
}

/// # Safety
/// `bm` must be null (no-op) or a live handle from `ryu_bitmap_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_clear(bm: *mut AtomicPageBitmap) {
    if !bm.is_null() {
        (*bm).clear();
    }
}

/// # Safety
/// `bm` must be null (returns 0) or a live handle from `ryu_bitmap_create`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_count_set(bm: *const AtomicPageBitmap) -> usize {
    if bm.is_null() {
        return 0;
    }
    (*bm).count_set_bits()
}

/// # Safety
/// `bm` must be null or a live handle from `ryu_bitmap_create`; `out_bit` must be null
/// or valid for writing one `usize`. Either being null returns `RYU_BITMAP_NOT_FOUND`.
#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_find_next_set(
    bm: *const AtomicPageBitmap,
    from_bit: usize,
    out_bit: *mut usize,
) -> i32 {
    if bm.is_null() || out_bit.is_null() {
        return RYU_BITMAP_NOT_FOUND;
    }
    match (*bm).find_next_set(from_bit) {
        Some(bit) => {
            *out_bit = bit;
            RYU_BITMAP_OK
        }
        None => RYU_BITMAP_NOT_FOUND,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_matches_constant() {
        assert_eq!(ryu_memory_tracking_version(), RYU_MEMORY_TRACKING_VERSION);
    }

    #[test]
    fn null_pointers_are_rejected() {
        unsafe {
            ryu_bitmap_destroy(ptr::null_mut());
            assert_eq!(ryu_bitmap_is_set(ptr::null(), 0), 0);
            assert_eq!(ryu_bitmap_is_set_range(ptr::null(), 0, 8), 0);
            ryu_bitmap_set(ptr::null_mut(), 0, 1);
            ryu_bitmap_set_range(ptr::null_mut(), 0, 8, 1);
            assert_eq!(ryu_bitmap_any_set(ptr::null()), 0);
            ryu_bitmap_clear(ptr::null_mut());
            assert_eq!(ryu_bitmap_count_set(ptr::null()), 0);

            let bm = ryu_bitmap_create(64, 1);
            let mut bit = 0usize;
            assert_eq!(
                ryu_bitmap_find_next_set(ptr::null(), 0, &mut bit),
                RYU_BITMAP_NOT_FOUND
            );
            assert_eq!(
                ryu_bitmap_find_next_set(bm, 0, ptr::null_mut()),
                RYU_BITMAP_NOT_FOUND
            );
            ryu_bitmap_destroy(bm);
        }
    }
}
