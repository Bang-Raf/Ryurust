pub fn decompress_lz4_block(src: &[u8], dst: &mut [u8]) -> Result<usize, &'static str> {
    if src.is_empty() {
        return Ok(0);
    }
    lz4_flex::block::decompress_into(src, dst).map_err(|_| "LZ4 decompression failed")
}

pub fn compress_lz4_block(src: &[u8], dst: &mut [u8]) -> Result<usize, &'static str> {
    if src.is_empty() {
        return Ok(0);
    }
    lz4_flex::block::compress_into(src, dst).map_err(|_| "LZ4 compression failed")
}
