//! Spec: docs/specs/VOLUME.md.

pub mod bvh;
pub mod csg;
pub mod grid;
pub mod sdf;

pub use csg::{CsgOp, combine, negate, smooth_min};
pub use grid::{BLOCK, SparseGrid};
pub use sdf::{PointUnion, SdfParams, mesh_to_sdf, points_to_sdf, spline_to_sdf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VolumeError {
    #[error("the grids do not sample the same lattice (voxel size or origin differ)")]
    GridMismatch,
}
