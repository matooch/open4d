//! Atom Array (docs/specs/GENERATORS3.md §1; docs/parity/generators-3.md, `atom-array/*`).

use std::collections::{BTreeMap, BTreeSet};

use forge_core::{Category, NodeType, ParamDesc, Unit, Value};
use forge_geometry::PolyMesh;
use glam::DVec3;

use crate::{BuildError, Params, sphere};

pub const TYPE: &str = "forge.atom_array";

pub(crate) fn node_type() -> NodeType {
    let radius = |id: &str, label: &str, default: f64| {
        ParamDesc::float(id, label, default)
            .range(0.0, f64::MAX)
            .unit(Unit::Distance)
    };
    NodeType::new(TYPE, "Atom Array", Category::Generator)
        .param(radius("cylinder_radius", "Cylinder Radius", 2.0))
        .param(radius("sphere_radius", "Sphere Radius", 5.0))
        .param(ParamDesc::int("subdivisions", "Subdivisions", 8).range(3, 1000))
        .param(ParamDesc::bool("single_elements", "Single Elements", false))
}

/// The unique edges of the polygons in order of first appearance, directed as first seen.
fn edges(mesh: &PolyMesh) -> Vec<(u32, u32)> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for poly in &mesh.polys {
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            if a != b && seen.insert((a.min(b), a.max(b))) {
                out.push((a, b));
            }
        }
    }
    out
}

/// The ring offsets of a cylinder along `d` (§1 rule 4).
fn ring(d: DVec3, r: f64, n: usize) -> Vec<DVec3> {
    let reference = if d.x.abs() > 1.0 - 1e-9 {
        DVec3::Y
    } else {
        DVec3::X
    };
    let u = reference.cross(d).normalize();
    let w = u.cross(d);
    (0..n)
        .map(|k| {
            let t = std::f64::consts::TAU * k as f64 / n as f64;
            r * (u * t.cos() + w * t.sin())
        })
        .collect()
}

/// A sphere at every point, then a cylinder along every edge of `input`.
pub fn atom_array(
    input: &PolyMesh,
    values: &BTreeMap<String, Value>,
) -> Result<PolyMesh, BuildError> {
    let ty = node_type();
    let p = Params { ty: &ty, values };
    if p.bool("single_elements") {
        return Err(BuildError::Unsupported("atom array single elements"));
    }
    let n = p.int("subdivisions");
    let sphere_ty = sphere::node_type();
    let sphere_values = BTreeMap::from([
        ("radius".to_string(), Value::Float(p.float("sphere_radius"))),
        ("segments".to_string(), Value::Int(n as i64)),
    ]);
    let ball = sphere::build(&Params {
        ty: &sphere_ty,
        values: &sphere_values,
    })?;
    let mut out = PolyMesh::default();
    let mut push = |points: Vec<DVec3>, polys: Vec<Vec<u32>>| {
        let base = out.points.len() as u32;
        out.points.extend(points);
        out.polys.extend(
            polys
                .into_iter()
                .map(|f| f.iter().map(|i| i + base).collect()),
        );
    };
    for c in &input.points {
        push(
            ball.points.iter().map(|q| *q + *c).collect(),
            ball.polys.clone(),
        );
    }
    let r = p.float("cylinder_radius");
    for (a, b) in edges(input) {
        let (pa, pb) = (input.points[a as usize], input.points[b as usize]);
        let d = (pb - pa).normalize_or_zero();
        if d == DVec3::ZERO {
            continue;
        }
        let points = ring(d, r, n)
            .into_iter()
            .flat_map(|o| [pa + o, pb + o])
            .collect();
        let polys = (0..n)
            .map(|k| {
                let (s, t) = (2 * k as u32, 2 * ((k + 1) % n) as u32);
                vec![s, s + 1, t + 1, t]
            })
            .collect();
        push(points, polys);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> PolyMesh {
        PolyMesh {
            points: vec![DVec3::ZERO, DVec3::X * 10.0, DVec3::Y * 10.0],
            polys: vec![vec![0, 1, 2]],
        }
    }

    #[test]
    fn spheres_then_cylinders_with_the_edge_order_of_the_polygons() {
        let out = atom_array(&triangle(), &BTreeMap::new()).unwrap();
        // 3 spheres of 26 points / 32 faces, 3 cylinders of 16 points / 8 faces
        assert_eq!(out.points.len(), 3 * 26 + 3 * 16);
        assert_eq!(out.polys.len(), 3 * 32 + 3 * 8);
        // the first cylinder runs from point 0 to point 1
        let first = out.points[3 * 26];
        assert!((first.length() - 2.0).abs() < 1e-9);
        assert!(
            out.polys
                .iter()
                .flatten()
                .all(|i| (*i as usize) < out.points.len())
        );
    }

    #[test]
    fn non_axis_edges_and_single_elements() {
        let mesh = PolyMesh {
            points: vec![DVec3::ZERO, DVec3::new(1.0, 2.0, 3.0)],
            polys: vec![vec![0, 1]],
        };
        let out = atom_array(&mesh, &BTreeMap::new()).unwrap();
        let d = DVec3::new(1.0, 2.0, 3.0).normalize();
        // every cylinder point is 2 from the axis
        for q in &out.points[2 * 26..] {
            let along = q.dot(d);
            let radial = (*q - d * along).length();
            assert!((radial - 2.0).abs() < 1e-9, "{radial}");
        }
        let on = BTreeMap::from([("single_elements".to_string(), Value::Bool(true))]);
        assert!(matches!(
            atom_array(&mesh, &on),
            Err(BuildError::Unsupported(_))
        ));
    }
}
