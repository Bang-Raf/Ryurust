use ryu_shader_cache::*;

#[test]
fn test_version() {
    assert_eq!(ryu_shader_cache_version(), 0x0001_0000);
}

#[test]
fn test_crc32_standard_vector() {
    // "123456789" has standard IEEE 802.3 CRC32 = 0xCBF43926
    let data = b"123456789";
    assert_eq!(compute_crc32(data), 0xCBF43926);
}

#[test]
fn test_toc_header_validation() {
    let mut header_bytes = [0u8; 32];
    header_bytes[0..4].copy_from_slice(&TOCS_MAGIC.to_le_bytes());
    header_bytes[4..8].copy_from_slice(&FILE_FORMAT_VERSION_PACKED.to_le_bytes());
    header_bytes[8..12].copy_from_slice(&7354u32.to_le_bytes()); // codegen
    header_bytes[16..24].copy_from_slice(&1234567890u64.to_le_bytes()); // timestamp

    let parsed = validate_header(&header_bytes, TOCS_MAGIC).expect("should be valid");
    assert_eq!(parsed.magic, TOCS_MAGIC);
    assert_eq!(parsed.format_version, FILE_FORMAT_VERSION_PACKED);
    assert_eq!(parsed.codegen_version, 7354);
    assert_eq!(parsed.timestamp, 1234567890);

    // Invalid magic
    assert_eq!(
        validate_header(&header_bytes, TOCH_MAGIC),
        Err(TocError::InvalidMagic)
    );

    // Invalid version
    header_bytes[4..8].copy_from_slice(&0x9999_9999u32.to_le_bytes());
    assert_eq!(
        validate_header(&header_bytes, TOCS_MAGIC),
        Err(TocError::UnsupportedVersion)
    );
}

#[test]
fn test_toc_entries_parse() {
    let mut data = vec![0u8; 32]; // header
    data[0..4].copy_from_slice(&TOCH_MAGIC.to_le_bytes());
    data[4..8].copy_from_slice(&FILE_FORMAT_VERSION_PACKED.to_le_bytes());

    // Add 100 entries (each 16 bytes)
    for i in 0..100u64 {
        data.extend_from_slice(&(i * 1024).to_le_bytes()); // offset
        data.extend_from_slice(&4096u32.to_le_bytes()); // uncompressed size
        data.extend_from_slice(&1024u32.to_le_bytes()); // compressed size
    }

    let mut entries = vec![OffsetAndSize::default(); 100];
    let count = parse_toc_entries(&data, &mut entries).expect("should parse");
    assert_eq!(count, 100);

    for i in 0..100 {
        assert_eq!(entries[i].offset, (i as u64) * 1024);
        assert_eq!(entries[i].uncompressed_size, 4096);
        assert_eq!(entries[i].compressed_size, 1024);
    }
}

#[test]
fn test_lz4_decompression_basic() {
    // Manually constructed simple LZ4 block:
    // 5 literals "HELLO", then match offset 5, match length 5 (repeated "HELLO")
    // Total decompressed: "HELLOHELLO" (10 bytes)
    // Token 1: lit_len = 5, match_len = 5 - 4 = 1 => token = (5 << 4) | 1 = 0x51
    // Literals: "HELLO" (5 bytes)
    // Offset: 5 (2 bytes LE = [0x05, 0x00])
    let compressed = [0x51, b'H', b'E', b'L', b'L', b'O', 0x05, 0x00];

    let mut output = [0u8; 10];
    let len = decompress_lz4_block(&compressed, &mut output).expect("lz4 decompress ok");
    assert_eq!(len, 10);
    assert_eq!(&output, b"HELLOHELLO");
}

#[test]
fn test_ffi_c_abi() {
    let mut header_bytes = [0u8; 32];
    header_bytes[0..4].copy_from_slice(&TOCS_MAGIC.to_le_bytes());
    header_bytes[4..8].copy_from_slice(&FILE_FORMAT_VERSION_PACKED.to_le_bytes());

    let mut out_header = TocHeader {
        magic: 0,
        format_version: 0,
        codegen_version: 0,
        padding: 0,
        timestamp: 0,
        reserved: 0,
    };

    unsafe {
        assert_eq!(
            ryu_shader_cache_validate_toc(header_bytes.as_ptr(), 32, TOCS_MAGIC, &mut out_header),
            0
        );
        assert_eq!(out_header.magic, TOCS_MAGIC);

        let crc = ryu_shader_cache_crc32(b"123456789".as_ptr(), 9);
        assert_eq!(crc, 0xCBF43926);
    }
}
