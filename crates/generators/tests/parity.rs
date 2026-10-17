//! Parity: docs/specs/PRIMITIVES.md §4 against tests/golden/{plane,cube,sphere}.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use forge_core::{ParamKind, Registry, Value};
use forge_generators::{BuildError, Primitives, build};
use forge_parity_harness::{Mesh, compare_meshes, discover_cases};

/// Cases whose options are observed but not implemented yet. A case leaving or
/// joining this set fails the test until the list is updated.
const UNSUPPORTED: &[&str] = &[
    "cube/fillet",
    "cube/fillet-segments",
    "sphere/type-hexahedron",
    "sphere/type-hexahedron-segments",
    "sphere/type-icosahedron",
    "sphere/type-icosahedron-segments",
    "sphere/type-octahedron",
    "sphere/type-octahedron-segments",
    "sphere/type-tetrahedron",
    "sphere/type-tetrahedron-segments",
];

fn registry() -> Registry {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r
}

/// The case's `[params]` as validated values, keyed like our param ids.
fn params(
    reg: &Registry,
    type_id: &str,
    raw: &BTreeMap<String, toml::Value>,
) -> BTreeMap<String, Value> {
    let ty = reg.get(type_id).unwrap();
    raw.iter()
        .map(|(k, v)| {
            let desc = ty
                .find_param(k)
                .unwrap_or_else(|| panic!("{type_id}: case key {k:?} is not a param"));
            let value = match (&desc.kind, v) {
                (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
                (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
                (ParamKind::Float { .. }, toml::Value::Integer(i)) => Value::Float(*i as f64),
                (ParamKind::Float { .. }, toml::Value::Float(f)) => Value::Float(*f),
                (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(s.clone()),
                other => panic!("{type_id}.{k}: unexpected value {other:?}"),
            };
            (k.clone(), desc.validate(value).expect("valid case value"))
        })
        .collect()
}

#[test]
fn primitives_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut unsupported = BTreeSet::new();
    let mut matched = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if !["plane", "cube", "sphere"].contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let manifest = case.manifest().unwrap();
        let type_id = format!("forge.{}", case.feature);
        let p = params(&reg, &type_id, &manifest.params);
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
    assert!(matched.len() >= 19, "only {} cases matched", matched.len());
}
