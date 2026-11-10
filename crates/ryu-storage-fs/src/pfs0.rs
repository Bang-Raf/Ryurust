pub const PFS0_MAGIC: &[u8; 4] = b"PFS0";
pub const PFS0_HEADER_SIZE: usize = 16;
pub const PFS0_ENTRY_SIZE: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pfs0HeaderInfo {
    pub num_files: u32,
    pub string_table_size: u32,
    pub header_total_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pfs0FileEntry {
    pub offset: u64,
    pub size: u64,
    pub name_offset: u32,
}

pub fn parse_pfs0_header(data: &[u8]) -> Result<Pfs0HeaderInfo, &'static str> {
    if data.len() < PFS0_HEADER_SIZE {
        return Err("Header buffer too small for PFS0 header");
    }
    if &data[0..4] != PFS0_MAGIC {
        return Err("Invalid PFS0 magic identifier");
    }

    let num_files = u32::from_le_bytes(data[4..8].try_into().unwrap());
    let string_table_size = u32::from_le_bytes(data[8..12].try_into().unwrap());

    let entries_size = (num_files as usize)
        .checked_mul(PFS0_ENTRY_SIZE)
        .ok_or("Arithmetic overflow in entries size")?;
    let header_total_size = PFS0_HEADER_SIZE
        .checked_add(entries_size)
        .and_then(|s| s.checked_add(string_table_size as usize))
        .ok_or("Arithmetic overflow in total header size")?;

    if data.len() < header_total_size {
        return Err("Buffer too small for PFS0 entries and string table");
    }

    Ok(Pfs0HeaderInfo {
        num_files,
        string_table_size,
        header_total_size,
    })
}

pub fn get_pfs0_entry(data: &[u8], index: u32) -> Result<(Pfs0FileEntry, &str), &'static str> {
    let header = parse_pfs0_header(data)?;
    if index >= header.num_files {
        return Err("File entry index out of bounds");
    }

    let entry_start = PFS0_HEADER_SIZE + (index as usize) * PFS0_ENTRY_SIZE;
    let entry_bytes = &data[entry_start..entry_start + PFS0_ENTRY_SIZE];

    let offset = u64::from_le_bytes(entry_bytes[0..8].try_into().unwrap());
    let size = u64::from_le_bytes(entry_bytes[8..16].try_into().unwrap());
    let name_offset = u32::from_le_bytes(entry_bytes[16..20].try_into().unwrap());

    let str_table_start = PFS0_HEADER_SIZE + (header.num_files as usize) * PFS0_ENTRY_SIZE;
    let str_table = &data[str_table_start..str_table_start + (header.string_table_size as usize)];

    if (name_offset as usize) >= str_table.len() {
        return Err("File name offset outside string table");
    }

    let name_slice = &str_table[(name_offset as usize)..];
    let null_pos = name_slice
        .iter()
        .position(|&b| b == 0)
        .unwrap_or(name_slice.len());
    let name_str =
        std::str::from_utf8(&name_slice[..null_pos]).map_err(|_| "File name is not valid UTF-8")?;

    Ok((
        Pfs0FileEntry {
            offset,
            size,
            name_offset,
        },
        name_str,
    ))
}

pub fn find_pfs0_file(data: &[u8], target_name: &str) -> Result<Pfs0FileEntry, &'static str> {
    let header = parse_pfs0_header(data)?;
    for i in 0..header.num_files {
        let (entry, name) = get_pfs0_entry(data, i)?;
        if name == target_name {
            return Ok(entry);
        }
    }
    Err("File not found in PFS0 partition")
}
