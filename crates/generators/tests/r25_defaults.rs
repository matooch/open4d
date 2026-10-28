//! docs/parity/r25-defaults.md (O4): the descriptors carry the R25 defaults, and the R20
//! overlay `tests/defaults/r20.toml` holds exactly the old values (docs/specs/INFRA.md §3.1).

mod common;

use std::collections::BTreeMap;
use std::f64::consts::PI;

use common::repo_root;
use forge_core::{ParamKind, Registry, Value};
use forge_generators::{BuildError, Primitives, build};
use forge_parity_harness::{AppVersion, defaults_overlay};

fn registry() -> Registry {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r
}

fn i(v: i64) -> Value {
    Value::Int(v)
}
fn f(v: f64) -> Value {
    Value::Float(v)
}
fn e(v: &str) -> Value {
    Value::Enum(v.into())
}

/// (type, key, R25 default). Rows applied from the `!` rows of O4, in our units (radians).
fn applied() -> Vec<(&'static str, &'static str, Value)> {
    vec![
        ("plane", "segments_w", i(10)),
        ("plane", "segments_h", i(10)),
        ("cube", "fillet_radius", f(3.0)),
        ("cube", "fillet_segments", i(3)),
        ("sphere", "segments", i(16)),
        ("cylinder", "height_segments", i(4)),
        ("cylinder", "rotation_segments", i(16)),
        ("cylinder", "fillet_segments", i(3)),
        ("cylinder", "fillet_radius", f(3.0)),
        ("cylinder", "slice_to", f(PI)),
        ("cone", "height_segments", i(4)),
        ("cone", "rotation_segments", i(16)),
        ("cone", "cap_segments", i(1)),
        ("cone", "fillet_segments", i(3)),
        ("cone", "slice_to", f(PI)),
        ("disc", "disc_segments", i(1)),
        ("disc", "rotation_segments", i(16)),
        ("disc", "slice_to", f(PI)),
        ("torus", "ring_radius", f(100.0)),
        ("torus", "ring_segments", i(32)),
        ("torus", "pipe_segments", i(16)),
        ("torus", "slice_to", f(PI)),
        ("tube", "outer_radius", f(100.0)),
        ("tube", "rotation_segments", i(16)),
        ("tube", "height_segments", i(4)),
        ("tube", "slice_to", f(PI)),
        ("capsule", "height_segments", i(4)),
        ("capsule", "cap_segments", i(4)),
        ("capsule", "rotation_segments", i(16)),
        ("capsule", "slice_to", f(PI)),
        ("subdivision", "level_render", i(2)),
        ("subdivision", "subdivide_uvs", e("edge")),
        ("boole", "hide_new_edges", Value::Bool(true)),
        ("extrude", "movement", Value::Vec3([0.0, 0.0, 100.0])),
        ("lathe", "subdivision", i(32)),
        ("loft", "mesh_subdivision_u", i(16)),
        ("loft", "mesh_subdivision_v", i(4)),
    ]
}

/// `!` rows not applied: (type, key, why).
const NOT_APPLIED: &[(&str, &str)] = &[
    ("circle_spline", "intermediate"), // R25 Uniform: the build only supports Adaptive 5°
    ("helix_spline", "intermediate"),  // same
    ("loft", "cap_type"),              // R25 N-gon: loft caps are always ear-clipped triangles
                                       // no param of ours: the isoparm subdivisions
];

#[test]
fn descriptors_have_the_r25_defaults() {
    let reg = registry();
    for (ty, key, want) in applied() {
        let desc = reg
            .get(&format!("forge.{ty}"))
            .unwrap()
            .find_param(key)
            .unwrap_or_else(|| panic!("{ty}.{key}"));
        assert_eq!(desc.default, want, "{ty}.{key}");
    }
    // kept R20 defaults
    for (ty, key, kept) in [
        ("circle_spline", "intermediate", "adaptive"),
        ("helix_spline", "intermediate", "adaptive"),
        ("loft", "cap_type", "triangles"),
    ] {
        let desc = reg
            .get(&format!("forge.{ty}"))
            .unwrap()
            .find_param(key)
            .unwrap();
        assert_eq!(desc.default, e(kept), "{ty}.{key}");
    }
}

#[test]
fn the_r20_overlay_is_the_old_value_of_every_row_and_never_the_new_one() {
    let reg = registry();
    let root = repo_root();
    let mut rows = 0;
    for (ty, key, r25) in applied() {
        let o = defaults_overlay(&root, AppVersion::R20, ty);
        let r20 = o.get(key).unwrap_or_else(|| panic!("overlay {ty}.{key}"));
        // same kind as the descriptor, a different value (the overlay is what R20 had)
        let desc = reg
            .get(&format!("forge.{ty}"))
            .unwrap()
            .find_param(key)
            .unwrap();
        let as_value = match (&desc.kind, r20) {
            (ParamKind::Int { .. }, toml::Value::Integer(v)) => Value::Int(*v),
            (ParamKind::Float { .. }, toml::Value::Float(v)) => Value::Float(*v),
            (ParamKind::Enum { .. }, toml::Value::String(s)) => Value::Enum(s.clone()),
            (ParamKind::Bool, toml::Value::Boolean(b)) => Value::Bool(*b),
            (ParamKind::Vec3 { .. }, toml::Value::Array(a)) => {
                Value::Vec3([0, 1, 2].map(|k| a[k].as_float().unwrap()))
            }
            other => panic!("{ty}.{key}: {other:?}"),
        };
        assert_ne!(
            as_value, r25,
            "{ty}.{key}: the overlay equals the R25 default"
        );
        rows += 1;
    }
    assert_eq!(rows, 37);
    // the kept ones carry the R20 value too (a no-op while the descriptor keeps it)
    for (ty, key) in NOT_APPLIED {
        assert!(
            defaults_overlay(&root, AppVersion::R20, ty).contains_key(*key),
            "{ty}.{key}"
        );
    }
}

#[test]
fn r25_defaults_build_the_observed_sizes() {
    // r25-drift.md: sphere 114 / 128; cylinder fillet 178 / 192; torus: ring 32, pipe 16
    let sphere = build("forge.sphere", &BTreeMap::new()).unwrap();
    assert_eq!((sphere.points.len(), sphere.polys.len()), (114, 128));
    let cylinder = build(
        "forge.cylinder",
        &BTreeMap::from([("fillet".to_string(), Value::Bool(true))]),
    )
    .unwrap();
    assert_eq!((cylinder.points.len(), cylinder.polys.len()), (178, 192));
    let torus = build("forge.torus", &BTreeMap::new()).unwrap();
    assert_eq!(torus.points.len(), 32 * 16);
}

#[test]
fn widened_ranges_build_at_their_minimum() {
    // R25 lets lathe subdivision go down to 1 and loft down to 2: nothing may panic
    use forge_core::Document;
    use forge_generators::evaluate;
    use std::sync::Arc;
    let reg = Arc::new(registry());
    for (ty, key, v) in [("lathe", "subdivision", 1), ("lathe", "subdivision", 2)] {
        let mut d = Document::new(reg.clone());
        let g = d
            .transact("t", |tx| {
                let g = tx.create(&format!("forge.{ty}"), None, None)?;
                let c = tx.create("forge.rectangle_spline", Some(g), None)?;
                tx.set_param(c, "transform.pos", Value::Vec3([150.0, 0.0, 0.0]))?;
                tx.set_param(g, key, Value::Int(v))?;
                Ok(g)
            })
            .unwrap();
        match evaluate(d.scene(), g) {
            Ok(_) | Err(BuildError::Unsupported(_)) => {}
            Err(e) => panic!("{ty}.{key}={v}: {e}"),
        }
    }
    for u in [2, 3] {
        let mut d = Document::new(reg.clone());
        let g = d
            .transact("t", |tx| {
                let g = tx.create("forge.loft", None, None)?;
                for z in [0.0, 100.0] {
                    let c = tx.create("forge.circle_spline", Some(g), None)?;
                    tx.set_param(c, "transform.pos", Value::Vec3([0.0, 0.0, z]))?;
                }
                tx.set_param(g, "mesh_subdivision_u", Value::Int(u))?;
                tx.set_param(g, "mesh_subdivision_v", Value::Int(2))?;
                Ok(g)
            })
            .unwrap();
        match evaluate(d.scene(), g) {
            Ok(_) | Err(BuildError::Unsupported(_)) => {}
            Err(e) => panic!("loft u={u}: {e}"),
        }
    }
}
