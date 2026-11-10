pub fn decompress_zstd_block(src: &[u8], dst: &mut [u8]) -> Result<usize, &'static str> {
    if src.is_empty() {
        return Ok(0);
    }
    thread_local! {
        // Reused per thread: creating a zstd context costs more than
        // decompressing a small block.
        static DECOMPRESSOR: std::cell::RefCell<Option<zstd::bulk::Decompressor<'static>>> =
            const { std::cell::RefCell::new(None) };
    }
    DECOMPRESSOR.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(
                zstd::bulk::Decompressor::new()
                    .map_err(|_| "Failed to initialize Zstandard decompressor")?,
            );
        }
        slot.as_mut()
            .unwrap()
            .decompress_to_buffer(src, dst)
            .map_err(|_| "Zstandard decompression failed")
    })
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
