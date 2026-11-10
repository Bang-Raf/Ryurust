use ryu_storage_fs::aes_ctr::*;
use ryu_storage_fs::ffi::*;
use ryu_storage_fs::lz4_codec::*;
use ryu_storage_fs::pfs0::*;
use ryu_storage_fs::zstd_codec::*;
use std::ptr;

// --- STEP 1: INVARIANTS & SAFETY TESTS ---

#[test]
fn test_step1_ffi_null_safety() {
    unsafe {
        assert_eq!(
            ryu_storage_nca_decrypt_stream(
                ptr::null(),
                ptr::null_mut(),
                100,
                ptr::null(),
                ptr::null()
            ),
            RYU_STORAGE_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_storage_nca_decrypt_ctr_offset(
                ptr::null(),
                ptr::null_mut(),
                100,
                ptr::null(),
                ptr::null(),
                0
            ),
            RYU_STORAGE_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_storage_lz4_decompress_block(ptr::null(), 10, ptr::null_mut(), 10, ptr::null_mut()),
            RYU_STORAGE_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_storage_zstd_decompress_block(
                ptr::null(),
                10,
                ptr::null_mut(),
                10,
                ptr::null_mut()
            ),
            RYU_STORAGE_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_storage_pfs0_verify_header(ptr::null(), 10, ptr::null_mut(), ptr::null_mut()),
            RYU_STORAGE_ERR_NULL_PTR
        );
        assert_eq!(
            ryu_storage_pfs0_find_file(
                ptr::null(),
                10,
                ptr::null(),
                5,
                ptr::null_mut(),
                ptr::null_mut()
            ),
            RYU_STORAGE_ERR_NULL_PTR
        );
    }
}

#[test]
fn test_step1_zero_length_safety() {
    unsafe {
        let dummy = [0u8; 16];
        let mut out = [0u8; 16];
        let mut out_len = 0usize;

        assert_eq!(
            ryu_storage_nca_decrypt_stream(
                dummy.as_ptr(),
                out.as_mut_ptr(),
                0,
                dummy.as_ptr(),
                dummy.as_ptr()
            ),
            RYU_STORAGE_OK
        );
        assert_eq!(
            ryu_storage_lz4_decompress_block(dummy.as_ptr(), 0, out.as_mut_ptr(), 16, &mut out_len),
            RYU_STORAGE_OK
        );
        assert_eq!(out_len, 0);

        assert_eq!(
            ryu_storage_zstd_decompress_block(
                dummy.as_ptr(),
                0,
                out.as_mut_ptr(),
                16,
                &mut out_len
            ),
            RYU_STORAGE_OK
        );
        assert_eq!(out_len, 0);
    }
}

#[test]
fn test_step1_corrupt_pfs0_header() {
    let mut bad_header = [0u8; 32];
    bad_header[0..4].copy_from_slice(b"BAD0");
    let mut num_files = 0u32;
    let mut str_size = 0u32;

    unsafe {
        assert_eq!(
            ryu_storage_pfs0_verify_header(
                bad_header.as_ptr(),
                bad_header.len(),
                &mut num_files,
                &mut str_size
            ),
            RYU_STORAGE_ERR_CORRUPT_HEADER
        );
    }
}

// --- STEP 2: DUAL-EXECUTION BIT-IDENTICAL TESTS ---

#[test]
fn test_step2_aes_ctr_bit_identical_roundtrip() {
    let key: [u8; 16] = [
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54, 0x32,
        0x10,
    ];
    let counter: [u8; 16] = [
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0,
        0x01,
    ];

    let test_sizes = [16, 64, 137, 1024, 4099, 65536];

    for &size in &test_sizes {
        let plaintext: Vec<u8> = (0..size).map(|i| ((i * 37 + 13) & 0xff) as u8).collect();
        let mut encrypted = vec![0u8; size];
        let mut decrypted = vec![0u8; size];

        // Encrypt with offset 0
        decrypt_nca_ctr(&plaintext, &mut encrypted, &key, &counter, 0).unwrap();
        // Ciphertext should not match plaintext
        assert_ne!(plaintext, encrypted);

        // Decrypt with offset 0
        decrypt_nca_ctr(&encrypted, &mut decrypted, &key, &counter, 0).unwrap();
        assert_eq!(plaintext, decrypted, "Decryption failed at size {}", size);
    }
}

#[test]
fn test_step2_aes_ctr_offset_bit_identical() {
    let key = [0x55u8; 16];
    let counter = [0xAAu8; 16];

    let total_len = 10000;
    let full_plaintext: Vec<u8> = (0..total_len).map(|i| (i % 251) as u8).collect();
    let mut full_encrypted = vec![0u8; total_len];

    // Encrypt full buffer from offset 0
    decrypt_nca_ctr(&full_plaintext, &mut full_encrypted, &key, &counter, 0).unwrap();

    // Verify sub-slice decryptions at arbitrary unaligned offsets match the full stream exactly
    let test_offsets = [0, 1, 15, 16, 17, 63, 64, 65, 512, 1023, 4095];
    for &offset in &test_offsets {
        let chunk_len = 256.min(total_len - offset);
        let mut chunk_decrypted = vec![0u8; chunk_len];

        decrypt_nca_ctr(
            &full_encrypted[offset..offset + chunk_len],
            &mut chunk_decrypted,
            &key,
            &counter,
            offset as u64,
        )
        .unwrap();

        assert_eq!(
            &chunk_decrypted[..],
            &full_plaintext[offset..offset + chunk_len],
            "Chunk at offset {} does not match bit-identically",
            offset
        );
    }
}

#[test]
fn test_step2_lz4_bit_identical_roundtrip() {
    let original_data: Vec<u8> = (0..65536)
        .map(|i| {
            if i % 16 < 8 {
                (i % 10) as u8
            } else {
                ((i * 17) & 0xff) as u8
            }
        })
        .collect();

    let mut compressed = vec![0u8; 65536 * 2];
    let compressed_len = compress_lz4_block(&original_data, &mut compressed).unwrap();
    assert!(compressed_len > 0);
    assert!(compressed_len < original_data.len()); // Should achieve compression

    let mut decompressed = vec![0u8; original_data.len()];
    let decompressed_len =
        decompress_lz4_block(&compressed[..compressed_len], &mut decompressed).unwrap();

    assert_eq!(decompressed_len, original_data.len());
    assert_eq!(original_data, decompressed);
}

#[test]
fn test_step2_zstd_bit_identical_roundtrip() {
    let original_data: Vec<u8> = (0..65536).map(|i| (i % 128) as u8).collect();

    let mut compressed = vec![0u8; 65536 * 2];
    let compressed_len = compress_zstd_block(&original_data, &mut compressed, 3).unwrap();
    assert!(compressed_len > 0);
    assert!(compressed_len < original_data.len());

    let mut decompressed = vec![0u8; original_data.len()];
    let decompressed_len =
        decompress_zstd_block(&compressed[..compressed_len], &mut decompressed).unwrap();

    assert_eq!(decompressed_len, original_data.len());
    assert_eq!(original_data, decompressed);
}

#[test]
fn test_step2_pfs0_parsing_and_file_lookup() {
    // Construct authentic PFS0 binary partition
    let files: [(&str, &[u8]); 3] = [
        ("main", b"Switch Executable Code NSO Payload 1234567890"),
        ("main.npdm", b"Nintendo Program Descriptor Meta header"),
        ("sdk", b"NintendoSDK 16.0.0 Target Libraries"),
    ];

    let num_files = files.len() as u32;
    let mut str_table = Vec::new();
    let mut name_offsets = Vec::new();

    for (name, _) in &files {
        name_offsets.push(str_table.len() as u32);
        str_table.extend_from_slice(name.as_bytes());
        str_table.push(0); // Null terminator
    }

    let string_table_size = str_table.len() as u32;
    let header_size = 16 + (num_files as usize) * 24 + (string_table_size as usize);
    let mut partition = vec![0u8; header_size];

    // Magic PFS0
    partition[0..4].copy_from_slice(b"PFS0");
    partition[4..8].copy_from_slice(&num_files.to_le_bytes());
    partition[8..12].copy_from_slice(&string_table_size.to_le_bytes());
    partition[12..16].copy_from_slice(&0u32.to_le_bytes());

    let mut cur_data_offset = 0u64;
    for (i, ((_, payload), &name_off)) in files.iter().zip(name_offsets.iter()).enumerate() {
        let entry_start = 16 + i * 24;
        let file_size = payload.len() as u64;
        partition[entry_start..entry_start + 8].copy_from_slice(&cur_data_offset.to_le_bytes());
        partition[entry_start + 8..entry_start + 16].copy_from_slice(&file_size.to_le_bytes());
        partition[entry_start + 16..entry_start + 20].copy_from_slice(&name_off.to_le_bytes());
        partition[entry_start + 20..entry_start + 24].copy_from_slice(&0u32.to_le_bytes());
        cur_data_offset += file_size;
    }

    let str_start = 16 + (num_files as usize) * 24;
    partition[str_start..str_start + str_table.len()].copy_from_slice(&str_table);

    // Now test parsing with Rust API
    let parsed_info = parse_pfs0_header(&partition).unwrap();
    assert_eq!(parsed_info.num_files, 3);
    assert_eq!(parsed_info.string_table_size, string_table_size);

    for (i, (name, payload)) in files.iter().enumerate() {
        let (entry, found_name) = get_pfs0_entry(&partition, i as u32).unwrap();
        assert_eq!(&found_name, name);
        assert_eq!(entry.size, payload.len() as u64);

        let file_found = find_pfs0_file(&partition, name).unwrap();
        assert_eq!(file_found.size, payload.len() as u64);
        assert_eq!(file_found.offset, entry.offset);
    }

    assert!(find_pfs0_file(&partition, "nonexistent.file").is_err());
}
