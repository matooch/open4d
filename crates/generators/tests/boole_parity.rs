//! Parity: docs/specs/BOOLE.md §6 against tests/golden/boole (geometric comparison).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod common;

use common::{Table, resolved_params, typed_params};
use forge_core::{Registry, Value};
use forge_generators::{BuildError, Primitives, boole, boole_objects, build};
use forge_parity_harness::{Compare, Mesh, compare_geometric, compare_meshes, discover_cases};
use glam::DVec3;

/// A cube input table (`[params.input]` form: type, pos and the cube's own params).
fn cube_input(reg: &Registry, t: &Table) -> forge_geometry::PolyMesh {
    assert_eq!(t["type"].as_str(), Some("cube"));
    let own: Table = t
        .iter()
        .filter(|(k, _)| *k != "type" && *k != "pos")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let mut m = build("forge.cube", &typed_params(reg, "forge.cube", &own)).unwrap();
    let pos = t["pos"].as_array().unwrap();
    let c = |k: usize| {
        pos[k]
            .as_float()
            .or(pos[k].as_integer().map(|i| i as f64))
            .unwrap()
    };
    for p in &mut m.points {
        *p += DVec3::new(c(0), c(1), c(2));
    }
    m
}

/// Observed options whose geometry is not derived yet.
const UNSUPPORTED: &[&str] = &["boole/quality-off"];

/// Built, but the strict comparison (R25 point set and numbering) differs.
const KNOWN_MISMATCH: &[&str] = &[
    // pending brief 043 (BOOLE.md §5a): the objects, point counts, face counts, polygon
    // sizes and surface match; the new points' numbering, the face order of the union
    // family and the start corners of the cut faces are not derived for all four cases.
    "boole/r25-a_minus_b",
    "boole/r25-a_without_b",
    "boole/r25-intersect",
    "boole/r25-union",
];

#[test]
fn boole_matches_the_reference_geometrically() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut unsupported = BTreeSet::new();
    let mut matched = Vec::new();
    let mut mismatch = BTreeSet::new();
    let reg = {
        let mut r = Registry::with_builtins();
        r.install(&Primitives).unwrap();
        r
    };
    for case in discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "boole")
    {
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let r25 = m.params.contains_key("input2");
        if !r25 {
            assert_eq!(m.tolerance.compare, Compare::Geometric, "{key}");
        }

        let (a, b) = if r25 {
            let t = |k: &str| m.params[k].as_table().unwrap().clone();
            (
                cube_input(&reg, &t("input")),
                cube_input(&reg, &t("input2")),
            )
        } else {
            let a = build("forge.cube", &BTreeMap::new()).unwrap();
            let mut b = a.clone();
            let pos = m.params["b_pos"].as_array().unwrap();
            let c = |k: usize| pos[k].as_float().unwrap();
            let offset = DVec3::new(c(0), c(1), c(2));
            for p in &mut b.points {
                *p += offset;
            }
            (a, b)
        };
        let mut values = BTreeMap::new();
        for (k, v) in &resolved_params(&case, &m) {
            let v = match v {
                toml::Value::String(s) => Value::Enum(s.clone()),
                toml::Value::Boolean(b) => Value::Bool(*b),
                _ => continue,
            };
            values.insert(k.clone(), v);
        }

        // R25 cases are compared in the export layout (two objects, own points).
        let result = if r25 {
            boole_objects(&a, &b, &values)
        } else {
            boole(&a, &b, &values)
        };
        match result {
            Err(BuildError::Unsupported(what)) => {
                println!("{key}: unsupported ({what})");
                unsupported.insert(key);
            }
            Err(e) => panic!("{key}: {e}"),
            Ok(ours) => {
                let ours = Mesh {
                    positions: ours.points.iter().map(|p| p.to_array()).collect(),
                    faces: ours.polys,
                };
                let expected = case.expected_mesh().unwrap().unwrap();
                if m.tolerance.compare == Compare::Geometric {
                    let r = compare_geometric(&expected, &ours, &m.tolerance);
                    assert!(r.passed(), "{key}: {:?}", r.failures);
                    println!(
                        "{key}: points {:?}, planes {:?}, closed {:?}, faces {:?}",
                        r.points, r.planes, r.closed, r.face_counts
                    );
                    matched.push(key);
                } else if compare_meshes(&expected, &ours, &m.tolerance).passed() {
                    matched.push(key);
                } else {
                    println!("{key}: known mismatch (strict)");
                    // what is derived: layout, counts, polygon sizes and the surface
                    assert_eq!(expected.positions.len(), ours.positions.len(), "{key}");
                    let sizes = |m: &Mesh| {
                        let mut s: Vec<usize> = m.faces.iter().map(Vec::len).collect();
                        s.sort();
                        s
                    };
                    assert_eq!(sizes(&expected), sizes(&ours), "{key}: polygon sizes");
                    let r = compare_geometric(&expected, &ours, &m.tolerance);
                    assert!(r.passed(), "{key}: {:?}", r.failures);
                    mismatch.insert(key);
                }
            }
        }
    }
    println!("matched geometrically: {}", matched.join(", "));
    let want: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    let want: BTreeSet<String> = KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect();
    assert_eq!(mismatch, want, "known-mismatch set changed");
    assert_eq!(matched.len(), 5, "{matched:?}");
}

/// The cut-face polygon counts the reference shows: n-gons with hide on, quads off.
#[test]
fn boole_face_sizes_follow_hide_new_edges() {
    let a = build("forge.cube", &BTreeMap::new()).unwrap();
    let mut b = a.clone();
    for p in &mut b.points {
        *p += DVec3::splat(50.0);
    }
    let sizes = |hide: bool| {
        let v = BTreeMap::from([("hide_new_edges".to_string(), Value::Bool(hide))]);
        let mut s: Vec<usize> = boole(&a, &b, &v)
            .unwrap()
            .polys
            .iter()
            .map(Vec::len)
            .collect();
        s.sort();
        s
    };
    assert_eq!(
        sizes(true),
        [4, 4, 4, 4, 4, 4, 6, 6, 6],
        "hide-edges: 4-gons 6, 6-gons 3"
    );
    assert_eq!(sizes(false), [4; 12], "mode-a_minus_b: 4-gons 12");
}
