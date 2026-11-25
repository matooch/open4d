//! The prepared scene: two-level BVH over the snapshot (RENDER.md R6).

use std::collections::HashMap;

use forge_render_api::{Instance, MeshData, RenderError, Snapshot};
use glam::{DMat3, DMat4, DVec3};

use crate::bvh::{Aabb, Bvh};

#[derive(Debug, Clone, Copy)]
struct Tri {
    p0: DVec3,
    e1: DVec3,
    e2: DVec3,
}

#[derive(Debug)]
pub struct Blas {
    bvh: Bvh,
    tris: Vec<Tri>,
    /// True when every edge is shared by exactly two triangles.
    pub closed: bool,
    /// Negate the winding normal to get the outward one.
    flip: bool,
}

#[derive(Debug)]
pub struct Inst {
    pub blas: usize,
    pub mesh: usize,
    pub to_world: DMat4,
    to_obj: DMat4,
    normal_m: DMat3,
    pub material: usize,
    pub object_id: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct Hit {
    pub t: f64,
    pub inst: usize,
    pub tri: usize,
    pub u: f64,
    pub v: f64,
}

/// Everything the shader needs from a hit, in world space.
#[derive(Debug, Clone, Copy)]
pub struct HitGeom {
    pub p: DVec3,
    /// Outward geometric normal (open meshes: the winding normal).
    pub ng: DVec3,
    /// Shading normal on the same side as `ng`.
    pub ns: DVec3,
    pub uvw: DVec3,
    pub closed: bool,
    pub corners: [DVec3; 3],
    pub corner_uvw: [DVec3; 3],
    pub bary: [f64; 3],
}

#[derive(Debug)]
pub struct Scene {
    pub blases: Vec<Blas>,
    pub insts: Vec<Inst>,
    tlas: Bvh,
    pub bounds: Aabb,
    /// Diagonal of the scene bounds (at least 1e-3).
    pub scale: f64,
    pub triangles: usize,
}

fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

fn build_blas(m: &MeshData) -> Blas {
    let p = |i: u32| DVec3::from(m.positions[i as usize]);
    let mut tris = Vec::with_capacity(m.triangles.len());
    let mut bounds = Vec::with_capacity(m.triangles.len());
    let mut vol = 0.0;
    let mut edges: HashMap<(u32, u32), u32> = HashMap::new();
    // Welded vertex ids so meshes with split corners still count as closed.
    let mut ext = Aabb::EMPTY;
    for q in &m.positions {
        ext.grow(DVec3::from(*q));
    }
    let q = if ext.is_empty() {
        1.0
    } else {
        ((ext.max - ext.min).max_element() * 1e-6).max(1e-12)
    };
    let mut weld: HashMap<[i64; 3], u32> = HashMap::new();
    let mut wid = |v: DVec3| -> u32 {
        let n = weld.len() as u32;
        *weld
            .entry([
                (v.x / q).round() as i64,
                (v.y / q).round() as i64,
                (v.z / q).round() as i64,
            ])
            .or_insert(n)
    };
    for t in &m.triangles {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        tris.push(Tri {
            p0: a,
            e1: b - a,
            e2: c - a,
        });
        let mut bb = Aabb::EMPTY;
        bb.grow(a);
        bb.grow(b);
        bb.grow(c);
        bounds.push(bb);
        vol += a.dot(b.cross(c)) / 6.0;
        let ids = [wid(a), wid(b), wid(c)];
        for k in 0..3 {
            *edges.entry(edge_key(ids[k], ids[(k + 1) % 3])).or_insert(0) += 1;
        }
    }
    let closed = !tris.is_empty() && edges.values().all(|&n| n == 2);
    Blas {
        bvh: Bvh::build(&bounds),
        tris,
        closed,
        // Winding normal e1 x e2 points outward when the signed volume is positive.
        flip: closed && vol < 0.0,
    }
}

impl Blas {
    /// True when the winding normal must be negated to point outward (R6).
    pub fn flip(&self) -> bool {
        self.flip
    }

    fn intersect(
        &self,
        o: DVec3,
        d: DVec3,
        tmax: f64,
        any: bool,
    ) -> Option<(f64, usize, f64, f64)> {
        let mut best = None;
        self.bvh.traverse(o, d, tmax, any, &mut |i, tm| {
            let tri = &self.tris[i as usize];
            let pv = d.cross(tri.e2);
            let det = tri.e1.dot(pv);
            if det.abs() < 1e-18 {
                return None;
            }
            let inv = 1.0 / det;
            let tv = o - tri.p0;
            let u = tv.dot(pv) * inv;
            if !(0.0..=1.0).contains(&u) {
                return None;
            }
            let qv = tv.cross(tri.e1);
            let v = d.dot(qv) * inv;
            if v < 0.0 || u + v > 1.0 {
                return None;
            }
            let t = tri.e2.dot(qv) * inv;
            if t > 1e-12 && t < tm {
                best = Some((t, i as usize, u, v));
                Some(t)
            } else {
                None
            }
        });
        best
    }
}

impl Scene {
    pub fn build(snap: &Snapshot) -> Result<Scene, RenderError> {
        snap.validate()?;
        let blases: Vec<Blas> = snap.meshes.iter().map(build_blas).collect();
        let triangles = snap.meshes.iter().map(|m| m.triangles.len()).sum();
        let mut insts = Vec::new();
        let mut boxes = Vec::new();
        let mut bounds = Aabb::EMPTY;
        for i in &snap.instances {
            let Instance {
                mesh,
                transform,
                material,
                object_id,
            } = i;
            let m = DMat4::from_cols_array(transform);
            let det = m.determinant();
            if !det.is_finite() || det.abs() < 1e-24 {
                return Err(RenderError::Snapshot(
                    "an instance transform is singular".into(),
                ));
            }
            let inv = m.inverse();
            let bb = blases[*mesh].bvh.bounds();
            let wb = if bb.is_empty() {
                Aabb::EMPTY
            } else {
                bb.transformed(&m)
            };
            if !wb.is_empty() {
                bounds = bounds.union(wb);
            }
            boxes.push(wb);
            insts.push(Inst {
                blas: *mesh,
                mesh: *mesh,
                to_world: m,
                to_obj: inv,
                normal_m: DMat3::from_mat4(inv.transpose()),
                material: *material,
                object_id: *object_id,
            });
        }
        // Empty instances get a degenerate box far away so the build stays valid.
        for b in &mut boxes {
            if b.is_empty() {
                *b = Aabb {
                    min: DVec3::splat(1e300),
                    max: DVec3::splat(1e300),
                };
            }
        }
        let scale = if bounds.is_empty() {
            1.0
        } else {
            (bounds.max - bounds.min).length().max(1e-3)
        };
        Ok(Scene {
            blases,
            insts,
            tlas: Bvh::build(&boxes),
            bounds,
            scale,
            triangles,
        })
    }

    fn trace(&self, o: DVec3, d: DVec3, tmax: f64, any: bool) -> Option<Hit> {
        let mut best: Option<Hit> = None;
        self.tlas.traverse(o, d, tmax, any, &mut |i, tm| {
            let inst = &self.insts[i as usize];
            let lo = inst.to_obj.transform_point3(o);
            let ld = inst.to_obj.transform_vector3(d);
            let (t, tri, u, v) = self.blases[inst.blas].intersect(lo, ld, tm, any)?;
            best = Some(Hit {
                t,
                inst: i as usize,
                tri,
                u,
                v,
            });
            Some(t)
        });
        best
    }

    /// Nearest hit with `t < tmax` along the (unit) direction.
    pub fn intersect(&self, o: DVec3, d: DVec3, tmax: f64) -> Option<Hit> {
        self.trace(o, d, tmax, false)
    }

    /// Whether anything is hit with `t < tmax`.
    pub fn occluded(&self, o: DVec3, d: DVec3, tmax: f64) -> bool {
        self.trace(o, d, tmax, true).is_some()
    }

    /// World-space geometry of a hit.
    pub fn geometry(&self, snap: &Snapshot, o: DVec3, d: DVec3, h: &Hit) -> HitGeom {
        let inst = &self.insts[h.inst];
        let blas = &self.blases[inst.blas];
        let mesh = &snap.meshes[inst.mesh];
        let tri = &blas.tris[h.tri];
        let w = 1.0 - h.u - h.v;
        let bary = [w, h.u, h.v];
        let mut ng = tri.e1.cross(tri.e2).normalize_or_zero();
        if blas.flip {
            ng = -ng;
        }
        let ng_w = (inst.normal_m * ng).normalize_or_zero();
        let ns_w = if mesh.normals.is_empty() {
            ng_w
        } else {
            let n = |k: usize| DVec3::from(mesh.normals[h.tri * 3 + k]);
            let mut ns = (n(0) * w + n(1) * h.u + n(2) * h.v).normalize_or_zero();
            if ns == DVec3::ZERO {
                ns = ng;
            }
            let mut ns_w = (inst.normal_m * ns).normalize_or_zero();
            if ns_w.dot(ng_w) < 0.0 {
                ns_w = -ns_w;
            }
            ns_w
        };
        let uv = |k: usize| {
            if mesh.uvs.is_empty() {
                DVec3::ZERO
            } else {
                DVec3::from(mesh.uvs[h.tri * 3 + k])
            }
        };
        let corner_uvw = [uv(0), uv(1), uv(2)];
        let corners = [
            inst.to_world.transform_point3(tri.p0),
            inst.to_world.transform_point3(tri.p0 + tri.e1),
            inst.to_world.transform_point3(tri.p0 + tri.e2),
        ];
        HitGeom {
            p: o + d * h.t,
            ng: ng_w,
            ns: ns_w,
            uvw: corner_uvw[0] * w + corner_uvw[1] * h.u + corner_uvw[2] * h.v,
            closed: blas.closed,
            corners,
            corner_uvw,
            bary,
        }
    }
}
