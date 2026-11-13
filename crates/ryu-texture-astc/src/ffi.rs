use crate::decoder::{decode_astc_to_rgba8, decompress_block, query_decompressed_size};

pub const RYU_TEXTURE_ASTC_VERSION: u32 = 0x0001_0000;

#[no_mangle]
pub extern "C" fn ryu_texture_astc_version() -> u32 {
    RYU_TEXTURE_ASTC_VERSION
}

#[no_mangle]
pub extern "C" fn ryu_astc_query_decompressed_size(
    width: i32,
    height: i32,
    depth: i32,
    levels: i32,
    layers: i32,
) -> usize {
    if width <= 0 || height <= 0 || depth <= 0 || levels <= 0 || layers <= 0 {
        return 0;
    }
    query_decompressed_size(
        width as usize,
        height as usize,
        depth as usize,
        levels as usize,
        layers as usize,
    )
}

/// Decodes one block. Returns `0` on success, `1` for an illegal or unsupported block
/// (the texels are then zero, as in C#), `-1` for null pointers or bad block sizes.
/// Block sizes above 12x12 are rejected with `-1`, as C# rejects them with an exception.
///
/// # Safety
/// `input_16_bytes` must be valid for reads of 16 bytes and `output_rgba8` valid for
/// writes of `block_width * block_height` `u32`s; the two must not overlap.
#[no_mangle]
pub unsafe extern "C" fn ryu_astc_decompress_block(
    input_16_bytes: *const u8,
    output_rgba8: *mut u32,
    block_width: i32,
    block_height: i32,
) -> i32 {
    if input_16_bytes.is_null()
        || output_rgba8.is_null()
        || !(1..=12).contains(&block_width)
        || !(1..=12).contains(&block_height)
    {
        return -1;
    }

    let input_bytes: &[u8; 16] = &*(input_16_bytes as *const [u8; 16]);
    let out_slice =
        std::slice::from_raw_parts_mut(output_rgba8, (block_width * block_height) as usize);

    if decompress_block(
        input_bytes,
        out_slice,
        block_width as usize,
        block_height as usize,
    ) {
        0
    } else {
        1
    }
}

/// Decodes a whole ASTC texture to RGBA8 (see [`decode_astc_to_rgba8`]).
///
/// Returns:
/// * `0`: every block decoded (C# `TryDecodeToRgba8` returns `true`).
/// * `1`: the output was fully written but at least one block was illegal or
///   unsupported and was written as zeros (C# returns `false`); also returned,
///   with nothing written, when `input_len` / `output_len` are too small for the texture.
/// * `-1`: a null pointer. `-2`: a non-positive dimension, level or layer count.
///
/// # Safety
/// `input_ptr` must be valid for reads of `input_len` bytes and `output_ptr` valid for
/// writes of `output_len` bytes; the two buffers must not overlap.
#[no_mangle]
pub unsafe extern "C" fn ryu_astc_decode_to_rgba8(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    block_width: i32,
    block_height: i32,
    width: i32,
    height: i32,
    depth: i32,
    levels: i32,
    layers: i32,
) -> i32 {
    if input_ptr.is_null() || output_ptr.is_null() {
        return -1;
    }
    if block_width <= 0
        || block_height <= 0
        || width <= 0
        || height <= 0
        || depth <= 0
        || levels <= 0
        || layers <= 0
    {
        return -2;
    }

    let in_slice = std::slice::from_raw_parts(input_ptr, input_len);
    let out_slice = std::slice::from_raw_parts_mut(output_ptr, output_len);

    let ok = decode_astc_to_rgba8(
        in_slice,
        out_slice,
        block_width as usize,
        block_height as usize,
        width as usize,
        height as usize,
        depth as usize,
        levels as usize,
        layers as usize,
    );

    if ok {
        0
    } else {
        1
    }
}

/// Parallel version of [`ryu_astc_decode_to_rgba8`]; same arguments and return codes.
///
/// # Safety
/// Same requirements as [`ryu_astc_decode_to_rgba8`].
#[no_mangle]
pub unsafe extern "C" fn ryu_astc_decode_to_rgba8_parallel(
    input_ptr: *const u8,
    input_len: usize,
    output_ptr: *mut u8,
    output_len: usize,
    block_width: i32,
    block_height: i32,
    width: i32,
    height: i32,
    depth: i32,
    levels: i32,
    layers: i32,
) -> i32 {
    if input_ptr.is_null() || output_ptr.is_null() {
        return -1;
    }
    if block_width <= 0
        || block_height <= 0
        || width <= 0
        || height <= 0
        || depth <= 0
        || levels <= 0
        || layers <= 0
    {
        return -2;
    }

    let in_slice = std::slice::from_raw_parts(input_ptr, input_len);
    let out_slice = std::slice::from_raw_parts_mut(output_ptr, output_len);

    #[cfg(feature = "parallel")]
    let ok = crate::decoder::decode_astc_to_rgba8_parallel(
        in_slice,
        out_slice,
        block_width as usize,
        block_height as usize,
        width as usize,
        height as usize,
        depth as usize,
        levels as usize,
        layers as usize,
    );

    #[cfg(not(feature = "parallel"))]
    let ok = decode_astc_to_rgba8(
        in_slice,
        out_slice,
        block_width as usize,
        block_height as usize,
        width as usize,
        height as usize,
        depth as usize,
        levels as usize,
        layers as usize,
    );

    if ok {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ptr;

    #[test]
    fn version_export_returns_constant() {
        assert_eq!(ryu_texture_astc_version(), RYU_TEXTURE_ASTC_VERSION);
    }

    #[test]
    fn null_pointers_and_bad_block_sizes_are_rejected() {
        let input = [0u8; 16];
        let mut out = [0u32; 16 * 16];
        let mut bytes = [0u8; 64];
        // SAFETY: every non-null pointer points at a live buffer large enough for the call.
        unsafe {
            assert_eq!(
                ryu_astc_decompress_block(ptr::null(), out.as_mut_ptr(), 4, 4),
                -1
            );
            assert_eq!(
                ryu_astc_decompress_block(input.as_ptr(), ptr::null_mut(), 4, 4),
                -1
            );
            assert_eq!(
                ryu_astc_decompress_block(input.as_ptr(), out.as_mut_ptr(), 13, 4),
                -1
            );
            assert_eq!(
                ryu_astc_decompress_block(input.as_ptr(), out.as_mut_ptr(), 4, 16),
                -1
            );
            for f in [ryu_astc_decode_to_rgba8, ryu_astc_decode_to_rgba8_parallel] {
                assert_eq!(
                    f(ptr::null(), 16, bytes.as_mut_ptr(), 64, 4, 4, 4, 4, 1, 1, 1),
                    -1
                );
                assert_eq!(
                    f(input.as_ptr(), 16, ptr::null_mut(), 64, 4, 4, 4, 4, 1, 1, 1),
                    -1
                );
                // A 16x16 footprint is not ASTC; it must fail, not overrun the texel scratch.
                let mut big = vec![0u8; 16 * 16 * 4];
                let len = big.len();
                assert_eq!(
                    f(
                        input.as_ptr(),
                        16,
                        big.as_mut_ptr(),
                        len,
                        16,
                        16,
                        16,
                        16,
                        1,
                        1,
                        1
                    ),
                    1
                );
            }
        }
    }
}
