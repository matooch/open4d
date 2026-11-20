//! Primitive UV layouts (docs/specs/UV.md section U2). Derived from the mesh geometry and the
//! node's params; the mesh itself comes from [`crate::build`].

use std::collections::BTreeMap;
use std::f64::consts::{PI, TAU};

use forge_core::Value;
use forge_geometry::{CornerUvs, PolyMesh};
use glam::DVec3;

use crate::{BuildError, Params};

const EPS: f64 = 1e-6;

/// Mesh and UVs together (`None` when no layout is derived for the options).
pub fn build_with_uvs(
    type_id: &str,
    values: &BTreeMap<String, Value>,
) -> Result<(PolyMesh, Option<CornerUvs>), BuildError> {
    let mesh = crate::build(type_id, values)?;
    let uvs = primitive_uvs(type_id, values, &mesh);
    Ok((mesh, uvs))
}

/// The corner UVs of a primitive's mesh, or `None` where the layout is not derived.
pub fn primitive_uvs(
    type_id: &str,
    values: &BTreeMap<String, Value>,
    mesh: &PolyMesh,
) -> Option<CornerUvs> {
    let ty = crate::types().into_iter().find(|t| t.id == type_id)?;
    let p = Params { ty: &ty, values };
    let up = |p: &Params| p.enumeration("orientation") == "pos_y";
    let corners = match type_id {
        "forge.plane" if up(&p) => plane(mesh),
        "forge.cube" if !p.bool("fillet") && !p.bool("separate_surfaces") => cube(mesh),
        "forge.disc" if up(&p) && !p.bool("slice") => planar(mesh, 1.0),
        "forge.sphere" if p.enumeration("type") == "standard" => {
            revolve(mesh, p.int("segments"), Wall::Sphere)
        }
        "forge.cylinder" if up(&p) && !p.bool("slice") && !p.bool("fillet") => {
            revolve(mesh, p.int("rotation_segments"), Wall::Linear)
        }
        "forge.cone"
            if up(&p) && !p.bool("slice") && !p.bool("fillet_top") && !p.bool("fillet_bottom") =>
        {
            revolve(mesh, p.int("rotation_segments"), Wall::Linear)
        }
        "forge.tube" if up(&p) && !p.bool("slice") && !p.bool("fillet") => {
            revolve(mesh, p.int("rotation_segments"), Wall::Tube)
        }
        "forge.capsule" if up(&p) && !p.bool("slice") => {
            let profile = capsule_profile(&p);
            revolve(mesh, p.int("rotation_segments"), Wall::Capsule(profile))
        }
        "forge.torus" if up(&p) && !p.bool("slice") => torus(
            mesh,
            p.int("ring_segments"),
            p.int("pipe_segments"),
            p.float("ring_radius"),
        ),
        _ => None,
    }?;
    let uvs = CornerUvs { corners };
    uvs.matches(mesh).then_some(uvs)
}

type Corners = Vec<Vec<[f64; 2]>>;

fn bounds(mesh: &PolyMesh) -> (DVec3, DVec3) {
    let mut lo = DVec3::splat(f64::MAX);
    let mut hi = DVec3::splat(f64::MIN);
    for p in &mesh.points {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    (lo, hi)
}

fn norm(v: f64, lo: f64, hi: f64) -> f64 {
    if hi - lo > EPS {
        (v - lo) / (hi - lo)
    } else {
        0.0
    }
}

fn plane(mesh: &PolyMesh) -> Option<Corners> {
    let (lo, hi) = bounds(mesh);
    Some(
        mesh.polys
            .iter()
            .map(|poly| {
                poly.iter()
                    .map(|&i| {
                        let p = mesh.points[i as usize];
                        [norm(p.x, lo.x, hi.x), norm(p.z, lo.z, hi.z)]
                    })
                    .collect()
            })
            .collect(),
    )
}

/// Planar projection on x, z around the origin: `u = 0.5 + s x / (2 R)`, `v = 0.5 + z / (2 R)`.
fn planar(mesh: &PolyMesh, flip_u: f64) -> Option<Corners> {
    let r = mesh
        .points
        .iter()
        .map(|p| p.x.hypot(p.z))
        .fold(0.0, f64::max);
    if r < EPS {
        return None;
    }
    Some(
        mesh.polys
            .iter()
            .map(|poly| {
                poly.iter()
                    .map(|&i| {
                        let p = mesh.points[i as usize];
                        [0.5 + flip_u * p.x / (2.0 * r), 0.5 + p.z / (2.0 * r)]
                    })
                    .collect()
            })
            .collect(),
    )
}

fn axis_dir(v: DVec3) -> Option<[i8; 3]> {
    let a = v.abs();
    let m = a.max_element();
    if m < EPS || a.to_array().iter().filter(|c| **c > m * 1e-6).count() != 1 {
        return None;
    }
    Some(v.to_array().map(|c| {
        if c.abs() > m * 1e-6 {
            c.signum() as i8
        } else {
            0
        }
    }))
}

fn cube(mesh: &PolyMesh) -> Option<Corners> {
    // face key = (e_u, e_v) of the polygon's corner order
    let mut axes = Vec::new();
    for poly in &mesh.polys {
        if poly.len() != 4 {
            return None;
        }
        let p = |c: usize| mesh.points[poly[c] as usize];
        axes.push((axis_dir(p(3) - p(0))?, axis_dir(p(1) - p(0))?));
    }
    let mut ext: BTreeMap<([i8; 3], [i8; 3]), [f64; 4]> = BTreeMap::new();
    let proj = |dir: [i8; 3], p: DVec3| -> f64 {
        p.x * f64::from(dir[0]) + p.y * f64::from(dir[1]) + p.z * f64::from(dir[2])
    };
    for (poly, key) in mesh.polys.iter().zip(&axes) {
        let e = ext
            .entry(*key)
            .or_insert([f64::MAX, f64::MIN, f64::MAX, f64::MIN]);
        for &i in poly {
            let p = mesh.points[i as usize];
            e[0] = e[0].min(proj(key.0, p));
            e[1] = e[1].max(proj(key.0, p));
            e[2] = e[2].min(proj(key.1, p));
            e[3] = e[3].max(proj(key.1, p));
        }
    }
    Some(
        mesh.polys
            .iter()
            .zip(&axes)
            .map(|(poly, key)| {
                let e = ext[key];
                poly.iter()
                    .map(|&i| {
                        let p = mesh.points[i as usize];
                        [
                            norm(proj(key.0, p), e[0], e[1]),
                            norm(proj(key.1, p), e[2], e[3]),
                        ]
                    })
                    .collect()
            })
            .collect(),
    )
}

enum Wall {
    Sphere,
    Linear,
    Tube,
    /// The meridian as (y, cumulative v) rings from the south pole.
    Capsule(Vec<(f64, f64)>),
}

/// Capsule rings (y, v) with v the cumulative chord length over the total.
fn capsule_profile(p: &Params) -> Vec<(f64, f64)> {
    let (r, h) = (p.float("radius"), p.float("height"));
    let (cs, hs) = (p.int("cap_segments"), p.int("height_segments"));
    let mid = (h - 2.0 * r).max(0.0);
    let mut pts: Vec<(f64, f64)> = Vec::new(); // (rho, y)
    for k in 0..=cs {
        let t = PI / 2.0 * k as f64 / cs as f64;
        pts.push((r * t.sin(), -mid / 2.0 - r * t.cos()));
    }
    for j in 1..=hs {
        pts.push((r, -mid / 2.0 + mid * j as f64 / hs as f64));
    }
    for k in (0..cs).rev() {
        let t = PI / 2.0 * k as f64 / cs as f64;
        pts.push((r * t.sin(), mid / 2.0 + r * t.cos()));
    }
    let mut cum = vec![0.0];
    for w in pts.windows(2) {
        let d = (w[1].0 - w[0].0).hypot(w[1].1 - w[0].1);
        cum.push(cum[cum.len() - 1] + d);
    }
    let total = cum[cum.len() - 1];
    pts.iter()
        .zip(&cum)
        .map(|(p, c)| (p.1, c / total))
        .collect()
}

fn angle(x: f64, z: f64) -> f64 {
    let a = z.atan2(x);
    if a < 0.0 { a + TAU } else { a }
}

fn wrap(d: f64) -> f64 {
    let d = d % TAU;
    if d > PI {
        d - TAU
    } else if d < -PI {
        d + TAU
    } else {
        d
    }
}

fn revolve(mesh: &PolyMesh, n: usize, wall: Wall) -> Option<Corners> {
    let (lo, hi) = bounds(mesh);
    let rmax = mesh
        .points
        .iter()
        .map(|p| p.x.hypot(p.z))
        .fold(0.0, f64::max);
    let rmin_wall = mesh
        .points
        .iter()
        .map(|p| p.x.hypot(p.z))
        .filter(|r| *r > EPS)
        .fold(f64::MAX, f64::min);
    let step = TAU / n as f64;
    let rho = |i: u32| {
        let p = mesh.points[i as usize];
        p.x.hypot(p.z)
    };
    let mut out = Vec::with_capacity(mesh.polys.len());
    for poly in &mesh.polys {
        let pts: Vec<DVec3> = poly.iter().map(|&i| mesh.points[i as usize]).collect();
        // planar caps: every corner at one end height
        let flat_y = pts.iter().all(|p| (p.y - pts[0].y).abs() < EPS);
        if flat_y && !matches!(wall, Wall::Sphere | Wall::Capsule(_)) {
            let bottom = (pts[0].y - lo.y).abs() < EPS;
            let top = (pts[0].y - hi.y).abs() < EPS;
            if !(bottom || top) {
                return None;
            }
            let s = if bottom { -1.0 } else { 1.0 };
            out.push(
                pts.iter()
                    .map(|p| [0.5 + s * p.x / (2.0 * rmax), 0.5 + p.z / (2.0 * rmax)])
                    .collect(),
            );
            continue;
        }
        let (sx, sz) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.z));
        let c = angle(sx, sz);
        let sector = ((c / step).floor() as usize).min(n - 1);
        let (u0, u1) = (sector as f64 / n as f64, (sector + 1) as f64 / n as f64);
        let inner =
            matches!(wall, Wall::Tube) && poly.iter().all(|&i| (rho(i) - rmin_wall).abs() < EPS);
        let v_of = |p: DVec3| -> f64 {
            match &wall {
                Wall::Sphere => p.x.hypot(p.z).atan2(-p.y) / PI,
                Wall::Linear | Wall::Tube => {
                    if inner {
                        norm(hi.y - p.y, 0.0, hi.y - lo.y)
                    } else {
                        norm(p.y, lo.y, hi.y)
                    }
                }
                Wall::Capsule(rings) => rings
                    .iter()
                    .min_by(|a, b| (a.0 - p.y).abs().total_cmp(&(b.0 - p.y).abs()))
                    .map_or(0.0, |r| r.1),
            }
        };
        let uv: Vec<[f64; 2]> = if poly.len() == 4 {
            let us = [u0, u0, u1, u1];
            (0..4).map(|k| [us[k], v_of(pts[k])]).collect()
        } else if poly.len() == 3 {
            // a pole triangle: the corner on the axis
            let pole = pts.iter().position(|p| p.x.hypot(p.z) < EPS)?;
            let side = |p: DVec3| {
                if wrap(angle(p.x, p.z) - c) < 0.0 {
                    u0
                } else {
                    u1
                }
            };
            let pole_u = match wall {
                Wall::Capsule(_) => (u0 + u1) / 2.0,
                _ => u0,
            };
            pts.iter()
                .enumerate()
                .map(|(k, p)| {
                    if k == pole {
                        [pole_u, v_of(*p)]
                    } else {
                        [side(*p), v_of(*p)]
                    }
                })
                .collect()
        } else {
            return None;
        };
        out.push(uv);
    }
    Some(out)
}

fn torus(mesh: &PolyMesh, ring: usize, pipe: usize, big_r: f64) -> Option<Corners> {
    let (ds, dp) = (TAU / ring as f64, TAU / pipe as f64);
    let mut out = Vec::with_capacity(mesh.polys.len());
    for poly in &mesh.polys {
        if poly.len() != 4 {
            return None;
        }
        let pts: Vec<DVec3> = poly.iter().map(|&i| mesh.points[i as usize]).collect();
        let (sx, sz) = pts.iter().fold((0.0, 0.0), |a, p| (a.0 + p.x, a.1 + p.z));
        let i = ((angle(sx, sz) / ds).floor() as usize).min(ring - 1);
        let (cr, cy) = pts
            .iter()
            .fold((0.0, 0.0), |a, p| (a.0 + p.x.hypot(p.z) - big_r, a.1 + p.y));
        let j = ((angle(cr, cy) / dp).floor() as usize).min(pipe - 1);
        let (u0, u1) = (i as f64 / ring as f64, (i + 1) as f64 / ring as f64);
        let (v0, v1) = (j as f64 / pipe as f64, (j + 1) as f64 / pipe as f64);
        out.push(vec![[u0, v0], [u0, v1], [u1, v1], [u1, v0]]);
    }
    Some(out)
}
