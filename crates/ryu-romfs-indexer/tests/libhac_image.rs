//! Lookups against a RomFS image built by the reference implementation: LibHac's
//! `RomFsBuilder`. The manifest was written by reading the same image back through
//! LibHac's `RomFsFileSystem`. See `tests/fixtures/README.md`; all content is synthetic.

use ryu_romfs_indexer::*;
use std::collections::BTreeSet;
use std::ffi::CString;

const IMAGE: &[u8] = include_bytes!("fixtures/libhac_synthetic.romfs");
const MANIFEST: &str = include_str!("fixtures/libhac_synthetic.manifest.tsv");

/// Entry counts LibHac reported when the fixture was generated (directories exclude root).
const MANIFEST_DIRS: usize = 80;
const MANIFEST_FILES: usize = 314;

struct ManifestFile {
    path: &'static str,
    offset: u64,
    size: u64,
    fnv1a64: u64,
}

struct Manifest {
    dirs: Vec<&'static str>,
    files: Vec<ManifestFile>,
}

fn manifest() -> Manifest {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    for line in MANIFEST.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        match cols.as_slice() {
            ["D", path] => dirs.push(*path),
            ["F", path, offset, size, hash] => files.push(ManifestFile {
                path,
                offset: offset.parse().unwrap(),
                size: size.parse().unwrap(),
                fnv1a64: u64::from_str_radix(hash, 16).unwrap(),
            }),
            _ => panic!("bad manifest line: {line:?}"),
        }
    }
    assert_eq!(dirs.len(), MANIFEST_DIRS);
    assert_eq!(files.len(), MANIFEST_FILES);
    Manifest { dirs, files }
}

fn fnv1a64(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |h, &b| {
        (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

fn contents(info: &FileInfo) -> &'static [u8] {
    let start = usize::try_from(info.offset).unwrap();
    let end = start + usize::try_from(info.size).unwrap();
    &IMAGE[start..end]
}

/// The fixture has the layout `RomFsBuilder` produces: file data right after the header,
/// the four tables after the data. Guards against the fixture being swapped for a
/// hand-made "tables first" image.
#[test]
fn fixture_has_libhac_layout() {
    let h = RomFsHeader::parse(IMAGE).unwrap();
    assert_eq!(h.header_size, 0x50);
    assert_eq!(h.data_offset, 0x200);
    assert!(h.dir_hash_offset > h.data_offset);
    assert!(h.dir_hash_offset < h.dir_meta_offset);
    assert!(h.dir_meta_offset < h.file_hash_offset);
    assert!(h.file_hash_offset < h.file_meta_offset);
    assert_eq!(h.file_meta_offset + h.file_meta_size, IMAGE.len() as u64);
}

#[test]
fn every_manifest_file_is_found_with_libhac_offset_size_and_contents() {
    let m = manifest();
    let reader = RomFsReader::from_bytes(IMAGE).expect("reader over LibHac image");
    let owned = RyuRomFs::from_metadata_bytes(IMAGE).expect("RyuRomFs over LibHac image");

    for f in &m.files {
        let info = reader
            .find_file(f.path)
            .unwrap_or_else(|| panic!("not found: {}", f.path));
        assert_eq!(info.offset, f.offset, "offset of {}", f.path);
        assert_eq!(info.size, f.size, "size of {}", f.path);
        assert_eq!(
            fnv1a64(contents(&info)),
            f.fnv1a64,
            "contents of {}",
            f.path
        );

        // Same answer from the owning handle, and without the leading slash.
        assert_eq!(owned.find_file(f.path), Some(info), "owned: {}", f.path);
        assert_eq!(reader.find_file(&f.path[1..]), Some(info), "{}", f.path);

        // The file's parent is the directory the path names.
        let parent = &f.path[..f.path.rfind('/').unwrap()];
        assert_eq!(reader.find_dir(parent), Some(info.parent_dir), "{}", f.path);
    }
}

#[test]
fn every_manifest_directory_is_found() {
    let m = manifest();
    let reader = RomFsReader::from_bytes(IMAGE).unwrap();

    assert_eq!(reader.find_dir("/"), Some(0));
    assert_eq!(reader.find_dir(""), Some(0));

    let mut offsets = BTreeSet::from([0u32]);
    for d in &m.dirs {
        let off = reader
            .find_dir(d)
            .unwrap_or_else(|| panic!("directory not found: {d}"));
        assert!(offsets.insert(off), "duplicate directory offset for {d}");
        assert_eq!(reader.find_dir(&format!("{d}/")), Some(off), "{d}");

        // The entry at that offset carries the last path component and the right parent.
        let (entry, name) = reader.read_dir_entry(off).unwrap();
        let (parent, leaf) = d.rsplit_once('/').unwrap();
        assert_eq!(name, leaf.as_bytes(), "{d}");
        assert_eq!(reader.find_dir(parent), Some(entry.parent), "{d}");

        // A directory is not a file.
        assert_eq!(reader.find_file(d), None, "directory found as file: {d}");
    }

    // Empty directories exist and have no children.
    for d in ["/emptydir", "/deep/l1/emptyleaf", "/utf8/\u{7a7a}"] {
        let (entry, _) = reader.read_dir_entry(reader.find_dir(d).unwrap()).unwrap();
        assert_eq!(entry.child_dir, ROMFS_ENTRY_EMPTY, "{d}");
        assert_eq!(entry.child_file, ROMFS_ENTRY_EMPTY, "{d}");
    }
}

#[test]
fn files_are_not_directories() {
    let m = manifest();
    let reader = RomFsReader::from_bytes(IMAGE).unwrap();
    let dirs: BTreeSet<&str> = m.dirs.iter().copied().collect();
    for f in &m.files {
        assert!(!dirs.contains(f.path));
        assert_eq!(
            reader.find_dir(f.path),
            None,
            "file found as dir: {}",
            f.path
        );
    }
}

#[test]
fn missing_wrong_case_and_through_file_paths_are_not_found() {
    let m = manifest();
    let reader = RomFsReader::from_bytes(IMAGE).unwrap();
    let existing: BTreeSet<&str> = m.files.iter().map(|f| f.path).collect();
    let existing_dirs: BTreeSet<&str> = m.dirs.iter().copied().collect();

    let absent = [
        // Missing.
        "/nope.bin",
        "/many/file_0220.bin",
        "/many/file_000.bin",
        "/many/file_0000.bi",
        "/many/file_0000.bin0",
        "/nodir/root.bin",
        "/deep/l1/l2/l3/l4/l5/l6/l7/l8/l9/leaf.dat",
        "/deep/l1/l2/l3/l4/l5/l6/l7/leaf.dat",
        "/utf8/cafe.txt",
        "/utf8/caf\u{00c9}.txt",
        "/pad/ppppppppp",
        "/pad/",
        "/emptydir/x",
        // File from one directory asked for in another.
        "/many/item.bin",
        "/dirs/d12/sub/item.bin",
        "/case/x.bin",
        // Wrong case.
        "/ROOT.BIN",
        "/Root.bin",
        "/case/README.txt",
        "/case/ReadMe.txt",
        "/Case/readme.txt",
        "/case/dIR/x.bin",
        "/case/dir/X.BIN",
        "/case/DIR/x.bin",
        "/MANY/file_0000.bin",
        "/many/FILE_0000.bin",
        "/utf8/\u{00fc}ber/stra\u{00df}e.txt",
        // Through a file.
        "/root.bin/x",
        "/root.bin/root.bin",
        "/a/b",
        "/empty.bin/empty.bin",
        "/many/file_0000.bin/file_0001.bin",
        "/deep/data/data/data/data/data",
        "/deep/data/data.bin/data",
        // Not a file at all.
        "",
        "/",
        "//",
    ];
    for p in absent {
        assert!(!existing.contains(p), "test bug: {p} exists");
        assert_eq!(reader.find_file(p), None, "unexpectedly found file {p}");
    }

    let absent_dirs = [
        "/nope",
        "/CASE",
        "/case/DiR",
        "/Deep/l1",
        "/deep/L1",
        "/root.bin",
        "/root.bin/x",
        "/deep/data/data/data/data",
        "/deep/data/data/data/data/data",
        "/dirs/d40",
        "/dirs/d12/sub",
        "/emptydir/emptydir",
        "/utf8/\u{7a7a}/\u{7a7a}",
    ];
    for p in absent_dirs {
        assert!(!existing_dirs.contains(p), "test bug: {p} exists");
        assert_eq!(reader.find_dir(p), None, "unexpectedly found dir {p}");
    }
}

/// Every pair of case-variant names resolves to its own entry.
#[test]
fn case_variants_are_distinct_entries() {
    let reader = RomFsReader::from_bytes(IMAGE).unwrap();
    let sizes: Vec<u64> = ["/case/Readme.txt", "/case/README.TXT", "/case/readme.txt"]
        .iter()
        .map(|p| reader.find_file(p).unwrap().size)
        .collect();
    assert_eq!(sizes, [33, 34, 35]);
    let sizes: Vec<u64> = ["/case/Dir/x.bin", "/case/dir/x.bin", "/case/DIR/X.BIN"]
        .iter()
        .map(|p| reader.find_file(p).unwrap().size)
        .collect();
    assert_eq!(sizes, [5, 6, 7]);
}

/// Walks the tree from the root through the child/sibling links and checks that it yields
/// exactly the manifest's directories and files.
#[test]
fn tree_walk_enumerates_exactly_the_manifest() {
    let m = manifest();
    let reader = RomFsReader::from_bytes(IMAGE).unwrap();

    let mut found_dirs = BTreeSet::new();
    let mut found_files = BTreeSet::new();
    let mut stack = vec![(0u32, String::new())];
    while let Some((dir_off, dir_path)) = stack.pop() {
        let (dir, _) = reader.read_dir_entry(dir_off).unwrap();

        let mut file_off = dir.child_file;
        while file_off != ROMFS_ENTRY_EMPTY {
            let (file, name) = reader.read_file_entry(file_off).unwrap();
            assert_eq!(file.parent, dir_off);
            let path = format!("{dir_path}/{}", std::str::from_utf8(name).unwrap());
            let info = reader.find_file(&path).expect("walked file is findable");
            assert_eq!(info.size, file.data_size);
            assert_eq!(info.offset, reader.header.data_offset + file.data_offset);
            assert!(found_files.insert(path), "file listed twice");
            file_off = file.sibling;
        }

        let mut child_off = dir.child_dir;
        while child_off != ROMFS_ENTRY_EMPTY {
            let (child, name) = reader.read_dir_entry(child_off).unwrap();
            assert_eq!(child.parent, dir_off);
            let path = format!("{dir_path}/{}", std::str::from_utf8(name).unwrap());
            assert_eq!(reader.find_dir(&path), Some(child_off));
            assert!(found_dirs.insert(path.clone()), "dir listed twice");
            stack.push((child_off, path));
            child_off = child.sibling;
        }
    }

    let want_dirs: BTreeSet<String> = m.dirs.iter().map(|d| d.to_string()).collect();
    let want_files: BTreeSet<String> = m.files.iter().map(|f| f.path.to_string()).collect();
    assert_eq!(found_dirs, want_dirs);
    assert_eq!(found_files, want_files);
}

#[test]
fn entry_counts_match_libhac() {
    let reader = RomFsReader::from_bytes(IMAGE).unwrap();
    assert_eq!(reader.count_entries(), (MANIFEST_DIRS + 1, MANIFEST_FILES));

    let owned = RyuRomFs::from_metadata_bytes(IMAGE).unwrap();
    assert_eq!(owned.dir_count(), MANIFEST_DIRS + 1);
    assert_eq!(owned.file_count(), MANIFEST_FILES);
    assert_eq!(owned.header(), &RomFsHeader::parse(IMAGE).unwrap());
}

#[test]
fn c_abi_over_libhac_image() {
    let m = manifest();
    // SAFETY: `IMAGE` is a valid buffer of `IMAGE.len()` bytes.
    let handle = unsafe { ryu_romfs_create(IMAGE.as_ptr(), IMAGE.len()) };
    assert!(
        !handle.is_null(),
        "ryu_romfs_create rejected the LibHac image"
    );

    // SAFETY: `handle` is live until the `ryu_romfs_destroy` below; paths are NUL-terminated
    // `CString`s and the outputs are local `u64`s.
    unsafe {
        assert_eq!(ryu_romfs_get_dir_count(handle), MANIFEST_DIRS + 1);
        assert_eq!(ryu_romfs_get_file_count(handle), MANIFEST_FILES);
    }

    for f in &m.files {
        let path = CString::new(f.path).unwrap();
        let (mut offset, mut size) = (u64::MAX, u64::MAX);
        // SAFETY: see above.
        let rc = unsafe { ryu_romfs_find_file(handle, path.as_ptr(), &mut offset, &mut size) };
        assert_eq!(rc, 1, "{}", f.path);
        assert_eq!((offset, size), (f.offset, f.size), "{}", f.path);
    }
    for p in ["/nope.bin", "/ROOT.BIN", "/root.bin/x", "/case"] {
        let path = CString::new(p).unwrap();
        let (mut offset, mut size) = (7u64, 7u64);
        // SAFETY: see above.
        let rc = unsafe { ryu_romfs_find_file(handle, path.as_ptr(), &mut offset, &mut size) };
        assert_eq!(rc, 0, "{p}");
        assert_eq!((offset, size), (7, 7), "outputs written on a miss: {p}");
    }

    // SAFETY: `handle` came from `ryu_romfs_create` and is destroyed exactly once.
    unsafe { ryu_romfs_destroy(handle) };
}

/// The handle only needs the header and the tables; with LibHac's layout that means the
/// image up to the end of the last table, not up to `data_offset`.
#[test]
fn truncated_images_are_rejected_not_misread() {
    let h = RomFsHeader::parse(IMAGE).unwrap();
    for len in [
        0usize,
        0x4f,
        0x50,
        h.data_offset as usize,
        h.dir_hash_offset as usize,
        h.file_meta_offset as usize,
        IMAGE.len() - 1,
    ] {
        assert!(RomFsReader::from_bytes(&IMAGE[..len]).is_none(), "{len}");
        assert!(
            RyuRomFs::from_metadata_bytes(&IMAGE[..len]).is_none(),
            "{len}"
        );
        // SAFETY: `len <= IMAGE.len()`, so the buffer is valid for `len` reads.
        assert!(
            unsafe { ryu_romfs_create(IMAGE.as_ptr(), len) }.is_null(),
            "{len}"
        );
    }
}

/// A corrupted image whose hash chain points back at itself must end the lookup.
#[test]
fn cyclic_hash_chain_terminates() {
    let h = RomFsHeader::parse(IMAGE).unwrap();
    let reader = RomFsReader::from_bytes(IMAGE).unwrap();
    let mut image = IMAGE.to_vec();

    // Make every file entry's next-in-bucket link point at itself, likewise directories.
    let mut off = 0usize;
    while off + 32 <= h.file_meta_size as usize {
        let (entry, _) = reader.read_file_entry(off as u32).unwrap();
        let at = h.file_meta_offset as usize + off + 24;
        image[at..at + 4].copy_from_slice(&(off as u32).to_le_bytes());
        off += 32 + ((entry.name_len as usize + 3) & !3);
    }
    let mut off = 0usize;
    while off + 24 <= h.dir_meta_size as usize {
        let (entry, _) = reader.read_dir_entry(off as u32).unwrap();
        let at = h.dir_meta_offset as usize + off + 16;
        image[at..at + 4].copy_from_slice(&(off as u32).to_le_bytes());
        off += 24 + ((entry.name_len as usize + 3) & !3);
    }

    let broken = RomFsReader::from_bytes(&image).unwrap();
    for i in 0..300 {
        assert_eq!(broken.find_file(&format!("/many/absent_{i}.bin")), None);
        assert_eq!(broken.find_dir(&format!("/absent_{i}")), None);
    }
    // Entries at the head of their chain are still reachable, the rest are cut off.
    let found = manifest()
        .files
        .iter()
        .filter(|f| broken.find_file(f.path).is_some())
        .count();
    assert!(found > 0 && found < MANIFEST_FILES, "found {found}");
}
