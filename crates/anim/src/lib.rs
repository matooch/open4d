//! forge-anim: evaluates keyframe tracks and animated scenes (docs/specs/ANIM.md).

mod interp;
mod scene_at;

pub use interp::{TICKS_PER_SECOND, quantize_time, value_at};
pub use scene_at::scene_at;
