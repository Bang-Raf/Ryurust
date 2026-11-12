pub const TOCS_MAGIC: u32 = 0x53434F54; // "TOCS"
pub const TOCH_MAGIC: u32 = 0x48434F54; // "TOCH"
pub const SHDI_MAGIC: u32 = 0x49444853; // "SHDI"
pub const BUFD_MAGIC: u32 = 0x44465542; // "BUFD"
pub const TEXD_MAGIC: u32 = 0x44584554; // "TEXD"

pub const FILE_FORMAT_VERSION_MAJOR: u16 = 1;
pub const FILE_FORMAT_VERSION_MINOR: u16 = 2;
pub const FILE_FORMAT_VERSION_PACKED: u32 =
    ((FILE_FORMAT_VERSION_MAJOR as u32) << 16) | (FILE_FORMAT_VERSION_MINOR as u32);

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TocHeader {
    pub magic: u32,
    pub format_version: u32,
    pub codegen_version: u32,
    pub padding: u32,
    pub timestamp: u64,
    pub reserved: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OffsetAndSize {
    pub offset: u64,
    pub uncompressed_size: u32,
    pub compressed_size: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TocError {
    BufferTooSmall,
    InvalidMagic,
    UnsupportedVersion,
    CorruptedData,
}

pub fn validate_header(data: &[u8], expected_magic: u32) -> Result<TocHeader, TocError> {
    if data.len() < 32 {
        return Err(TocError::BufferTooSmall);
    }

    let magic = u32::from_le_bytes(data[0..4].try_into().unwrap());
    if magic != expected_magic {
        return Err(TocError::InvalidMagic);
    }

    let format_version = u32::from_le_bytes(data[4..8].try_into().unwrap());
    if format_version != FILE_FORMAT_VERSION_PACKED {
        return Err(TocError::UnsupportedVersion);
    }

    let codegen_version = u32::from_le_bytes(data[8..12].try_into().unwrap());
    let padding = u32::from_le_bytes(data[12..16].try_into().unwrap());
    let timestamp = u64::from_le_bytes(data[16..24].try_into().unwrap());
    let reserved = u64::from_le_bytes(data[24..32].try_into().unwrap());

    Ok(TocHeader {
        magic,
        format_version,
        codegen_version,
        padding,
        timestamp,
        reserved,
    })
}

/// Parses the entries of a host shader cache TOC file (shared `TOCS` or host `TOCH`)
/// into `out_entries`, returning how many were written.
///
/// The 32-byte header is validated first: its magic must be [`TOCS_MAGIC`] or
/// [`TOCH_MAGIC`] and its format version [`FILE_FORMAT_VERSION_PACKED`]. Use
/// [`parse_toc_entries_with_magic`] to require one specific magic.
pub fn parse_toc_entries(
    data: &[u8],
    out_entries: &mut [OffsetAndSize],
) -> Result<usize, TocError> {
    let expected_magic = match data.get(0..4) {
        Some(m) if u32::from_le_bytes(m.try_into().unwrap()) == TOCH_MAGIC => TOCH_MAGIC,
        _ => TOCS_MAGIC,
    };
    parse_toc_entries_with_magic(data, expected_magic, out_entries)
}

/// Like [`parse_toc_entries`], but requires the header magic to be `expected_magic`.
pub fn parse_toc_entries_with_magic(
    data: &[u8],
    expected_magic: u32,
    out_entries: &mut [OffsetAndSize],
) -> Result<usize, TocError> {
    validate_header(data, expected_magic)?;

    let entries_bytes = &data[32..];
    let count = entries_bytes.len() / 16;
    let to_copy = count.min(out_entries.len());

    for (i, out_entry) in out_entries.iter_mut().enumerate().take(to_copy) {
        let offset = i * 16;
        let entry_slice = &entries_bytes[offset..offset + 16];

        let off = u64::from_le_bytes(entry_slice[0..8].try_into().unwrap());
        let uncomp = u32::from_le_bytes(entry_slice[8..12].try_into().unwrap());
        let comp = u32::from_le_bytes(entry_slice[12..16].try_into().unwrap());

        *out_entry = OffsetAndSize {
            offset: off,
            uncompressed_size: uncomp,
            compressed_size: comp,
        };
    }

    Ok(to_copy)
}
