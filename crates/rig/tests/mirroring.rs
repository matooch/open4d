//! Rig mirroring (docs/specs/RIG.md §5): a mirrored rig with mirrored weights deforms
//! symmetrically.

use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_rig::mirror::{
    Plane, RenameRule, Side, add_joints_to_tag, joint_mirror_map, mirror_chain, mirror_tag_weights,
    mirror_weights,
};
use forge_rig::weights::{WeightMap, bind, read_weights};
use forge_rig::{Rig, tools};
use glam::DVec3;

struct Scene {
    doc: Document,
    body: NodeId,
    tag: NodeId,
    left: Vec<NodeId>,
    right: Vec<NodeId>,
}

/// A cube body (100 wide, 4 segments each way, so points sit on the x = 0 plane), a centre
/// joint and a left arm chain; the weights are authored on the left half only.
fn build() -> Scene {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    reg.install(&Rig).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let (body, tag, spine, left) = doc
        .transact("scene", |tx| {
            let body = tx.create("forge.cube", None, None)?;
            for k in ["size_x", "size_y", "size_z"] {
                tx.set_param(body, k, Value::Float(100.0))?;
            }
            for k in ["segments_x", "segments_y", "segments_z"] {
                tx.set_param(body, k, Value::Int(4))?;
            }
            let spine =
                tools::create_chain(tx, None, &[DVec3::new(0.0, -50.0, 0.0)], "spine", None)?[0];
            let left = tools::create_chain(
                tx,
                None,
                &[DVec3::new(25.0, 0.0, 0.0), DVec3::new(75.0, 0.0, 0.0)],
                "arm_L_",
                None,
            )?;
            let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, body, None)?;
            let mut all = vec![spine];
            all.extend(&left);
            tx.set_param(tag, "joints", Value::LinkList(all))?;
            tx.create(forge_rig::SKIN, Some(body), None)?;
            Ok((body, tag, spine, left))
        })
        .unwrap();
    // weights on the left half: arm_L_1 grows with x, the spine takes the rest; centre and
    // right points are the spine's
    // no weights and no bind yet: the skin leaves the cube alone
    let mesh = evaluate(doc.scene(), body, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let dense: Vec<Vec<f64>> = mesh
        .points
        .iter()
        .map(|p| {
            if p.x > 1e-9 {
                let a = (p.x / 50.0).clamp(0.0, 1.0);
                vec![1.0 - a, a * 0.5, a * 0.5]
            } else {
                vec![1.0, 0.0, 0.0]
            }
        })
        .collect();
    doc.transact("w", |tx| {
        tx.set_param(tag, "weights", WeightMap::from_dense(&dense).to_value())?;
        bind(tx, tag)
    })
    .unwrap();
    // mirror the arm and register the copy with the tag
    let right_root = doc
        .transact("mirror", |tx| {
            mirror_chain(tx, left[0], Plane::Yz, &RenameRule::LeftRight)
        })
        .unwrap();
    let right: Vec<NodeId> = {
        let s = doc.scene();
        let mut v = vec![right_root];
        v.extend(s.node(right_root).unwrap().children.clone());
        v
    };
    let _ = spine;
    Scene {
        doc,
        body,
        tag,
        left,
        right,
    }
}

#[test]
fn a_mirrored_rig_deforms_symmetrically() {
    let mut s = build();
    assert_eq!(s.doc.scene().node(s.right[0]).unwrap().name, "arm_R_1");
    let mesh = evaluate(s.doc.scene(), s.body, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let right = s.right.clone();
    let tag = s.tag;
    let report = s
        .doc
        .transact("w", |tx| {
            add_joints_to_tag(tx, tag, &right)?;
            let r = mirror_tag_weights(
                tx,
                tag,
                &mesh,
                Plane::Yz,
                &RenameRule::LeftRight,
                Side::Positive,
            )?;
            bind(tx, tag)?;
            Ok(r)
        })
        .unwrap();
    assert_eq!(report.unmatched, 0);
    assert!(report.copied > 0);
    // pose: left arm B +0.6, the mirrored arm B -0.6 (a reflection flips the sense of a
    // rotation about z)
    let (l0, r0) = (s.left[0], s.right[0]);
    s.doc
        .transact("pose", |tx| {
            tx.set_param(l0, "transform.rot", Value::Vec3([0.0, 0.0, 0.6]))?;
            tx.set_param(r0, "transform.rot", Value::Vec3([0.0, 0.0, -0.6]))
        })
        .unwrap();
    let posed = evaluate(s.doc.scene(), s.body, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let moved = posed
        .points
        .iter()
        .zip(&mesh.points)
        .filter(|(a, b)| (**a - **b).length() > 1e-6)
        .count();
    assert!(moved > 10, "the pose moves the mesh ({moved} points)");
    // every deformed point has a twin at the reflected position
    for (i, p) in posed.points.iter().enumerate() {
        let twin = DVec3::new(-p.x, p.y, p.z);
        let found = posed.points.iter().any(|q| (*q - twin).length() < 1e-9);
        assert!(found, "point {i} {p:?} has no mirror image");
    }
}

#[test]
fn the_mirrored_weights_are_the_reflected_copy() {
    let mut s = build();
    let mesh = evaluate(s.doc.scene(), s.body, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let right = s.right.clone();
    let tag = s.tag;
    s.doc
        .transact("w", |tx| {
            add_joints_to_tag(tx, tag, &right)?;
            mirror_tag_weights(
                tx,
                tag,
                &mesh,
                Plane::Yz,
                &RenameRule::LeftRight,
                Side::Positive,
            )
        })
        .unwrap();
    let w = read_weights(s.doc.scene(), s.tag);
    // slots: 0 spine, 1-2 arm_L, 3-4 arm_R
    for (i, p) in mesh.points.iter().enumerate() {
        if p.x < -1e-9 {
            let j = mesh
                .points
                .iter()
                .position(|q| (*q - DVec3::new(-p.x, p.y, p.z)).length() < 1e-9)
                .unwrap();
            let src = w.get(j as u32);
            let dst = w.get(i as u32);
            let swap = |slot: u32| match slot {
                1 => 3,
                2 => 4,
                s => s,
            };
            let want: Vec<(u32, f64)> = {
                let mut v: Vec<(u32, f64)> = src.iter().map(|(s, x)| (swap(*s), *x)).collect();
                v.sort_by_key(|(s, _)| *s);
                v
            };
            assert_eq!(dst, want.as_slice(), "point {i}");
        }
    }
}

#[test]
fn twin_joints_are_found_by_name_and_centre_joints_map_to_themselves() {
    let s = build();
    let mut joints = vec![];
    for id in s.doc.scene().roots() {
        if forge_rig::joint::is_joint(s.doc.scene(), *id) {
            joints.push(*id);
            joints.extend(s.doc.scene().node(*id).unwrap().children.clone());
        }
    }
    let names: Vec<String> = joints
        .iter()
        .map(|j| s.doc.scene().node(*j).unwrap().name.clone())
        .collect();
    let map = joint_mirror_map(s.doc.scene(), &joints, &RenameRule::LeftRight);
    for (slot, n) in names.iter().enumerate() {
        let twin = &names[map[slot]];
        if n.contains("_L_") {
            assert_eq!(twin, &n.replace("_L_", "_R_"));
        } else if n.contains("_R_") {
            assert_eq!(twin, &n.replace("_R_", "_L_"));
        } else {
            assert_eq!(map[slot], slot, "{n} has no twin");
        }
    }
}

#[test]
fn unmatched_reflections_leave_the_weights_alone() {
    // two points that are not mirror images of each other
    let points = [DVec3::new(1.0, 0.0, 0.0), DVec3::new(-5.0, 0.0, 0.0)];
    let w = WeightMap::from_dense(&[vec![1.0, 0.0], vec![0.0, 1.0]]);
    let r = mirror_weights(&points, &w, &[1, 0], Plane::Yz, Side::Positive, 1e-6, 0.1);
    assert_eq!((r.copied, r.unmatched), (0, 1));
    assert_eq!(r.weights, w);
    // with a generous distance the nearest source is used and its slots are swapped
    let r = mirror_weights(&points, &w, &[1, 0], Plane::Yz, Side::Positive, 1e-6, 10.0);
    assert_eq!(r.copied, 1);
    assert_eq!(r.weights.get(1), &[(1, 1.0)]);
    // mirroring from the negative side copies the other way
    let r = mirror_weights(&points, &w, &[1, 0], Plane::Yz, Side::Negative, 1e-6, 10.0);
    assert_eq!(r.weights.get(0), &[(0, 1.0)]);
}
