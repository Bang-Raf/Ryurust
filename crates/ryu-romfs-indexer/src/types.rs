pub const ROMFS_HEADER_SIZE: usize = 0x50; // 80 bytes
pub const ROMFS_ENTRY_EMPTY: u32 = 0xFFFF_FFFF;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct RomFsHeader {
    pub header_size: u64,
    pub dir_hash_offset: u64,
    pub dir_hash_size: u64,
    pub dir_meta_offset: u64,
    pub dir_meta_size: u64,
    pub file_hash_offset: u64,
    pub file_hash_size: u64,
    pub file_meta_offset: u64,
    pub file_meta_size: u64,
    pub data_offset: u64,
}

impl RomFsHeader {
    pub fn parse(buf: &[u8]) -> Option<Self> {
        if buf.len() < ROMFS_HEADER_SIZE {
            return None;
        }

        let read_u64 = |offset: usize| -> u64 {
            let slice: [u8; 8] = buf[offset..offset + 8].try_into().unwrap();
            u64::from_le_bytes(slice)
        };

        Some(Self {
            header_size: read_u64(0x00),
            dir_hash_offset: read_u64(0x08),
            dir_hash_size: read_u64(0x10),
            dir_meta_offset: read_u64(0x18),
            dir_meta_size: read_u64(0x20),
            file_hash_offset: read_u64(0x28),
            file_hash_size: read_u64(0x30),
            file_meta_offset: read_u64(0x38),
            file_meta_size: read_u64(0x40),
            data_offset: read_u64(0x48),
        })
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct DirEntryHeader {
    pub parent: u32,
    pub sibling: u32,
    pub child_dir: u32,
    pub child_file: u32,
    pub next_hash: u32,
    pub name_len: u32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FileEntryHeader {
    pub parent: u32,
    pub sibling: u32,
    pub data_offset: u64,
    pub data_size: u64,
    pub next_hash: u32,
    pub name_len: u32,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct FileInfo {
    pub offset: u64,
    pub size: u64,
    pub parent_dir: u32,
}
