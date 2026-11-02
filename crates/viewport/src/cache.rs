//! Revision-keyed cache for uploaded GPU objects (docs/specs/VIEWPORT.md §10.2).

use std::collections::HashMap;
use std::hash::Hash;

use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;

/// Cumulative counters of a [`RevisionCache`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CacheStats {
    /// Times `build` ran (a first upload or a changed revision).
    pub uploads: u64,
    /// Times the stored value was reused.
    pub hits: u64,
}

/// Stores one value per key together with the revision it was built from.
#[derive(Debug)]
pub struct RevisionCache<K, V> {
    entries: HashMap<K, (u64, V)>,
    stats: CacheStats,
}

impl<K, V> Default for RevisionCache<K, V> {
    fn default() -> Self {
        RevisionCache {
            entries: HashMap::new(),
            stats: CacheStats::default(),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> RevisionCache<K, V> {
    pub fn new() -> Self {
        Self::default()
    }

    /// The stored value if `key` has the same `revision`; else `build()` is called and its result
    /// replaces the entry.
    pub fn get_or_insert_with(&mut self, key: K, revision: u64, build: impl FnOnce() -> V) -> V {
        if let Some((rev, v)) = self.entries.get(&key)
            && *rev == revision
        {
            self.stats.hits += 1;
            return v.clone();
        }
        let v = build();
        self.stats.uploads += 1;
        self.entries.insert(key, (revision, v.clone()));
        v
    }

    /// Keeps only the entries whose key satisfies `keep`.
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        self.entries.retain(|k, _| keep(k));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    pub fn stats(&self) -> CacheStats {
        self.stats
    }

    pub fn reset_stats(&mut self) {
        self.stats = CacheStats::default();
    }
}

const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a style hash over a stream of 64-bit words (one multiply per word, then a xor-shift so
/// the high bits reach the low ones).
#[derive(Debug, Clone, Copy)]
pub struct Hasher(u64);

impl Default for Hasher {
    fn default() -> Self {
        Hasher(FNV_OFFSET)
    }
}

impl Hasher {
    pub fn new() -> Hasher {
        Hasher::default()
    }

    pub fn word(&mut self, w: u64) {
        self.0 = (self.0 ^ w).wrapping_mul(FNV_PRIME);
        self.0 ^= self.0 >> 29;
    }

    pub fn f64(&mut self, v: f64) {
        self.word(v.to_bits());
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

/// Content hash of a mesh: counts, point coordinate bits and polygon vertex lists.
pub fn mesh_revision(mesh: &PolyMesh) -> u64 {
    let mut h = Hasher::new();
    h.word(mesh.points.len() as u64);
    h.word(mesh.polys.len() as u64);
    for p in &mesh.points {
        h.f64(p.x);
        h.f64(p.y);
        h.f64(p.z);
    }
    for poly in &mesh.polys {
        h.word(poly.len() as u64);
        for v in poly {
            h.word(u64::from(*v));
        }
    }
    h.finish()
}

/// Hash of the three index sets of a selection.
pub fn selection_revision(sel: &Selection) -> u64 {
    let mut h = Hasher::new();
    for set in [&sel.points, &sel.edges, &sel.polys] {
        h.word(set.len() as u64);
        for i in set.iter() {
            h.word(u64::from(i));
        }
    }
    h.finish()
}

/// Mixes two revisions (order matters).
pub fn combine(a: u64, b: u64) -> u64 {
    let mut h = Hasher::new();
    h.word(a);
    h.word(b);
    h.finish()
}

/// Revision of an instance set: the prototype revisions, the world matrix and, for every
/// visible clone, its index, matrix, colour and prototype. Changes to hidden clones do not count.
pub fn instances_revision(
    proto_revisions: &[u64],
    input: &crate::instancing::InstanceInput,
    world: &glam::DMat4,
) -> u64 {
    let mut h = Hasher::new();
    h.word(proto_revisions.len() as u64);
    for r in proto_revisions {
        h.word(*r);
    }
    for v in world.to_cols_array() {
        h.f64(v);
    }
    h.word(input.len() as u64);
    for i in 0..input.len() {
        if !input.visible.get(i).copied().unwrap_or(true) {
            continue;
        }
        h.word(i as u64);
        for v in input.matrices[i].to_cols_array() {
            h.f64(v);
        }
        for v in input.colors[i] {
            h.f64(v);
        }
        h.word(u64::from(input.child.get(i).copied().unwrap_or(0)));
    }
    h.finish()
}
