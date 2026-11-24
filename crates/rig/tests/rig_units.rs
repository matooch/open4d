//! Unit tests for the joint tools, the weight tag and the skin (docs/specs/RIG.md).

use std::sync::Arc;

use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_rig::display::{bone_display, octahedron};
use forge_rig::joint::{bone_vector, chain_world_matrices, joint_length};
use forge_rig::mirror::{Plane, RenameRule, mirror_chain, mirror_name};
use forge_rig::skin::{SkinSettings, SkinType, effective_weights, skin_points};
use forge_rig::weights::{WeightMap, bind, read_weights};
use forge_rig::{Axis, Rig, tools};
use glam::{DMat4, DVec3};

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    reg.install(&Rig).unwrap();
    Arc::new(reg)
}

fn chain(doc: &mut Document, points: &[DVec3], align: Option<Axis>) -> Vec<NodeId> {
    doc.transact("chain", |tx| {
        tools::create_chain(tx, None, points, "J", align)
    })
    .unwrap()
}

fn close(a: DVec3, b: DVec3) -> bool {
    (a - b).abs().max_element() < 1e-9
}

#[test]
fn a_chain_has_world_positions_at_the_given_points() {
    let mut doc = Document::new(registry());
    let pts = [
        DVec3::new(0.0, -150.0, 0.0),
        DVec3::ZERO,
        DVec3::new(0.0, 150.0, 0.0),
    ];
    let ids = chain(&mut doc, &pts, None);
    let s = doc.scene();
    assert_eq!(s.node(ids[1]).unwrap().name, "J2");
    let world = chain_world_matrices(s, ids[0]);
    assert_eq!(world.len(), 3);
    for ((_, m), p) in world.iter().zip(pts) {
        assert!(close(m.w_axis.truncate(), p));
    }
    // O34: J2 sits at (0, 150, 0) relative to J1
    assert!(close(
        s.local_transform(ids[1]).unwrap().pos,
        DVec3::new(0.0, 150.0, 0.0)
    ));
}

#[test]
fn the_bone_runs_to_the_child_or_along_the_axis() {
    let mut doc = Document::new(registry());
    let ids = chain(&mut doc, &[DVec3::ZERO, DVec3::new(30.0, 40.0, 0.0)], None);
    let s = doc.scene();
    assert!(close(bone_vector(s, ids[0]), DVec3::new(30.0, 40.0, 0.0)));
    assert!((joint_length(s, ids[0]) - 50.0).abs() < 1e-12);
    // the last joint has no child: default length 150 along the default axis z
    assert!(close(bone_vector(s, ids[1]), DVec3::new(0.0, 0.0, 150.0)));
}

#[test]
fn align_turns_the_axis_to_the_child_and_keeps_the_children_in_place() {
    let mut doc = Document::new(registry());
    let pts = [
        DVec3::ZERO,
        DVec3::new(100.0, 100.0, 0.0),
        DVec3::new(100.0, 100.0, 200.0),
    ];
    let ids = chain(&mut doc, &pts, None);
    doc.transact("align", |tx| tools::align_joint(tx, ids[0], Axis::X))
        .unwrap();
    let s = doc.scene();
    let w = s.world_matrix(ids[0]).unwrap();
    let dir = (pts[1] - pts[0]).normalize();
    assert!(close(w.transform_vector3(DVec3::X), dir));
    for (id, p) in ids.iter().zip(pts) {
        assert!(close(s.world_matrix(*id).unwrap().w_axis.truncate(), p));
    }
    // aligning the second joint leaves the third where it was too
    doc.transact("align2", |tx| tools::align_joint(tx, ids[1], Axis::Y))
        .unwrap();
    let s = doc.scene();
    let w2 = s.world_matrix(ids[1]).unwrap();
    assert!(close(
        w2.transform_vector3(DVec3::Y),
        DVec3::new(0.0, 0.0, 1.0)
    ));
    assert!(close(
        s.world_matrix(ids[2]).unwrap().w_axis.truncate(),
        pts[2]
    ));
}

#[test]
fn create_chain_can_align_every_joint() {
    let mut doc = Document::new(registry());
    let pts = [
        DVec3::ZERO,
        DVec3::new(0.0, 100.0, 0.0),
        DVec3::new(100.0, 100.0, 0.0),
    ];
    let ids = chain(&mut doc, &pts, Some(Axis::Z));
    let s = doc.scene();
    let w0 = s.world_matrix(ids[0]).unwrap();
    assert!(close(w0.transform_vector3(DVec3::Z), DVec3::Y));
    let w1 = s.world_matrix(ids[1]).unwrap();
    assert!(close(w1.transform_vector3(DVec3::Z), DVec3::X));
}

#[test]
fn mirroring_reflects_positions_and_swaps_the_names() {
    let mut doc = Document::new(registry());
    let ids = doc
        .transact("c", |tx| {
            let ids = tools::create_chain(
                tx,
                None,
                &[
                    DVec3::new(10.0, 0.0, 0.0),
                    DVec3::new(20.0, 30.0, 5.0),
                    DVec3::new(25.0, 60.0, 5.0),
                ],
                "arm_L_",
                Some(Axis::Y),
            )?;
            Ok(ids)
        })
        .unwrap();
    let new_root = doc
        .transact("m", |tx| {
            mirror_chain(tx, ids[0], Plane::Yz, &RenameRule::LeftRight)
        })
        .unwrap();
    let s = doc.scene();
    assert_eq!(s.node(new_root).unwrap().name, "arm_R_1");
    let mirrored = chain_world_matrices(s, new_root);
    assert_eq!(mirrored.len(), 3);
    let originals = chain_world_matrices(s, ids[0]);
    for ((_, m), (_, o)) in mirrored.iter().zip(&originals) {
        let p = o.w_axis.truncate();
        assert!(close(m.w_axis.truncate(), DVec3::new(-p.x, p.y, p.z)));
        // proper rotations: no negative scale
        assert!(m.determinant() > 0.0);
        // the bone axis is reflected too
        let a = o.transform_vector3(DVec3::Y);
        assert!(close(
            m.transform_vector3(DVec3::Y),
            DVec3::new(-a.x, a.y, a.z)
        ));
    }
}

#[test]
fn mirror_names() {
    let r = RenameRule::LeftRight;
    assert_eq!(mirror_name("Arm_L", &r), "Arm_R");
    assert_eq!(mirror_name("hand.R.001", &r), "hand.L.001");
    assert_eq!(mirror_name("LeftLeg", &r), "RightLeg");
    assert_eq!(mirror_name("rightfoot", &r), "leftfoot");
    assert_eq!(mirror_name("Spine", &r), "Spine_mirror");
    assert_eq!(mirror_name("Lower", &r), "Lower_mirror");
    assert_eq!(mirror_name("A", &RenameRule::Suffix("_x".into())), "A_x");
}

#[test]
fn the_bone_display_is_an_octahedron_between_the_joints() {
    let mut doc = Document::new(registry());
    let ids = chain(&mut doc, &[DVec3::ZERO, DVec3::new(0.0, 100.0, 0.0)], None);
    let b = bone_display(doc.scene(), ids[0]).unwrap();
    assert!(close(b.start, DVec3::ZERO));
    assert!(close(b.end, DVec3::new(0.0, 100.0, 0.0)));
    assert!(close(b.octahedron[0], b.start) && close(b.octahedron[5], b.end));
    for w in &b.octahedron[1..5] {
        assert!((w.y - 10.0).abs() < 1e-9);
        assert!(((w.x * w.x + w.z * w.z).sqrt() - 10.0).abs() < 1e-9);
    }
    assert!(octahedron(DVec3::ONE, DVec3::ONE).is_none());
    doc.transact("hide", |tx| {
        tx.set_param(ids[0], "display_object", Value::Enum("none".into()))
    })
    .unwrap();
    assert!(bone_display(doc.scene(), ids[0]).is_none());
}

#[test]
fn weight_maps_normalise_prune_and_round_trip() {
    let mut m = WeightMap::from_dense(&[
        vec![2.0, 2.0, 0.0],
        vec![0.0, 0.0, 0.0],
        vec![0.1, 0.0, 0.9],
    ]);
    assert_eq!(m.get(1), &[]);
    m.normalise();
    assert_eq!(m.get(0), &[(0, 0.5), (1, 0.5)]);
    m.prune(0.2);
    assert_eq!(m.get(2), &[(2, 0.9)]);
    let back = WeightMap::from_triples(&m.to_triples());
    assert_eq!(back, m);
    assert_eq!(m.points().collect::<Vec<_>>(), vec![0, 2]);
}

#[test]
fn influences_are_limited_then_normalised() {
    let s = SkinSettings {
        max_influences: 2,
        ..SkinSettings::default()
    };
    let w = effective_weights(&[(0, 0.2), (1, 0.5), (2, 0.3)], &s);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].0, 1);
    assert_eq!(w[1].0, 2);
    assert!((w.iter().map(|(_, x)| x).sum::<f64>() - 1.0).abs() < 1e-12);
    let off = SkinSettings {
        normalise: false,
        ..SkinSettings::default()
    };
    assert_eq!(effective_weights(&[(0, 0.2)], &off), vec![(0, 0.2)]);
}

#[test]
fn identity_matrices_leave_points_alone_and_zero_weight_points_stay() {
    let mut pts = vec![DVec3::new(1.0, 2.0, 3.0), DVec3::new(4.0, 5.0, 6.0)];
    let before = pts.clone();
    let w = WeightMap::from_dense(&[vec![0.3, 0.7]]);
    for kind in [SkinType::Linear, SkinType::DualQuaternion] {
        let s = SkinSettings {
            kind,
            ..SkinSettings::default()
        };
        skin_points(&mut pts, &w, &[DMat4::IDENTITY; 2], &s);
        for (a, b) in pts.iter().zip(&before) {
            assert!(close(*a, *b));
        }
    }
}

#[test]
fn dual_quaternion_keeps_volume_where_linear_collapses() {
    // two joints rotating +-90 degrees about the Y axis, weights 0.5 each: linear skinning
    // collapses a point on x = 1 towards the axis, dual quaternions keep it at radius 1.
    let k = [
        DMat4::from_rotation_y(std::f64::consts::FRAC_PI_2),
        DMat4::from_rotation_y(-std::f64::consts::FRAC_PI_2),
    ];
    let w = WeightMap::from_dense(&[vec![0.5, 0.5]]);
    let mut lin = vec![DVec3::X];
    let mut dq = vec![DVec3::X];
    skin_points(&mut lin, &w, &k, &SkinSettings::default());
    skin_points(
        &mut dq,
        &w,
        &k,
        &SkinSettings {
            kind: SkinType::DualQuaternion,
            ..SkinSettings::default()
        },
    );
    assert!(lin[0].length() < 0.1);
    assert!((dq[0].length() - 1.0).abs() < 1e-9);
}

#[test]
fn blended_is_the_mix_of_linear_and_dual_quaternion() {
    let k = [
        DMat4::from_rotation_y(std::f64::consts::FRAC_PI_2),
        DMat4::from_rotation_y(-std::f64::consts::FRAC_PI_2),
    ];
    let w = WeightMap::from_dense(&[vec![0.5, 0.5]]);
    let run = |kind, blend| {
        let mut p = vec![DVec3::X];
        skin_points(
            &mut p,
            &w,
            &k,
            &SkinSettings {
                kind,
                blend,
                ..SkinSettings::default()
            },
        );
        p[0]
    };
    let lin = run(SkinType::Linear, 0.5);
    let dq = run(SkinType::DualQuaternion, 0.5);
    assert!(close(run(SkinType::Blended, 0.5), (lin + dq) * 0.5));
    assert!(close(run(SkinType::Blended, 0.0), lin));
    assert!(close(run(SkinType::Blended, 1.0), dq));
}

#[test]
fn dual_quaternion_applies_a_rigid_transform_exactly() {
    let m = DMat4::from_translation(DVec3::new(1.0, 2.0, 3.0)) * DMat4::from_rotation_z(0.7);
    let w = WeightMap::from_dense(&[vec![1.0]]);
    let mut p = vec![DVec3::new(4.0, -1.0, 2.0)];
    let want = m.transform_point3(p[0]);
    skin_points(
        &mut p,
        &w,
        &[m],
        &SkinSettings {
            kind: SkinType::DualQuaternion,
            ..SkinSettings::default()
        },
    );
    assert!(close(p[0], want));
}

/// A plane (4 x 4 points) bound to a two-joint chain, bound and then bent.
fn bent_plane(kind: &str, tag_weight: f64, enabled: bool) -> Vec<DVec3> {
    let reg = registry();
    let mut doc = Document::new(reg);
    let (obj, joints, tag, skin) = doc
        .transact("scene", |tx| {
            let obj = tx.create("forge.cylinder", None, None)?;
            tx.set_param(obj, "radius", Value::Float(10.0))?;
            tx.set_param(obj, "height", Value::Float(100.0))?;
            tx.set_param(obj, "height_segments", Value::Int(4))?;
            tx.set_param(obj, "rotation_segments", Value::Int(4))?;
            let joints = tools::create_chain(
                tx,
                None,
                &[
                    DVec3::new(0.0, -50.0, 0.0),
                    DVec3::ZERO,
                    DVec3::new(0.0, 50.0, 0.0),
                ],
                "B",
                None,
            )?;
            let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, obj, None)?;
            tx.set_param(tag, "joints", Value::LinkList(joints.clone()))?;
            let skin = tx.create(forge_rig::SKIN, Some(obj), None)?;
            Ok((obj, joints, tag, skin))
        })
        .unwrap();
    let mesh = evaluate(doc.scene(), obj, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let dense: Vec<Vec<f64>> = mesh
        .points
        .iter()
        .map(|p| {
            let t = ((p.y + 50.0) / 100.0).clamp(0.0, 1.0);
            vec![1.0 - t, t, 0.0]
        })
        .collect();
    doc.transact("w", |tx| {
        tx.set_param(tag, "weights", WeightMap::from_dense(&dense).to_value())?;
        tx.set_param(tag, "weight", Value::Float(tag_weight))?;
        tx.set_param(skin, "type", Value::Enum(kind.into()))?;
        tx.set_param(skin, "enabled", Value::Bool(enabled))?;
        bind(tx, tag)?;
        tx.set_param(joints[1], "transform.rot", Value::Vec3([0.0, 0.0, 0.5]))
    })
    .unwrap();
    evaluate(doc.scene(), obj, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .points
}

#[test]
fn skin_parameters_take_effect() {
    let full = bent_plane("linear", 1.0, true);
    let off = bent_plane("linear", 1.0, false);
    let half = bent_plane("linear", 0.5, true);
    let dq = bent_plane("dual_quaternion", 1.0, true);
    let top = |pts: &Vec<DVec3>| pts.iter().map(|p| p.x).fold(f64::MIN, f64::max);
    assert!((top(&off) - 10.0).abs() < 1e-9);
    assert!(top(&full) > 20.0);
    // half strength moves half as far
    assert!(((top(&half) - 10.0) * 2.0 - (top(&full) - 10.0)).abs() < 1e-6);
    // the dual quaternion result differs from the linear one
    assert!(full.iter().zip(&dq).any(|(a, b)| !close(*a, *b)));
}

#[test]
fn weights_are_stored_on_the_tag() {
    let mut doc = Document::new(registry());
    let (obj, tag) = doc
        .transact("t", |tx| {
            let obj = tx.create("forge.cube", None, None)?;
            let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, obj, None)?;
            tx.set_param(
                tag,
                "weights",
                WeightMap::from_dense(&[vec![1.0], vec![0.25, 0.75]]).to_value(),
            )?;
            Ok((obj, tag))
        })
        .unwrap();
    let m = read_weights(doc.scene(), tag);
    assert_eq!(m.get(1), &[(0, 0.25), (1, 0.75)]);
    assert_eq!(
        forge_rig::weights::find_weight_tag(doc.scene(), obj),
        Some(tag)
    );
    let _ = obj;
}
