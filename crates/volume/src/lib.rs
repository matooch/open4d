//! forge-volume: sparse voxel grids, signed distance fields, CSG and the volume mesher.
//! Spec: docs/specs/VOLUME.md.

pub mod bvh;
pub mod csg;
pub mod filters;
pub mod grid;
pub mod mesher;
pub mod node;
pub mod preview;
pub mod sdf;
pub mod vdb;

pub use csg::{CsgOp, combine, negate, smooth_min};
pub use grid::{BLOCK, SparseGrid};
pub use mesher::{
    CellMesh, MeshOptions, Polarity, cell_mesh, gradient_normals, merge_blocks, mesh_volume,
    volume_to_mesh,
};
pub use node::{
    BUILDER, FILTER, MESHER, Volume, VolumeBuildError, VolumeKind, VolumePlugin, build_volume,
    mesh_volume_node, sdf_to_fog,
};
pub use node::{BuildOptions, build_volume_with, mesh_volume_node_with};
pub use preview::{
    DEFAULT_PREVIEW_CELLS, Preview, PreviewFn, PreviewSnapshot, PreviewWorker, SliceSample,
    build_preview, slice_range, slice_samples, volume_bounds,
};
pub use sdf::{PointUnion, SdfParams, mesh_to_sdf, points_to_sdf, spline_to_sdf};
pub use vdb::{
    GridClass, VdbError, VdbFile, VdbGrid, WriteOptions, read_vdb, write_vdb, write_vdb_with,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum VolumeError {
    #[error("the grids do not sample the same lattice (voxel size or origin differ)")]
    GridMismatch,
}
