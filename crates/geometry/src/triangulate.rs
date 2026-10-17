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
    let u = n.any_orthogonal_vector().normalize();
    let v = n.cross(u);
    let p2: Vec<DVec2> = poly
        .iter()
        .map(|&i| {
            let p = points[i as usize];
            DVec2::new(p.dot(u), p.dot(v))
        })
        .collect();
    let (lo, hi) = p2
        .iter()
        .fold((p2[0], p2[0]), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
    let eps = 1e-12 * (hi - lo).length_squared().max(1e-300);

    let mut rem: Vec<usize> = (0..poly.len()).collect();
    let mut out = Vec::with_capacity(poly.len() - 2);
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
            let rest: Vec<u32> = rem.iter().map(|&k| poly[k]).collect();
            out.extend(fan(&rest));
            return out;
        };
        let (a, b, c) = (rem[(i + m - 1) % m], rem[i], rem[(i + 1) % m]);
        out.push([poly[a], poly[b], poly[c]]);
        rem.remove(i);
    }
    out.push([poly[rem[0]], poly[rem[1]], poly[rem[2]]]);
    out
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
