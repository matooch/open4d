//! Shared geometry for the helper-driven deformers (docs/specs/MODIFIERS.md §14).

use forge_core::{NodeId, Scene, Value};
use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate_poly;
use glam::{DMat4, DVec3};

/// Evaluates a linked helper object to its local-space mesh (with its own deformers).
pub type Resolver<'a> = &'a dyn Fn(NodeId) -> Option<PolyMesh>;

/// The triangles of a mesh: every polygon triangulated, in polygon order.
pub(crate) fn triangles(mesh: &PolyMesh) -> Vec<[u32; 3]> {
    mesh.polys
        .iter()
        .filter(|p| p.len() >= 3)
        .flat_map(|p| triangulate_poly(&mesh.points, p))
        .collect()
}

/// The points of `mesh` taken to world space with the world matrix of `node`.
pub(crate) fn world_points(scene: &Scene, node: NodeId, mesh: &PolyMesh) -> Option<Vec<DVec3>> {
    let w = scene.world_matrix(node)?;
    Some(mesh.points.iter().map(|p| w.transform_point3(*p)).collect())
}

/// Edge neighbours of every point: sorted, unique.
pub(crate) fn neighbours(mesh: &PolyMesh) -> Vec<Vec<u32>> {
    let mut n: Vec<Vec<u32>> = vec![Vec::new(); mesh.points.len()];
    for poly in &mesh.polys {
        for (i, &a) in poly.iter().enumerate() {
            let b = poly[(i + 1) % poly.len()];
            if a == b || a as usize >= n.len() || b as usize >= n.len() {
                continue;
            }
            n[a as usize].push(b);
            n[b as usize].push(a);
        }
    }
    for l in &mut n {
        l.sort_unstable();
        l.dedup();
    }
    n
}

/// A `Vec3List` param as points (empty when missing).
pub(crate) fn vec3_list(scene: &Scene, node: NodeId, id: &str) -> Vec<DVec3> {
    match scene.param(node, id) {
        Some(Value::Vec3List(l)) => l.iter().map(|v| DVec3::from_array(*v)).collect(),
        _ => Vec::new(),
    }
}

/// A `Link` param.
pub(crate) fn link(scene: &Scene, node: NodeId, id: &str) -> Option<NodeId> {
    match scene.param(node, id) {
        Some(Value::Link(l)) => *l,
        _ => None,
    }
}

/// A `LinkList` param.
pub(crate) fn link_list(scene: &Scene, node: NodeId, id: &str) -> Vec<NodeId> {
    match scene.param(node, id) {
        Some(Value::LinkList(l)) => l.clone(),
        _ => Vec::new(),
    }
}

/// Closest point of triangle `abc` to `p` and its barycentric coordinates (Ericson,
/// Real-Time Collision Detection 5.1.5).
pub(crate) fn closest_on_triangle(p: DVec3, a: DVec3, b: DVec3, c: DVec3) -> (DVec3, [f64; 3]) {
    let (ab, ac, ap) = (b - a, c - a, p - a);
    let (d1, d2) = (ab.dot(ap), ac.dot(ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }
    let bp = p - b;
    let (d3, d4) = (ab.dot(bp), ac.dot(bp));
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return (a + ab * v, [1.0 - v, v, 0.0]);
    }
    let cp = p - c;
    let (d5, d6) = (ab.dot(cp), ac.dot(cp));
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return (a + ac * w, [1.0 - w, 0.0, w]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return (b + (c - b) * w, [0.0, 1.0 - w, w]);
    }
    let denom = 1.0 / (va + vb + vc);
    let (v, w) = (vb * denom, vc * denom);
    (a + ab * v + ac * w, [1.0 - v - w, v, w])
}

/// `m` applied to a point, `None` when `m` is not invertible.
pub(crate) fn inverse(m: DMat4) -> Option<DMat4> {
    let i = m.inverse();
    i.is_finite().then_some(i)
}
