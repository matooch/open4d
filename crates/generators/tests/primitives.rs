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
    let sep = p(&[("separate_surfaces", Value::Bool(true))]);
    assert!(matches!(
        build("forge.cube", &sep),
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

/// PRIMITIVES.md §2.1, §3.1: fillet and platonic meshes are closed at untested settings.
#[test]
fn fillet_and_platonic_meshes_are_closed() {
    let closed = |id: &str, params: &[(&str, Value)]| {
        let m = build(id, &p(params)).unwrap();
        m.validate().unwrap();
        let he = forge_geometry::HalfEdgeMesh::build(&m).unwrap();
        assert!(he.is_closed(), "{id} {params:?}");
    };
    for s in [1, 2, 7] {
        closed(
            "forge.cube",
            &[
                ("fillet", Value::Bool(true)),
                ("fillet_segments", Value::Int(s)),
                ("segments_x", Value::Int(3)),
                ("segments_z", Value::Int(2)),
                ("size_y", Value::Float(80.0)),
            ],
        );
    }
    for kind in ["tetrahedron", "hexahedron", "octahedron", "icosahedron"] {
        for segments in [3, 13] {
            closed(
                "forge.sphere",
                &[
                    ("type", Value::Enum(kind.into())),
                    ("segments", Value::Int(segments)),
                ],
            );
        }
    }
}

/// PRIMITIVES2.md: the round primitives and the pyramid are valid, and the solid ones
/// closed, at settings outside the golden cases (slices included).
#[test]
fn primitives_ii_are_valid_and_closed() {
    let check = |id: &str, params: &[(&str, Value)], closed: bool| {
        let m = build(id, &p(params)).unwrap();
        m.validate()
            .unwrap_or_else(|e| panic!("{id} {params:?}: {e:?}"));
        if closed {
            let he = forge_geometry::HalfEdgeMesh::build(&m).unwrap();
            assert!(he.is_closed(), "{id} {params:?}");
        }
    };
    let slice = |on: bool| {
        vec![
            ("slice", Value::Bool(on)),
            ("slice_from", Value::Float(0.3)),
            ("slice_to", Value::Float(4.0)),
        ]
    };
    for on in [false, true] {
        let mut cyl = slice(on);
        cyl.extend([
            ("fillet", Value::Bool(true)),
            ("cap_segments", Value::Int(2)),
            ("height_segments", Value::Int(3)),
            ("rotation_segments", Value::Int(5)),
        ]);
        check("forge.cylinder", &cyl, true);
        let mut cone = slice(on);
        cone.extend([
            ("top_radius", Value::Float(40.0)),
            ("fillet_top", Value::Bool(true)),
            ("fillet_bottom", Value::Bool(true)),
            ("fillet_segments", Value::Int(2)),
        ]);
        check("forge.cone", &cone, true);
        let mut tube = slice(on);
        tube.extend([
            ("fillet", Value::Bool(true)),
            ("cap_segments", Value::Int(2)),
            ("fillet_segments", Value::Int(3)),
        ]);
        check("forge.tube", &tube, true);
        check("forge.torus", &slice(on), true);
        check("forge.capsule", &slice(on), true);
        check("forge.disc", &slice(on), false);
    }
    // the default cone keeps its degenerate apex quads, as exported
    let cone = build("forge.cone", &p(&[])).unwrap();
    let apex = cone.polys.iter().filter(|f| f[1] == f[2]).count();
    assert_eq!(apex, 36);
    check("forge.pyramid", &[("segments", Value::Int(4))], true);
    assert!(matches!(
        build(
            "forge.torus",
            &p(&[
                ("slice", Value::Bool(true)),
                ("regular_grid", Value::Bool(true))
            ])
        ),
        Err(BuildError::Unsupported(_))
    ));
}
