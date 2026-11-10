//! VP9 building blocks for Ryubing's NVDEC path.
//!
//! **This crate is not a VP9 decoder.** It provides:
//!
//! * the VP9 inverse transforms (IDCT / IADST, 4x4 to 32x32, with add-to-prediction),
//!   bit-exact with libvpx for in-range coefficients ([`idct`]);
//! * the VP9 in-loop deblocking filter kernels ([`loop_filter`]);
//! * a boolean entropy decoder ([`entropy`]);
//! * [`Vp9DecoderPipeline`], a *synthetic* DC-only reconstruction: it reads a toy
//!   bitstream (skip / has-coeffs flags and one 8-bit DC value each for Y, U and V per
//!   16x16 block), applies DC-only 8x8 IDCTs and runs the loop filter. It does not parse
//!   real VP9 frames (no compressed header, mode/motion-vector or coefficient token
//!   decoding, intra/inter prediction or reference frames), and its header parser only
//!   reads the frame marker, frame type and key-frame size.

pub mod entropy;
pub mod ffi;
pub mod idct;
pub mod loop_filter;
pub mod vp9;

pub use entropy::BoolDecoder;
pub use ffi::*;
pub use idct::*;
pub use loop_filter::*;
pub use vp9::*;
