//! docs/specs/GEOMETRY.md §6.1–6.2: the reference's triangulation of cut and cap polygons,
//! checked against the exported goldens.

use std::collections::BTreeSet;
use std::path::Path;

use forge_geometry::PolyMesh;
use forge_geometry::triangulate::{
    order_triangles, split_pentagon, triangulate, triangulate_delaunay,
};
use forge_parity_harness::parse_obj;
use glam::DVec3;

fn golden(path: &str) -> forge_parity_harness::Mesh {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let src = std::fs::read_to_string(root.join(path)).expect("golden mesh");
    parse_obj(&src).expect("obj")
}

fn points(m: &forge_parity_harness::Mesh) -> Vec<DVec3> {
    m.positions.iter().map(|p| DVec3::from_array(*p)).collect()
}

/// A cut or cap of an exported mesh: the corners in walk order and the exported triangles
/// (mesh indices), re-wound like the walk (the start cut of a lathe is exported reversed).
struct Cap {
    name: &'static str,
    walk: Vec<u32>,
    exported: Vec<[u32; 3]>,
    /// The export is wound against the walk.
    reversed: bool,
}

fn tail(m: &forge_parity_harness::Mesh, from: usize, count: usize) -> Vec<[u32; 3]> {
    m.faces[from..from + count]
        .iter()
        .map(|f| {
            assert_eq!(f.len(), 3);
            [f[0], f[1], f[2]]
        })
        .collect()
}

fn caps() -> Vec<(forge_parity_harness::Mesh, Vec<Cap>)> {
    let loft = golden("loft/defaults/mesh.obj");
    let loft3 = golden("loft/three/mesh.obj");
    let cone = golden("cone/slice/mesh.obj");
    let torus = golden("torus/slice/mesh.obj");
    let loft_caps = |m: &forge_parity_harness::Mesh, end_first: u32| {
        let n = m.faces.len();
        vec![
            Cap {
                name: "loft start cap",
                walk: (0..29).collect(),
                exported: tail(m, n - 54, 27),
                reversed: false,
            },
            Cap {
                name: "loft end cap",
                walk: (end_first..end_first + 29).rev().collect(),
                exported: tail(m, n - 27, 27),
                reversed: false,
            },
        ]
    };
    let cone_caps = vec![
        Cap {
            name: "cone start cut",
            walk: (0..12).collect(),
            exported: tail(&cone, cone.faces.len() - 20, 10),
            reversed: true,
        },
        Cap {
            name: "cone end cut",
            walk: [0].into_iter().chain(362..372).chain([11]).collect(),
            exported: tail(&cone, cone.faces.len() - 10, 10),
            reversed: false,
        },
    ];
    let torus_caps = vec![
        Cap {
            name: "torus start cut",
            walk: (0..18).collect(),
            exported: tail(&torus, torus.faces.len() - 32, 16),
            reversed: true,
        },
        Cap {
            name: "torus end cut",
            walk: (648..666).collect(),
            exported: tail(&torus, torus.faces.len() - 16, 16),
            reversed: false,
        },
    ];
    let l1 = loft_caps(&loft, 290);
    let l3 = loft_caps(&loft3, 580);
    vec![
        (loft, l1),
        (loft3, l3),
        (cone, cone_caps),
        (torus, torus_caps),
    ]
}

fn rewound(c: &Cap) -> Vec<[u32; 3]> {
    c.exported
        .iter()
        .map(|&[a, b, d]| if c.reversed { [a, d, b] } else { [a, b, d] })
        .collect()
}

fn as_set(tris: &[[u32; 3]]) -> BTreeSet<[u32; 3]> {
    tris.iter()
        .map(|t| {
            let mut s = *t;
            s.sort_unstable();
            s
        })
        .collect()
}

/// R2: the listing rule holds for every exported cut, whatever set it lists.
#[test]
fn emission_order_reproduces_every_exported_listing() {
    for (mesh, caps) in caps() {
        let p = points(&mesh);
        for c in &caps {
            let ordered = order_triangles(&p, &c.walk, &rewound(c));
            // the listing starts at the lowest walk corner of each triangle: compare the
            // exported (re-wound) triangle with the same rotation
            let want: Vec<[u32; 3]> = rewound(c)
                .into_iter()
                .map(|t| {
                    let pos = |v: u32| c.walk.iter().position(|&w| w == v).unwrap();
                    let k = (0..3).min_by_key(|&k| pos(t[k])).unwrap();
                    [t[k], t[(k + 1) % 3], t[(k + 2) % 3]]
                })
                .collect();
            assert_eq!(ordered, want, "{}", c.name);
        }
    }
}

/// R1: where the in-circle tests have no ties the triangle set is the export's, in the
/// export's order.
#[test]
fn delaunay_cut_matches_loft_and_cone() {
    for (mesh, caps) in caps() {
        let p = points(&mesh);
        for c in caps.iter().filter(|c| !c.name.starts_with("torus")) {
            let ours = triangulate_delaunay(&p, &c.walk);
            let want = order_triangles(&p, &c.walk, &rewound(c));
            // `loft/three` end cap: one exactly cocircular (mirror-symmetric) quad whose
            // diagonal the export decides the other way (R1, parity-pending tie rule)
            let tie = mesh.faces.len() == 634 && c.name == "loft end cap";
            if tie {
                let (a, b) = (as_set(&ours), as_set(&want));
                assert_eq!(a.symmetric_difference(&b).count(), 4, "{}", c.name);
            } else {
                assert_eq!(ours, want, "{}", c.name);
            }
        }
    }
}

/// R1, the documented limit: the cocircular torus cut is a valid triangulation with the
/// same area, but its set is decided by noise the export no longer carries.
#[test]
fn cocircular_torus_cut_is_valid_but_not_the_export() {
    let (mesh, caps) = caps().pop().unwrap();
    let p = points(&mesh);
    for c in &caps {
        let ours = triangulate_delaunay(&p, &c.walk);
        assert_eq!(ours.len(), 16);
        let area = |ts: &[[u32; 3]]| -> f64 {
            ts.iter()
                .map(|t| forge_geometry::normals::poly_area(&p, t))
                .sum()
        };
        assert!((area(&ours) - area(&c.exported)).abs() < 1e-3, "{}", c.name);
        assert_ne!(as_set(&ours), as_set(&c.exported), "{}", c.name);
    }
}

/// R3: the dodecahedron's 12 pentagons are exported as a triangle (faces 0..11) and a quad
/// (faces 12..23); face `i` has its ear at vertex `i`.
#[test]
fn dodecahedron_pentagons_split_as_exported() {
    let m = golden("platonic/type-dodeca/mesh.obj");
    let p = points(&m);
    let (tris, quads) = m.faces.split_at(12);
    for (i, t) in tris.iter().enumerate() {
        assert_eq!(t.len(), 3);
        assert_eq!(t[2], i as u32, "triangle {i} hangs on vertex {i}");
        // the quad that holds the triangle's edge previous -> next
        let (n, pv) = (t[0], t[1]);
        let q = quads
            .iter()
            .find(|q| (0..4).any(|k| q[k] == pv && q[(k + 1) % 4] == n))
            .expect("quad across the diagonal");
        let k = q.iter().position(|&v| v == pv).unwrap();
        let ring = [t[2], n, q[(k + 2) % 4], q[(k + 3) % 4], pv];
        // the merged face is a planar convex pentagon
        let poly: Vec<u32> = ring.to_vec();
        let nrm = forge_geometry::normals::poly_normal(&p, &poly);
        for &v in &poly {
            assert!((p[v as usize] - p[poly[0] as usize]).dot(nrm).abs() < 1e-3);
        }
        let (tri, quad) = split_pentagon(&ring, 0);
        assert_eq!(tri.to_vec(), *t, "triangle {i}");
        assert_eq!(quad.to_vec(), *q, "quad of triangle {i}");
    }
}

/// §6.2: edges for drawing are the polygon boundary edges, never the diagonals.
#[test]
fn a_pentagon_has_five_edges_but_three_triangles() {
    let pts: Vec<DVec3> = (0..5)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 5.0;
            DVec3::new(a.cos(), a.sin(), 0.0)
        })
        .collect();
    let mesh = PolyMesh {
        points: pts,
        polys: vec![vec![0, 1, 2, 3, 4]],
    };
    assert_eq!(mesh.edges().len(), 5);
    assert_eq!(triangulate(&mesh).len(), 3);
}

fn valid(points: &[DVec3], poly: &[u32]) {
    use forge_geometry::normals::{poly_area, poly_normal};
    let tris = triangulate_delaunay(points, poly);
    assert_eq!(tris.len(), poly.len() - 2);
    let n = poly_normal(points, poly);
    let mut area = 0.0;
    for t in &tris {
        assert!(poly_normal(points, t).dot(n) > 0.0, "{t:?} is flipped");
        area += poly_area(points, t);
    }
    assert!(
        (area - poly_area(points, poly)).abs() < 1e-9,
        "areas differ"
    );
}

#[test]
fn delaunay_triangulation_is_valid_for_concave_and_odd_input() {
    let flat = |pts: &[[f64; 2]]| -> Vec<DVec3> {
        pts.iter().map(|p| DVec3::new(p[0], p[1], 0.0)).collect()
    };
    let l = flat(&[[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]]);
    valid(&l, &[0, 1, 2, 3, 4, 5]);
    let star: Vec<[f64; 2]> = (0..10)
        .map(|i| {
            let r = if i % 2 == 0 { 2.0 } else { 0.8 };
            let a = i as f64 * std::f64::consts::PI / 5.0;
            [r * a.cos(), r * a.sin()]
        })
        .collect();
    let star = flat(&star);
    valid(&star, &(0..10).collect::<Vec<u32>>());
    // collinear corners on two sides (the cone cut)
    let tri = flat(&[[0., 0.], [1., 0.], [2., 0.], [2., 1.], [1., 1.], [0., 2.]]);
    valid(&tri, &[0, 1, 2, 3, 4, 5]);
    // a quad in the YZ plane, wound both ways
    let yz: Vec<DVec3> = [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]]
        .iter()
        .map(|a| DVec3::from_array(*a))
        .collect();
    valid(&yz, &[0, 1, 2, 3]);
    valid(&yz, &[3, 2, 1, 0]);
}
