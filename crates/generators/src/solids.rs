//! The Platonic primitive (docs/specs/PRIMITIVES3.md §1; docs/parity/primitives-3.md).

use std::collections::BTreeMap;

use forge_core::{Category, NodeType, ParamDesc};
use forge_geometry::PolyMesh;
use forge_geometry::triangulate::split_pentagon;
use glam::{DQuat, DVec3};

use crate::platonic::{flat_subdivision, triangle_solid};
use crate::round::{count, distance};
use crate::{BuildError, Params};

pub(crate) fn node_type() -> NodeType {
    NodeType::new("forge.platonic", "Platonic", Category::Generator)
        .param(distance("radius", "Radius", 100.0))
        .param(count("segments", "Segments", 1, 1))
        .param(ParamDesc::enumeration(
            "type",
            "Type",
            "icosa",
            &[
                ("tetra", "Tetra"),
                ("hexa", "Hexa"),
                ("octa", "Octa"),
                ("dodeca", "Dodeca"),
                ("icosa", "Icosa"),
                ("bucky", "Bucky"),
            ],
        ))
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    let (r, n) = (p.float("radius"), p.int("segments"));
    match p.enumeration("type").as_str() {
        "tetra" => Ok(triangles("tetrahedron", r, n)),
        "octa" => Ok(triangles("octahedron", r, n)),
        "hexa" => Ok(hexa(r, n)),
        "dodeca" => Ok(cut_flat(dodeca(r), n)),
        "bucky" => Ok(cut_flat(bucky(r), n)),
        _ => Ok(triangles("icosahedron", r, n)),
    }
}

/// The rounded corner length the reference divides the icosahedron's corners by.
const ICOSA_LENGTH: f64 = 1.90211;

/// Tetra, octa or icosa: the sphere types' solid, cut flat (P1, P2).
fn triangles(kind: &str, r: f64, n: usize) -> PolyMesh {
    let (base, faces) = triangle_solid(kind);
    // The reference normalises the icosahedron's (0, ±1, ±φ) corners by the rounded
    // length 1.90211 instead of √(1+φ²), so its radius is 100.00016 (PRIMITIVES3.md P1).
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let fix = if kind == "icosahedron" {
        (1.0 + phi * phi).sqrt() / ICOSA_LENGTH
    } else {
        1.0
    };
    let base: Vec<DVec3> = base.iter().map(|&q| q * r * fix).collect();
    let faces: Vec<Vec<u32>> = faces.iter().map(|f| f.to_vec()).collect();
    flat_subdivision(&base, &faces, n, |q| q)
}

/// The cube with circumradius `r` (P1, P2): the Cube primitive's point and face order with
/// edge length `2r/√3` and `n` segments per axis (`platonic/hexa-seg3`, `type-hexa`).
fn hexa(r: f64, n: usize) -> PolyMesh {
    use forge_core::Value;
    let edge = 2.0 * r / 3f64.sqrt();
    let ty = crate::cube::node_type();
    let mut values = BTreeMap::new();
    for axis in ["x", "y", "z"] {
        values.insert(format!("size_{axis}"), Value::Float(edge));
        values.insert(format!("segments_{axis}"), Value::Int(n as i64));
    }
    for q in &ty.params {
        values
            .entry(q.id.clone())
            .or_insert_with(|| q.default.clone());
    }
    crate::cube::build(&Params {
        ty: &ty,
        values: &values,
    })
    .expect("a plain cube always builds")
}

/// The exported triangles and quads of a dodeca/bucky cut flat at `n` (P2): every face is
/// cut as a triangle or quad (`platonic/dodeca-seg2`: 74 points, 48 + 48 faces).
fn cut_flat(mesh: PolyMesh, n: usize) -> PolyMesh {
    if n == 1 {
        return mesh;
    }
    flat_subdivision(&mesh.points, &mesh.polys, n, |q| q)
}

/// Pentagon rings of the dodecahedron, ear tip first, in face winding: face `i` of the
/// export has its ear at point `i` (GEOMETRY.md §6.1 R3; PRIMITIVES3.md P3).
const DODECA_RINGS: [[u32; 5]; 12] = [
    [0, 19, 8, 9, 16],
    [1, 12, 10, 11, 15],
    [2, 12, 1, 0, 16],
    [3, 17, 4, 5, 13],
    [4, 17, 9, 8, 18],
    [5, 14, 11, 10, 13],
    [6, 14, 5, 4, 18],
    [7, 19, 0, 1, 15],
    [8, 19, 7, 6, 18],
    [9, 17, 3, 2, 16],
    [10, 12, 2, 3, 13],
    [11, 14, 6, 7, 15],
];

/// Splits pentagon rings into the triangles (in ring order) followed by the quads, sorted
/// by the lower index of their two inner points, the quad whose inner points descend first
/// (`platonic/type-dodeca`: quads 12..24 pair up as (1,0), (0,1), (3,2), (2,3), …).
fn split_rings(rings: &[[u32; 5]]) -> Vec<Vec<u32>> {
    let split: Vec<([u32; 3], [u32; 4])> = rings.iter().map(|r| split_pentagon(r, 0)).collect();
    let mut quads: Vec<[u32; 4]> = split.iter().map(|s| s.1).collect();
    quads.sort_by_key(|q| (q[2].min(q[3]), q[2] < q[3]));
    split
        .iter()
        .map(|s| s.0.to_vec())
        .chain(quads.iter().map(|q| q.to_vec()))
        .collect()
}

fn dodeca(r: f64) -> PolyMesh {
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let (a, b) = (1.0 / phi, phi);
    let s = r / 3f64.sqrt();
    let mut points: Vec<DVec3> = [
        (0.0, a, -b),
        (0.0, -a, -b),
        (b, 0.0, -a),
        (b, 0.0, a),
        (0.0, a, b),
        (0.0, -a, b),
        (-b, 0.0, a),
        (-b, 0.0, -a),
        (-a, b, 0.0),
        (a, b, 0.0),
        (a, -b, 0.0),
        (-a, -b, 0.0),
    ]
    .iter()
    .map(|&(x, y, z)| DVec3::new(x, y, z))
    .collect();
    points.extend(
        [
            (1.0, -1.0, -1.0),
            (1.0, -1.0, 1.0),
            (-1.0, -1.0, 1.0),
            (-1.0, -1.0, -1.0),
            (1.0, 1.0, -1.0),
            (1.0, 1.0, 1.0),
            (-1.0, 1.0, 1.0),
            (-1.0, 1.0, -1.0),
        ]
        .iter()
        .map(|&(x, y, z)| DVec3::new(x, y, z)),
    );
    for q in &mut points {
        *q *= s;
    }
    PolyMesh {
        points,
        polys: split_rings(&DODECA_RINGS),
    }
}

/// Regular truncated icosahedron with a pentagon facing +Z (P4).
fn bucky(r: f64) -> PolyMesh {
    let phi = (1.0 + 5f64.sqrt()) / 2.0;
    let mut ico: Vec<DVec3> = Vec::new();
    for (a, b) in [(1.0, phi), (-1.0, phi), (1.0, -phi), (-1.0, -phi)] {
        ico.push(DVec3::new(0.0, a, b));
        ico.push(DVec3::new(a, b, 0.0));
        ico.push(DVec3::new(b, 0.0, a));
    }
    let turn = DQuat::from_rotation_arc(ico[0].normalize(), DVec3::Z);
    for q in &mut ico {
        *q = turn * q.normalize();
    }
    let near = |i: usize, j: usize| (ico[i] - ico[j]).length() < 1.2;
    let neighbours =
        |i: usize| -> Vec<usize> { (0..12).filter(|&j| j != i && near(i, j)).collect() };
    // Truncation point near vertex i towards j.
    let cut = |i: usize, j: usize| (ico[i] * 2.0 + ico[j]) / 3.0;

    // The 60 points, in layers of descending Z, counter-clockwise inside a layer.
    let mut keyed: Vec<((usize, usize), DVec3)> = Vec::new();
    for i in 0..12 {
        for j in neighbours(i) {
            keyed.push(((i, j), cut(i, j)));
        }
    }
    let scale = r / keyed[0].1.length();
    keyed.sort_by(|a, b| {
        let layer = |p: &DVec3| -((p.z * 1e6).round() as i64);
        let angle = |p: &DVec3| p.y.atan2(p.x).rem_euclid(std::f64::consts::TAU);
        layer(&a.1)
            .cmp(&layer(&b.1))
            .then(angle(&a.1).total_cmp(&angle(&b.1)))
    });
    let index: BTreeMap<(usize, usize), u32> = keyed
        .iter()
        .enumerate()
        .map(|(k, (key, _))| (*key, k as u32))
        .collect();
    let points: Vec<DVec3> = keyed.iter().map(|(_, p)| *p * scale).collect();

    // Pentagons: the cuts around each icosahedron vertex, counter-clockwise from outside.
    let mut rings: Vec<[u32; 5]> = Vec::new();
    for i in 0..12 {
        let axis = ico[i];
        let u = if axis.x.abs() < 0.9 {
            DVec3::X
        } else {
            DVec3::Y
        };
        let e1 = axis.cross(u).normalize();
        let e2 = axis.cross(e1);
        let mut ns = neighbours(i);
        ns.sort_by(|&a, &b| {
            let ang = |j: usize| {
                let d = ico[j] - axis;
                d.dot(e2).atan2(d.dot(e1))
            };
            ang(a).total_cmp(&ang(b))
        });
        let ring: Vec<u32> = ns.iter().map(|&j| index[&(i, j)]).collect();
        rings.push(ring.try_into().expect("five neighbours"));
    }
    let mut polys = split_rings(&rings);

    // Hexagons: two quads per icosahedron face.
    for i in 0..12 {
        for j in neighbours(i) {
            for k in neighbours(j) {
                if k <= j || j <= i || !near(i, k) {
                    continue;
                }
                let (j, k) = if (ico[j] - ico[i]).cross(ico[k] - ico[i]).dot(ico[i]) > 0.0 {
                    (j, k)
                } else {
                    (k, j)
                };
                let h = [
                    index[&(i, j)],
                    index[&(j, i)],
                    index[&(j, k)],
                    index[&(k, j)],
                    index[&(k, i)],
                    index[&(i, k)],
                ];
                polys.push(vec![h[0], h[1], h[2], h[3]]);
                polys.push(vec![h[3], h[4], h[5], h[0]]);
            }
        }
    }
    PolyMesh { points, polys }
}
