use crate::types::{
    DirEntryHeader, FileEntryHeader, FileInfo, RomFsHeader, ROMFS_ENTRY_EMPTY, ROMFS_HEADER_SIZE,
};

#[inline(always)]
pub fn romfs_hash(parent: u32, name: &[u8]) -> u32 {
    let mut hash = parent ^ 12345678u32;
    for &b in name {
        hash = hash.rotate_right(5) ^ (b as u32);
    }
    hash
}

#[inline(always)]
fn read_u32_le(buf: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 > buf.len() {
        return None;
    }
    Some(u32::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
    ]))
}

#[inline(always)]
fn read_u64_le(buf: &[u8], offset: usize) -> Option<u64> {
    if offset + 8 > buf.len() {
        return None;
    }
    Some(u64::from_le_bytes([
        buf[offset],
        buf[offset + 1],
        buf[offset + 2],
        buf[offset + 3],
        buf[offset + 4],
        buf[offset + 5],
        buf[offset + 6],
        buf[offset + 7],
    ]))
}

pub struct RomFsReader<'a> {
    pub data: &'a [u8],
    pub header: RomFsHeader,
    dir_hash: &'a [u8],
    dir_meta: &'a [u8],
    file_hash: &'a [u8],
    file_meta: &'a [u8],
    dir_bucket_count: usize,
    file_bucket_count: usize,
}

impl<'a> RomFsReader<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Option<Self> {
        let header = RomFsHeader::parse(data)?;
        if header.header_size < ROMFS_HEADER_SIZE as u64 {
            return None;
        }

        let slice_range = |off: u64, sz: u64| -> Option<&'a [u8]> {
            let start = off as usize;
            let len = sz as usize;
            let end = start.checked_add(len)?;
            if end <= data.len() {
                Some(&data[start..end])
            } else {
                None
            }
        };

        let dir_hash = slice_range(header.dir_hash_offset, header.dir_hash_size)?;
        let dir_meta = slice_range(header.dir_meta_offset, header.dir_meta_size)?;
        let file_hash = slice_range(header.file_hash_offset, header.file_hash_size)?;
        let file_meta = slice_range(header.file_meta_offset, header.file_meta_size)?;

        let dir_bucket_count = dir_hash.len() / 4;
        let file_bucket_count = file_hash.len() / 4;

        if dir_bucket_count == 0 || file_bucket_count == 0 {
            return None;
        }

        Some(Self {
            data,
            header,
            dir_hash,
            dir_meta,
            file_hash,
            file_meta,
            dir_bucket_count,
            file_bucket_count,
        })
    }

    #[inline]
    pub fn read_dir_entry(&self, offset: u32) -> Option<(DirEntryHeader, &'a [u8])> {
        let off = offset as usize;
        if off + 24 > self.dir_meta.len() {
            return None;
        }
        let parent = read_u32_le(self.dir_meta, off)?;
        let sibling = read_u32_le(self.dir_meta, off + 4)?;
        let child_dir = read_u32_le(self.dir_meta, off + 8)?;
        let child_file = read_u32_le(self.dir_meta, off + 12)?;
        let next_hash = read_u32_le(self.dir_meta, off + 16)?;
        let name_len = read_u32_le(self.dir_meta, off + 20)?;

        let name_start = off + 24;
        let name_end = name_start.checked_add(name_len as usize)?;
        if name_end > self.dir_meta.len() {
            return None;
        }

        let hdr = DirEntryHeader {
            parent,
            sibling,
            child_dir,
            child_file,
            next_hash,
            name_len,
        };
        Some((hdr, &self.dir_meta[name_start..name_end]))
    }

    #[inline]
    pub fn read_file_entry(&self, offset: u32) -> Option<(FileEntryHeader, &'a [u8])> {
        let off = offset as usize;
        if off + 32 > self.file_meta.len() {
            return None;
        }
        let parent = read_u32_le(self.file_meta, off)?;
        let sibling = read_u32_le(self.file_meta, off + 4)?;
        let data_offset = read_u64_le(self.file_meta, off + 8)?;
        let data_size = read_u64_le(self.file_meta, off + 16)?;
        let next_hash = read_u32_le(self.file_meta, off + 24)?;
        let name_len = read_u32_le(self.file_meta, off + 28)?;

        let name_start = off + 32;
        let name_end = name_start.checked_add(name_len as usize)?;
        if name_end > self.file_meta.len() {
            return None;
        }

        let hdr = FileEntryHeader {
            parent,
            sibling,
            data_offset,
            data_size,
            next_hash,
            name_len,
        };
        Some((hdr, &self.file_meta[name_start..name_end]))
    }

    #[inline]
    fn find_dir_in_parent(&self, parent: u32, name: &[u8]) -> Option<u32> {
        let hash = romfs_hash(parent, name);
        let bucket = (hash as usize) % self.dir_bucket_count;
        let mut curr = read_u32_le(self.dir_hash, bucket * 4)?;

        while curr != ROMFS_ENTRY_EMPTY {
            let (entry, entry_name) = self.read_dir_entry(curr)?;
            if entry.parent == parent && entry_name == name {
                return Some(curr);
            }
            curr = entry.next_hash;
        }
        None
    }

    #[inline]
    fn find_file_in_parent(&self, parent: u32, name: &[u8]) -> Option<FileInfo> {
        let hash = romfs_hash(parent, name);
        let bucket = (hash as usize) % self.file_bucket_count;
        let mut curr = read_u32_le(self.file_hash, bucket * 4)?;

        while curr != ROMFS_ENTRY_EMPTY {
            let (entry, entry_name) = self.read_file_entry(curr)?;
            if entry.parent == parent && entry_name == name {
                return Some(FileInfo {
                    offset: self.header.data_offset + entry.data_offset,
                    size: entry.data_size,
                    parent_dir: parent,
                });
            }
            curr = entry.next_hash;
        }
        None
    }

    #[inline]
    pub fn find_file(&self, path: &str) -> Option<FileInfo> {
        let path_bytes = path.as_bytes();
        let mut current_dir: u32 = 0; // Root directory is always at offset 0

        // Strip leading '/' if present
        let mut start = 0;
        while start < path_bytes.len() && path_bytes[start] == b'/' {
            start += 1;
        }
        let mut end = path_bytes.len();
        while end > start && path_bytes[end - 1] == b'/' {
            end -= 1;
        }
        if start >= end {
            return None;
        }

        let trimmed = &path_bytes[start..end];
        let mut seg_start = 0;
        let len = trimmed.len();

        for i in 0..len {
            if trimmed[i] == b'/' {
                let seg = &trimmed[seg_start..i];
                if !seg.is_empty() {
                    current_dir = self.find_dir_in_parent(current_dir, seg)?;
                }
                seg_start = i + 1;
            }
        }

        let file_name = &trimmed[seg_start..len];
        if file_name.is_empty() {
            return None;
        }

        self.find_file_in_parent(current_dir, file_name)
    }

    pub fn find_dir(&self, path: &str) -> Option<u32> {
        let path_bytes = path.as_bytes();
        let mut current_dir: u32 = 0;

        let mut start = 0;
        while start < path_bytes.len() && path_bytes[start] == b'/' {
            start += 1;
        }
        let mut end = path_bytes.len();
        while end > start && path_bytes[end - 1] == b'/' {
            end -= 1;
        }
        if start >= end {
            return Some(0); // Root
        }

        let trimmed = &path_bytes[start..end];
        let mut seg_start = 0;
        let len = trimmed.len();

        for i in 0..len {
            if trimmed[i] == b'/' {
                let seg = &trimmed[seg_start..i];
                if !seg.is_empty() {
                    current_dir = self.find_dir_in_parent(current_dir, seg)?;
                }
                seg_start = i + 1;
            }
        }

        let last_seg = &trimmed[seg_start..len];
        if !last_seg.is_empty() {
            current_dir = self.find_dir_in_parent(current_dir, last_seg)?;
        }

        Some(current_dir)
    }

    pub fn count_entries(&self) -> (usize, usize) {
        let mut dirs = 0usize;
        let mut off = 0usize;
        while off + 24 <= self.dir_meta.len() {
            if let Some(name_len) = read_u32_le(self.dir_meta, off + 20) {
                dirs += 1;
                let aligned_name = ((name_len as usize) + 3) & !3;
                off += 24 + aligned_name;
            } else {
                break;
            }
        }

        let mut files = 0usize;
        let mut foff = 0usize;
        while foff + 32 <= self.file_meta.len() {
            if let Some(name_len) = read_u32_le(self.file_meta, foff + 28) {
                files += 1;
                let aligned_name = ((name_len as usize) + 3) & !3;
                foff += 32 + aligned_name;
            } else {
                break;
            }
        }

        (dirs, files)
    }
}

pub struct RyuRomFs {
    metadata: Vec<u8>,
    header: RomFsHeader,
    dir_count: usize,
    file_count: usize,
}

impl RyuRomFs {
    pub fn from_metadata_bytes(bytes: &[u8]) -> Option<Self> {
        let header = RomFsHeader::parse(bytes)?;
        let required_len = header.data_offset.min(bytes.len() as u64) as usize;
        let metadata = bytes[..required_len].to_vec();
        let reader = RomFsReader::from_bytes(&metadata)?;
        let (dir_count, file_count) = reader.count_entries();

        Some(Self {
            metadata,
            header,
            dir_count,
            file_count,
        })
    }

    #[inline]
    pub fn find_file(&self, path: &str) -> Option<FileInfo> {
        let reader = RomFsReader::from_bytes(&self.metadata)?;
        reader.find_file(path)
    }

    #[inline]
    pub fn header(&self) -> &RomFsHeader {
        &self.header
    }

    #[inline]
    pub fn dir_count(&self) -> usize {
        self.dir_count
    }

    #[inline]
    pub fn file_count(&self) -> usize {
        self.file_count
    }
}

/// Helper builder to construct binary-compliant Switch RomFS metadata images for testing & benchmarking
pub struct MockRomFsBuilder {
    dirs: Vec<(u32, String)>,            // (parent_idx, name)
    files: Vec<(u32, String, u64, u64)>, // (parent_idx, name, data_off, data_sz)
}

impl Default for MockRomFsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockRomFsBuilder {
    pub fn new() -> Self {
        Self {
            dirs: vec![(0, String::new())], // Index 0 is root
            files: Vec::new(),
        }
    }

    pub fn add_dir(&mut self, parent_idx: u32, name: &str) -> u32 {
        let idx = self.dirs.len() as u32;
        self.dirs.push((parent_idx, name.to_string()));
        idx
    }

    pub fn add_file(&mut self, parent_idx: u32, name: &str, data_off: u64, data_sz: u64) {
        self.files
            .push((parent_idx, name.to_string(), data_off, data_sz));
    }

    pub fn build_metadata(&self) -> Vec<u8> {
        let dir_count = self.dirs.len();
        let file_count = self.files.len();

        let dir_buckets = (dir_count.max(1) * 2).next_power_of_two().max(16);
        let file_buckets = (file_count.max(1) * 2).next_power_of_two().max(16);

        let mut dir_offsets = vec![0u32; dir_count];
        let mut dir_meta_size = 0usize;
        for (i, (_, name)) in self.dirs.iter().enumerate() {
            dir_offsets[i] = dir_meta_size as u32;
            let aligned_name = (name.len() + 3) & !3;
            dir_meta_size += 24 + aligned_name;
        }

        let mut file_offsets = vec![0u32; file_count];
        let mut file_meta_size = 0usize;
        for (i, (_, name, _, _)) in self.files.iter().enumerate() {
            file_offsets[i] = file_meta_size as u32;
            let aligned_name = (name.len() + 3) & !3;
            file_meta_size += 32 + aligned_name;
        }

        let mut dir_hash_table = vec![ROMFS_ENTRY_EMPTY; dir_buckets];
        let mut dir_next_hashes = vec![ROMFS_ENTRY_EMPTY; dir_count];

        for (i, (parent_idx, name)) in self.dirs.iter().enumerate() {
            if i == 0 {
                continue; // Root not hashed under parent
            }
            let parent_off = dir_offsets[*parent_idx as usize];
            let h = romfs_hash(parent_off, name.as_bytes());
            let b = (h as usize) % dir_buckets;
            dir_next_hashes[i] = dir_hash_table[b];
            dir_hash_table[b] = dir_offsets[i];
        }

        let mut file_hash_table = vec![ROMFS_ENTRY_EMPTY; file_buckets];
        let mut file_next_hashes = vec![ROMFS_ENTRY_EMPTY; file_count];

        for (i, (parent_idx, name, _, _)) in self.files.iter().enumerate() {
            let parent_off = dir_offsets[*parent_idx as usize];
            let h = romfs_hash(parent_off, name.as_bytes());
            let b = (h as usize) % file_buckets;
            file_next_hashes[i] = file_hash_table[b];
            file_hash_table[b] = file_offsets[i];
        }

        let dir_hash_offset = ROMFS_HEADER_SIZE as u64;
        let dir_hash_size = (dir_buckets * 4) as u64;
        let dir_meta_offset = dir_hash_offset + dir_hash_size;
        let file_hash_offset = dir_meta_offset + (dir_meta_size as u64);
        let file_hash_size = (file_buckets * 4) as u64;
        let file_meta_offset = file_hash_offset + file_hash_size;
        let data_offset = file_meta_offset + (file_meta_size as u64);

        let mut out = vec![0u8; data_offset as usize];

        let write_u64 = |buf: &mut [u8], off: usize, val: u64| {
            buf[off..off + 8].copy_from_slice(&val.to_le_bytes());
        };
        let write_u32 = |buf: &mut [u8], off: usize, val: u32| {
            buf[off..off + 4].copy_from_slice(&val.to_le_bytes());
        };

        // Write Header
        write_u64(&mut out, 0x00, ROMFS_HEADER_SIZE as u64);
        write_u64(&mut out, 0x08, dir_hash_offset);
        write_u64(&mut out, 0x10, dir_hash_size);
        write_u64(&mut out, 0x18, dir_meta_offset);
        write_u64(&mut out, 0x20, dir_meta_size as u64);
        write_u64(&mut out, 0x28, file_hash_offset);
        write_u64(&mut out, 0x30, file_hash_size);
        write_u64(&mut out, 0x38, file_meta_offset);
        write_u64(&mut out, 0x40, file_meta_size as u64);
        write_u64(&mut out, 0x48, data_offset);

        // Write Dir Hash Table
        for (i, &val) in dir_hash_table.iter().enumerate() {
            write_u32(&mut out, dir_hash_offset as usize + i * 4, val);
        }

        // Write Dir Meta Table
        for (i, (parent_idx, name)) in self.dirs.iter().enumerate() {
            let base = dir_meta_offset as usize + dir_offsets[i] as usize;
            let parent_off = if i == 0 {
                0
            } else {
                dir_offsets[*parent_idx as usize]
            };
            write_u32(&mut out, base, parent_off);
            write_u32(&mut out, base + 4, ROMFS_ENTRY_EMPTY);
            write_u32(&mut out, base + 8, ROMFS_ENTRY_EMPTY);
            write_u32(&mut out, base + 12, ROMFS_ENTRY_EMPTY);
            write_u32(&mut out, base + 16, dir_next_hashes[i]);
            write_u32(&mut out, base + 20, name.len() as u32);
            out[base + 24..base + 24 + name.len()].copy_from_slice(name.as_bytes());
        }

        // Write File Hash Table
        for (i, &val) in file_hash_table.iter().enumerate() {
            write_u32(&mut out, file_hash_offset as usize + i * 4, val);
        }

        // Write File Meta Table
        for (i, (parent_idx, name, doff, dsz)) in self.files.iter().enumerate() {
            let base = file_meta_offset as usize + file_offsets[i] as usize;
            let parent_off = dir_offsets[*parent_idx as usize];
            write_u32(&mut out, base, parent_off);
            write_u32(&mut out, base + 4, ROMFS_ENTRY_EMPTY);
            write_u64(&mut out, base + 8, *doff);
            write_u64(&mut out, base + 16, *dsz);
            write_u32(&mut out, base + 24, file_next_hashes[i]);
            write_u32(&mut out, base + 28, name.len() as u32);
            out[base + 32..base + 32 + name.len()].copy_from_slice(name.as_bytes());
        }

        out
    }
}
