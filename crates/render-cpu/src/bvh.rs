//! Axis-aligned boxes and a binned SAH BVH (RENDER.md R6).

use glam::{DMat4, DVec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: DVec3,
    pub max: DVec3,
}

impl Aabb {
    pub const EMPTY: Aabb = Aabb {
        min: DVec3::splat(f64::INFINITY),
        max: DVec3::splat(f64::NEG_INFINITY),
    };

    pub fn grow(&mut self, p: DVec3) {
        self.min = self.min.min(p);
        self.max = self.max.max(p);
    }

    pub fn union(self, o: Aabb) -> Aabb {
        Aabb {
            min: self.min.min(o.min),
            max: self.max.max(o.max),
        }
    }

    pub fn centroid(&self) -> DVec3 {
        (self.min + self.max) * 0.5
    }

    pub fn area(&self) -> f64 {
        let d = (self.max - self.min).max(DVec3::ZERO);
        2.0 * (d.x * d.y + d.y * d.z + d.z * d.x)
    }

    pub fn is_empty(&self) -> bool {
        self.min.x > self.max.x
    }

    /// The box of the eight transformed corners.
    pub fn transformed(&self, m: &DMat4) -> Aabb {
        let mut b = Aabb::EMPTY;
        for i in 0..8 {
            let p = DVec3::new(
                if i & 1 == 0 { self.min.x } else { self.max.x },
                if i & 2 == 0 { self.min.y } else { self.max.y },
                if i & 4 == 0 { self.min.z } else { self.max.z },
            );
            b.grow(m.transform_point3(p));
        }
        b
    }

    /// Slab test; returns whether the ray enters the box before `tmax`.
    #[inline]
    pub fn hit(&self, o: DVec3, inv_d: DVec3, tmax: f64) -> bool {
        let t0 = (self.min - o) * inv_d;
        let t1 = (self.max - o) * inv_d;
        let tn = t0.min(t1).max_element().max(0.0);
        let tf = t0.max(t1).min_element().min(tmax);
        tn <= tf
    }
}

#[derive(Debug, Clone, Copy)]
struct Node {
    bounds: Aabb,
    /// Leaf: first primitive slot. Interior: index of the left child (right = left + 1).
    first: u32,
    /// Leaf: primitive count (> 0). Interior: 0.
    count: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Bvh {
    nodes: Vec<Node>,
    /// Primitive ids in leaf order.
    prims: Vec<u32>,
}

const BINS: usize = 16;
const LEAF: usize = 4;

impl Bvh {
    /// Builds over `bounds[i]` (primitive `i`). Empty input gives an empty tree.
    pub fn build(bounds: &[Aabb]) -> Bvh {
        let mut bvh = Bvh {
            nodes: Vec::new(),
            prims: (0..bounds.len() as u32).collect(),
        };
        if bounds.is_empty() {
            return bvh;
        }
        let cents: Vec<DVec3> = bounds.iter().map(Aabb::centroid).collect();
        bvh.nodes.push(Node {
            bounds: Aabb::EMPTY,
            first: 0,
            count: 0,
        });
        bvh.split(0, 0, bounds.len(), bounds, &cents);
        bvh
    }

    fn split(&mut self, node: usize, lo: usize, hi: usize, b: &[Aabb], c: &[DVec3]) {
        let mut bb = Aabb::EMPTY;
        let mut cb = Aabb::EMPTY;
        for &p in &self.prims[lo..hi] {
            bb = bb.union(b[p as usize]);
            cb.grow(c[p as usize]);
        }
        self.nodes[node].bounds = bb;
        let n = hi - lo;
        let leaf = |s: &mut Bvh| {
            s.nodes[node].first = lo as u32;
            s.nodes[node].count = n as u32;
        };
        if n <= LEAF.min(2) {
            return leaf(self);
        }
        let ext = cb.max - cb.min;
        let axis = if ext.x >= ext.y && ext.x >= ext.z {
            0
        } else if ext.y >= ext.z {
            1
        } else {
            2
        };
        let mid = if ext[axis] <= 0.0 {
            // All centroids coincide: median split by order.
            if n <= LEAF {
                return leaf(self);
            }
            lo + n / 2
        } else {
            // Binned SAH.
            let bin_of = |p: DVec3| -> usize {
                (((p[axis] - cb.min[axis]) / ext[axis] * BINS as f64) as usize).min(BINS - 1)
            };
            let mut cnt = [0usize; BINS];
            let mut bx = [Aabb::EMPTY; BINS];
            for &p in &self.prims[lo..hi] {
                let k = bin_of(c[p as usize]);
                cnt[k] += 1;
                bx[k] = bx[k].union(b[p as usize]);
            }
            let mut best = (f64::INFINITY, 0usize);
            let mut left_area = [0.0; BINS];
            let mut left_cnt = [0usize; BINS];
            let (mut acc, mut ac) = (Aabb::EMPTY, 0usize);
            for k in 0..BINS - 1 {
                acc = acc.union(bx[k]);
                ac += cnt[k];
                left_area[k] = if ac > 0 { acc.area() } else { 0.0 };
                left_cnt[k] = ac;
            }
            let (mut acc, mut ac) = (Aabb::EMPTY, 0usize);
            for k in (1..BINS).rev() {
                acc = acc.union(bx[k]);
                ac += cnt[k];
                let l = left_cnt[k - 1];
                if l == 0 || ac == 0 {
                    continue;
                }
                let cost = left_area[k - 1] * l as f64 + acc.area() * ac as f64;
                if cost < best.0 {
                    best = (cost, k);
                }
            }
            let leaf_cost = bb.area() * n as f64;
            if best.0.is_infinite() || (n <= LEAF && best.0 + bb.area() * 0.5 >= leaf_cost) {
                if n <= LEAF {
                    return leaf(self);
                }
                lo + n / 2
            } else {
                let split_bin = best.1;
                // Partition in place.
                let (mut i, mut j) = (lo, hi);
                while i < j {
                    if bin_of(c[self.prims[i] as usize]) < split_bin {
                        i += 1;
                    } else {
                        j -= 1;
                        self.prims.swap(i, j);
                    }
                }
                if i == lo || i == hi { lo + n / 2 } else { i }
            }
        };
        if mid == lo || mid == hi {
            return leaf(self);
        }
        let left = self.nodes.len();
        for _ in 0..2 {
            self.nodes.push(Node {
                bounds: Aabb::EMPTY,
                first: 0,
                count: 0,
            });
        }
        self.nodes[node].first = left as u32;
        self.nodes[node].count = 0;
        self.split(left, lo, mid, b, c);
        self.split(left + 1, mid, hi, b, c);
    }

    pub fn bounds(&self) -> Aabb {
        self.nodes.first().map_or(Aabb::EMPTY, |n| n.bounds)
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Visits the primitives whose leaf boxes the ray enters (nearest child first).
    /// `f(prim, tmax)` returns `Some(t)` for a hit closer than `tmax`, which becomes the new
    /// `tmax`; with `any` the traversal stops at the first hit. Returns the final `tmax`.
    pub fn traverse(
        &self,
        o: DVec3,
        d: DVec3,
        mut tmax: f64,
        any: bool,
        f: &mut impl FnMut(u32, f64) -> Option<f64>,
    ) -> f64 {
        if self.nodes.is_empty() {
            return tmax;
        }
        let inv = DVec3::new(1.0 / d.x, 1.0 / d.y, 1.0 / d.z);
        let mut stack = [0u32; 128];
        let mut sp = 0;
        let mut cur = 0u32;
        loop {
            let n = &self.nodes[cur as usize];
            if n.bounds.hit(o, inv, tmax) {
                if n.count > 0 {
                    for k in n.first..n.first + n.count {
                        if let Some(t) = f(self.prims[k as usize], tmax) {
                            tmax = t;
                            if any {
                                return tmax;
                            }
                        }
                    }
                } else {
                    let l = n.first;
                    let (lb, rb) = (&self.nodes[l as usize], &self.nodes[l as usize + 1]);
                    let dl = (lb.bounds.centroid() - o).length_squared();
                    let dr = (rb.bounds.centroid() - o).length_squared();
                    let (near, far) = if dl <= dr { (l, l + 1) } else { (l + 1, l) };
                    stack[sp] = far;
                    sp += 1;
                    cur = near;
                    continue;
                }
            }
            if sp == 0 {
                return tmax;
            }
            sp -= 1;
            cur = stack[sp];
        }
    }
}
