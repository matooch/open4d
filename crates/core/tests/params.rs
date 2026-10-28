//! docs/specs/CORE.md §1.

use std::sync::Arc;

use forge_core::{Category, MeshData, ParamDesc, ParamError, ParamKind, Unit, Value};

#[test]
fn builders_set_kind_default_and_ui() {
    let d = ParamDesc::float("radius", "Radius", 50.0)
        .range(0.0, 100.0)
        .unit(Unit::Distance)
        .group("Object")
        .soft_range(0.0, 10.0)
        .step(0.5);
    assert_eq!(d.id, "radius");
    assert_eq!(d.label, "Radius");
    assert_eq!(d.default, Value::Float(50.0));
    assert_eq!(
        d.kind,
        ParamKind::Float {
            min: 0.0,
            max: 100.0
        }
    );
    assert_eq!(d.ui.unit, Unit::Distance);
    assert_eq!(d.ui.group, "Object");
    assert_eq!(
        (d.ui.soft_min, d.ui.soft_max, d.ui.step),
        (Some(0.0), Some(10.0), Some(0.5))
    );
    assert!(!d.ui.hidden);
    assert!(d.animatable);
}

#[test]
fn animatable_defaults_by_kind() {
    assert!(ParamDesc::bool("a", "A", true).animatable);
    assert!(ParamDesc::int("a", "A", 1).animatable);
    assert!(ParamDesc::vec3("a", "A", [0.0; 3]).animatable);
    assert!(ParamDesc::color("a", "A", [0.0; 3]).animatable);
    assert!(!ParamDesc::string("a", "A", "").animatable);
    assert!(!ParamDesc::enumeration("a", "A", "x", &[("x", "X")]).animatable);
    assert!(!ParamDesc::link("a", "A", &[]).animatable);
    assert!(!ParamDesc::float("a", "A", 0.0).animatable(false).animatable);
    assert!(ParamDesc::string("a", "A", "").hidden().ui.hidden);
}

#[test]
fn numbers_are_clamped_to_hard_range() {
    let f = ParamDesc::float("f", "F", 0.0).range(-1.0, 1.0);
    assert_eq!(f.validate(Value::Float(5.0)), Ok(Value::Float(1.0)));
    assert_eq!(f.validate(Value::Float(-5.0)), Ok(Value::Float(-1.0)));
    assert_eq!(f.validate(Value::Float(0.25)), Ok(Value::Float(0.25)));

    let i = ParamDesc::int("i", "I", 3).range(1, 10);
    assert_eq!(i.validate(Value::Int(0)), Ok(Value::Int(1)));
    assert_eq!(i.validate(Value::Int(11)), Ok(Value::Int(10)));

    let v = ParamDesc::vec3("v", "V", [0.0; 3]).range(0.0, 2.0);
    assert_eq!(
        v.validate(Value::Vec3([-1.0, 1.0, 3.0])),
        Ok(Value::Vec3([0.0, 1.0, 2.0]))
    );
}

#[test]
fn soft_range_does_not_affect_validation() {
    let f = ParamDesc::float("f", "F", 0.0).soft_range(0.0, 1.0);
    assert_eq!(f.validate(Value::Float(50.0)), Ok(Value::Float(50.0)));
}

#[test]
fn int_is_accepted_for_float_other_mismatches_are_errors() {
    let f = ParamDesc::float("f", "F", 0.0);
    assert_eq!(f.validate(Value::Int(3)), Ok(Value::Float(3.0)));
    assert_eq!(f.validate(Value::Bool(true)), Err(ParamError::TypeMismatch));
    let i = ParamDesc::int("i", "I", 0);
    assert_eq!(i.validate(Value::Float(1.0)), Err(ParamError::TypeMismatch));
    let s = ParamDesc::string("s", "S", "");
    assert_eq!(
        s.validate(Value::String("ok".into())),
        Ok(Value::String("ok".into()))
    );
    assert_eq!(
        s.validate(Value::Enum("ok".into())),
        Err(ParamError::TypeMismatch)
    );
}

#[test]
fn non_finite_and_negative_colour_are_rejected() {
    let f = ParamDesc::float("f", "F", 0.0);
    assert_eq!(
        f.validate(Value::Float(f64::NAN)),
        Err(ParamError::NotFinite)
    );
    assert_eq!(
        f.validate(Value::Float(f64::INFINITY)),
        Err(ParamError::NotFinite)
    );
    let v = ParamDesc::vec3("v", "V", [0.0; 3]);
    assert_eq!(
        v.validate(Value::Vec3([0.0, f64::NAN, 0.0])),
        Err(ParamError::NotFinite)
    );
    let c = ParamDesc::color("c", "C", [0.0; 3]);
    assert_eq!(
        c.validate(Value::Color([4.0, 0.5, 0.0])),
        Ok(Value::Color([4.0, 0.5, 0.0]))
    );
    assert_eq!(
        c.validate(Value::Color([-0.1, 0.0, 0.0])),
        Err(ParamError::OutOfRange)
    );
}

#[test]
fn enum_values_must_be_declared_options() {
    let e = ParamDesc::enumeration("e", "E", "a", &[("a", "A"), ("b", "B")]);
    assert_eq!(
        e.validate(Value::Enum("b".into())),
        Ok(Value::Enum("b".into()))
    );
    assert_eq!(
        e.validate(Value::Enum("c".into())),
        Err(ParamError::UnknownOption)
    );
    let ParamKind::Enum { options } = &e.kind else {
        panic!("enum kind")
    };
    assert_eq!(options[1].id, "b");
    assert_eq!(options[1].label, "B");
}

#[test]
fn links_validate_shape_only() {
    let l = ParamDesc::link("l", "L", &[Category::Object]);
    assert_eq!(l.default, Value::Link(None));
    assert_eq!(
        l.kind,
        ParamKind::Link {
            accepts: vec![Category::Object]
        }
    );
    let any = forge_core::NodeId(42);
    assert_eq!(
        l.validate(Value::Link(Some(any))),
        Ok(Value::Link(Some(any)))
    );
}

fn mesh(points: Vec<[f64; 3]>, polys: Vec<Vec<u32>>) -> Value {
    Value::Mesh(Arc::new(MeshData { points, polys }))
}

#[test]
fn mesh_builder_is_hidden_not_animatable_and_empty_by_default() {
    let d = ParamDesc::mesh("mesh", "Mesh");
    assert_eq!(d.kind, ParamKind::Mesh);
    assert!(d.ui.hidden);
    assert!(!d.animatable);
    assert_eq!(d.default, Value::Mesh(Arc::new(MeshData::default())));
    assert_eq!(d.validate(d.default.clone()), Ok(d.default.clone()));
}

#[test]
fn mesh_validation_checks_indices_corners_and_coordinates() {
    let d = ParamDesc::mesh("mesh", "Mesh");
    let tri = vec![[0.0; 3], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
    let ok = mesh(tri.clone(), vec![vec![0, 1, 2]]);
    assert_eq!(d.validate(ok.clone()), Ok(ok));
    // points without polygons are fine
    let loose = mesh(tri.clone(), vec![]);
    assert_eq!(d.validate(loose.clone()), Ok(loose));
    // a repeated index is kept (the cone apex quads), as long as 3 are distinct
    let apex = mesh(tri.clone(), vec![vec![0, 1, 1, 2]]);
    assert_eq!(d.validate(apex.clone()), Ok(apex));
    for (polys, label) in [
        (vec![vec![0, 1, 3]], "index past the points"),
        (vec![vec![0, 1]], "two corners"),
        (vec![vec![]], "no corners"),
        (vec![vec![0, 1, 1]], "only two distinct indices"),
        (vec![vec![2, 2, 2, 2]], "one distinct index"),
    ] {
        assert_eq!(
            d.validate(mesh(tri.clone(), polys)),
            Err(ParamError::OutOfRange),
            "{label}"
        );
    }
    let mut bad = tri;
    bad[1][2] = f64::NAN;
    assert_eq!(
        d.validate(mesh(bad.clone(), vec![vec![0, 1, 2]])),
        Err(ParamError::NotFinite)
    );
    bad[1][2] = f64::INFINITY;
    assert_eq!(d.validate(mesh(bad, vec![])), Err(ParamError::NotFinite));
}

#[test]
fn mesh_and_other_kinds_do_not_mix() {
    let d = ParamDesc::mesh("mesh", "Mesh");
    assert_eq!(d.validate(Value::Bool(true)), Err(ParamError::TypeMismatch));
    let f = ParamDesc::float("f", "F", 0.0);
    assert_eq!(
        f.validate(mesh(vec![], vec![])),
        Err(ParamError::TypeMismatch)
    );
}
