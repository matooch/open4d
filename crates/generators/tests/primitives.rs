//! docs/specs/PRIMITIVES.md §0–3: plugin, defaults and unsupported options.

use std::collections::BTreeMap;

use forge_core::{Category, Registry, Value};
use forge_generators::{BuildError, Primitives, build};

fn p(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect()
}

#[test]
fn plugin_registers_generators_with_transform_params() {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    for id in ["forge.plane", "forge.cube", "forge.sphere"] {
        let t = r.get(id).unwrap();
        assert_eq!(t.category, Category::Generator);
        assert_eq!(t.params[0].id, "transform.pos");
    }
    let sphere = r.get("forge.sphere").unwrap();
    assert_eq!(
        sphere.find_param("segments").unwrap().default,
        Value::Int(24)
    );
}

#[test]
fn missing_params_use_defaults() {
    let plane = build("forge.plane", &BTreeMap::new()).unwrap();
    assert_eq!((plane.points.len(), plane.polys.len()), (441, 400));
    let cube = build("forge.cube", &BTreeMap::new()).unwrap();
    assert_eq!((cube.points.len(), cube.polys.len()), (8, 6));
    let sphere = build("forge.sphere", &BTreeMap::new()).unwrap();
    assert_eq!((sphere.points.len(), sphere.polys.len()), (266, 288));
    for m in [&plane, &cube, &sphere] {
        m.validate().unwrap();
    }
}

#[test]
fn errors_for_unknown_types_and_unimplemented_options() {
    assert!(matches!(
        build("forge.nope", &BTreeMap::new()),
        Err(BuildError::UnknownType(_))
    ));
    let fillet = p(&[("fillet", Value::Bool(true))]);
    assert!(matches!(
        build("forge.cube", &fillet),
        Err(BuildError::Unsupported(_))
    ));
    let sep = p(&[("separate_surfaces", Value::Bool(true))]);
    assert!(matches!(
        build("forge.cube", &sep),
        Err(BuildError::Unsupported(_))
    ));
    let ico = p(&[("type", Value::Enum("icosahedron".into()))]);
    assert!(matches!(
        build("forge.sphere", &ico),
        Err(BuildError::Unsupported(_))
    ));
    // hemisphere needs an even band count: 10 segments -> 5 bands
    let odd = p(&[
        ("type", Value::Enum("hemisphere".into())),
        ("segments", Value::Int(10)),
    ]);
    assert!(matches!(
        build("forge.sphere", &odd),
        Err(BuildError::Unsupported(_))
    ));
}

#[test]
fn odd_segments_round_up() {
    let s = build("forge.sphere", &p(&[("segments", Value::Int(9))])).unwrap();
    assert_eq!((s.points.len(), s.polys.len()), (42, 50));
}
