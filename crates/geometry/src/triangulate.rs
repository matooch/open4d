//! Ear-clipping polygon triangulation (docs/specs/GEOMETRY.md §6).
//! Ref: Eberly, "Triangulation by Ear Clipping", Geometric Tools (2002).

use glam::{DVec2, DVec3};

use crate::PolyMesh;
use crate::normals::poly_normal;

fn cross2(a: DVec2, b: DVec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn fan(poly: &[u32]) -> Vec<[u32; 3]> {
    (1..poly.len() - 1)
        .map(|i| [poly[0], poly[i], poly[i + 1]])
        .collect()
}

/// Right-handed 2D frame `(u, v)` of the plane with normal `n`, shared by the ear clipper
/// and the Delaunay refinement.
fn plane_frame(n: DVec3) -> (DVec3, DVec3) {
    let u = n.any_orthogonal_vector().normalize();
    (u, n.cross(u))
}

fn project(points: &[DVec3], poly: &[u32], n: DVec3) -> Vec<DVec2> {
    let (u, v) = plane_frame(n);
    poly.iter()
        .map(|&i| {
            let p = points[i as usize];
            DVec2::new(p.dot(u), p.dot(v))
        })
        .collect()
}

/// Ear clipping on a counter-clockwise 2D polygon: triangles of corner positions, each
/// counter-clockwise (GEOMETRY.md §6).
fn ear_clip(p2: &[DVec2]) -> Vec<[usize; 3]> {
    let (lo, hi) = p2
        .iter()
        .fold((p2[0], p2[0]), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    let eps = 1e-12 * (hi - lo).length_squared().max(1e-300);

    let mut rem: Vec<usize> = (0..p2.len()).collect();
    let mut out = Vec::with_capacity(p2.len() - 2);
    while rem.len() > 3 {
        let m = rem.len();
        let ear = (0..m).find(|&i| {
            let (a, b, c) = (rem[(i + m - 1) % m], rem[i], rem[(i + 1) % m]);
            let (pa, pb, pc) = (p2[a], p2[b], p2[c]);
            if cross2(pb - pa, pc - pb) <= eps {
                return false; // reflex or collinear
            }
            !rem.iter().any(|&o| {
                o != a
                    && o != b
                    && o != c
                    && cross2(pb - pa, p2[o] - pa) >= -eps
                    && cross2(pc - pb, p2[o] - pb) >= -eps
                    && cross2(pa - pc, p2[o] - pc) >= -eps
            })
        });
        let Some(i) = ear else {
            out.extend(fan_positions(&rem));
            return out;
        };
        out.push([rem[(i + m - 1) % m], rem[i], rem[(i + 1) % m]]);
        rem.remove(i);
    }
    out.push([rem[0], rem[1], rem[2]]);
    out
}

fn fan_positions(rem: &[usize]) -> Vec<[usize; 3]> {
    (1..rem.len() - 1)
        .map(|i| [rem[0], rem[i], rem[i + 1]])
        .collect()
}

/// `k − 2` triangles of mesh vertex indices, wound like `poly`.
pub fn triangulate_poly(points: &[DVec3], poly: &[u32]) -> Vec<[u32; 3]> {
    if poly.len() == 3 {
        return vec![[poly[0], poly[1], poly[2]]];
    }
    let n = poly_normal(points, poly);
    if n == DVec3::ZERO {
        return fan(poly);
    }
    // Right-handed 2D frame (u, v, n): the polygon is counter-clockwise in it.
    let p2 = project(points, poly, n);
    ear_clip(&p2)
        .into_iter()
        .map(|t| t.map(|k| poly[k]))
        .collect()
}

fn in_circle(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> f64 {
    let (a, b, c) = (a - d, b - d, c - d);
    a.length_squared() * cross2(b, c) - b.length_squared() * cross2(a, c)
        + c.length_squared() * cross2(a, b)
}

/// Relative tolerance of the in-circle test (GEOMETRY.md §6.1 R1).
const DELAUNAY_EPS: f64 = 1e-9;

/// Lawson flips on counter-clockwise triangles of corner positions until every interior
/// edge of a convex quad is locally Delaunay (up to [`DELAUNAY_EPS`]).
fn delaunay_flips(p2: &[DVec2], tris: &mut [[usize; 3]]) {
    let limit = 8 * p2.len() * p2.len() + 16;
    for _ in 0..limit {
        let Some((i, j, flipped)) = find_flip(p2, tris) else {
            return;
        };
        tris[i] = flipped[0];
        tris[j] = flipped[1];
    }
}

type Flip = (usize, usize, [[usize; 3]; 2]);

fn find_flip(p2: &[DVec2], tris: &[[usize; 3]]) -> Option<Flip> {
    for (i, t) in tris.iter().enumerate() {
        for k in 0..3 {
            let (a, b, c) = (t[k], t[(k + 1) % 3], t[(k + 2) % 3]);
            // the neighbour across a -> b holds b -> a
            let Some((j, d)) = tris.iter().enumerate().find_map(|(j, o)| {
                (0..3)
                    .find_map(|m| (o[m] == b && o[(m + 1) % 3] == a).then_some((j, o[(m + 2) % 3])))
            }) else {
                continue;
            };
            let (pa, pb, pc, pd) = (p2[a], p2[b], p2[c], p2[d]);
            let scale = [pa - pb, pb - pc, pc - pa, pa - pd, pb - pd]
                .iter()
                .map(|e| e.length())
                .fold(0.0, f64::max);
            let tol = 1e-12 * scale * scale;
            // the quad a, d, b, c must be strictly convex at a and at b
            if cross2(pa - pc, pd - pa) <= tol || cross2(pb - pd, pc - pb) <= tol {
                continue;
            }
            if in_circle(pa, pb, pc, pd) > DELAUNAY_EPS * scale.powi(4) {
                return Some((i, j, [[c, a, d], [c, d, b]]));
            }
        }
    }
    None
}

/// Constrained Delaunay triangulation of `poly` in the order the reference exports it
/// (GEOMETRY.md §6.1 R1, R2): `k − 2` triangles of mesh vertex indices wound like `poly`,
/// each written from its lowest corner, listed by [`order_triangles`]. The corners are
/// taken in walk order, so a caller wanting the reference's start cap passes the ring as
/// wound and an end cap passes the reversed ring.
pub fn triangulate_delaunay(points: &[DVec3], poly: &[u32]) -> Vec<[u32; 3]> {
    if poly.len() <= 3 {
        return triangulate_poly(points, poly);
    }
    let n = poly_normal(points, poly);
    if n == DVec3::ZERO {
        return fan(poly);
    }
    let p2 = project(points, poly, n);
    let mut tris = ear_clip(&p2);
    delaunay_flips(&p2, &mut tris);
    let tris: Vec<[u32; 3]> = tris.into_iter().map(|t| t.map(|k| poly[k])).collect();
    order_triangles(points, poly, &tris)
}

/// Direction "down" of GEOMETRY.md §6.1 R2: world `-Y` projected into the plane with
/// normal `n`, or `-Z` when the normal is parallel to `Y`.
fn down_in_plane(n: DVec3) -> DVec3 {
    for axis in [DVec3::NEG_Y, DVec3::NEG_Z] {
        let d = axis - n * axis.dot(n);
        if d.length() > 1e-6 {
            return d.normalize();
        }
    }
    n.any_orthogonal_vector().normalize()
}

/// The reference's listing of a triangle set (GEOMETRY.md §6.1 R2): every triangle of `tris`
/// (mesh vertex indices, wound like `poly`) rotated to start at its corner that comes first
/// in `poly`, listed by that corner and then by the counter-clockwise angle of its next
/// corner around it, measured from "down" over `[0, 2 pi)`.
pub fn order_triangles(points: &[DVec3], poly: &[u32], tris: &[[u32; 3]]) -> Vec<[u32; 3]> {
    let position = |v: u32| poly.iter().position(|&c| c == v).unwrap_or(usize::MAX);
    let n = poly_normal(points, poly);
    let (down, side) = if n == DVec3::ZERO {
        (DVec3::X, DVec3::Y)
    } else {
        let d = down_in_plane(n);
        (d, n.cross(d))
    };
    let mut keyed: Vec<(usize, f64, [u32; 3])> = tris
        .iter()
        .map(|t| {
            let first = (0..3).min_by_key(|&k| position(t[k])).unwrap_or(0);
            let r = [t[first], t[(first + 1) % 3], t[(first + 2) % 3]];
            let w = points[r[1] as usize] - points[r[0] as usize];
            let mut angle = w.dot(side).atan2(w.dot(down));
            if angle < 0.0 {
                angle += std::f64::consts::TAU;
            }
            if angle > std::f64::consts::TAU - 1e-9 {
                angle = 0.0;
            }
            (position(r[0]), angle, r)
        })
        .collect();
    keyed.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
    keyed.into_iter().map(|k| k.2).collect()
}

/// The reference's split of a pentagon into a triangle and a quad (GEOMETRY.md §6.1 R3):
/// `ring` holds the corners in face winding, `tip` the position of the ear. Returns the
/// triangle `[next, previous, tip]` and the quad `[previous, next, next + 1, next + 2]`.
pub fn split_pentagon(ring: &[u32; 5], tip: usize) -> ([u32; 3], [u32; 4]) {
    let at = |k: usize| ring[(tip + k) % 5];
    ([at(1), at(4), at(0)], [at(4), at(1), at(2), at(3)])
}

/// `(poly index, triangle)` for every polygon, in order.
pub fn triangulate(mesh: &PolyMesh) -> Vec<(u32, [u32; 3])> {
    mesh.polys
        .iter()
        .enumerate()
        .flat_map(|(f, poly)| {
            triangulate_poly(&mesh.points, poly)
                .into_iter()
                .map(move |t| (f as u32, t))
        })
        .collect()
}
