pub mod bitstream;
pub mod decoder;
pub mod encodings_data;
pub mod ffi;
pub mod integer_encoded;

pub use decoder::{
    decode_astc_to_rgba8, decompress_block, query_decompressed_size, AstcLevel, AstcPixel,
    TexelWeightParams, ASTC_ERROR_COLOR,
};

#[cfg(feature = "parallel")]
pub use decoder::decode_astc_to_rgba8_parallel;
