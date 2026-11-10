use crate::types::ParcelHeader;

#[inline(always)]
pub fn align_up_4(val: usize) -> usize {
    (val + 3) & !3
}

pub struct ParcelReader<'a> {
    buf: &'a [u8],
    pub header: ParcelHeader,
    payload_pos: usize,
    #[allow(dead_code)]
    object_pos: usize,
}

impl<'a> ParcelReader<'a> {
    pub fn new(data: &'a [u8]) -> Option<Self> {
        if data.len() < 16 {
            return None;
        }

        let header = ParcelHeader {
            payload_size: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            payload_offset: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            objects_size: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            object_offset: u32::from_le_bytes([data[12], data[13], data[14], data[15]]),
        };

        let po = header.payload_offset as usize;
        let ps = header.payload_size as usize;
        if po + ps > data.len() {
            return None;
        }

        Some(Self {
            buf: data,
            header,
            payload_pos: po,
            object_pos: header.object_offset as usize,
        })
    }

    #[inline]
    pub fn read_i32(&mut self) -> Option<i32> {
        let p = self.payload_pos;
        if p + 4 > self.buf.len() {
            return None;
        }
        let val = i32::from_le_bytes([
            self.buf[p],
            self.buf[p + 1],
            self.buf[p + 2],
            self.buf[p + 3],
        ]);
        self.payload_pos = p + 4;
        Some(val)
    }

    #[inline]
    pub fn read_u32(&mut self) -> Option<u32> {
        self.read_i32().map(|v| v as u32)
    }

    #[inline]
    pub fn read_i64(&mut self) -> Option<i64> {
        let p = self.payload_pos;
        if p + 8 > self.buf.len() {
            return None;
        }
        let val = i64::from_le_bytes([
            self.buf[p],
            self.buf[p + 1],
            self.buf[p + 2],
            self.buf[p + 3],
            self.buf[p + 4],
            self.buf[p + 5],
            self.buf[p + 6],
            self.buf[p + 7],
        ]);
        self.payload_pos = p + 8;
        Some(val)
    }

    #[inline]
    pub fn read_u64(&mut self) -> Option<u64> {
        self.read_i64().map(|v| v as u64)
    }

    #[inline]
    pub fn read_bytes_inplace(&mut self, size: usize) -> Option<&'a [u8]> {
        let p = self.payload_pos;
        if p + size > self.buf.len() {
            return None;
        }
        let slice = &self.buf[p..p + size];
        self.payload_pos = p + align_up_4(size);
        Some(slice)
    }

    pub fn read_interface_token(&mut self) -> Option<String> {
        // Skip strict policy flags
        let _strict_policy = self.read_i32()?;
        let char_len = self.read_i32()?;
        if char_len <= 0 {
            return Some(String::new());
        }

        let byte_len = ((char_len as usize) + 1) * 2;
        let slice = self.read_bytes_inplace(byte_len)?;
        // Decode UTF-16LE ignoring last null character
        let u16_chars: Vec<u16> = slice[..(char_len as usize) * 2]
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        String::from_utf16(&u16_chars).ok()
    }
}

pub struct ParcelWriter<'a> {
    buf: &'a mut [u8],
    payload_start: usize,
    payload_pos: usize,
    objects_pos: usize,
}

impl<'a> ParcelWriter<'a> {
    pub fn new(dest: &'a mut [u8]) -> Option<Self> {
        if dest.len() < 16 {
            return None;
        }
        Some(Self {
            buf: dest,
            payload_start: 16,
            payload_pos: 16,
            objects_pos: 0,
        })
    }

    #[inline]
    pub fn write_i32(&mut self, val: i32) -> bool {
        let p = self.payload_pos;
        if p + 4 > self.buf.len() {
            return false;
        }
        self.buf[p..p + 4].copy_from_slice(&val.to_le_bytes());
        self.payload_pos = p + 4;
        true
    }

    #[inline]
    pub fn write_u32(&mut self, val: u32) -> bool {
        self.write_i32(val as i32)
    }

    #[inline]
    pub fn write_i64(&mut self, val: i64) -> bool {
        let p = self.payload_pos;
        if p + 8 > self.buf.len() {
            return false;
        }
        self.buf[p..p + 8].copy_from_slice(&val.to_le_bytes());
        self.payload_pos = p + 8;
        true
    }

    #[inline]
    pub fn write_u64(&mut self, val: u64) -> bool {
        self.write_i64(val as i64)
    }

    #[inline]
    pub fn write_bytes_inplace(&mut self, data: &[u8]) -> bool {
        let p = self.payload_pos;
        let aligned = align_up_4(data.len());
        if p + aligned > self.buf.len() {
            return false;
        }
        self.buf[p..p + data.len()].copy_from_slice(data);
        if aligned > data.len() {
            self.buf[p + data.len()..p + aligned].fill(0);
        }
        self.payload_pos = p + aligned;
        true
    }

    pub fn finish(&mut self) -> usize {
        let payload_size = (self.payload_pos - self.payload_start) as u32;
        let objects_size = self.objects_pos as u32;
        let payload_offset = self.payload_start as u32;
        let object_offset = payload_offset + payload_size;

        self.buf[0..4].copy_from_slice(&payload_size.to_le_bytes());
        self.buf[4..8].copy_from_slice(&payload_offset.to_le_bytes());
        self.buf[8..12].copy_from_slice(&objects_size.to_le_bytes());
        self.buf[12..16].copy_from_slice(&object_offset.to_le_bytes());

        16 + (payload_size as usize) + (objects_size as usize)
    }
}
