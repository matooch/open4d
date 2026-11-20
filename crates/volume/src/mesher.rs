//! Iso-surface extraction: one vertex per mixed cell, a quad across every crossed lattice edge
//! (VOLUME.md section 7).

use std::collections::{BTreeMap, BTreeSet};

use forge_geometry::PolyMesh;
use glam::{DVec3, IVec3};

use crate::grid::SparseGrid;
use crate::node::{Volume, VolumeKind};

const CORNERS: [IVec3; 8] = [
    IVec3::new(0, 0, 0),
    IVec3::new(1, 0, 0),
    IVec3::new(0, 1, 0),
    IVec3::new(1, 1, 0),
    IVec3::new(0, 0, 1),
    IVec3::new(1, 0, 1),
    IVec3::new(0, 1, 1),
    IVec3::new(1, 1, 1),
];

/// The 12 cell edges as corner index pairs.
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

fn key(i: IVec3) -> (i32, i32, i32) {
    (i.x, i.y, i.z)
}

/// Which side of the iso value is inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Polarity {
    /// Distance fields: inside is below the iso value.
    LowInside,
    /// Density fields: inside is above the iso value.
    HighInside,
}

fn inside(v: f32, iso: f64, pol: Polarity) -> bool {
    match pol {
        Polarity::LowInside => f64::from(v) < iso,
        Polarity::HighInside => f64::from(v) > iso,
    }
}

/// Marching cell mesh of the iso-surface `value == iso` of `grid`, in world space. Quads are
/// wound counter-clockwise seen from outside.
pub fn volume_to_mesh(grid: &SparseGrid<f32>, iso: f64, pol: Polarity) -> PolyMesh {
    let active: Vec<IVec3> = grid.active_voxels().into_iter().map(|(i, _)| i).collect();
    // Lattice edges with a sign change, from every edge touching an active voxel.
    let mut edges: BTreeSet<((i32, i32, i32), usize)> = BTreeSet::new();
    let axes = [IVec3::X, IVec3::Y, IVec3::Z];
    for q in &active {
        for (a, e) in axes.iter().enumerate() {
            for lo in [*q, *q - *e] {
                let (v0, v1) = (grid.get(lo), grid.get(lo + *e));
                if inside(v0, iso, pol) != inside(v1, iso, pol) {
                    edges.insert((key(lo), a));
                }
            }
        }
    }
    // Cells around the crossed edges, with one vertex each.
    let mut cells: BTreeMap<(i32, i32, i32), u32> = BTreeMap::new();
    let mut points: Vec<DVec3> = Vec::new();
    let mut cell_vertex = |c: IVec3, grid: &SparseGrid<f32>| -> u32 {
        *cells.entry(key(c)).or_insert_with(|| {
            let vals: Vec<f32> = CORNERS.iter().map(|d| grid.get(c + *d)).collect();
            let mut sum = DVec3::ZERO;
            let mut n = 0.0;
            for &(a, b) in &EDGES {
                if inside(vals[a], iso, pol) != inside(vals[b], iso, pol) {
                    let (va, vb) = (f64::from(vals[a]), f64::from(vals[b]));
                    let t = ((iso - va) / (vb - va)).clamp(0.0, 1.0);
                    let pa = (c + CORNERS[a]).as_dvec3();
                    let pb = (c + CORNERS[b]).as_dvec3();
                    sum += pa + (pb - pa) * t;
                    n += 1.0;
                }
            }
            points.push(grid.origin() + sum / n * grid.voxel_size());
            (points.len() - 1) as u32
        })
    };
    let mut polys: Vec<Vec<u32>> = Vec::with_capacity(edges.len());
    for &((x, y, z), a) in &edges {
        let p = IVec3::new(x, y, z);
        let (u, v) = match a {
            0 => (IVec3::Y, IVec3::Z),
            1 => (IVec3::Z, IVec3::X),
            _ => (IVec3::X, IVec3::Y),
        };
        let ring = [p - u - v, p - v, p, p - u];
        let mut quad: Vec<u32> = ring.iter().map(|c| cell_vertex(*c, grid)).collect();
        // `p` inside: the surface faces +axis (outside is beyond the upper end).
        if !inside(grid.get(p), iso, pol) {
            quad.reverse();
        }
        polys.push(quad);
    }
    PolyMesh { points, polys }
}

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
    match volume.kind {
        VolumeKind::Sdf => {
            // see VOLUME.md section 7.2: thresholds within 5 % of a voxel do not move the surface
            let iso = if opts.iso.abs() <= 0.05 * grid.voxel_size() {
                0.0
            } else {
                opts.iso
            };
            volume_to_mesh(grid, iso, Polarity::LowInside)
        }
        VolumeKind::Fog => volume_to_mesh(grid, opts.iso, Polarity::HighInside),
    }
}
