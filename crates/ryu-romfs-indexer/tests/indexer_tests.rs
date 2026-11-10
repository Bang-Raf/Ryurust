use ryu_romfs_indexer::*;
use std::ffi::CString;

#[test]
fn test_romfs_hash() {
    let h1 = romfs_hash(0, b"Model");
    let h2 = romfs_hash(0, b"Model");
    assert_eq!(h1, h2);

    let h3 = romfs_hash(100, b"Model");
    assert_ne!(h1, h3); // Different parent offset yields different hash
}

#[test]
fn test_mock_romfs_build_and_find() {
    let mut builder = MockRomFsBuilder::new();
    let dir_model = builder.add_dir(0, "Model");
    let dir_movie = builder.add_dir(0, "Movie");

    builder.add_file(dir_model, "Link.bfres", 0x1000, 500_000);
    builder.add_file(dir_model, "Zelda.bfres", 0x80000, 750_000);
    builder.add_file(dir_movie, "Opening.mp4", 0x140000, 10_000_000);

    let romfs_bytes = builder.build_metadata();
    assert!(romfs_bytes.len() > 0x50);

    let reader = RomFsReader::from_bytes(&romfs_bytes).expect("Failed to parse RomFS");
    let (dirs, files) = reader.count_entries();
    assert_eq!(dirs, 3); // Root + Model + Movie
    assert_eq!(files, 3);

    // Find files with and without leading slash
    let f1 = reader
        .find_file("/Model/Link.bfres")
        .expect("File not found");
    assert_eq!(f1.offset, reader.header.data_offset + 0x1000);
    assert_eq!(f1.size, 500_000);

    let f2 = reader
        .find_file("Model/Zelda.bfres")
        .expect("File not found");
    assert_eq!(f2.offset, reader.header.data_offset + 0x80000);
    assert_eq!(f2.size, 750_000);

    let f3 = reader
        .find_file("/Movie/Opening.mp4")
        .expect("File not found");
    assert_eq!(f3.offset, reader.header.data_offset + 0x140000);
    assert_eq!(f3.size, 10_000_000);

    // Missing file
    assert!(reader.find_file("/Model/Ganon.bfres").is_none());
    assert!(reader.find_file("/NonExistent/file.bin").is_none());
}

#[test]
fn test_deep_nested_hierarchy() {
    let mut builder = MockRomFsBuilder::new();
    let d1 = builder.add_dir(0, "level1");
    let d2 = builder.add_dir(d1, "level2");
    let d3 = builder.add_dir(d2, "level3");
    builder.add_file(d3, "target.dat", 0x5000, 1024);

    let romfs_bytes = builder.build_metadata();
    let reader = RomFsReader::from_bytes(&romfs_bytes).unwrap();

    let f = reader
        .find_file("/level1/level2/level3/target.dat")
        .expect("Deep file not found");
    assert_eq!(f.size, 1024);
}

#[test]
fn test_c_abi_lifecycle() {
    assert_eq!(ryu_romfs_indexer_version(), 0x00010000);

    let mut builder = MockRomFsBuilder::new();
    let d_sound = builder.add_dir(0, "Sound");
    builder.add_file(d_sound, "bgm.bfwav", 0x200, 4096);

    let romfs_bytes = builder.build_metadata();
    let handle = ryu_romfs_create(romfs_bytes.as_ptr(), romfs_bytes.len());
    assert!(!handle.is_null());

    assert_eq!(ryu_romfs_get_dir_count(handle), 2);
    assert_eq!(ryu_romfs_get_file_count(handle), 1);

    let path = CString::new("/Sound/bgm.bfwav").unwrap();
    let mut offset: u64 = 0;
    let mut size: u64 = 0;
    let res = ryu_romfs_find_file(handle, path.as_ptr(), &mut offset, &mut size);
    assert_eq!(res, 1);
    assert_eq!(size, 4096);

    let missing = CString::new("/Sound/missing.wav").unwrap();
    let res_miss = ryu_romfs_find_file(handle, missing.as_ptr(), &mut offset, &mut size);
    assert_eq!(res_miss, 0);

    ryu_romfs_destroy(handle);
}

/// Known values from the LibHac formula (`hash = 123456789 ^ parent; hash = c ^ rotr(hash, 5)`),
/// computed independently.
#[test]
fn test_romfs_hash_known_values() {
    assert_eq!(ROMFS_HASH_SEED, 123_456_789);
    assert_eq!(romfs_hash(0, b""), 123_456_789);
    assert_eq!(romfs_hash(0, b"a"), 2_822_430_217);
    assert_eq!(romfs_hash(0, b"data"), 0x0459_5017);
    assert_eq!(romfs_hash(0x18, b"sound.bin"), 0xABB2_4C6A);
    assert_eq!(ryu_romfs_hash(7, std::ptr::null(), 0), 7 ^ 123_456_789);
    assert_eq!(
        ryu_romfs_hash(0x18, b"sound.bin".as_ptr(), 9),
        romfs_hash(0x18, b"sound.bin")
    );
}

#[test]
fn test_ryu_romfs_cached_lookups() {
    let mut builder = MockRomFsBuilder::new();
    let d = builder.add_dir(0, "Model");
    builder.add_file(d, "Link.bfres", 0x1000, 500);
    builder.add_file(0, "root.bin", 0x2000, 10);
    let romfs = RyuRomFs::from_metadata_bytes(&builder.build_metadata()).unwrap();

    for _ in 0..3 {
        assert_eq!(romfs.find_file("/Model/Link.bfres").unwrap().size, 500);
        assert_eq!(romfs.find_file("root.bin").unwrap().size, 10);
        assert!(romfs.find_file("Model/missing").is_none());
    }
    assert_eq!(
        romfs.reader().find_dir("Model"),
        romfs.reader().find_dir("/Model/")
    );
    assert_eq!(
        romfs.header().data_offset,
        romfs.reader().header.data_offset
    );
}
