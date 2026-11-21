//! docs/specs/ANIM.md §1.5b, §1.8–1.11: tracks for every value type, point animation,
//! behaviours, tangent presets, key reduction, bake, rotation.

use std::sync::Arc;

use forge_anim::{
    bake_param, euler_filter, int_at, points_at, quaternion_rotation_at, reduce_keys, scene_at,
    speed_at, step_at, tangent, value_at_exact,
};
use forge_core::{
    Behavior, Document, Interp, Key, KeyTangent, MeshData, NodeId, Registry, TangentMode, Track,
    TrackError, Value, file, parse_track_target,
};
use forge_generators::Primitives;
use glam::DVec3;

fn doc() -> Document {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Document::new(Arc::new(reg))
}

fn create(doc: &mut Document, ty: &str) -> NodeId {
    doc.transact("create", |tx| tx.create(ty, None, None))
        .unwrap()
}

fn step(keys: &[(f64, f64)]) -> Track {
    Track::new(
        keys.iter()
            .map(|(t, v)| Key::new(*t, *v).with_interp(Interp::Step))
            .collect(),
    )
}

#[test]
fn target_kinds_resolve_against_the_descriptors() {
    let d = doc();
    let reg = d.scene().registry();
    let cog = reg.get("forge.cogwheel_spline").unwrap();
    let find = |p: &str| cog.find_param(p);
    use forge_core::TrackTarget as T;
    assert_eq!(
        parse_track_target("show_guides", find),
        Ok(T::Bool("show_guides".into()))
    );
    assert_eq!(
        parse_track_target("guide_color.g", find),
        Ok(T::Channel("guide_color".into(), 1))
    );
    assert!(
        parse_track_target("guide_color", find).is_err(),
        "no whole colour"
    );
    assert!(parse_track_target("guide_color.x", find).is_err());
    let boole = reg.get("forge.boole").unwrap();
    assert_eq!(
        parse_track_target("mode", |p| boole.find_param(p)),
        Ok(T::Enum("mode".into()))
    );
    let poly = reg.get("forge.polygon").unwrap();
    assert_eq!(
        parse_track_target("mesh.points", |p| poly.find_param(p)),
        Ok(T::Points("mesh".into()))
    );
    assert!(parse_track_target("mesh", |p| poly.find_param(p)).is_err());
}

#[test]
fn bool_enum_colour_int_and_link_tracks_drive_their_parameters() {
    let mut d = doc();
    let cog = create(&mut d, "forge.cogwheel_spline");
    let boole = create(&mut d, "forge.boole");
    let cube = create(&mut d, "forge.cube");
    let inst = create(&mut d, "forge.instance");
    d.transact("keys", |tx| {
        tx.set_track(cog, "show_guides", Some(step(&[(0.0, 0.0), (1.0, 1.0)])))?;
        tx.set_track(boole, "mode", Some(step(&[(0.0, 0.0), (1.0, 2.0)])))?;
        tx.set_track(
            cube,
            "segments_x",
            Some(Track::new(vec![
                Key::new(0.0, 1.0).with_interp(Interp::Linear),
                Key::new(1.0, 10.0),
            ])),
        )?;
        tx.set_key(
            cog,
            "guide_color.g",
            Key::new(0.0, 0.0).with_interp(Interp::Linear),
        )?;
        tx.set_key(cog, "guide_color.g", Key::new(1.0, 1.0))?;
        tx.set_track(
            inst,
            "reference",
            Some(step(&[(0.0, 0.0), (1.0, cube.0 as f64)])),
        )
    })
    .unwrap();
    let s0 = scene_at(d.scene(), 0.5);
    assert_eq!(s0.param(cog, "show_guides"), Some(&Value::Bool(false)));
    assert_eq!(s0.param(boole, "mode"), Some(&Value::Enum("union".into())));
    assert_eq!(s0.param(cube, "segments_x"), Some(&Value::Int(5)));
    assert_eq!(s0.param(inst, "reference"), Some(&Value::Link(None)));
    match s0.param(cog, "guide_color") {
        Some(Value::Color(c)) => {
            assert!((c[1] - 0.5).abs() < 1e-9);
            assert!((c[0] - 1.0).abs() < 1e-9, "red keeps the stored value");
        }
        other => panic!("{other:?}"),
    }
    let s1 = scene_at(d.scene(), 1.0);
    assert_eq!(s1.param(cog, "show_guides"), Some(&Value::Bool(true)));
    assert_eq!(
        s1.param(boole, "mode"),
        Some(&Value::Enum("intersect".into()))
    );
    assert_eq!(s1.param(cube, "segments_x"), Some(&Value::Int(10)));
    assert_eq!(s1.param(inst, "reference"), Some(&Value::Link(Some(cube))));
    // an enum index past the options is ignored
    d.transact("bad", |tx| {
        tx.set_track(boole, "mode", Some(step(&[(0.0, 17.0)])))
    })
    .unwrap();
    assert_eq!(
        scene_at(d.scene(), 0.0).param(boole, "mode"),
        d.scene().param(boole, "mode")
    );
}

#[test]
fn steps_ignore_the_key_interpolation() {
    let t = Track::new(vec![Key::new(0.0, 0.0), Key::new(1.0, 1.0)]);
    assert_eq!(step_at(&t, 0.99), 0.0);
    assert_eq!(step_at(&t, 1.0), 1.0);
    assert_eq!(step_at(&t, -5.0), 0.0, "the first key holds before it");
}

#[test]
fn int_tracks_floor_the_line_with_a_tolerance() {
    let mut t = Track::new(vec![Key::new(0.0, 1.0), Key::new(1.0, 10.0)]);
    for i in 0..2 {
        t.set_tangent(i, KeyTangent::chord());
    }
    let at = |f: f64| int_at(&t, f / 30.0, true);
    assert_eq!(
        (at(0.0), at(3.0), at(4.0), at(10.0), at(13.0), at(30.0)),
        (1, 1, 2, 4, 4, 10)
    );
}

#[test]
fn point_level_tracks_blend_whole_arrays() {
    let mut d = doc();
    let poly = create(&mut d, "forge.polygon");
    let rest = MeshData {
        points: vec![[0.0, 0.0, 0.0], [100.0, 0.0, 0.0], [0.0, 0.0, 100.0]],
        polys: vec![vec![0, 1, 2]],
    };
    let mut moved = rest.points.clone();
    moved[0] = [0.0, 100.0, 0.0];
    let mut t = Track::default();
    assert!(t.set_points_key(0.0, Interp::Linear, rest.points.clone()));
    assert!(t.set_points_key(1.0, Interp::Linear, moved));
    assert!(
        !t.set_points_key(2.0, Interp::Linear, vec![[0.0; 3]]),
        "point count must match"
    );
    assert_eq!(t.validate(), Ok(()));
    assert_eq!(points_at(&t, 0.5).unwrap()[0], [0.0, 50.0, 0.0]);
    assert_eq!(points_at(&t, 9.0).unwrap()[0], [0.0, 100.0, 0.0]);
    d.transact("mesh", |tx| {
        tx.set_param(poly, "mesh", Value::Mesh(Arc::new(rest.clone())))?;
        tx.set_track(poly, "mesh.points", Some(t.clone()))
    })
    .unwrap();
    let Some(Value::Mesh(m)) = scene_at(d.scene(), 0.5).param(poly, "mesh").cloned() else {
        panic!("mesh");
    };
    assert_eq!(m.points[0], [0.0, 50.0, 0.0]);
    assert_eq!(m.points[1], [100.0, 0.0, 0.0]);
    assert_eq!(m.polys, rest.polys, "polygons are kept");
    // inconsistent side data is rejected
    let mut bad = t.clone();
    bad.points.pop();
    assert_eq!(bad.validate(), Err(TrackError::SideData));
    // keys inserted or removed keep the side data aligned
    let mut t2 = t.clone();
    t2.set_key(Key::new(0.5, 0.0));
    assert_eq!(t2.validate(), Ok(()));
    assert!(t2.remove_key(0.0));
    assert_eq!(t2.validate(), Ok(()));
}

#[test]
fn the_new_track_data_survives_a_save_and_load() {
    let mut d = doc();
    let cube = create(&mut d, "forge.cube");
    let cog = create(&mut d, "forge.cogwheel_spline");
    let mut t = Track::new(vec![
        Key::new(0.0, 0.0),
        Key::new(1.0, 5.0),
        Key::new(2.0, 1.0),
    ]);
    t.set_tangent(1, KeyTangent::user((-0.3, -1.0), (0.4, 2.0)));
    t.before = Behavior::Repeat;
    t.after = Behavior::OffsetRepeat;
    d.transact("keys", |tx| {
        tx.set_track(cube, "transform.pos.x", Some(t.clone()))?;
        tx.set_track(cog, "show_guides", Some(step(&[(0.0, 0.0), (1.0, 1.0)])))
    })
    .unwrap();
    let text = file::save(d.scene());
    let (scene, report) = file::load(d.scene().registry().clone(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(scene.node(cube).unwrap().tracks["transform.pos.x"], t);
    assert_eq!(file::save(&scene), text);
}

fn base_track() -> Track {
    // 0 -> 100 over 30 frames at 30 fps, straight line, so the cycle values are easy
    let mut t = Track::new(vec![Key::new(0.0, 0.0), Key::new(1.0, 100.0)]);
    for i in 0..2 {
        t.set_tangent(i, KeyTangent::chord());
    }
    t
}

#[test]
fn behaviours_remap_time() {
    let at = |t: &Track, f: f64| value_at_exact(t, &[], f / 30.0);
    let mut t = base_track();
    t.after = Behavior::Repeat;
    assert!((at(&t, 45.0) - 50.0).abs() < 1e-3);
    assert!(
        (at(&t, 60.0) - 100.0).abs() < 1e-3,
        "a boundary reads the cycle end"
    );
    t.after = Behavior::OffsetRepeat;
    assert!((at(&t, 45.0) - 150.0).abs() < 1e-3);
    assert!((at(&t, 60.0) - 200.0).abs() < 1e-3);
    t.after = Behavior::Oscillate;
    assert!((at(&t, 45.0) - 50.0).abs() < 1e-3);
    assert!(at(&t, 60.0).abs() < 1e-3);
    assert!((at(&t, 90.0) - 100.0).abs() < 1e-3);
    t.after = Behavior::Continue;
    assert!(
        (at(&t, 45.0) - 150.0).abs() < 1e-3,
        "the chord handle keeps its slope"
    );
    t.after = Behavior::Constant;
    assert_eq!(at(&t, 45.0), 100.0);
    t.before = Behavior::Repeat;
    assert!((at(&t, -15.0) - 50.0).abs() < 1e-3);
    assert!((at(&t, -30.0) - 100.0).abs() < 1e-3);
    t.before = Behavior::OffsetRepeat;
    assert!((at(&t, -15.0) + 50.0).abs() < 1e-3);
    t.before = Behavior::Oscillate;
    assert!((at(&t, -15.0) - 50.0).abs() < 1e-3);
    assert!((at(&t, -45.0) - 50.0).abs() < 1e-3);
    t.before = Behavior::Continue;
    assert!((at(&t, -30.0) + 100.0).abs() < 1e-3);
    // a single key never repeats
    let mut one = Track::new(vec![Key::new(1.0, 7.0)]);
    one.after = Behavior::Repeat;
    assert_eq!(value_at_exact(&one, &[], 5.0), 7.0);
}

#[test]
fn tangent_presets_and_handle_edits() {
    let t = Track::new(vec![
        Key::new(0.0, 0.0),
        Key::new(0.3, 30.0),
        Key::new(0.9, 0.0),
    ]);
    let e = tangent::ease(&t.keys, 1);
    assert!((e.left.dt + 0.1).abs() < 1e-12 && (e.right.dt - 0.2).abs() < 1e-12);
    assert_eq!((e.left.dv, e.right.dv), (0.0, 0.0));
    assert_eq!(e.left.mode, TangentMode::User);
    let a = KeyTangent::user((-0.05, -10.0), (0.3, 20.0));
    let ei = tangent::ease_in(&t.keys, 1, a);
    assert_eq!((ei.left.dv, ei.right.dv), (0.0, 20.0));
    let eo = tangent::ease_out(&t.keys, 1, a);
    assert_eq!((eo.left.dv, eo.right.dv), (-10.0, 0.0));
    let za = tangent::zero_angle(&t.keys, 1, a);
    assert_eq!(
        (za.left.dt, za.left.dv, za.right.dt, za.right.dv),
        (-0.05, 0.0, 0.3, 0.0)
    );
    let zl = tangent::zero_length(a);
    assert_eq!((zl.left.dt, zl.right.dt, zl.right.dv), (0.0, 0.0, 0.0));
    // unbroken unweighted: the right handle keeps a third of the span, the left mirrors the slope
    let r = tangent::set_right_handle(&t.keys, 1, KeyTangent::default(), 0.05, 10.0);
    assert!((r.right.dt - 0.2).abs() < 1e-12 && (r.right.dv - 40.0).abs() < 1e-9);
    assert!((r.left.dt + 0.1).abs() < 1e-12 && (r.left.dv + 20.0).abs() < 1e-9);
    // broken: the left side is untouched
    let broken = KeyTangent {
        broken: true,
        left: a.left,
        ..KeyTangent::default()
    };
    assert_eq!(
        tangent::set_right_handle(&t.keys, 1, broken, 0.05, 10.0).left,
        a.left
    );
    // weighted: the stored time component is free
    let w = KeyTangent {
        weighted: true,
        ..KeyTangent::default()
    };
    let r = tangent::set_right_handle(&t.keys, 1, w, 0.05, 10.0);
    assert!((r.right.dt - 0.05).abs() < 1e-12 && (r.right.dv - 10.0).abs() < 1e-9);
    // an edited handle changes the curve: a flat user handle at the first key is an ease
    let mut tr = base_track();
    tr.set_tangent(0, KeyTangent::user((0.0, 0.0), (1.0 / 3.0, 0.0)));
    assert!(value_at_exact(&tr, &[], 0.1) < 10.0);
}

#[test]
fn speed_is_the_derivative_of_the_curve() {
    let t = base_track();
    assert!(
        (speed_at(&t, 0.4) - 100.0).abs() < 1e-3,
        "chord handles: constant speed"
    );
    let ease = Track::new(vec![Key::new(0.0, 0.0), Key::new(1.0, 100.0)]);
    assert!(
        (speed_at(&ease, 0.5) - 150.0).abs() < 1e-3,
        "an ease peaks at 1.5x the mean"
    );
    assert!(speed_at(&ease, 0.01) < 10.0);
    assert_eq!(speed_at(&ease, 5.0), 0.0);
    let lin = Track::new(vec![
        Key::new(0.0, 0.0).with_interp(Interp::Linear),
        Key::new(2.0, 10.0),
    ]);
    assert_eq!(speed_at(&lin, 1.0), 5.0);
    assert_eq!(speed_at(&step(&[(0.0, 0.0), (1.0, 4.0)]), 0.5), 0.0);
    // the numerical derivative agrees
    let h = 1e-4;
    let num =
        (value_at_exact(&ease, &[], 0.3 + h) - value_at_exact(&ease, &[], 0.3 - h)) / (2.0 * h);
    assert!((num - speed_at(&ease, 0.3)).abs() < 1.0);
}

#[test]
fn reduction_drops_keys_on_the_curve_and_keeps_the_shape() {
    let lin = |f: f64| f64::from(f as i32);
    let keys: Vec<Key> = (0..=10)
        .map(|i| Key::new(f64::from(i), lin(f64::from(i)) * 2.0).with_interp(Interp::Linear))
        .collect();
    let r = reduce_keys(&Track::new(keys), 1e-9);
    assert_eq!(r.keys.len(), 2, "a straight line needs two keys");
    // a peak survives
    let peak: Vec<Key> = [0.0, 1.0, 4.0, 1.0, 0.0]
        .iter()
        .enumerate()
        .map(|(i, v)| Key::new(i as f64, *v).with_interp(Interp::Linear))
        .collect();
    let r = reduce_keys(&Track::new(peak.clone()), 0.1);
    assert!(r.keys.iter().any(|k| k.value == 4.0));
    let loose = reduce_keys(&Track::new(peak), 100.0);
    assert_eq!(loose.keys.len(), 2);
}

#[test]
fn bake_samples_the_animated_scene() {
    let mut d = doc();
    let cube = create(&mut d, "forge.cube");
    d.transact("key", |tx| {
        tx.set_key(
            cube,
            "transform.pos.x",
            Key::new(0.0, 0.0).with_interp(Interp::Linear),
        )?;
        tx.set_key(cube, "transform.pos.x", Key::new(1.0, 100.0))
    })
    .unwrap();
    let times: Vec<f64> = (0..=4).map(|i| f64::from(i) * 0.25).collect();
    let t = bake_param(d.scene(), cube, "transform.pos", Some(0), &times).unwrap();
    assert_eq!(t.keys.len(), 5);
    assert!((t.keys[2].value - 50.0).abs() < 1e-9);
    assert!(t.keys.iter().all(|k| k.interp == Interp::Linear));
    assert!(bake_param(d.scene(), cube, "transform.pos", None, &times).is_none());
    assert!(reduce_keys(&t, 1e-9).keys.len() == 2);
}

#[test]
fn euler_filter_unwraps_adjacent_keys() {
    let deg = f64::to_radians;
    let mut t = Track::new(vec![
        Key::new(0.0, deg(170.0)),
        Key::new(1.0, deg(-170.0)),
        Key::new(2.0, deg(170.0)),
    ]);
    assert!(euler_filter(&mut t));
    let v: Vec<f64> = t
        .keys
        .iter()
        .map(|k| k.value.to_degrees().round())
        .collect();
    assert_eq!(v, vec![170.0, 190.0, 170.0]);
    assert!(!euler_filter(&mut t), "idempotent");
}

#[test]
fn quaternion_tracks_take_the_short_way() {
    let deg = f64::to_radians;
    let h = Track::new(vec![Key::new(0.0, deg(170.0)), Key::new(1.0, deg(-170.0))]);
    // Euler: through 0
    assert!(value_at_exact(&h, &[], 0.5).abs() < 1e-3);
    // quaternion: through 180
    let r = quaternion_rotation_at([Some(&h), None, None], DVec3::ZERO, 0.5);
    assert!((r.x.abs().to_degrees() - 180.0).abs() < 1e-3, "{r:?}");
    assert!(r.y.abs() < 1e-9 && r.z.abs() < 1e-9);
    // the ends are the key values
    let r0 = quaternion_rotation_at([Some(&h), None, None], DVec3::ZERO, 0.0);
    assert!((r0.x.to_degrees() - 170.0).abs() < 1e-9);
    // and scene_at uses it when the flag is set
    let mut d = doc();
    let cube = create(&mut d, "forge.cube");
    let mut q = h.clone();
    q.quaternion = true;
    d.transact("rot", |tx| {
        tx.set_track(cube, "transform.rot.x", Some(q.clone()))
    })
    .unwrap();
    let Some(Value::Vec3(rot)) = scene_at(d.scene(), 0.5)
        .param(cube, "transform.rot")
        .cloned()
    else {
        panic!("rot");
    };
    assert!((rot[0].abs().to_degrees() - 180.0).abs() < 1.0, "{rot:?}");
    // without the flag it goes the long way
    d.transact("rot", |tx| {
        tx.set_track(cube, "transform.rot.x", Some(h.clone()))
    })
    .unwrap();
    let Some(Value::Vec3(rot)) = scene_at(d.scene(), 0.5)
        .param(cube, "transform.rot")
        .cloned()
    else {
        panic!("rot");
    };
    assert!(rot[0].abs().to_degrees() < 5.0);
}
