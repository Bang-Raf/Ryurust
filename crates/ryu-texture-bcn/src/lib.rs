#![allow(unused_parens, unused_assignments)]
//! High-performance SIMD BC1-BC7 block compression texture decoder for Ryubing.

pub mod bc1;
pub mod bc2;
pub mod bc3;
pub mod bc4;
pub mod bc5;
pub mod bc6h;
pub mod bc7;
pub mod block;
pub mod ffi;
pub mod tables;

pub use bc1::{decode_bc1_block, decode_bc1_surface};
pub use bc2::{decode_bc2_block, decode_bc2_surface};
pub use bc3::{decode_bc3_block, decode_bc3_surface};
pub use bc4::{decode_bc4_block, decode_bc4_surface, decode_bc4_surface_stride};
pub use bc5::{decode_bc5_block, decode_bc5_surface, decode_bc5_surface_stride};
pub use bc6h::{decode_bc6h_block, decode_bc6h_surface, HALF_ONE};
pub use bc7::{decode_bc7_block, decode_bc7_surface};
pub use block::Block;
