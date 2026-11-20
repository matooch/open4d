//! Auto weights (docs/specs/RIG.md §4): level C plausibility on the O34 arm. The reference's
//! auto-weight case was not observed (O34: no auto-weight call, the command opens a dialog), so
//! the comparison partner is the nearest bone, the answer heat diffusion approaches on a tube.

use std::sync::Arc;

use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_rig::auto_weights::{AutoWeightOptions, Bone, auto_bind, auto_weights, bones_for_joints};
use forge_rig::weights::read_weights;
use forge_rig::{Rig, tools};
use glam::DVec3;

fn arm() -> (
    Document,
    forge_core::NodeId,
    forge_core::NodeId,
    Vec<forge_core::NodeId>,
) {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    reg.install(&Rig).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let (cyl, tag, joints) = doc
        .transact("arm", |tx| {
            let cyl = tx.create("forge.cylinder", None, None)?;
            tx.set_param(cyl, "radius", Value::Float(20.0))?;
            tx.set_param(cyl, "height", Value::Float(300.0))?;
            tx.set_param(cyl, "height_segments", Value::Int(12))?;
            tx.set_param(cyl, "rotation_segments", Value::Int(8))?;
            let joints = tools::create_chain(
                tx,
                None,
                &[
                    DVec3::new(0.0, -150.0, 0.0),
                    DVec3::ZERO,
                    DVec3::new(0.0, 150.0, 0.0),
                ],
                "J",
                None,
            )?;
            let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, cyl, None)?;
            tx.set_param(tag, "joints", Value::LinkList(joints.clone()))?;
            tx.create(forge_rig::SKIN, Some(cyl), None)?;
            Ok((cyl, tag, joints))
        })
        .unwrap();
    (doc, cyl, tag, joints)
}

fn dominant(row: &[(u32, f64)]) -> u32 {
    row.iter()
        .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
        .map(|(s, _)| *s)
        .unwrap()
}

#[test]
fn heat_weights_follow_the_nearest_bone_on_the_arm() {
    let (mut doc, cyl, tag, joints) = arm();
    let mesh = evaluate(doc.scene(), cyl, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let bones = bones_for_joints(doc.scene(), cyl, &joints);
    assert_eq!(bones.len(), 3);
    doc.transact("auto", |tx| {
        auto_bind(tx, tag, &mesh, &AutoWeightOptions::default())
    })
    .unwrap();
    let w = read_weights(doc.scene(), tag);
    let (mut agree, mut counted) = (0, 0);
    for (i, p) in mesh.points.iter().enumerate() {
        let row = w.get(i as u32);
        let sum: f64 = row.iter().map(|(_, x)| x).sum();
        assert!((sum - 1.0).abs() < 1e-9, "point {i}: sum {sum}");
        // nearest bone, skipping points that tie (equidistant to two bones)
        let mut d: Vec<(f64, u32)> = bones
            .iter()
            .enumerate()
            .map(|(b, bone)| ((bone.closest(*p) - *p).length(), b as u32))
            .collect();
        d.sort_by(|a, b| a.0.total_cmp(&b.0));
        if (d[1].0 - d[0].0).abs() < 1e-6 {
            continue;
        }
        counted += 1;
        if dominant(row) == d[0].1 {
            agree += 1;
        }
    }
    let ratio = f64::from(agree) / f64::from(counted);
    println!("dominant joint agrees for {agree} of {counted} points ({ratio:.3})");
    assert!(ratio >= 0.9, "{ratio}");
}

#[test]
fn weights_are_symmetric_around_the_axis_and_smooth_along_it() {
    let (mut doc, cyl, tag, _) = arm();
    let mesh = evaluate(doc.scene(), cyl, &EvalCtx::default())
        .unwrap()
        .unwrap();
    doc.transact("auto", |tx| {
        auto_bind(tx, tag, &mesh, &AutoWeightOptions::default())
    })
    .unwrap();
    let w = read_weights(doc.scene(), tag);
    // points of one ring share their weights
    let ring: Vec<usize> = mesh
        .points
        .iter()
        .enumerate()
        .filter(|(_, p)| (p.y + 75.0).abs() < 1e-9 && p.x.hypot(p.z) > 1.0)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(ring.len(), 8);
    let first = w.get(ring[0] as u32).to_vec();
    for i in &ring {
        for ((s0, w0), (s1, w1)) in first.iter().zip(w.get(*i as u32)) {
            assert_eq!(s0, s1);
            assert!((w0 - w1).abs() < 1e-6);
        }
    }
    // J1's weight falls along the arm: lowest ring 1, top ring 0
    let j1 = |y: f64| {
        let i = mesh
            .points
            .iter()
            .position(|p| (p.y - y).abs() < 1e-9 && p.x.hypot(p.z) > 1.0)
            .unwrap();
        w.get(i as u32)
            .iter()
            .find(|(s, _)| *s == 0)
            .map_or(0.0, |(_, x)| *x)
    };
    let mut prev = f64::MAX;
    for k in 0..13 {
        let v = j1(-150.0 + 25.0 * f64::from(k));
        assert!(v <= prev + 1e-9, "J1 weight rises at level {k}");
        prev = v;
    }
    assert!(j1(-150.0) > 0.95 && j1(150.0) < 0.05);
}

#[test]
fn the_solve_is_deterministic_and_handles_a_far_component() {
    let (doc, cyl, _, joints) = arm();
    let mut mesh = evaluate(doc.scene(), cyl, &EvalCtx::default())
        .unwrap()
        .unwrap();
    // a second, detached triangle far from the arm: no heat reaches it, it takes its nearest bone
    let n = mesh.points.len() as u32;
    mesh.points.extend([
        DVec3::new(500.0, 140.0, 0.0),
        DVec3::new(520.0, 140.0, 0.0),
        DVec3::new(500.0, 160.0, 0.0),
    ]);
    mesh.polys.push(vec![n, n + 1, n + 2]);
    let bones = bones_for_joints(doc.scene(), cyl, &joints);
    let a = auto_weights(&mesh, &bones, &AutoWeightOptions::default());
    let b = auto_weights(&mesh, &bones, &AutoWeightOptions::default());
    assert_eq!(a, b);
    for k in 0..3 {
        let row = a.get(n + k);
        let sum: f64 = row.iter().map(|(_, x)| x).sum();
        assert!((sum - 1.0).abs() < 1e-9);
        // the detached triangle is nearest to J3's point (0, 150, 0)? no: nearest to the J2-J3 bone
        assert_eq!(dominant(row), 1);
    }
    let _: &[Bone] = &bones;
}

#[test]
fn heat_weights_pose_the_arm_without_tearing() {
    let (mut doc, cyl, tag, joints) = arm();
    let mesh = evaluate(doc.scene(), cyl, &EvalCtx::default())
        .unwrap()
        .unwrap();
    doc.transact("auto", |tx| {
        auto_bind(tx, tag, &mesh, &AutoWeightOptions::default())
    })
    .unwrap();
    doc.transact("pose", |tx| {
        tx.set_param(joints[1], "transform.rot", Value::Vec3([0.0, 0.0, 0.8]))
    })
    .unwrap();
    let posed = evaluate(doc.scene(), cyl, &EvalCtx::default())
        .unwrap()
        .unwrap();
    // neighbours stay close: no edge grows more than 60 % (the cylinder edges are at most 50)
    for poly in &mesh.polys {
        for k in 0..poly.len() {
            let (a, b) = (poly[k] as usize, poly[(k + 1) % poly.len()] as usize);
            let before = (mesh.points[a] - mesh.points[b]).length();
            let after = (posed.points[a] - posed.points[b]).length();
            assert!(
                after < before * 1.6 + 1e-9,
                "edge {a}-{b}: {before} -> {after}"
            );
        }
    }
}
