//! Polygon Reduction (docs/specs/MODIFIERS.md §22.3): quadric edge collapse.

use std::collections::BTreeSet;

use forge_geometry::PolyMesh;
use forge_geometry::normals::poly_normal;
use glam::DVec3;

pub struct ReductionParams {
    /// Fraction (the percent param is stored as 0..1).
    pub strength: f64,
    pub quality: f64,
    pub co_planar: bool,
    pub boundary: bool,
    pub polygon_quality: bool,
}

impl Default for ReductionParams {
    fn default() -> Self {
        Self {
            strength: 0.9,
            quality: 1000.0,
            co_planar: true,
            boundary: true,
            polygon_quality: false,
        }
    }
}

/// Ten coefficients of a symmetric 4x4 quadric (a, b, c, d, e, f, g, h, i, j) for
/// `ax² + by² + cz² + 2dxy + 2exz + 2fyz + 2gx + 2hy + 2iz + j`.
#[derive(Clone, Copy, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    fn plane(n: DVec3, p: DVec3, weight: f64) -> Quadric {
        let d = -n.dot(p);
        let (a, b, c) = (n.x, n.y, n.z);
        Quadric([
            a * a * weight,
            b * b * weight,
            c * c * weight,
            a * b * weight,
            a * c * weight,
            b * c * weight,
            a * d * weight,
            b * d * weight,
            c * d * weight,
            d * d * weight,
        ])
    }

    fn add(self, o: Quadric) -> Quadric {
        let mut q = self.0;
        for (a, b) in q.iter_mut().zip(o.0) {
            *a += b;
        }
        Quadric(q)
    }

    fn eval(self, p: DVec3) -> f64 {
        let q = self.0;
        q[0] * p.x * p.x
            + q[1] * p.y * p.y
            + q[2] * p.z * p.z
            + 2.0 * q[3] * p.x * p.y
            + 2.0 * q[4] * p.x * p.z
            + 2.0 * q[5] * p.y * p.z
            + 2.0 * q[6] * p.x
            + 2.0 * q[7] * p.y
            + 2.0 * q[8] * p.z
            + q[9]
    }
}

fn clean(poly: &mut Vec<u32>) {
    poly.dedup();
    if poly.len() > 1 && poly.first() == poly.last() {
        poly.pop();
    }
}

/// Collapses the cheapest edge (quadric error at the midpoint) until the face target is
/// reached. Faces become triangles when a collapse leaves three corners.
pub fn reduce(mesh: &mut PolyMesh, p: &ReductionParams) {
    let target = (((1.0 - p.strength.clamp(0.0, 1.0)).powi(2) * mesh.polys.len() as f64).round()
        as usize)
        .max(4);
    let mut quad = vec![Quadric::default(); mesh.points.len()];
    for poly in &mesh.polys {
        if poly.len() < 3 {
            continue;
        }
        let n = poly_normal(&mesh.points, poly).normalize_or_zero();
        if n.length_squared() == 0.0 {
            continue;
        }
        let area = area(&mesh.points, poly);
        let q = Quadric::plane(n, mesh.points[poly[0] as usize], area.max(1e-9));
        for v in poly {
            quad[*v as usize] = quad[*v as usize].add(q);
        }
    }
    let mut guard = 0;
    while mesh.polys.len() > target && guard < mesh.points.len() * 4 {
        guard += 1;
        let edges = mesh.edges();
        let mut best: Option<(f64, u32, u32)> = None;
        for e in &edges {
            let (a, b) = (e.0, e.1);
            if !collapsible(mesh, a, b) {
                continue;
            }
            let mid = (mesh.points[a as usize] + mesh.points[b as usize]) * 0.5;
            let cost = quad[a as usize].add(quad[b as usize]).eval(mid);
            if best.is_none_or(|(c, _, _)| cost < c) {
                best = Some((cost, a, b));
            }
        }
        let Some((_, a, b)) = best else {
            break;
        };
        collapse(mesh, a, b);
        quad[a as usize] = quad[a as usize].add(quad[b as usize]);
    }
    compact(mesh);
}

fn area(points: &[DVec3], poly: &[u32]) -> f64 {
    let p0 = points[poly[0] as usize];
    (1..poly.len() - 1)
        .map(|k| {
            let p1 = points[poly[k] as usize];
            let p2 = points[poly[k + 1] as usize];
            (p1 - p0).cross(p2 - p0).length() * 0.5
        })
        .sum()
}

fn faces_with(mesh: &PolyMesh, a: u32, b: u32) -> usize {
    mesh.polys
        .iter()
        .filter(|f| f.contains(&a) && f.contains(&b))
        .count()
}

/// An edge can collapse when exactly two faces share it (a manifold interior edge) and its
/// endpoints have exactly two common neighbours (the link condition).
fn collapsible(mesh: &PolyMesh, a: u32, b: u32) -> bool {
    if a == b || faces_with(mesh, a, b) != 2 {
        return false;
    }
    let mut neighbours_a = BTreeSet::new();
    let mut neighbours_b = BTreeSet::new();
    for f in &mesh.polys {
        if f.contains(&a) {
            neighbours_a.extend(f.iter().copied());
        }
        if f.contains(&b) {
            neighbours_b.extend(f.iter().copied());
        }
    }
    neighbours_a.remove(&a);
    neighbours_a.remove(&b);
    neighbours_b.remove(&a);
    neighbours_b.remove(&b);
    neighbours_a.intersection(&neighbours_b).count() == 2
}

fn collapse(mesh: &mut PolyMesh, a: u32, b: u32) {
    mesh.points[a as usize] = (mesh.points[a as usize] + mesh.points[b as usize]) * 0.5;
    for f in &mut mesh.polys {
        for v in f.iter_mut() {
            if *v == b {
                *v = a;
            }
        }
        clean(f);
    }
    mesh.polys.retain(|f| f.len() >= 3);
}

fn compact(mesh: &mut PolyMesh) {
    let mut used = BTreeSet::new();
    for f in &mesh.polys {
        used.extend(f.iter().copied());
    }
    let mut remap = vec![u32::MAX; mesh.points.len()];
    let mut points = Vec::new();
    for old in used {
        remap[old as usize] = points.len() as u32;
        points.push(mesh.points[old as usize]);
    }
    for f in &mut mesh.polys {
        for v in f.iter_mut() {
            *v = remap[*v as usize];
        }
    }
    mesh.points = points;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_sphere_gets_smaller_and_stays_valid() {
        // A small octahedron stands in for a sphere.
        let mut mesh = PolyMesh {
            points: vec![
                DVec3::X * 100.0,
                -DVec3::X * 100.0,
                DVec3::Y * 100.0,
                -DVec3::Y * 100.0,
                DVec3::Z * 100.0,
                -DVec3::Z * 100.0,
            ],
            polys: vec![
                vec![0, 2, 4],
                vec![2, 1, 4],
                vec![1, 3, 4],
                vec![3, 0, 4],
                vec![2, 0, 5],
                vec![1, 2, 5],
                vec![3, 1, 5],
                vec![0, 3, 5],
            ],
        };
        let before = mesh.polys.len();
        reduce(
            &mut mesh,
            &ReductionParams {
                strength: 0.5,
                ..Default::default()
            },
        );
        assert!(mesh.polys.len() < before);
        assert!(mesh.polys.iter().all(|f| f.len() >= 3));
        assert!(mesh.points.iter().all(|p| p.is_finite()));
    }

    #[test]
    fn zero_strength_leaves_the_mesh_alone() {
        let mut mesh = PolyMesh {
            points: vec![DVec3::ZERO, DVec3::X, DVec3::Y, DVec3::Z],
            polys: vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![0, 2, 3]],
        };
        let before = mesh.clone();
        reduce(
            &mut mesh,
            &ReductionParams {
                strength: 0.0,
                ..Default::default()
            },
        );
        assert_eq!(mesh.polys, before.polys);
    }
}
