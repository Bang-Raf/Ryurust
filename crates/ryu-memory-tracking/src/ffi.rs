use crate::bitmap::AtomicPageBitmap;

pub const RYU_BITMAP_OK: i32 = 0;
pub const RYU_BITMAP_NOT_FOUND: i32 = -1;

#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_create(
    bit_count: usize,
    initial_set: i32,
) -> *mut AtomicPageBitmap {
    Box::into_raw(Box::new(AtomicPageBitmap::new(bit_count, initial_set != 0)))
}

#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_destroy(bm: *mut AtomicPageBitmap) {
    if !bm.is_null() {
        drop(Box::from_raw(bm));
    }
}

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

#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_set(bm: *mut AtomicPageBitmap, bit: usize, value: i32) {
    if !bm.is_null() {
        (*bm).set(bit, value != 0);
    }
}

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

#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_clear(bm: *mut AtomicPageBitmap) {
    if !bm.is_null() {
        (*bm).clear();
    }
}

#[no_mangle]
pub unsafe extern "C" fn ryu_bitmap_count_set(bm: *const AtomicPageBitmap) -> usize {
    if bm.is_null() {
        return 0;
    }
    (*bm).count_set_bits()
}

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
