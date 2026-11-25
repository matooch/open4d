//! The Pose Morph tag (docs/specs/RIG.md section 15), tested against the spec's rules.
//! Own design (D20): there is no reference data, every rule is parity-pending (O37).

use std::sync::Arc;

use forge_anim::scene_at;
use forge_core::{Document, EvalCtx, Interp, Key, NodeId, Registry, Value};
use forge_generators::{Primitives, evaluate, make_editable};
use forge_modifiers::Deformers;
use forge_rig::pose_morph::{self as pm, MAX_TARGETS};
use forge_rig::weights::{WeightMap, bind};
use forge_rig::{POSE_MORPH_TAG, Rig, tools};
use glam::DVec3;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    reg.install(&Rig).unwrap();
    Arc::new(reg)
}

/// A polygon object (a 4-sided cylinder, 2 height segments) with an empty Pose Morph tag.
fn scene() -> (Document, NodeId, NodeId) {
    let mut doc = Document::new(registry());
    let (obj, tag) = doc
        .transact("scene", |tx| {
            let c = tx.create("forge.cylinder", None, None)?;
            tx.set_param(c, "radius", Value::Float(10.0))?;
            tx.set_param(c, "height", Value::Float(100.0))?;
            tx.set_param(c, "height_segments", Value::Int(2))?;
            tx.set_param(c, "rotation_segments", Value::Int(4))?;
            let obj = make_editable(tx, c).unwrap();
            let tag = tx.attach_tag(POSE_MORPH_TAG, obj, None)?;
            Ok((obj, tag))
        })
        .unwrap();
    (doc, obj, tag)
}

fn points(doc: &Document, obj: NodeId) -> Vec<DVec3> {
    points_in(doc.scene(), obj)
}

fn points_in(scene: &forge_core::Scene, obj: NodeId) -> Vec<DVec3> {
    evaluate(scene, obj, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .points
}

/// The unmorphed points of the object (what the tag captures).
fn own_points(doc: &Document, obj: NodeId) -> Vec<DVec3> {
    match doc.scene().param(obj, "mesh") {
        Some(Value::Mesh(m)) => m.points.iter().map(|p| DVec3::from_array(*p)).collect(),
        _ => panic!("no mesh"),
    }
}

fn move_up(pts: &[DVec3], dy: f64) -> Vec<DVec3> {
    // morph A: every point above y = 25 moves up by dy
    pts.iter()
        .map(|p| {
            if p.y > 25.0 {
                *p + DVec3::new(0.0, dy, 0.0)
            } else {
                *p
            }
        })
        .collect()
}

fn stretch_x(pts: &[DVec3]) -> Vec<DVec3> {
    // morph B: x scaled by 1.5
    pts.iter()
        .map(|p| DVec3::new(p.x * 1.5, p.y, p.z))
        .collect()
}

fn set_strength(doc: &mut Document, tag: NodeId, slot: usize, s: f64) {
    doc.transact("strength", |tx| {
        tx.set_param(tag, &pm::strength_id(slot), Value::Float(s))
    })
    .unwrap();
}

fn add(doc: &mut Document, tag: NodeId, name: &str, pts: &[DVec3]) -> usize {
    doc.transact("add target", |tx| {
        if pm::used_slots(tx.scene(), tag).is_empty() && !pm::has_base(tx.scene(), tag) {
            // the base is the object's own points, which the first test case passes as `base`
            let obj = tx.scene().node(tag).unwrap().host.unwrap();
            let own = match tx.scene().param(obj, "mesh") {
                Some(Value::Mesh(m)) => m
                    .points
                    .iter()
                    .map(|p| DVec3::from_array(*p))
                    .collect::<Vec<_>>(),
                _ => unreachable!(),
            };
            pm::capture_base(tx, tag, &own)?;
        }
        pm::add_target(tx, tag, Some(name), pts)
    })
    .unwrap()
}

fn assert_points(a: &[DVec3], b: &[DVec3]) {
    assert_eq!(a.len(), b.len());
    for (i, (p, q)) in a.iter().zip(b).enumerate() {
        assert!(
            (*p - *q).abs().max_element() < 1e-9,
            "point {i}: {p:?} vs {q:?}"
        );
    }
}

#[test]
fn an_empty_tag_and_zero_strengths_leave_the_object_alone() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    assert_points(&points(&doc, obj), &base);
    let a = move_up(&base, 50.0);
    let slot = add(&mut doc, tag, "Up", &a);
    assert_eq!(slot, 0);
    // the base was captured from the object, strength 0
    assert_points(&points(&doc, obj), &base);
}

#[test]
fn one_target_at_0_50_100_and_150_percent() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    let a = move_up(&base, 50.0);
    add(&mut doc, tag, "Up", &a);
    for s in [0.0, 0.5, 1.0, 1.5, -0.5] {
        set_strength(&mut doc, tag, 0, s);
        let expect: Vec<DVec3> = base
            .iter()
            .zip(&a)
            .map(|(b, t)| *b + (*t - *b) * s)
            .collect();
        assert_points(&points(&doc, obj), &expect);
    }
    // at 100 % the moved points are exactly the target, the others exactly the base
    set_strength(&mut doc, tag, 0, 1.0);
    assert_points(&points(&doc, obj), &a);
    set_strength(&mut doc, tag, 0, 1.5);
    let top = points(&doc, obj)
        .iter()
        .map(|p| p.y)
        .fold(f64::MIN, f64::max);
    assert!((top - 125.0).abs() < 1e-9, "top {top}");
}

#[test]
fn two_targets_add_their_offsets() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    let a = move_up(&base, 50.0);
    let b = stretch_x(&base);
    add(&mut doc, tag, "A", &a);
    add(&mut doc, tag, "B", &b);
    set_strength(&mut doc, tag, 0, 1.0);
    set_strength(&mut doc, tag, 1, 0.5);
    let expect: Vec<DVec3> = (0..base.len())
        .map(|k| base[k] + (a[k] - base[k]) + (b[k] - base[k]) * 0.5)
        .collect();
    assert_points(&points(&doc, obj), &expect);
    // a mixed point carries both: a top point moved up and its x grown by half of 50 %
    let k = base.iter().position(|p| p.y > 25.0 && p.x > 1.0).unwrap();
    let got = points(&doc, obj)[k];
    assert!((got.y - (base[k].y + 50.0)).abs() < 1e-9);
    assert!((got.x - base[k].x * 1.25).abs() < 1e-9);
}

#[test]
fn relative_and_absolute_differ_once_the_object_moved_from_the_base() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    let a = move_up(&base, 50.0);
    add(&mut doc, tag, "A", &a);
    set_strength(&mut doc, tag, 0, 1.0);
    // with the object at its base both modes give the target
    doc.transact("mode", |tx| {
        tx.set_param(tag, "mode", Value::Enum("absolute".into()))
    })
    .unwrap();
    assert_points(&points(&doc, obj), &a);
    // the object's own points are edited afterwards: everything moves 7 along x
    let shifted: Vec<DVec3> = base
        .iter()
        .map(|p| *p + DVec3::new(7.0, 0.0, 0.0))
        .collect();
    doc.transact("edit", |tx| {
        let m = forge_core::MeshData {
            points: shifted.iter().map(|p| p.to_array()).collect(),
            polys: match tx.scene().param(obj, "mesh") {
                Some(Value::Mesh(m)) => m.polys.clone(),
                _ => unreachable!(),
            },
        };
        tx.set_param(obj, "mesh", Value::Mesh(Arc::new(m)))
    })
    .unwrap();
    // absolute at 100 %: the stored goal, wherever the object was
    assert_points(&points(&doc, obj), &a);
    set_strength(&mut doc, tag, 0, 0.5);
    let expect_abs: Vec<DVec3> = shifted
        .iter()
        .zip(&a)
        .map(|(c, t)| *c + (*t - *c) * 0.5)
        .collect();
    assert_points(&points(&doc, obj), &expect_abs);
    // relative: the stored offset is added onto the moved points
    doc.transact("mode", |tx| {
        tx.set_param(tag, "mode", Value::Enum("relative".into()))
    })
    .unwrap();
    let expect_rel: Vec<DVec3> = (0..base.len())
        .map(|k| shifted[k] + (a[k] - base[k]) * 0.5)
        .collect();
    assert_points(&points(&doc, obj), &expect_rel);
}

#[test]
fn a_keyed_strength_is_sampled_at_frames() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    let a = move_up(&base, 50.0);
    add(&mut doc, tag, "A", &a);
    doc.transact("keys", |tx| {
        for (t, v) in [(0.0, 0.0), (1.0, 1.0), (2.0, 0.25)] {
            tx.set_key(
                tag,
                &pm::strength_id(0),
                Key {
                    time: t,
                    value: v,
                    interp: Interp::Linear,
                },
            )?;
        }
        Ok(())
    })
    .unwrap();
    let top = |t: f64| {
        let s = scene_at(doc.scene(), t);
        points_in(&s, obj)
            .iter()
            .map(|p| p.y)
            .fold(f64::MIN, f64::max)
    };
    assert!((top(0.0) - 50.0).abs() < 1e-9);
    assert!((top(0.5) - 75.0).abs() < 1e-9);
    assert!((top(1.0) - 100.0).abs() < 1e-9);
    assert!((top(1.5) - (50.0 + 50.0 * 0.625)).abs() < 1e-9);
    assert!((top(2.0) - (50.0 + 12.5)).abs() < 1e-9);
}

#[test]
fn a_morph_under_a_skin_is_skinned_after_morphing() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    let (joints, wtag) = doc
        .transact("rig", |tx| {
            let joints = tools::create_chain(
                tx,
                None,
                &[
                    DVec3::new(0.0, -50.0, 0.0),
                    DVec3::ZERO,
                    DVec3::new(0.0, 50.0, 0.0),
                ],
                "J",
                None,
            )?;
            let wtag = tx.attach_tag(forge_rig::WEIGHT_TAG, obj, None)?;
            tx.set_param(wtag, "joints", Value::LinkList(joints.clone()))?;
            tx.create(forge_rig::SKIN, Some(obj), None)?;
            Ok((joints, wtag))
        })
        .unwrap();
    // every point follows the middle joint alone
    let dense: Vec<Vec<f64>> = base.iter().map(|_| vec![0.0, 1.0, 0.0]).collect();
    doc.transact("weights", |tx| {
        tx.set_param(wtag, "weights", WeightMap::from_dense(&dense).to_value())?;
        bind(tx, wtag)
    })
    .unwrap();
    let a = move_up(&base, 50.0);
    add(&mut doc, tag, "A", &a);
    set_strength(&mut doc, tag, 0, 1.0);
    // the middle joint moves 5 along x: skin of the morphed points
    doc.transact("pose", |tx| {
        tx.set_param(joints[1], "transform.pos", Value::Vec3([5.0, 50.0, 0.0]))
    })
    .unwrap();
    let expect: Vec<DVec3> = a.iter().map(|p| *p + DVec3::new(5.0, 0.0, 0.0)).collect();
    assert_points(&points(&doc, obj), &expect);
    // morph off: the skinned base
    set_strength(&mut doc, tag, 0, 0.0);
    let expect: Vec<DVec3> = base
        .iter()
        .map(|p| *p + DVec3::new(5.0, 0.0, 0.0))
        .collect();
    assert_points(&points(&doc, obj), &expect);
}

#[test]
fn a_disabled_tag_or_a_topology_change_does_nothing() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    add(&mut doc, tag, "A", &move_up(&base, 50.0));
    set_strength(&mut doc, tag, 0, 1.0);
    doc.transact("off", |tx| tx.set_param(tag, "enabled", Value::Bool(false)))
        .unwrap();
    assert_points(&points(&doc, obj), &base);
    doc.transact("on", |tx| tx.set_param(tag, "enabled", Value::Bool(true)))
        .unwrap();
    // fewer points than the base: untouched
    let mut pts = base.clone();
    pts.pop();
    let mut small = pts.clone();
    pm::morph_points(
        &mut small,
        &base,
        &[(&move_up(&base, 50.0), 1.0)],
        pm::Mode::Relative,
    );
    assert_points(&small, &pts);
}

#[test]
fn targets_are_made_edited_and_cleared_with_undo() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    let depth = doc.undo_depth();
    let a = move_up(&base, 50.0);
    let slot = add(&mut doc, tag, "Up", &a);
    assert_eq!(doc.undo_depth(), depth + 1, "one undo step");
    assert_eq!(pm::used_slots(doc.scene(), tag), vec![0]);
    assert!(matches!(doc.scene().param(tag, "name_0"), Some(Value::String(s)) if s == "Up"));
    // edit
    let a2 = move_up(&base, 80.0);
    doc.transact("edit", |tx| pm::set_target(tx, tag, slot, &a2))
        .unwrap();
    set_strength(&mut doc, tag, 0, 1.0);
    assert_points(&points(&doc, obj), &a2);
    // wrong point count
    let r = doc.transact("bad", |tx| pm::set_target(tx, tag, slot, &a2[1..]));
    assert!(r.is_err());
    let r = doc.transact("bad", |tx| pm::add_target(tx, tag, None, &a2[1..]));
    assert!(r.is_err());
    // no base, no target
    let (mut doc2, _, tag2) = scene();
    assert!(
        doc2.transact("nobase", |tx| pm::add_target(tx, tag2, None, &a2))
            .is_err()
    );
    // an empty slot cannot be edited
    assert!(
        doc.transact("bad", |tx| pm::set_target(tx, tag, 3, &a2))
            .is_err()
    );
    // clear
    doc.transact("clear", |tx| pm::clear_target(tx, tag, 0))
        .unwrap();
    assert!(pm::used_slots(doc.scene(), tag).is_empty());
    assert_points(&points(&doc, obj), &base);
    // undo the clear, the edit: the old state comes back
    assert!(doc.undo());
    assert_points(&points(&doc, obj), &a2);
    assert!(doc.undo()); // strength
    assert!(doc.undo()); // edit
    set_strength(&mut doc, tag, 0, 1.0);
    assert_points(&points(&doc, obj), &a);
    // undo back through the target creation
    assert!(doc.undo());
    assert!(doc.undo());
    assert!(pm::used_slots(doc.scene(), tag).is_empty());
    assert_eq!(doc.undo_depth(), depth);
}

#[test]
fn the_slots_run_out_at_the_limit() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    for i in 0..MAX_TARGETS {
        assert_eq!(add(&mut doc, tag, "m", &base), i);
    }
    assert!(
        doc.transact("full", |tx| pm::add_target(tx, tag, None, &base))
            .is_err()
    );
    doc.transact("clear", |tx| pm::clear_target(tx, tag, 5))
        .unwrap();
    assert_eq!(add(&mut doc, tag, "again", &base), 5);
}

#[test]
fn the_tag_survives_a_file_round_trip() {
    let (mut doc, obj, tag) = scene();
    let base = own_points(&doc, obj);
    add(&mut doc, tag, "Up", &move_up(&base, 50.0));
    add(&mut doc, tag, "Wide", &stretch_x(&base));
    set_strength(&mut doc, tag, 0, 0.75);
    set_strength(&mut doc, tag, 1, -0.25);
    doc.transact("mode", |tx| {
        tx.set_param(tag, "mode", Value::Enum("absolute".into()))
    })
    .unwrap();
    doc.transact("key", |tx| {
        tx.set_key(
            tag,
            &pm::strength_id(1),
            Key {
                time: 0.0,
                value: 0.0,
                interp: Interp::Linear,
            },
        )?;
        tx.set_key(
            tag,
            &pm::strength_id(1),
            Key {
                time: 1.0,
                value: 1.0,
                interp: Interp::Linear,
            },
        )
    })
    .unwrap();
    let text = forge_core::file::save(doc.scene());
    let (loaded, _) = forge_core::file::load(registry(), &text).unwrap();
    for id in [
        "mode",
        "base",
        "target_0",
        "target_1",
        "name_0",
        "name_1",
        "strength_0",
        "strength_1",
    ] {
        assert_eq!(doc.scene().param(tag, id), loaded.param(tag, id), "{id}");
    }
    assert_eq!(
        doc.scene().node(tag).unwrap().tracks,
        loaded.node(tag).unwrap().tracks
    );
    assert_points(&points_in(&loaded, obj), &points(&doc, obj));
}

#[test]
fn the_descriptors_are_data_driven() {
    let reg = registry();
    let ty = reg.get(POSE_MORPH_TAG).unwrap();
    for i in 0..MAX_TARGETS {
        let d = ty.find_param(&pm::strength_id(i)).unwrap();
        assert!(d.animatable);
        assert!(!d.ui.hidden);
        assert!(ty.find_param(&pm::target_id(i)).unwrap().ui.hidden);
    }
    assert!(ty.find_param("mode").is_some() && ty.find_param("base").is_some());
}
