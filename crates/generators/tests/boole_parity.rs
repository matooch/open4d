//! Parity: docs/specs/BOOLE.md §6 against tests/golden/boole (geometric comparison).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod common;

use common::resolved_params;
use forge_core::Value;
use forge_generators::{BuildError, boole, build};
use forge_parity_harness::{Compare, Mesh, compare_geometric, discover_cases};
use glam::DVec3;

/// Observed options whose geometry is not derived yet.
const UNSUPPORTED: &[&str] = &["boole/quality-off"];

#[test]
fn boole_matches_the_reference_geometrically() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut unsupported = BTreeSet::new();
    let mut matched = Vec::new();
    for case in discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "boole")
    {
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        assert_eq!(m.tolerance.compare, Compare::Geometric, "{key}");

        let a = build("forge.cube", &BTreeMap::new()).unwrap();
        let mut b = a.clone();
        let pos = m.params["b_pos"].as_array().unwrap();
        let c = |k: usize| pos[k].as_float().unwrap();
        let offset = DVec3::new(c(0), c(1), c(2));
        for p in &mut b.points {
            *p += offset;
        }
        let mut values = BTreeMap::new();
        for (k, v) in &resolved_params(&case, &m) {
            let v = match v {
                toml::Value::String(s) => Value::Enum(s.clone()),
                toml::Value::Boolean(b) => Value::Bool(*b),
                _ => continue,
            };
            values.insert(k.clone(), v);
        }

        match boole(&a, &b, &values) {
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
                let r = compare_geometric(&expected, &ours, &m.tolerance);
                assert!(r.passed(), "{key}: {:?}", r.failures);
                println!(
                    "{key}: points {:?}, planes {:?}, closed {:?}, faces {:?}",
                    r.points, r.planes, r.closed, r.face_counts
                );
                matched.push(key);
            }
        }
    }
    println!("matched geometrically: {}", matched.join(", "));
    let want: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
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
