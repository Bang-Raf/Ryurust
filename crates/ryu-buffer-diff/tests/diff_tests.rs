use ryu_buffer_diff::*;

#[test]
fn test_version() {
    assert_eq!(ryu_buffer_diff_version(), 0x0001_0000);
}

#[test]
fn test_identical_buffers() {
    let a = vec![0xABu8; 1024];
    let b = vec![0xABu8; 1024];

    assert!(is_equal_fast(&a, &b));
    assert_eq!(find_dirty_range(&a, &b), None);
}

#[test]
fn test_single_byte_diffs() {
    let mut a = vec![0u8; 256];
    let b = vec![0u8; 256];

    // First byte
    a[0] = 42;
    assert_eq!(find_dirty_range(&a, &b), Some(DirtyRange::new(0, 1)));
    a[0] = 0;

    // Middle byte
    a[127] = 99;
    assert_eq!(find_dirty_range(&a, &b), Some(DirtyRange::new(127, 1)));
    a[127] = 0;

    // Last byte
    a[255] = 1;
    assert_eq!(find_dirty_range(&a, &b), Some(DirtyRange::new(255, 1)));
}

#[test]
fn test_range_diff() {
    let mut a = vec![0u8; 512];
    let b = vec![0u8; 512];

    // Range from 50 to 150
    for (i, byte) in a.iter_mut().enumerate().take(151).skip(50) {
        *byte = (i % 255) as u8 + 1;
    }

    let range = find_dirty_range(&a, &b).expect("should be dirty");
    assert_eq!(range.offset, 50);
    assert_eq!(range.length, 101);
    assert_eq!(range.end(), 151);
}

#[test]
fn test_short_buffers() {
    for size in 1..40 {
        let mut a = vec![0u8; size];
        let b = vec![0u8; size];

        a[size - 1] = 0xFF;
        let diff = find_dirty_range(&a, &b).expect("should find diff");
        assert_eq!(diff.offset, size - 1);
        assert_eq!(diff.length, 1);
    }
}

#[test]
fn test_differential_verification_against_scalar() {
    // 2-step verification: Random test cases comparing AVX2 against scalar
    let size = 4096;
    let src = vec![0x55u8; size];
    let dst = vec![0x55u8; size];

    let test_ranges = [
        (0, 1),
        (0, 31),
        (0, 32),
        (1, 33),
        (31, 32),
        (100, 200),
        (512, 1024),
        (2048, 2049),
        (4000, 4096),
        (0, 4096),
    ];

    for (start, end) in test_ranges {
        let mut test_src = src.clone();
        for byte in &mut test_src[start..end] {
            *byte ^= 0xAA;
        }

        let scalar_result = find_dirty_range_scalar(&test_src, &dst, size);
        let simd_result = find_dirty_range(&test_src, &dst);

        assert_eq!(
            simd_result, scalar_result,
            "Mismatch at range ({}, {})",
            start, end
        );
    }
}

#[test]
fn test_diff_and_copy() {
    let mut src = vec![0u8; 1024];
    let mut dst = vec![0u8; 1024];

    src[100..200].fill(0x7F);

    let dirty = diff_and_copy(&src, &mut dst).expect("dirty expected");
    assert_eq!(dirty.offset, 100);
    assert_eq!(dirty.length, 100);

    assert_eq!(&dst[100..200], &src[100..200]);
    assert!(is_equal_fast(&src, &dst));

    // Second diff should now be None because dst matches src
    assert_eq!(diff_and_copy(&src, &mut dst), None);
}

#[test]
fn test_ffi_c_abi() {
    let src = [1u8, 2, 3, 4, 5, 6, 7, 8];
    let mut dst = [1u8, 2, 99, 4, 5, 6, 7, 8];
    let mut offset = 0usize;
    let mut length = 0usize;

    // SAFETY: `src`/`dst` are distinct live 8-byte arrays; out-pointers are live locals.
    unsafe {
        assert_eq!(ryu_buffer_is_equal(src.as_ptr(), dst.as_ptr(), 8), 0);

        let dirty =
            ryu_buffer_find_dirty_range(src.as_ptr(), dst.as_ptr(), 8, &mut offset, &mut length);
        assert_eq!(dirty, 1);
        assert_eq!(offset, 2);
        assert_eq!(length, 1);

        let copied =
            ryu_buffer_diff_and_copy(src.as_ptr(), dst.as_mut_ptr(), 8, &mut offset, &mut length);
        assert_eq!(copied, 1);
        assert_eq!(offset, 2);
        assert_eq!(length, 1);

        assert_eq!(ryu_buffer_is_equal(src.as_ptr(), dst.as_ptr(), 8), 1);
    }
}

#[test]
fn test_extensive_randomized_differential() {
    let mut rng_state = 123456789u64;
    let mut next_u32 = || {
        rng_state ^= rng_state << 13;
        rng_state ^= rng_state >> 7;
        rng_state ^= rng_state << 17;
        rng_state as u32
    };

    for _ in 0..1000 {
        let len = (next_u32() % 8192) as usize + 1;
        let mut a = vec![0x33u8; len];
        let b = vec![0x33u8; len];

        let should_diff = (next_u32() % 10) != 0;
        if should_diff {
            let start = (next_u32() as usize) % len;
            let max_sub_len = len - start;
            let sub_len = (next_u32() as usize % max_sub_len) + 1;
            for i in start..start + sub_len {
                a[i] = (next_u32() % 256) as u8;
                if a[i] == b[i] {
                    a[i] = b[i].wrapping_add(1);
                }
            }
        }

        let scalar = find_dirty_range_scalar(&a, &b, len);
        let simd = find_dirty_range(&a, &b);

        assert_eq!(simd, scalar, "Mismatch at len {}", len);
    }
}
