//! Parity: docs/specs/PRIMITIVES.md §4 against tests/golden/{plane,cube,sphere}.

use std::collections::BTreeSet;
use std::path::Path;

mod common;

use common::{resolved_params, typed_params};
use forge_core::Registry;
use forge_generators::{BuildError, Primitives, build};
use forge_parity_harness::{Mesh, Tolerance, compare_geometric, compare_meshes, discover_cases};

/// Cases whose options are observed but not implemented yet. A case leaving or
/// joining this set fails the test until the list is updated.
const UNSUPPORTED: &[&str] = &[];

const FEATURES: &[&str] = &[
    "plane", "cube", "sphere", "cylinder", "cone", "disc", "torus", "tube", "capsule", "pyramid",
];

/// Slices whose cut polygons the reference triangulates with an underived method
/// (docs/specs/PRIMITIVES2.md §2, §10): exact except the trailing cut triangles, plus a
/// geometric comparison of the whole mesh.
const CUT_GEOMETRIC: &[&str] = &["cone/slice", "torus/slice", "capsule/slice"];

fn registry() -> Registry {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r
}

#[test]
fn primitives_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut unsupported = BTreeSet::new();
    let mut matched = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let manifest = case.manifest().unwrap();
        let type_id = format!("forge.{}", case.feature);
        let p = typed_params(&reg, &type_id, &resolved_params(&case, &manifest));
        match build(&type_id, &p) {
            Err(BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
            }
            Err(e) => panic!("{key}: {e}"),
            Ok(mesh) => {
                let ours = Mesh {
                    positions: mesh.points.iter().map(|p| p.to_array()).collect(),
                    faces: mesh.polys.clone(),
                };
                let expected = case.expected_mesh().unwrap().unwrap();
                if CUT_GEOMETRIC.contains(&key.as_str()) {
                    check_cut_geometric(&key, &expected, &ours, &manifest.tolerance);
                    matched.push(key);
                    continue;
                }
                let report = compare_meshes(&expected, &ours, &manifest.tolerance);
                assert!(
                    report.passed(),
                    "{key}: {:?} (max error {:?})",
                    report.failures,
                    report.max_position_error
                );
                matched.push(key);
            }
        }
    }
    println!("matched exactly: {}", matched.join(", "));
    let expected: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, expected, "unsupported set changed");
    assert!(matched.len() >= 75, "only {} cases matched", matched.len());
}

/// Exact positions and faces up to a trailing run of triangles in both meshes (the cut
/// faces), and a geometric match of the whole (PRIMITIVES2.md §10).
fn check_cut_geometric(key: &str, expected: &Mesh, ours: &Mesh, tol: &Tolerance) {
    let prefix = Mesh {
        positions: expected.positions.clone(),
        faces: Vec::new(),
    };
    let same_points = compare_meshes(
        &prefix,
        &Mesh {
            faces: Vec::new(),
            ..ours.clone()
        },
        tol,
    );
    assert!(same_points.passed(), "{key}: {:?}", same_points.failures);
    assert_eq!(expected.faces.len(), ours.faces.len(), "{key}: face count");
    let first_diff = expected
        .faces
        .iter()
        .zip(&ours.faces)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.faces.len());
    for m in [expected, ours] {
        assert!(
            m.faces[first_diff..].iter().all(|f| f.len() == 3),
            "{key}: faces differ before the cut (face {first_diff})"
        );
    }
    let r = compare_geometric(expected, ours, tol);
    assert!(r.passed(), "{key}: {:?}", r.failures);
}
