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
    cell_mesh(grid, iso, pol).mesh
}

/// The cell mesh of [`volume_to_mesh`] with the lattice cell (lower corner index) of every
/// vertex, the input of the adaptive merge.
#[derive(Debug, Clone, PartialEq)]
pub struct CellMesh {
    pub mesh: PolyMesh,
    pub cells: Vec<IVec3>,
}

/// [`volume_to_mesh`] keeping the cell of each vertex.
pub fn cell_mesh(grid: &SparseGrid<f32>, iso: f64, pol: Polarity) -> CellMesh {
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
    let mut cell_of = vec![IVec3::ZERO; points.len()];
    for ((x, y, z), i) in &cells {
        cell_of[*i as usize] = IVec3::new(*x, *y, *z);
    }
    CellMesh {
        mesh: PolyMesh { points, polys },
        cells: cell_of,
    }
}

/// Cells per block edge at the coarsest merge level (the 8-voxel leaf of the sparse grid).
const MAX_LEVEL: u32 = 3;

/// Per-vertex unit normals of a mesh: normalised sums of the incident face normals.
fn vertex_normals(mesh: &PolyMesh) -> Vec<DVec3> {
    let mut n = vec![DVec3::ZERO; mesh.points.len()];
    for f in &mesh.polys {
        let p = |i: usize| mesh.points[f[i] as usize];
        let fn_ = (p(1) - p(0)).cross(p(2) - p(0));
        for &v in f {
            n[v as usize] += fn_;
        }
    }
    n.iter().map(|v| v.normalize_or_zero()).collect()
}

fn block_key(c: IVec3, level: u32) -> IVec3 {
    let s = 1 << level;
    IVec3::new(c.x.div_euclid(s), c.y.div_euclid(s), c.z.div_euclid(s))
}

/// Largest distance of the vertices of a block from the plane through their mean along their
/// mean normal.
fn plane_deviation(members: &[usize], points: &[DVec3], normals: &[DVec3]) -> f64 {
    let n = members.len() as f64;
    let mean = members.iter().map(|&i| points[i]).sum::<DVec3>() / n;
    let navg = members
        .iter()
        .map(|&i| normals[i])
        .sum::<DVec3>()
        .normalize_or_zero();
    members
        .iter()
        .map(|&i| (points[i] - mean).dot(navg).abs())
        .fold(0.0, f64::max)
}

/// Merges aligned blocks of 2, 4 or 8 cells into one vertex each (VOLUME.md section 7.5):
/// from the largest block down, a block whose vertices lie within `tolerance` of their mean
/// plane becomes a leaf with the mean of its vertices; the quads follow the leaves, losing
/// corners that fall together (triangles) and duplicates.
pub fn merge_blocks(cm: &CellMesh, tolerance: f64) -> PolyMesh {
    let points = &cm.mesh.points;
    let normals = vertex_normals(&cm.mesh);
    // leaf id of every vertex
    let mut leaf_of = vec![usize::MAX; points.len()];
    let mut leaves: Vec<Vec<usize>> = Vec::new();
    let mut top: BTreeMap<(i32, i32, i32), Vec<usize>> = BTreeMap::new();
    for (i, c) in cm.cells.iter().enumerate() {
        top.entry(key(block_key(*c, MAX_LEVEL)))
            .or_default()
            .push(i);
    }
    let mut stack: Vec<(u32, Vec<usize>)> =
        top.into_values().rev().map(|m| (MAX_LEVEL, m)).collect();
    while let Some((level, members)) = stack.pop() {
        if level == 0 || plane_deviation(&members, points, &normals) <= tolerance {
            for &i in &members {
                leaf_of[i] = leaves.len();
            }
            leaves.push(members);
            continue;
        }
        let mut children: BTreeMap<(i32, i32, i32), Vec<usize>> = BTreeMap::new();
        for &i in &members {
            children
                .entry(key(block_key(cm.cells[i], level - 1)))
                .or_default()
                .push(i);
        }
        for (_, m) in children.into_iter().rev() {
            stack.push((level - 1, m));
        }
    }
    let out_points: Vec<DVec3> = leaves
        .iter()
        .map(|m| m.iter().map(|&i| points[i]).sum::<DVec3>() / m.len() as f64)
        .collect();
    let mut seen: BTreeSet<Vec<usize>> = BTreeSet::new();
    let mut polys = Vec::new();
    for f in &cm.mesh.polys {
        let mut ring: Vec<usize> = Vec::with_capacity(4);
        for &v in f {
            let l = leaf_of[v as usize];
            if ring.last() != Some(&l) {
                ring.push(l);
            }
        }
        if ring.len() > 1 && ring.first() == ring.last() {
            ring.pop();
        }
        let mut distinct = ring.clone();
        distinct.sort_unstable();
        distinct.dedup();
        if distinct.len() == ring.len() && ring.len() >= 3 && seen.insert(distinct) {
            polys.push(ring.iter().map(|&l| l as u32).collect());
        }
    }
    PolyMesh {
        points: out_points,
        polys,
    }
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

/// The plane tolerance of the adaptive merge, in voxels per unit of the adaptive setting
/// (VOLUME.md section 7.5).
pub const ADAPTIVE_TOLERANCE_VOXELS: f64 = 2.0;

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
