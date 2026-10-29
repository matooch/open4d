//! docs/specs/ANIM.md §1.4–1.6: one test per evaluation rule, plus `scene_at`.

use std::sync::Arc;

use forge_anim::{quantize_time, scene_at, value_at};
use forge_core::{Document, Interp, Key, Registry, Track, Value};
use forge_generators::{Primitives, evaluate};

const F: f64 = 30.0;

fn track(keys: &[(f64, f64)]) -> Track {
    Track::new(keys.iter().map(|(f, v)| Key::new(f / F, *v)).collect())
}

fn at(t: &Track, frame: f64) -> f64 {
    value_at(t, frame / F)
}

#[test]
fn constant_before_and_after_the_keys() {
    let t = track(&[(20.0, 50.0), (40.0, 80.0)]);
    assert_eq!(at(&t, -100.0), 50.0);
    assert_eq!(at(&t, 0.0), 50.0);
    // frames 20 and 40 are not whole milliseconds: 20 rounds to 20.01 frames
    // (`before-after` reads 50.0000226 there), 40 to 39.99
    assert!((at(&t, 20.0) - 50.0).abs() < 1e-4);
    assert!((at(&t, 40.0) - 80.0).abs() < 1e-4);
    assert!(at(&t, 21.0) > 50.0);
    assert_eq!(at(&t, 90.0), 80.0);
    assert_eq!(value_at(&Track::default(), 1.0), 0.0, "empty track reads 0");
    let single = track(&[(10.0, 7.0)]);
    assert_eq!(
        (at(&single, 0.0), at(&single, 10.0), at(&single, 99.0)),
        (7.0, 7.0, 7.0)
    );
}

#[test]
fn step_holds_from_the_key_until_the_next() {
    let keys = |i| {
        Track::new(
            [(0.0, 0.0), (30.0, 100.0), (60.0, 0.0)]
                .iter()
                .map(|(f, v)| Key::new(f / F, *v).with_interp(i))
                .collect(),
        )
    };
    let t = keys(Interp::Step);
    assert_eq!(at(&t, 29.5), 0.0);
    assert_eq!(at(&t, 30.0), 100.0);
    assert_eq!(at(&t, 59.0), 100.0);
    assert_eq!(at(&t, 60.0), 0.0);
    let l = keys(Interp::Linear);
    assert!((at(&l, 15.0) - 50.0).abs() < 1e-9);
    assert!((at(&l, 45.0) - 50.0).abs() < 1e-9);
}

#[test]
fn a_keys_interpolation_governs_the_segment_after_it() {
    let mut t = track(&[(0.0, 0.0), (30.0, 100.0), (60.0, 0.0)]);
    t.keys[1].interp = Interp::Linear;
    assert!((at(&t, 45.0) - 50.0).abs() < 1e-9, "30..60 is linear");
    assert!(
        (at(&t, 31.0) - (100.0 - 100.0 * 0.99 / 30.0)).abs() < 1e-9,
        "ms-rounded time"
    );
    assert!((at(&t, 15.0) - 50.0).abs() < 1e-3);
    assert!(at(&t, 5.0) < 100.0 / 6.0, "0..30 still eases in");
}

#[test]
fn spline_eases_and_stays_inside_the_value_range() {
    let t = track(&[(0.0, 0.0), (30.0, 100.0)]);
    assert!((at(&t, 15.0) - 50.0).abs() < 1e-3);
    assert!(
        at(&t, 3.0) < 3.0 / 30.0 * 100.0,
        "slower than linear at the start"
    );
    assert!(
        at(&t, 27.0) > 27.0 / 30.0 * 100.0,
        "faster than linear near the end"
    );
    let mut last = 0.0;
    for i in 0..=300 {
        let v = at(&t, i as f64 / 10.0);
        assert!(
            (0.0..=100.0).contains(&v) && v >= last - 1e-9,
            "monotone at {i}: {v}"
        );
        last = v;
    }
}

#[test]
fn local_extremes_are_flat_and_never_overshoot() {
    let peak = track(&[(0.0, 0.0), (30.0, 100.0), (60.0, 0.0)]);
    for i in 0..=600 {
        let v = at(&peak, i as f64 / 10.0);
        assert!((-1e-9..=100.0 + 1e-9).contains(&v), "{i}: {v}");
    }
    assert_eq!(at(&peak, 30.0), 100.0);
    assert!(
        (at(&peak, 25.0) - at(&peak, 35.0)).abs() < 1e-3,
        "symmetric around the peak"
    );

    let plateau = track(&[(0.0, 0.0), (30.0, 100.0), (60.0, 100.0), (90.0, 0.0)]);
    for i in 300..=600 {
        assert!((at(&plateau, i as f64 / 10.0) - 100.0).abs() < 1e-8);
    }
}

#[test]
fn monotone_middle_keys_have_a_slope_without_overshoot() {
    let t = track(&[(0.0, 0.0), (30.0, 100.0), (60.0, 150.0)]);
    // central difference: 2.5 per frame at the key, so just after it the value rises
    // faster than a flat tangent would allow.
    let slope = at(&t, 31.0) - at(&t, 30.0);
    assert!((slope - 2.47).abs() < 0.05, "{slope}");
    assert!((at(&t, 31.0) - 102.47).abs() < 0.01);
    for i in 0..=900 {
        let v = at(&t, i as f64 / 10.0);
        assert!((-1e-9..=150.0 + 1e-9).contains(&v), "{i}: {v}");
    }
}

#[test]
fn a_steep_neighbour_cannot_make_a_middle_key_overshoot() {
    // a tiny second segment next to a huge first one: the clamp keeps the curve in range
    let t = track(&[(0.0, 0.0), (30.0, 1000.0), (31.0, 1001.0)]);
    for i in 0..=620 {
        let v = at(&t, i as f64 / 20.0);
        assert!((-1e-9..=1001.0 + 1e-9).contains(&v), "{i}: {v}");
    }
}

#[test]
fn the_curve_does_not_depend_on_scale_or_offset_of_the_values() {
    let a = track(&[(0.0, 0.0), (30.0, 1.0), (60.0, 0.5)]);
    let b = track(&[(0.0, 10.0), (30.0, 1010.0), (60.0, 510.0)]);
    for i in 0..=120 {
        let (x, y) = (at(&a, i as f64 / 2.0), at(&b, i as f64 / 2.0));
        assert!((10.0 + 1000.0 * x - y).abs() < 1e-9, "{i}");
    }
}

#[test]
fn time_is_rounded_to_milliseconds() {
    assert_eq!(quantize_time(1.0 / 30.0), 0.033);
    assert_eq!(quantize_time(2.0 / 30.0), 0.067);
    assert_eq!(quantize_time(0.5), 0.5);
    // on a linear segment the rounding shows directly
    let t = Track::new(vec![
        Key::new(0.0, 0.0).with_interp(Interp::Linear),
        Key::new(1.0, 1000.0),
    ]);
    assert_eq!(value_at(&t, 0.0004), 0.0);
    assert!((value_at(&t, 0.0006) - 1.0).abs() < 1e-9);
}

#[test]
fn a_time_that_rounds_onto_a_key_reads_the_key() {
    // 10 frames = 0.3333.. s rounds to 0.333 s, which is the key's millisecond
    let t = track(&[(0.0, 0.0), (10.0, 100.0), (60.0, 0.0)]);
    assert_eq!(at(&t, 10.0), 100.0);
    // 5 frames = 0.1666.. s rounds up to 0.167 s: just after the key, so still on the curve
    let z = track(&[(0.0, 0.0), (5.0, 100.0), (45.0, 0.0)]);
    let v = at(&z, 5.0);
    assert!(v < 100.0 && v > 99.99, "{v}");
}

#[test]
fn rotation_values_are_not_wrapped() {
    let t = Track::new(vec![Key::new(0.0, 0.0), Key::new(1.0, 270f64.to_radians())]);
    let mid = value_at(&t, 0.5).to_degrees();
    assert!((mid - 135.0).abs() < 1e-3, "{mid}");
    assert!((value_at(&t, 1.0).to_degrees() - 270.0).abs() < 1e-9);
}

#[test]
fn keys_closer_than_a_millisecond_stay_finite() {
    let t = Track::new(vec![
        Key::new(0.0, 0.0),
        Key::new(0.0001, 1.0),
        Key::new(0.0002, 0.0),
        Key::new(1.0, 5.0),
    ]);
    for i in 0..=1000 {
        assert!(value_at(&t, i as f64 / 1000.0).is_finite());
    }
}

fn cube_doc() -> (Document, forge_core::NodeId) {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let cube = doc
        .transact("cube", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    (doc, cube)
}

#[test]
fn scene_at_animates_one_component_of_a_cube() {
    let (mut doc, cube) = cube_doc();
    doc.transact("pos", |tx| {
        tx.set_param(cube, "transform.pos", Value::Vec3([1.0, 2.0, 3.0]))?;
        tx.set_key(
            cube,
            "transform.pos.x",
            Key::new(0.0, 0.0).with_interp(Interp::Linear),
        )?;
        tx.set_key(cube, "transform.pos.x", Key::new(2.0, 100.0))
    })
    .unwrap();
    let scene = doc.scene();
    let pos = |s: &forge_core::Scene| match s.param(cube, "transform.pos") {
        Some(Value::Vec3(v)) => *v,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        pos(&scene_at(scene, 0.0)),
        [0.0, 2.0, 3.0],
        "y and z keep their stored values"
    );
    assert_eq!(pos(&scene_at(scene, 1.0)), [50.0, 2.0, 3.0]);
    assert_eq!(pos(&scene_at(scene, 5.0)), [100.0, 2.0, 3.0]);
    assert_eq!(pos(scene), [1.0, 2.0, 3.0], "the document is untouched");

    // the world matrix follows
    let m = scene_at(scene, 1.0).world_matrix(cube).unwrap();
    assert_eq!(m.w_axis.truncate().to_array(), [50.0, 2.0, 3.0]);

    // geometry evaluates on the copy, and the copy at another time has another revision
    assert!(evaluate(&scene_at(scene, 1.0), cube).unwrap().is_some());
    let (a, b) = (scene_at(scene, 1.0), scene_at(scene, 1.5));
    assert_ne!(a.deep_revision(cube), b.deep_revision(cube));
}

#[test]
fn scene_at_combines_components_and_a_float_param() {
    let (mut doc, cube) = cube_doc();
    doc.transact("anim", |tx| {
        tx.set_key(cube, "transform.pos.y", Key::new(0.0, 0.0))?;
        tx.set_key(cube, "transform.pos.y", Key::new(1.0, 10.0))?;
        tx.set_key(cube, "transform.pos.z", Key::new(0.0, -5.0))?;
        tx.set_key(cube, "transform.rot.x", Key::new(0.0, 0.0))?;
        tx.set_key(cube, "transform.rot.x", Key::new(1.0, 1.0))
    })
    .unwrap();
    let s = scene_at(doc.scene(), 1.0);
    assert_eq!(
        s.param(cube, "transform.pos"),
        Some(&Value::Vec3([0.0, 10.0, -5.0]))
    );
    assert_eq!(
        s.param(cube, "transform.rot"),
        Some(&Value::Vec3([1.0, 0.0, 0.0]))
    );
    let before = scene_at(doc.scene(), -3.0);
    assert_eq!(
        before.param(cube, "transform.pos"),
        Some(&Value::Vec3([0.0, 0.0, -5.0]))
    );
}
