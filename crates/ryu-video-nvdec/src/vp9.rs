// VP9 Frame Pipeline & Surface Reconstruction
// Combines prediction, IDCT transform acceleration, and in-loop deblocking filter

use crate::entropy::BoolDecoder;
use crate::idct::idct8x8_add;
use crate::loop_filter::loop_filter_frame;

pub const VP9_SYNC_CODE: u32 = 0x498342;
pub const VP9_FRAME_MARKER: u8 = 0x2;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Vp9FrameConfig {
    pub width: u32,
    pub height: u32,
    pub is_key_frame: u32,
    pub intra_only: u32,
    pub filter_level: u32,
    pub sharpness: u32,
    pub base_qindex: u32,
    pub transform_mode: u32,
    pub lossless: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntraPredictionMode {
    Dc,
    Vertical,
    Horizontal,
    Tm,
}

pub struct Vp9DecoderPipeline;

impl Vp9DecoderPipeline {
    pub fn parse_uncompressed_header(data: &[u8]) -> Result<Vp9FrameConfig, i32> {
        if data.len() < 10 {
            return Err(-4); // Corrupt / truncated bitstream
        }

        let frame_marker = (data[0] >> 6) & 0x3;
        if frame_marker != VP9_FRAME_MARKER {
            return Err(-4);
        }

        let profile_low = (data[0] >> 5) & 0x1;
        let profile_high = (data[0] >> 4) & 0x1;
        let _profile = (profile_high << 1) | profile_low;

        let show_existing_frame = (data[0] >> 3) & 0x1;
        if show_existing_frame != 0 {
            return Ok(Vp9FrameConfig {
                width: 1920,
                height: 1080,
                is_key_frame: 0,
                intra_only: 0,
                filter_level: 10,
                sharpness: 0,
                base_qindex: 100,
                transform_mode: 0,
                lossless: 0,
            });
        }

        let frame_type = (data[0] >> 2) & 0x1; // 0: KeyFrame, 1: InterFrame
        let is_key_frame = if frame_type == 0 { 1u32 } else { 0u32 };

        let mut width = 1920u32;
        let mut height = 1080u32;

        if is_key_frame != 0 && data.len() >= 8 {
            // Check 24-bit sync code: 0x49 0x83 0x42
            let sync = ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | (data[3] as u32);
            if sync == VP9_SYNC_CODE {
                width = 1 + (((data[4] as u32) | ((data[5] as u32) << 8)) & 0xFFFF);
                height = 1 + (((data[6] as u32) | ((data[7] as u32) << 8)) & 0xFFFF);
            }
        }

        Ok(Vp9FrameConfig {
            width,
            height,
            is_key_frame,
            intra_only: is_key_frame,
            filter_level: 12,
            sharpness: 1,
            base_qindex: 80,
            transform_mode: 1,
            lossless: 0,
        })
    }

    pub fn decode_frame(
        bitstream: &[u8],
        config: &Vp9FrameConfig,
        y_out: &mut [u8],
        u_out: &mut [u8],
        v_out: &mut [u8],
        y_stride: usize,
        uv_stride: usize,
    ) -> Result<(), i32> {
        let width = config.width as usize;
        let height = config.height as usize;

        if y_out.len() < height * y_stride {
            return Err(-3); // Buffer too small
        }
        let uv_height = (height + 1) / 2;
        if u_out.len() < uv_height * uv_stride || v_out.len() < uv_height * uv_stride {
            return Err(-3);
        }

        // Initialize with default mid-grey base prediction (Y=128, U=128, V=128)
        y_out[..height * y_stride].fill(128);
        u_out[..uv_height * uv_stride].fill(128);
        v_out[..uv_height * uv_stride].fill(128);

        // Parse macroblock stream with boolean entropy decoder
        let mut reader = BoolDecoder::new(bitstream);
        let blocks_x = (width + 15) / 16;
        let blocks_y = (height + 15) / 16;

        let mut coeffs = [0i32; 64];

        for by in 0..blocks_y {
            for bx in 0..blocks_x {
                let y_x = bx * 16;
                let y_y = by * 16;

                // Read prediction mode and transform type
                let skip = if !reader.is_exhausted() {
                    reader.read_bit(128)
                } else {
                    0
                };

                if skip == 0 && !reader.is_exhausted() {
                    let has_coeffs = reader.read_bit(128);
                    if has_coeffs != 0 {
                        let dc_val = (reader.read_literal(8) as i32) - 128;
                        coeffs.fill(0);
                        coeffs[0] = dc_val;

                        // Apply IDCT to 4 8x8 subblocks in this 16x16 macroblock
                        for sby in 0..2 {
                            for sbx in 0..2 {
                                let px = y_x + sbx * 8;
                                let py = y_y + sby * 8;
                                if px + 8 <= width && py + 8 <= height {
                                    let offset = py * y_stride + px;
                                    idct8x8_add(&coeffs, &mut y_out[offset..], y_stride, 1);
                                }
                            }
                        }

                        // Apply to chroma
                        let cx = bx * 8;
                        let cy = by * 8;
                        if cx + 8 <= width / 2 && cy + 8 <= uv_height {
                            let u_offset = cy * uv_stride + cx;
                            let v_offset = cy * uv_stride + cx;
                            idct8x8_add(&coeffs, &mut u_out[u_offset..], uv_stride, 1);
                            idct8x8_add(&coeffs, &mut v_out[v_offset..], uv_stride, 1);
                        }
                    }
                }
            }
        }

        // Apply in-loop deblocking filter across reconstructed frame
        if config.filter_level > 0 {
            loop_filter_frame(
                y_out,
                u_out,
                v_out,
                width,
                height,
                y_stride,
                uv_stride,
                config.filter_level as u8,
                config.sharpness as u8,
            );
        }

        Ok(())
    }
}
