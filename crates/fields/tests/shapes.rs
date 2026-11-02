//! Behaviour of the brief 058 field types that the golden cases do not see (FIELDS.md F6).

use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Tx, Value};
use forge_fields::{FieldError, FieldList, FieldToolset};
use glam::DVec3;

fn doc() -> Document {
    let mut reg = Registry::with_builtins();
    reg.install(&FieldToolset).unwrap();
    Document::new(Arc::new(reg))
}

fn make(doc: &mut Document, ty: &str, params: &[(&str, Value)]) -> NodeId {
    doc.transact("make", |tx: &mut Tx| {
        let n = tx.create(ty, None, None)?;
        for (k, v) in params {
            tx.set_param(n, k, v.clone())?;
        }
        Ok(n)
    })
    .unwrap()
}

fn e(s: &str) -> Value {
    Value::Enum(s.to_string())
}

fn value(doc: &Document, n: NodeId, p: DVec3) -> f64 {
    FieldList::from_nodes(doc.scene(), &[n])
        .unwrap()
        .sample(p, &EvalCtx::default())
        .value
}

#[test]
fn directions_rotate_the_cylinder_cone_and_capsule() {
    let mut d = doc();
    for ty in [
        "forge.field.cylinder",
        "forge.field.cone",
        "forge.field.capsule",
    ] {
        let y = make(&mut d, ty, &[]);
        let x = make(&mut d, ty, &[("direction", e("x_pos"))]);
        let z = make(&mut d, ty, &[("direction", e("z_pos"))]);
        for p in [
            DVec3::new(10.0, -40.0, 20.0),
            DVec3::new(-30.0, 15.0, 5.0),
            DVec3::new(0.0, 60.0, 0.0),
        ] {
            let vy = value(&d, y, p);
            // The axis Y is mapped to X by (x, y, z) -> (y, x, z) and to Z by (z, x, y).
            assert!(
                (value(&d, x, DVec3::new(p.y, p.x, p.z)) - vy).abs() < 1e-12,
                "{ty}"
            );
            assert!(
                (value(&d, z, DVec3::new(p.z, p.x, p.y)) - vy).abs() < 1e-12,
                "{ty}"
            );
        }
    }
}

#[test]
fn a_negative_direction_mirrors_the_cone() {
    let mut d = doc();
    let up = make(&mut d, "forge.field.cone", &[]);
    let down = make(&mut d, "forge.field.cone", &[("direction", e("y_neg"))]);
    for p in [DVec3::new(30.0, -40.0, 0.0), DVec3::new(10.0, 20.0, 5.0)] {
        let m = DVec3::new(p.x, -p.y, p.z);
        assert_eq!(value(&d, up, p), value(&d, down, m));
    }
}

#[test]
fn torus_axes_agree_and_the_tube_is_round() {
    let mut d = doc();
    let y = make(&mut d, "forge.field.torus", &[]);
    let x = make(&mut d, "forge.field.torus", &[("direction", e("x_pos"))]);
    // On the ring: 1. One tube radius away: 0. Half way: inner offset 50 % makes it 1 and then falls.
    assert_eq!(value(&d, y, DVec3::new(100.0, 0.0, 0.0)), 1.0);
    assert_eq!(value(&d, y, DVec3::new(100.0, 35.0, 0.0)), 0.0);
    assert_eq!(
        value(&d, y, DVec3::new(100.0, 0.0, 135.0)),
        value(&d, y, DVec3::new(135.0, 0.0, 0.0))
    );
    assert_eq!(
        value(&d, x, DVec3::new(0.0, 100.0, 0.0)),
        value(&d, y, DVec3::new(100.0, 0.0, 0.0))
    );
}

#[test]
fn capsule_radius_runs_from_bottom_to_top() {
    let mut d = doc();
    let c = make(
        &mut d,
        "forge.field.capsule",
        &[
            ("top_radius", Value::Float(100.0)),
            ("bottom_radius", Value::Float(50.0)),
            ("inner_offset", Value::Float(0.0)),
        ],
    );
    // t = 1 - r / R(a): at the bottom end of the segment R = 50, at the top R = 100.
    assert!((value(&d, c, DVec3::new(25.0, -75.0, 0.0)) - 0.5).abs() < 1e-12);
    assert!((value(&d, c, DVec3::new(50.0, 75.0, 0.0)) - 0.5).abs() < 1e-12);
    // Beyond the end the cap is round.
    assert!((value(&d, c, DVec3::new(0.0, 75.0 + 50.0, 0.0)) - 0.5).abs() < 1e-12);
}

#[test]
fn clip_to_shape_zeroes_outside_for_the_new_shapes() {
    let mut d = doc();
    for (ty, far) in [
        ("forge.field.cylinder", DVec3::new(500.0, 0.0, 0.0)),
        ("forge.field.cone", DVec3::new(500.0, 0.0, 0.0)),
        ("forge.field.torus", DVec3::new(0.0, 0.0, 0.0)),
        ("forge.field.capsule", DVec3::new(0.0, 500.0, 0.0)),
    ] {
        let n = make(
            &mut d,
            ty,
            &[
                ("invert", Value::Bool(true)),
                ("clip_to_shape", Value::Bool(true)),
            ],
        );
        let free = make(&mut d, ty, &[("invert", Value::Bool(true))]);
        assert_eq!(value(&d, free, far), 1.0, "{ty}");
        assert_eq!(value(&d, n, far), 0.0, "{ty}");
    }
}

#[test]
fn radial_axes_follow_the_right_hand_rule() {
    let mut d = doc();
    let half = |axis: &str| {
        (
            ("axis", e(axis)),
            ("end_angle", Value::Float(std::f64::consts::PI)),
        )
    };
    let mut nodes = vec![];
    for axis in ["x", "y", "z"] {
        let (a, b) = half(axis);
        nodes.push(make(&mut d, "forge.field.radial", &[a, b]));
    }
    // The first half turn is the side where the rotation takes the first axis towards the
    // second: Y: x -> -z, X: y -> z, Z: x -> y.
    assert_eq!(value(&d, nodes[1], DVec3::new(1.0, 0.0, -1.0)), 1.0);
    assert_eq!(value(&d, nodes[1], DVec3::new(1.0, 0.0, 1.0)), 0.0);
    assert_eq!(value(&d, nodes[0], DVec3::new(0.0, 1.0, 1.0)), 1.0);
    assert_eq!(value(&d, nodes[0], DVec3::new(0.0, 1.0, -1.0)), 0.0);
    assert_eq!(value(&d, nodes[2], DVec3::new(1.0, 1.0, 0.0)), 1.0);
    assert_eq!(value(&d, nodes[2], DVec3::new(1.0, -1.0, 0.0)), 0.0);
}

#[test]
fn radial_start_transition_and_iterations() {
    let mut d = doc();
    let pi = std::f64::consts::PI;
    let at = |deg: f64| DVec3::new(deg.to_radians().cos(), 0.0, -deg.to_radians().sin());
    let n = make(
        &mut d,
        "forge.field.radial",
        &[
            ("start_angle", Value::Float(pi / 2.0)),
            ("start_transition", Value::Float(pi / 2.0)),
            ("end_angle", Value::Float(1.5 * pi)),
        ],
    );
    assert_eq!(value(&d, n, at(45.0)), 0.0);
    assert!((value(&d, n, at(135.0)) - 0.5).abs() < 1e-12);
    assert_eq!(value(&d, n, at(225.0)), 1.0);
    // Three iterations repeat the half-turn range three times around the circle.
    let it = make(
        &mut d,
        "forge.field.radial",
        &[
            ("end_angle", Value::Float(pi)),
            ("iterations", Value::Int(3)),
        ],
    );
    assert_eq!(value(&d, it, at(10.0)), 1.0);
    assert_eq!(value(&d, it, at(70.0)), 0.0);
    assert_eq!(value(&d, it, at(130.0)), 1.0);
}

#[test]
fn random_is_deterministic_and_the_seed_matters() {
    let mut d = doc();
    let a = make(&mut d, "forge.field.random", &[]);
    let b = make(&mut d, "forge.field.random", &[]);
    let c = make(&mut d, "forge.field.random", &[("seed", Value::Int(7))]);
    let p = DVec3::new(12.0, -34.0, 56.0);
    assert_eq!(value(&d, a, p), value(&d, b, p));
    assert_ne!(value(&d, a, p), value(&d, c, p));
    for mode in ["random", "sorted", "noise"] {
        let n = make(&mut d, "forge.field.random", &[("random_mode", e(mode))]);
        let v = value(&d, n, p);
        assert!((0.0..=1.0).contains(&v), "{mode}: {v}");
    }
}

#[test]
fn random_spaces_follow_the_field_transform() {
    use forge_core::transform;
    let mut d = doc();
    let mk = |d: &mut Document, space: &str| {
        make(
            d,
            "forge.field.random",
            &[
                ("space", e(space)),
                (transform::POS, Value::Vec3([40.0, 0.0, 0.0])),
            ],
        )
    };
    let global = mk(&mut d, "global");
    let field = mk(&mut d, "field");
    let none = mk(&mut d, "none");
    let at_origin = make(&mut d, "forge.field.random", &[]);
    let p = DVec3::new(40.0, 0.0, 0.0);
    // In field space the moved field sees its own origin there, like an unmoved one at the origin.
    assert_eq!(value(&d, field, p), value(&d, at_origin, DVec3::ZERO));
    assert_ne!(value(&d, global, p), value(&d, field, p));
    assert_eq!(
        value(&d, none, p),
        value(&d, none, DVec3::new(1.0, 2.0, 3.0))
    );
}

#[test]
fn sorted_values_ascend_in_sample_order() {
    let mut d = doc();
    let n = make(
        &mut d,
        "forge.field.random",
        &[("random_mode", e("sorted"))],
    );
    let pts: Vec<DVec3> = (0..50)
        .map(|i| DVec3::new(f64::from(i) * 7.0, 0.0, 0.0))
        .collect();
    let list = FieldList::from_nodes(d.scene(), &[n]).unwrap();
    let v: Vec<f64> = list
        .sample_many(&pts, &EvalCtx::default())
        .iter()
        .map(|s| s.value)
        .collect();
    assert!(v.windows(2).all(|w| w[0] <= w[1]));
    assert!(v[49] - v[0] > 0.5);
}

#[test]
fn animation_speed_moves_the_noise() {
    let mut d = doc();
    let still = make(&mut d, "forge.field.random", &[]);
    let moving = make(
        &mut d,
        "forge.field.random",
        &[("animation_speed", Value::Float(1.0))],
    );
    let p = DVec3::new(12.0, -34.0, 56.0);
    let list = |n| FieldList::from_nodes(d.scene(), &[n]).unwrap();
    let at = |t: f64| EvalCtx {
        time: t,
        ..EvalCtx::default()
    };
    assert_eq!(
        list(still).sample(p, &at(0.0)).value,
        list(still).sample(p, &at(2.0)).value
    );
    assert_ne!(
        list(moving).sample(p, &at(0.0)).value,
        list(moving).sample(p, &at(2.0)).value
    );
}

#[test]
fn undefined_noise_types_are_unsupported_and_defined_ones_work() {
    let mut d = doc();
    for (id, _, kind) in forge_fields::NOISE_TYPES {
        let n = make(&mut d, "forge.field.random", &[("noise_type", e(id))]);
        match FieldList::from_nodes(d.scene(), &[n]) {
            Ok(list) => {
                assert!(kind.is_some(), "{id}");
                let v = list
                    .sample(DVec3::new(3.0, 4.0, 5.0), &EvalCtx::default())
                    .value;
                assert!((0.0..=1.0).contains(&v), "{id}: {v}");
            }
            Err(err) => {
                assert!(kind.is_none(), "{id}");
                assert_eq!(err, FieldError::Unsupported("noise type"));
            }
        }
    }
    assert_eq!(
        forge_fields::NOISE_TYPES
            .iter()
            .filter(|t| t.2.is_none())
            .count(),
        16
    );
}
