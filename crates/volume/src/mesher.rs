//! Iso-surface extraction: one vertex per mixed cell, a quad across every crossed lattice edge
//! (VOLUME.md section 7).

use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::node::{Volume, VolumeKind};

pub use forge_geometry::isosurface::{
    ADAPTIVE_TOLERANCE_VOXELS, CellMesh, Polarity, cell_mesh, merge_blocks, volume_to_mesh,
};

/// The mesher's settings (VOLUME.md section 7).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshOptions {
    /// Surface threshold.
    pub iso: f64,
    /// Use the absolute value of the field.
    pub absolute: bool,
    /// Adaptivity, 0 to 1.
    pub adaptive: f64,
}

/// The mesh of a volume in the volume's own space.
pub fn mesh_volume(volume: &Volume, opts: &MeshOptions) -> PolyMesh {
    let grid = &volume.grid;
    let voxel = grid.voxel_size();
    let (field, iso, pol);
    let absolute;
    match volume.kind {
        VolumeKind::Sdf => {
            // VOLUME.md section 7.2: thresholds within 5 % of a voxel do not move the surface
            iso = if opts.iso.abs() <= 0.05 * voxel {
                0.0
            } else {
                opts.iso
            };
            pol = Polarity::LowInside;
        }
        VolumeKind::Fog => {
            iso = opts.iso;
            pol = Polarity::HighInside;
        }
    }
    // the absolute value of the field meshes the shell `|value| < threshold`
    if opts.absolute {
        absolute = grid.map(grid.background().abs(), f32::abs);
        field = &absolute;
    } else {
        field = grid;
    }
    let cm = cell_mesh(field, iso, pol);
    if opts.adaptive > 0.0 {
        merge_blocks(&cm, opts.adaptive * ADAPTIVE_TOLERANCE_VOXELS * voxel)
    } else {
        cm.mesh
    }
}

/// Unit surface normals at `points` from the central-difference gradient of the field
/// (VOLUME.md section 7.6): outward for distance fields, towards decreasing density for fog.
pub fn gradient_normals(volume: &Volume, points: &[DVec3]) -> Vec<DVec3> {
    let grid = &volume.grid;
    let h = grid.voxel_size() * 0.5;
    let sign = match volume.kind {
        VolumeKind::Sdf => 1.0,
        VolumeKind::Fog => -1.0,
    };
    points
        .iter()
        .map(|p| {
            let d = |axis: DVec3| {
                f64::from(grid.trilinear(*p + axis * h)) - f64::from(grid.trilinear(*p - axis * h))
            };
            (sign * DVec3::new(d(DVec3::X), d(DVec3::Y), d(DVec3::Z))).normalize_or_zero()
        })
        .collect()
}
