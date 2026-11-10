pub fn decompress_zstd_block(src: &[u8], dst: &mut [u8]) -> Result<usize, &'static str> {
    if src.is_empty() {
        return Ok(0);
    }
    let mut decompressor = zstd::bulk::Decompressor::new()
        .map_err(|_| "Failed to initialize Zstandard decompressor")?;
    decompressor
        .decompress_to_buffer(src, dst)
        .map_err(|_| "Zstandard decompression failed")
}

pub fn compress_zstd_block(src: &[u8], dst: &mut [u8], level: i32) -> Result<usize, &'static str> {
    if src.is_empty() {
        return Ok(0);
    }
    let mut compressor = zstd::bulk::Compressor::new(level)
        .map_err(|_| "Failed to initialize Zstandard compressor")?;
    compressor
        .compress_to_buffer(src, dst)
        .map_err(|_| "Zstandard compression failed")
}
