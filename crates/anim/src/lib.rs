//! forge-anim: evaluates keyframe tracks and animated scenes (docs/specs/ANIM.md).

mod bake;
mod interp;
pub mod motion;
mod rotation;
mod scene_at;
pub mod tangent;

pub use bake::{bake_param, reduce_keys};
pub use interp::{
    TICKS_PER_SECOND, UserTangent, int_at, points_at, quantize_time, speed_at, step_at, value_at,
    value_at_exact,
};
pub use rotation::{euler_filter, quaternion_rotation_at};
pub use scene_at::{overrides, scene_at, track_nodes};
