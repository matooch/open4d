//! Sparse block grid (docs/specs/VOLUME.md section 1).

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
