//! Sparse block grid and iso-surface extraction (docs/specs/VOLUME.md sections 1 and 7).

use std::collections::HashMap;

use glam::{DVec3, IVec3};

/// Voxels per block edge.
pub const BLOCK: i32 = 8;
const BLOCK_VOXELS: usize = 512;

#[derive(Debug, Clone, PartialEq)]
struct Block<T> {
    values: Box<[T; BLOCK_VOXELS]>,
    /// One bit per voxel, 64 voxels per word.
    active: [u64; 8],
}

fn split(index: IVec3) -> (IVec3, usize) {
    let key = IVec3::new(
        index.x.div_euclid(BLOCK),
        index.y.div_euclid(BLOCK),
        index.z.div_euclid(BLOCK),
    );
    let l = index - key * BLOCK;
    (key, (l.x + BLOCK * (l.y + BLOCK * l.z)) as usize)
}

fn join(key: IVec3, offset: usize) -> IVec3 {
    let o = offset as i32;
    key * BLOCK + IVec3::new(o % BLOCK, (o / BLOCK) % BLOCK, o / (BLOCK * BLOCK))
}

/// A sparse grid of `T` on a uniform lattice.
#[derive(Debug, Clone, PartialEq)]
pub struct SparseGrid<T: Copy> {
    background: T,
    voxel_size: f64,
    origin: DVec3,
    blocks: HashMap<IVec3, Block<T>>,
}

impl<T: Copy> SparseGrid<T> {
    pub fn new(background: T, voxel_size: f64, origin: DVec3) -> SparseGrid<T> {
        assert!(voxel_size > 0.0, "voxel size must be positive");
        SparseGrid {
            background,
            voxel_size,
            origin,
            blocks: HashMap::new(),
        }
    }

    pub fn background(&self) -> T {
        self.background
    }

    pub fn set_background(&mut self, background: T) {
        self.background = background;
    }

    pub fn voxel_size(&self) -> f64 {
        self.voxel_size
    }

    pub fn origin(&self) -> DVec3 {
        self.origin
    }

    /// True when `other` samples the same lattice.
    pub fn same_lattice<U: Copy>(&self, other: &SparseGrid<U>) -> bool {
        self.voxel_size == other.voxel_size && self.origin == other.origin
    }

    /// World position of the sample of voxel `index`.
    pub fn index_to_world(&self, index: IVec3) -> DVec3 {
        self.origin + index.as_dvec3() * self.voxel_size
    }

    /// Index space position (fractional) of a world point.
    pub fn world_to_index(&self, p: DVec3) -> DVec3 {
        (p - self.origin) / self.voxel_size
    }

    /// The value of a voxel; the background when it was never stored.
    pub fn get(&self, index: IVec3) -> T {
        let (key, o) = split(index);
        self.blocks
            .get(&key)
            .map_or(self.background, |b| b.values[o])
    }

    pub fn is_active(&self, index: IVec3) -> bool {
        let (key, o) = split(index);
        self.blocks
            .get(&key)
            .is_some_and(|b| b.active[o / 64] >> (o % 64) & 1 == 1)
    }

    fn block_mut(&mut self, key: IVec3) -> &mut Block<T> {
        let bg = self.background;
        self.blocks.entry(key).or_insert_with(|| Block {
            values: Box::new([bg; BLOCK_VOXELS]),
            active: [0; 8],
        })
    }

    /// Stores a value and activates the voxel.
    pub fn set(&mut self, index: IVec3, value: T) {
        let (key, o) = split(index);
        let b = self.block_mut(key);
        b.values[o] = value;
        b.active[o / 64] |= 1 << (o % 64);
    }

    /// Stores a value without activating the voxel (the filled interior).
    pub fn set_inactive(&mut self, index: IVec3, value: T) {
        let (key, o) = split(index);
        let b = self.block_mut(key);
        b.values[o] = value;
        b.active[o / 64] &= !(1 << (o % 64));
    }

    /// Deactivates a voxel and resets it to the background.
    pub fn clear(&mut self, index: IVec3) {
        let bg = self.background;
        self.set_inactive(index, bg);
    }

    pub fn active_count(&self) -> usize {
        self.blocks
            .values()
            .map(|b| {
                b.active
                    .iter()
                    .map(|w| w.count_ones() as usize)
                    .sum::<usize>()
            })
            .sum()
    }

    /// Number of allocated blocks.
    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    fn sorted_keys(&self) -> Vec<IVec3> {
        let mut keys: Vec<IVec3> = self.blocks.keys().copied().collect();
        keys.sort_by_key(|k| (k.x, k.y, k.z));
        keys
    }

    /// Active voxels with their values, in ascending `(x, y, z)` order.
    pub fn active_voxels(&self) -> Vec<(IVec3, T)> {
        let mut out = Vec::new();
        for key in self.sorted_keys() {
            let b = &self.blocks[&key];
            for o in 0..BLOCK_VOXELS {
                if b.active[o / 64] >> (o % 64) & 1 == 1 {
                    out.push((join(key, o), b.values[o]));
                }
            }
        }
        out.sort_by_key(|(i, _)| (i.x, i.y, i.z));
        out
    }

    /// Every stored voxel (active or not), ascending.
    pub fn stored_voxels(&self) -> Vec<(IVec3, T, bool)> {
        let mut out = Vec::new();
        for key in self.sorted_keys() {
            let b = &self.blocks[&key];
            for o in 0..BLOCK_VOXELS {
                out.push((
                    join(key, o),
                    b.values[o],
                    b.active[o / 64] >> (o % 64) & 1 == 1,
                ));
            }
        }
        out.sort_by_key(|(i, _, _)| (i.x, i.y, i.z));
        out
    }

    /// Every block with a stored voxel: key, the 512 values (x fastest) and the active bit words,
    /// ascending by key.
    pub fn blocks(&self) -> Vec<(IVec3, Vec<T>, [u64; 8])> {
        self.sorted_keys()
            .into_iter()
            .map(|k| {
                let b = &self.blocks[&k];
                (k, b.values.to_vec(), b.active)
            })
            .collect()
    }

    /// Stores a whole block (`values.len()` must be 512, x fastest).
    pub fn set_block(&mut self, key: IVec3, values: &[T], active: [u64; 8]) {
        assert_eq!(values.len(), BLOCK_VOXELS);
        let b = self.block_mut(key);
        b.values.copy_from_slice(values);
        b.active = active;
    }

    /// Index box `(min, max)` (inclusive) of the active voxels.
    pub fn bounds(&self) -> Option<(IVec3, IVec3)> {
        let mut it = self.active_voxels().into_iter().map(|(i, _)| i);
        let first = it.next()?;
        Some(it.fold((first, first), |(lo, hi), i| (lo.min(i), hi.max(i))))
    }

    /// Keeps the voxels for which `f(index, value)` holds; the others become inactive
    /// background. Blocks left without active voxels and holding only the background are freed.
    pub fn retain_active(&mut self, mut f: impl FnMut(IVec3, T) -> bool) {
        let bg = self.background;
        for key in self.sorted_keys() {
            let b = self.blocks.get_mut(&key).expect("key from the map");
            for o in 0..BLOCK_VOXELS {
                if b.active[o / 64] >> (o % 64) & 1 == 1 && !f(join(key, o), b.values[o]) {
                    b.active[o / 64] &= !(1 << (o % 64));
                    b.values[o] = bg;
                }
            }
        }
        self.blocks.retain(|_, b| b.active.iter().any(|w| *w != 0));
    }

    /// A grid of the same lattice with `f` applied to the values (active flags kept).
    pub fn map<U: Copy>(&self, background: U, mut f: impl FnMut(T) -> U) -> SparseGrid<U> {
        let mut out = SparseGrid::new(background, self.voxel_size, self.origin);
        for (key, b) in &self.blocks {
            let nb = Block {
                values: Box::new(std::array::from_fn(|o| f(b.values[o]))),
                active: b.active,
            };
            out.blocks.insert(*key, nb);
        }
        out
    }
}

impl SparseGrid<f32> {
    /// Trilinear sample at a world point; inactive voxels read as the background.
    pub fn trilinear(&self, p: DVec3) -> f32 {
        let q = self.world_to_index(p);
        let f = q.floor();
        let base = f.as_ivec3();
        let t = q - f;
        let mut acc = 0.0f64;
        for c in 0..8 {
            let d = IVec3::new(c & 1, (c >> 1) & 1, (c >> 2) & 1);
            let w = (if d.x == 1 { t.x } else { 1.0 - t.x })
                * (if d.y == 1 { t.y } else { 1.0 - t.y })
                * (if d.z == 1 { t.z } else { 1.0 - t.z });
            acc += w * f64::from(self.get(base + d));
        }
        acc as f32
    }

    /// Fills unstored voxels between two negative active voxels of the same x row with
    /// `-interior` (VOLUME.md section 2.5).
    pub fn signed_flood_fill(&mut self, interior: f32) {
        use std::collections::BTreeMap;
        let mut rows: BTreeMap<(i32, i32), Vec<i32>> = BTreeMap::new();
        for (i, v) in self.active_voxels() {
            if v < 0.0 {
                rows.entry((i.y, i.z)).or_default().push(i.x);
            }
        }
        let mut negatives: Vec<(i32, i32, i32)> = Vec::new();
        for ((y, z), xs) in &rows {
            // xs ascending; a gap between neighbours that are consecutive negative voxels
            for w in xs.windows(2) {
                if w[1] - w[0] > 1 {
                    // the gap must be unstored inside: every voxel between is inactive
                    let all_inactive =
                        (w[0] + 1..w[1]).all(|x| !self.is_active(IVec3::new(x, *y, *z)));
                    if all_inactive {
                        negatives.extend((w[0] + 1..w[1]).map(|x| (x, *y, *z)));
                    }
                }
            }
        }
        for (x, y, z) in negatives {
            self.set_inactive(IVec3::new(x, y, z), -interior);
        }
    }
}

use std::collections::{BTreeMap, BTreeSet};

use crate::mesh::PolyMesh;

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

/// The plane tolerance of the adaptive merge, in voxels per unit of the adaptive setting
/// (VOLUME.md section 7.5).
pub const ADAPTIVE_TOLERANCE_VOXELS: f64 = 2.0;

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
