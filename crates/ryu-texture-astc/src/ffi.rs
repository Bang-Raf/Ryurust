use crate::decoder::{decode_astc_to_rgba8, decompress_block, query_decompressed_size};

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

#[no_mangle]
pub unsafe extern "C" fn ryu_astc_decompress_block(
    input_16_bytes: *const u8,
    output_rgba8: *mut u32,
    block_width: i32,
    block_height: i32,
) -> i32 {
    if input_16_bytes.is_null() || output_rgba8.is_null() || block_width <= 0 || block_height <= 0 {
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
