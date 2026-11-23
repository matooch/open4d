//! forge-anim: evaluates keyframe tracks and animated scenes (docs/specs/ANIM.md).

mod bake;
mod interp;
pub mod keyedit;
pub mod motion;
mod rotation;
mod scene_at;
pub mod tangent;

pub use bake::{bake_param, bake_samples, reduce_keys};
pub use interp::{
    TICKS_PER_SECOND, UserTangent, handles, int_at, points_at, quantize_time, speed_at, step_at,
    value_at, value_at_exact,
};
pub use rotation::{euler_filter, quaternion_rotation_at};
pub use scene_at::{overrides, overrides_exact, scene_at, track_nodes};
