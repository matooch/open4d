//! Parity: docs/parity/r25-drift.md (O1) against tests/golden/r25-drift, built with the R25
//! defaults (docs/specs/INFRA.md §3.1, PRIMITIVES2.md, GENERATORS2.md §0a). Each case runs
//! through the builder of the feature it was copied from (`<feature>-<original case>`) and is
//! compared the way that feature's own parity test compares it.

mod common;

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use common::{Table, apply, resolved_params, typed_params};
use forge_core::{Document, Registry, Value};
use forge_generators::{BuildError, Primitives, boole, build, evaluate};
use forge_parity_harness::{
    Compare, Mesh, Tolerance, compare_geometric, compare_meshes, discover_cases, export_weld,
};
use glam::DVec3;

const PRIMITIVES: &[&str] = &[
    "plane", "cube", "sphere", "cylinder", "cone", "disc", "torus", "tube", "capsule", "pyramid",
];
const GENERATORS: &[&str] = &["extrude", "lathe", "loft", "sweep"];

/// Observed options we do not build (asserted exactly; reasons in the spec).
const UNSUPPORTED: &[&str] = &[];

/// Built but differing, pinned so a fix shows up (reasons in PRIMITIVES2.md §6 and
/// GENERATORS2.md §7).
const KNOWN_MISMATCH: &[&str] = &["r25-drift/extrude-rectangle-caps-quadrangles"];

/// Compared like `parity.rs`'s `CUT_GEOMETRIC`: exact up to the trailing cut triangles, then
/// geometric.
const CUT_GEOMETRIC: &[&str] = &["r25-drift/torus-slice"];

/// The feature a drift case was copied from: the text before the first `-` that names a
/// known feature.
fn original_feature(name: &str) -> Option<&str> {
    let (head, _) = name.split_once('-')?;
    let known = |f: &str| {
        PRIMITIVES.contains(&f) || GENERATORS.contains(&f) || ["boole", "subdivision"].contains(&f)
    };
    known(head).then_some(head)
}

fn world_mesh(mesh: &forge_geometry::PolyMesh) -> Mesh {
    Mesh {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        faces: mesh.polys.clone(),
    }
}

/// Our mesh for the case, or the reason it is not built.
fn build_case(reg: &Arc<Registry>, feature: &str, params: &Table) -> Result<Mesh, BuildError> {
    if PRIMITIVES.contains(&feature) {
        let type_id = format!("forge.{feature}");
        return build(&type_id, &typed_params(reg, &type_id, params)).map(|m| world_mesh(&m));
    }
    if feature == "boole" {
        let a = build("forge.cube", &Default::default()).unwrap();
        let mut b = a.clone();
        let pos = params["b_pos"].as_array().unwrap();
        let c = |k: usize| pos[k].as_float().unwrap();
        for p in &mut b.points {
            *p += DVec3::new(c(0), c(1), c(2));
        }
        let values = params
            .iter()
            .filter_map(|(k, v)| match v {
                toml::Value::String(s) => Some((k.clone(), Value::Enum(s.clone()))),
                toml::Value::Boolean(b) => Some((k.clone(), Value::Bool(*b))),
                _ => None,
            })
            .collect();
        return boole(&a, &b, &values).map(|m| world_mesh(&m));
    }
    // generators with spline children
    let mut inputs: Vec<(&String, &Table)> = params
        .iter()
        .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
        .collect();
    inputs.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));
    let mut doc = Document::new(reg.clone());
    let gen_type = format!("forge.{feature}");
    let generator = doc
        .transact("case", |tx| {
            let generator = tx.create(&gen_type, None, None)?;
            apply(tx, reg, generator, &gen_type, params);
            for (_, t) in &inputs {
                let ty = format!("forge.{}", t["type"].as_str().unwrap());
                let child = tx.create(&ty, Some(generator), None)?;
                apply(tx, reg, child, &ty, t);
            }
            Ok(generator)
        })
        .unwrap();
    let mesh = evaluate(doc.scene(), generator)?.expect("geometry");
    let world = doc.scene().local_transform(generator).unwrap().matrix();
    Ok(Mesh {
        positions: mesh
            .points
            .iter()
            .map(|p| world.transform_point3(*p).to_array())
            .collect(),
        faces: mesh.polys.clone(),
    })
}

/// Exact positions and faces up to a trailing run of triangles (the cut faces), and a
/// geometric match of the whole (as `parity.rs`).
fn check_cut_geometric(key: &str, expected: &Mesh, ours: &Mesh, tol: &Tolerance) {
    let points = |m: &Mesh| Mesh {
        positions: m.positions.clone(),
        faces: Vec::new(),
    };
    let same = compare_meshes(&points(expected), &points(ours), tol);
    assert!(same.passed(), "{key}: {:?}", same.failures);
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

#[test]
fn r25_drift_cases_match_with_r25_defaults() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let reg = Arc::new(reg);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut unsupported, mut mismatched) =
        (Vec::new(), BTreeSet::new(), BTreeSet::new());
    let mut seen = 0;
    for case in discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "r25-drift")
    {
        seen += 1;
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let feature = original_feature(&case.name)
            .unwrap_or_else(|| panic!("{key}: no known feature in the case name"));
        let params = resolved_params(&case, &m);
        let ours = match build_case(&reg, feature, &params) {
            Err(BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
                continue;
            }
            Err(e) => panic!("{key}: {e}"),
            Ok(mesh) => mesh,
        };
        let expected = case.expected_mesh().unwrap().unwrap();
        // the original features' comparisons: generators and boole weld both sides
        let weld = GENERATORS.contains(&feature) || feature == "boole";
        let (expected, ours) = if weld {
            (export_weld(&expected), export_weld(&ours))
        } else {
            (expected, ours)
        };
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            let strict = compare_meshes(&expected, &ours, &m.tolerance).passed();
            assert!(!strict, "{key} matches now: remove it from KNOWN_MISMATCH");
            // R25 vertex / corner order only: the surface is the same
            let geometric = compare_geometric(&expected, &ours, &m.tolerance);
            assert!(geometric.passed(), "{key}: {:?}", geometric.failures);
            mismatched.insert(key);
            continue;
        }
        if CUT_GEOMETRIC.contains(&key.as_str()) {
            check_cut_geometric(&key, &expected, &ours, &m.tolerance);
        } else if m.tolerance.compare == Compare::Geometric {
            let r = compare_geometric(&expected, &ours, &m.tolerance);
            assert!(r.passed(), "{key}: {:?}", r.failures);
        } else {
            let r = compare_meshes(&expected, &ours, &m.tolerance);
            assert!(
                r.passed(),
                "{key}: {:?} (max error {:?})",
                r.failures,
                r.max_position_error
            );
        }
        matched.push(key);
    }
    println!("matched: {}", matched.join(", "));
    let set = |l: &[&str]| l.iter().map(|s| s.to_string()).collect::<BTreeSet<_>>();
    assert_eq!(unsupported, set(UNSUPPORTED), "unsupported set changed");
    assert_eq!(
        mismatched,
        set(KNOWN_MISMATCH),
        "known-mismatch set changed"
    );
    assert_eq!(seen, 7, "r25-drift case count changed");
    assert_eq!(
        matched.len(),
        7 - UNSUPPORTED.len() - KNOWN_MISMATCH.len(),
        "{matched:?}"
    );
}
