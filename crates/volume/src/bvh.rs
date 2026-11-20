//! Triangle bounding volume hierarchy with winding number dipoles (VOLUME.md section 2).

use glam::DVec3;

#[derive(Debug, Clone, Copy)]
pub struct Tri {
    pub a: DVec3,
    pub b: DVec3,
    pub c: DVec3,
}

/// Triangles of a polygon mesh, each polygon fanned from corner 0.
pub fn triangles(points: &[DVec3], polys: &[Vec<u32>]) -> Vec<Tri> {
    let mut out = Vec::new();
    for poly in polys {
        for i in 1..poly.len().saturating_sub(1) {
            out.push(Tri {
                a: points[poly[0] as usize],
                b: points[poly[i] as usize],
                c: points[poly[i + 1] as usize],
            });
        }
    }
    out
}

/// Closest point on the triangle to `p` (Ericson 2004, section 5.1.5).
pub fn closest_point(t: &Tri, p: DVec3) -> DVec3 {
    let ab = t.b - t.a;
    let ac = t.c - t.a;
    let ap = p - t.a;
    let d1 = ab.dot(ap);
    let d2 = ac.dot(ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return t.a;
    }
    let bp = p - t.b;
    let d3 = ab.dot(bp);
    let d4 = ac.dot(bp);
    if d3 >= 0.0 && d4 <= d3 {
        return t.b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return t.a + ab * (d1 / (d1 - d3));
    }
    let cp = p - t.c;
    let d5 = ab.dot(cp);
    let d6 = ac.dot(cp);
    if d6 >= 0.0 && d5 <= d6 {
        return t.c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return t.a + ac * (d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return t.b + (t.c - t.b) * w;
    }
    let denom = 1.0 / (va + vb + vc);
    t.a + ab * (vb * denom) + ac * (vc * denom)
}

pub fn distance(t: &Tri, p: DVec3) -> f64 {
    (p - closest_point(t, p)).length()
}

/// Solid angle of a triangle seen from `p` (van Oosterom and Strackee 1983).
pub fn solid_angle(t: &Tri, p: DVec3) -> f64 {
    let a = t.a - p;
    let b = t.b - p;
    let c = t.c - p;
    let (la, lb, lc) = (a.length(), b.length(), c.length());
    let num = a.dot(b.cross(c));
    let den = la * lb * lc + a.dot(b) * lc + b.dot(c) * la + c.dot(a) * lb;
    2.0 * num.atan2(den)
}

#[derive(Debug)]
struct Node {
    lo: DVec3,
    hi: DVec3,
    /// Leaf (`count > 0`): range into `order`; inner: child indices.
    start: u32,
    count: u32,
    left: u32,
    right: u32,
    /// Dipole: area weighted centre, area vector and bounding radius around the centre.
    centre: DVec3,
    area_vec: DVec3,
    radius: f64,
}

#[derive(Debug)]
pub struct Bvh {
    tris: Vec<Tri>,
    order: Vec<u32>,
    nodes: Vec<Node>,
}

const LEAF: usize = 4;

impl Bvh {
    pub fn new(tris: Vec<Tri>) -> Bvh {
        let mut bvh = Bvh {
            order: (0..tris.len() as u32).collect(),
            tris,
            nodes: Vec::new(),
        };
        if !bvh.tris.is_empty() {
            let n = bvh.tris.len();
            bvh.build(0, n);
        }
        bvh
    }

    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    fn build(&mut self, start: usize, end: usize) -> u32 {
        let (mut lo, mut hi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
        let (mut clo, mut chi) = (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN));
        let (mut area, mut csum, mut avec) = (0.0, DVec3::ZERO, DVec3::ZERO);
        for &i in &self.order[start..end] {
            let t = &self.tris[i as usize];
            for p in [t.a, t.b, t.c] {
                lo = lo.min(p);
                hi = hi.max(p);
            }
            let cen = (t.a + t.b + t.c) / 3.0;
            clo = clo.min(cen);
            chi = chi.max(cen);
            let av = 0.5 * (t.b - t.a).cross(t.c - t.a);
            let a = av.length();
            area += a;
            csum += cen * a;
            avec += av;
        }
        let centre = if area > 0.0 {
            csum / area
        } else {
            (lo + hi) * 0.5
        };
        let mut radius: f64 = 0.0;
        for &i in &self.order[start..end] {
            let t = &self.tris[i as usize];
            for p in [t.a, t.b, t.c] {
                radius = radius.max((p - centre).length());
            }
        }
        let id = self.nodes.len() as u32;
        self.nodes.push(Node {
            lo,
            hi,
            start: start as u32,
            count: (end - start) as u32,
            left: 0,
            right: 0,
            centre,
            area_vec: avec,
            radius,
        });
        if end - start > LEAF {
            let ext = chi - clo;
            let axis = if ext.x >= ext.y && ext.x >= ext.z {
                0
            } else if ext.y >= ext.z {
                1
            } else {
                2
            };
            let tris = &self.tris;
            self.order[start..end].sort_by(|&i, &j| {
                let ci = tris[i as usize];
                let cj = tris[j as usize];
                let ki = (ci.a + ci.b + ci.c)[axis];
                let kj = (cj.a + cj.b + cj.c)[axis];
                ki.total_cmp(&kj).then(i.cmp(&j))
            });
            let mid = start + (end - start) / 2;
            let l = self.build(start, mid);
            let r = self.build(mid, end);
            let n = &mut self.nodes[id as usize];
            n.left = l;
            n.right = r;
            n.count = 0;
        }
        id
    }

    /// Distance from `p` to the nearest triangle when it is at most `limit`.
    pub fn distance_within(&self, p: DVec3, limit: f64) -> Option<f64> {
        if self.nodes.is_empty() {
            return None;
        }
        let mut best = limit;
        let mut found = false;
        let mut stack = vec![0u32];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n as usize];
            let d = (node.lo - p).max(p - node.hi).max(DVec3::ZERO).length();
            if d > best {
                continue;
            }
            if node.count > 0 {
                for &i in &self.order[node.start as usize..(node.start + node.count) as usize] {
                    let dist = distance(&self.tris[i as usize], p);
                    if dist <= best {
                        best = dist;
                        found = true;
                    }
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        found.then_some(best)
    }

    /// Generalized winding number at `p`.
    pub fn winding_number(&self, p: DVec3) -> f64 {
        if self.nodes.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        let mut stack = vec![0u32];
        while let Some(n) = stack.pop() {
            let node = &self.nodes[n as usize];
            let r = node.centre - p;
            let dist = r.length();
            if dist > 2.0 * node.radius && dist > 0.0 {
                total += node.area_vec.dot(r) / (dist * dist * dist);
            } else if node.count > 0 {
                for &i in &self.order[node.start as usize..(node.start + node.count) as usize] {
                    total += solid_angle(&self.tris[i as usize], p);
                }
            } else {
                stack.push(node.left);
                stack.push(node.right);
            }
        }
        total / (4.0 * std::f64::consts::PI)
    }
}
