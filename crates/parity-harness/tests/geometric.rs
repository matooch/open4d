//! Geometric "same surface" comparison (docs/specs/INFRA.md §4.2).

use std::path::Path;

use forge_parity_harness::{
    Compare, Mesh, Tolerance, compare_geometric, compare_meshes, discover_cases,
};

/// Unit cube, outward quads.
fn cube() -> Mesh {
    let positions = (0..8)
        .map(|i| [(i & 1) as f64, ((i >> 1) & 1) as f64, ((i >> 2) & 1) as f64])
        .collect();
    let faces = vec![
        vec![0, 2, 3, 1], // z = 0
        vec![4, 5, 7, 6], // z = 1
        vec![0, 1, 5, 4], // y = 0
        vec![2, 6, 7, 3], // y = 1
        vec![0, 4, 6, 2], // x = 0
        vec![1, 3, 7, 5], // x = 1
    ];
    Mesh { positions, faces }
}

fn has(failures: &[String], needle: &str) -> bool {
    failures.iter().any(|f| f.contains(needle))
}

#[test]
fn renumbered_and_resplit_cube_is_the_same_surface() {
    let a = cube();
    // Reverse the point order, rotate each face's start corner, split two faces.
    let mut b = Mesh {
        positions: a.positions.iter().rev().copied().collect(),
        faces: Vec::new(),
    };
    let m = |v: u32| 7 - v;
    for (i, f) in a.faces.iter().enumerate() {
        let f: Vec<u32> = f.iter().map(|&v| m(v)).collect();
        if i < 2 {
            b.faces.push(vec![f[0], f[1], f[2]]);
            b.faces.push(vec![f[0], f[2], f[3]]);
        } else {
            b.faces.push(vec![f[1], f[2], f[3], f[0]]);
        }
    }
    let t = Tolerance::default();
    assert!(
        !compare_meshes(&a, &b, &t).passed(),
        "strict sees the order"
    );
    let r = compare_geometric(&a, &b, &t);
    assert!(r.passed(), "{:?}", r.failures);
    assert_eq!(r.points, (8, 8));
    assert_eq!(r.planes, (6, 6));
    assert_eq!(r.closed, (true, true));
    let (ev, av) = r.volume.unwrap();
    assert!((ev - 1.0).abs() < 1e-12 && (av - 1.0).abs() < 1e-12);
    assert_eq!(r.face_counts, (6, 8), "informational only");
}

#[test]
fn moved_point_fails_points_and_planes() {
    let a = cube();
    let mut b = cube();
    b.positions[7][0] += 0.25;
    let r = compare_geometric(&a, &b, &Tolerance::default());
    assert!(
        has(&r.failures, "points: expected point"),
        "{:?}",
        r.failures
    );
    assert!(has(&r.failures, "planes"), "{:?}", r.failures);

    // within tolerance passes
    let mut c = cube();
    c.positions[7][0] += 0.5e-4;
    assert!(compare_geometric(&a, &c, &Tolerance::default()).passed());
}

#[test]
fn missing_face_fails_closedness_and_plane() {
    let a = cube();
    let mut b = cube();
    b.faces.pop();
    let r = compare_geometric(&a, &b, &Tolerance::default());
    assert_eq!(r.closed, (true, false));
    assert!(has(&r.failures, "closed"), "{:?}", r.failures);
    assert!(has(&r.failures, "missing"), "{:?}", r.failures);
    assert!(r.volume.is_none());
}

#[test]
fn flipped_face_is_a_different_oriented_plane() {
    let a = cube();
    let mut b = cube();
    b.faces[0].reverse();
    let r = compare_geometric(&a, &b, &Tolerance::default());
    assert!(has(&r.failures, "unexpected plane"), "{:?}", r.failures);
    assert!(has(&r.failures, "closed"), "{:?}", r.failures);
}

#[test]
fn coplanar_faces_compare_by_total_area() {
    // Two unit squares in z = 0; splitting the second into triangles is the same
    // surface, dropping half of it is not.
    let positions = vec![
        [0., 0., 0.],
        [1., 0., 0.],
        [1., 1., 0.],
        [0., 1., 0.],
        [2., 0., 0.],
        [2., 1., 0.],
    ];
    let a = Mesh {
        positions: positions.clone(),
        faces: vec![vec![0, 1, 2, 3], vec![1, 4, 5, 2]],
    };
    let b = Mesh {
        positions,
        faces: vec![vec![0, 1, 2, 3], vec![1, 4, 5], vec![1, 5, 2]],
    };
    assert!(compare_geometric(&a, &b, &Tolerance::default()).passed());
    let c = Mesh {
        faces: vec![vec![0, 1, 2, 3], vec![1, 4, 5]],
        ..a.clone()
    };
    let r = compare_geometric(&a, &c, &Tolerance::default());
    assert!(has(&r.failures, "area"), "{:?}", r.failures);
}

#[test]
fn case_toml_opts_in_with_compare_geometric() {
    let t: Tolerance = toml::from_str("compare = \"geometric\"").unwrap();
    assert_eq!(t.compare, Compare::Geometric);
    assert_eq!(Tolerance::default().compare, Compare::Strict);
    assert!(toml::from_str::<Tolerance>("compare = \"loose\"").is_err());
}

/// The reference's hide-edges and default exports cut A differently (6-gons vs quads)
/// but are the same solid; A without B is not.
#[test]
fn reference_boole_exports_agree_geometrically() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases = discover_cases(&root).unwrap();
    let mesh = |name: &str| {
        cases
            .iter()
            .find(|c| c.feature == "boole" && c.name == name)
            .unwrap()
            .expected_mesh()
            .unwrap()
            .unwrap()
    };
    let t = Tolerance::default();
    let (minus, hide) = (mesh("mode-a_minus_b"), mesh("hide-edges"));
    let r = compare_geometric(&minus, &hide, &t);
    assert!(r.passed(), "{:?}", r.failures);
    assert_eq!(r.closed, (true, true));
    let (v, _) = r.volume.unwrap();
    assert!((v - (200f64.powi(3) - 150f64.powi(3))).abs() < 1e-6, "{v}");

    let r = compare_geometric(&minus, &mesh("mode-a_without_b"), &t);
    assert_eq!(r.closed, (true, false));
    assert!(!r.passed());
}
