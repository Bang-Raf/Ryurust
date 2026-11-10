pub mod blender;
pub mod ffi;
pub mod scaler;
pub mod types;

pub use blender::blend_surface;
pub use scaler::{deinterlace_bob, deinterlace_motion_adaptive, deinterlace_weave};
pub use types::{VicColorMatrix, VicPixel, VicRectangle};
