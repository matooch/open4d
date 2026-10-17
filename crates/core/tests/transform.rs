//! docs/specs/TRANSFORM.md §2–3.

mod common;

use std::collections::BTreeMap;
use std::f64::consts::FRAC_PI_2;

use forge_core::transform::Transform;
use forge_core::{Category, NodeType, ParamDesc, Registry, RegistryError, Unit, Value};
use glam::DVec3;

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).abs().max_element() < 1e-9, "{a} != {b}");
}

#[test]
fn hierarchy_types_get_transform_params_first() {
    let r = common::registry();
    let null = r.get("forge.null").unwrap();
    let ids: Vec<&str> = null.params.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, ["transform.pos", "transform.rot", "transform.scale"]);
    let bx = r.get("test.box").unwrap();
    assert_eq!(bx.params[0].id, "transform.pos");
    assert_eq!(bx.params[3].id, "size");
    let rot = &null.params[1];
    assert_eq!(
        (rot.label.as_str(), rot.ui.unit, rot.ui.group.as_str()),
        ("Rotation", Unit::Angle, "Coordinates")
    );
    assert_eq!(null.params[2].default, Value::Vec3([1.0; 3]));
    assert_eq!(null.params[0].ui.unit, Unit::Distance);
    // tags get none
    assert!(
        r.get("test.sel")
            .unwrap()
            .params
            .iter()
            .all(|p| !p.id.starts_with("transform."))
    );
}

#[test]
fn transform_ids_are_reserved() {
    let t = NodeType::new("a.t", "T", Category::Object).param(ParamDesc::float(
        "transform.x",
        "X",
        0.0,
    ));
    assert!(matches!(
        Registry::new().register(t),
        Err(RegistryError::ReservedParam { .. })
    ));
    let tag =
        NodeType::new("a.tag", "T", Category::Tag).param(ParamDesc::float("transform.x", "X", 0.0));
    assert!(matches!(
        Registry::new().register(tag),
        Err(RegistryError::ReservedParam { .. })
    ));
}

#[test]
fn single_axis_rotations_negate_the_angle() {
    let a = 30f64.to_radians();
    let t = |rot: [f64; 3]| Transform {
        rot: DVec3::from_array(rot),
        ..Transform::IDENTITY
    };
    let v = DVec3::new(100.0, -100.0, -100.0);
    // field 1 (h) about Y: observed (136.6025, -100, -36.6025)
    close(
        t([a, 0.0, 0.0]).matrix().transform_point3(v),
        DVec3::new(136.60254037844385, -100.0, -36.60254037844386),
    );
    let w = DVec3::splat(-100.0);
    // field 2 (p) about X: observed (-100, -136.6025, -36.6025)
    close(
        t([0.0, a, 0.0]).matrix().transform_point3(w),
        DVec3::new(-100.0, -136.60254037844385, -36.60254037844386),
    );
    // field 3 (b) about Z: observed (-136.6025, -36.6025, -100)
    close(
        t([0.0, 0.0, a]).matrix().transform_point3(w),
        DVec3::new(-136.60254037844385, -36.60254037844386, -100.0),
    );
}

#[test]
fn order_is_scale_then_b_p_h_then_translate() {
    let t = Transform {
        pos: DVec3::new(10.0, 20.0, 30.0),
        rot: DVec3::new(30f64.to_radians(), 45f64.to_radians(), 60f64.to_radians()),
        scale: DVec3::ONE,
    };
    // rot-combined vertex 1, then translated
    let want = DVec3::new(-70.00498, -44.82877, -151.95290) + t.pos;
    let got = t.matrix().transform_point3(DVec3::splat(-100.0));
    assert!((got - want).abs().max_element() < 1e-4, "{got}");
    let s = Transform {
        scale: DVec3::new(2.0, 3.0, 4.0),
        ..Transform::IDENTITY
    };
    close(
        s.matrix().transform_point3(DVec3::ONE),
        DVec3::new(2.0, 3.0, 4.0),
    );
    assert_eq!(Transform::IDENTITY.matrix(), glam::DMat4::IDENTITY);
}

#[test]
fn from_params_falls_back_per_component() {
    let mut p = BTreeMap::new();
    p.insert("transform.pos".to_string(), Value::Vec3([1.0, 2.0, 3.0]));
    p.insert("transform.rot".to_string(), Value::Float(9.0)); // wrong kind -> identity
    let t = Transform::from_params(&p);
    assert_eq!(t.pos, DVec3::new(1.0, 2.0, 3.0));
    assert_eq!(t.rot, DVec3::ZERO);
    assert_eq!(t.scale, DVec3::ONE);
}

#[test]
fn world_matrix_composes_parents() {
    let mut doc = common::doc();
    let (parent, child, tag) = doc
        .transact("build", |tx| {
            let parent = tx.create("forge.null", None, None)?;
            tx.set_param(parent, "transform.pos", Value::Vec3([0.0, 50.0, 0.0]))?;
            tx.set_param(parent, "transform.rot", Value::Vec3([FRAC_PI_2, 0.0, 0.0]))?;
            let child = tx.create("forge.null", Some(parent), None)?;
            tx.set_param(child, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]))?;
            let tag = tx.attach_tag("test.sel", child, None)?;
            Ok((parent, child, tag))
        })
        .unwrap();
    let s = doc.scene();
    close(
        s.world_matrix(child).unwrap().transform_point3(DVec3::ZERO),
        DVec3::new(0.0, 50.0, 100.0),
    );
    assert_eq!(
        s.local_transform(parent).unwrap().pos,
        DVec3::new(0.0, 50.0, 0.0)
    );
    assert!(s.local_transform(tag).is_none());
    assert!(s.world_matrix(tag).is_none());
    assert!(s.world_matrix(forge_core::NodeId(999)).is_none());
}
