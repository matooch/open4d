//! Parity: docs/specs/RIG.md §10 against tests/golden/ik (O35), plus unit tests of the
//! solver.

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, NodeId, Registry, Value};
use forge_parity_harness::discover_cases;
use forge_parity_harness::samples::Samples;
use forge_rig::{IK_TAG, JOINT, Rig};
use glam::DVec3;

/// Cases we build but that differ from the recorded matrices by more than their tolerance
/// (RIG.md §10, "Observation caveats" and "Known mismatch").
const KNOWN_MISMATCH: &[&str] = &["reach-near", "reach-side", "reach-straight", "four-joints"];

/// The reference stops at its own 0.01 threshold: residuals of the noisy cases stay below this
/// (a guard on the pins, not a parity tolerance).
const NOISE_ENVELOPE: f64 = 0.06;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Rig).unwrap();
    Arc::new(reg)
}

struct Chain {
    doc: Document,
    joints: Vec<NodeId>,
    tag: NodeId,
}

/// A chain of `n` joints along +Y with `spacing`, every joint rotated H = 180 deg (the
/// recorded frames imply local x = -X, z = -Z), goal Null, optional pole Null, IK tag on J1.
fn build(n: usize, spacing: f64, goal: DVec3, pole: Option<DVec3>, twist_deg: f64) -> Chain {
    let mut doc = Document::new(registry());
    let (joints, tag) = doc
        .transact("chain", |tx| {
            let mut joints: Vec<NodeId> = Vec::new();
            for i in 0..n {
                let j = tx.create(JOINT, joints.last().copied(), None)?;
                let (pos, rot) = if i == 0 {
                    ([0.0; 3], [std::f64::consts::PI, 0.0, 0.0])
                } else {
                    ([0.0, spacing, 0.0], [0.0; 3])
                };
                tx.set_param(j, "transform.pos", Value::Vec3(pos))?;
                tx.set_param(j, "transform.rot", Value::Vec3(rot))?;
                joints.push(j);
            }
            let g = tx.create("forge.null", None, None)?;
            tx.set_param(g, "transform.pos", Value::Vec3(goal.to_array()))?;
            let tag = tx.attach_tag(IK_TAG, joints[0], None)?;
            tx.set_param(tag, "end", Value::Link(joints.last().copied()))?;
            tx.set_param(tag, "goal", Value::Link(Some(g)))?;
            if let Some(p) = pole {
                let q = tx.create("forge.null", None, None)?;
                tx.set_param(q, "transform.pos", Value::Vec3(p.to_array()))?;
                tx.set_param(tag, "pole", Value::Link(Some(q)))?;
            }
            tx.set_param(tag, "twist", Value::Float(twist_deg.to_radians()))?;
            Ok((joints, tag))
        })
        .unwrap();
    Chain { doc, joints, tag }
}

fn num(v: &toml::Value) -> f64 {
    v.as_float()
        .or(v.as_integer().map(|i| i as f64))
        .expect("number")
}

fn vec3(v: &toml::Value) -> DVec3 {
    let a = v.as_array().expect("vec3");
    DVec3::new(num(&a[0]), num(&a[1]), num(&a[2]))
}

#[test]
fn ik_cases() {
    let cases = discover_cases(Path::new("../../tests/golden")).unwrap();
    let mut seen = 0;
    let mut pinned = Vec::new();
    for case in cases.iter().filter(|c| c.feature == "ik") {
        seen += 1;
        let m = case.manifest().unwrap();
        let p = &m.params;
        let chain = build(
            p["joints"].as_integer().unwrap() as usize,
            num(&p["spacing"]),
            vec3(&p["goal_position"]),
            p.get("pole_position").map(vec3),
            p.get("twist_degrees").map_or(0.0, num),
        );
        let scene = forge_rig::tags::apply_rig_tags(chain.doc.scene().clone());
        let samples = case.samples().unwrap();
        let Samples::Clones { tolerance, clones } = &samples[0].1 else {
            panic!("clones expected");
        };
        let mut err: f64 = 0.0;
        for (j, want) in chain.joints.iter().zip(clones) {
            let w = scene.world_matrix(*j).unwrap();
            for (got, exp) in [
                (w.w_axis.truncate(), want.pos),
                (w.x_axis.truncate(), want.x),
                (w.y_axis.truncate(), want.y),
                (w.z_axis.truncate(), want.z),
            ] {
                err = err.max((got - DVec3::from_array(exp)).abs().max_element());
            }
        }
        let tip = scene.world_matrix(*chain.joints.last().unwrap()).unwrap();
        let tip_err = (tip.w_axis.truncate() - vec3(&p["goal_position"])).length();
        println!("{}: error {err:e}, tip to goal {tip_err:e}", case.name);
        if KNOWN_MISMATCH.contains(&case.name.as_str()) {
            assert!(err > *tolerance, "{} matches now: unpin it", case.name);
            // the tip still reaches the goal and the chain stays a chain
            assert!(tip_err < 0.02, "{}: tip off by {tip_err}", case.name);
            if case.name != "four-joints" {
                assert!(err <= NOISE_ENVELOPE, "{}: {err}", case.name);
            }
            pinned.push(case.name.clone());
        } else {
            assert!(
                err <= *tolerance,
                "{}: error {err} > {tolerance}",
                case.name
            );
        }
    }
    assert_eq!(seen, 9);
    let mut want: Vec<String> = KNOWN_MISMATCH.iter().map(|s| s.to_string()).collect();
    want.sort();
    pinned.sort();
    assert_eq!(pinned, want);
}

#[test]
fn out_of_reach_is_straight_and_chain_keeps_length() {
    let c = build(3, 100.0, DVec3::new(300.0, 300.0, 0.0), None, 0.0);
    let s = forge_rig::tags::apply_rig_tags(c.doc.scene().clone());
    let tip = s.world_matrix(c.joints[2]).unwrap().w_axis.truncate();
    assert!((tip.length() - 200.0).abs() < 1e-9);
    assert!((tip.normalize() - DVec3::new(1.0, 1.0, 0.0).normalize()).length() < 1e-9);
}

#[test]
fn bone_lengths_are_preserved_for_long_chains() {
    let c = build(5, 50.0, DVec3::new(80.0, 120.0, 30.0), None, 0.0);
    let s = forge_rig::tags::apply_rig_tags(c.doc.scene().clone());
    let pos: Vec<DVec3> = c
        .joints
        .iter()
        .map(|j| s.world_matrix(*j).unwrap().w_axis.truncate())
        .collect();
    for w in pos.windows(2) {
        assert!(((w[1] - w[0]).length() - 50.0).abs() < 1e-9);
    }
    assert!((pos[4] - DVec3::new(80.0, 120.0, 30.0)).length() < 0.02);
}

#[test]
fn disabled_or_unlinked_tags_write_nothing() {
    let mut c = build(3, 100.0, DVec3::new(100.0, 100.0, 0.0), None, 0.0);
    let before = c.doc.scene().clone();
    c.doc
        .transact("off", |tx| {
            tx.set_param(c.tag, "enabled", Value::Bool(false))
        })
        .unwrap();
    assert!(forge_rig::tags::tag_writes(c.doc.scene(), c.tag).is_empty());
    c.doc
        .transact("on", |tx| {
            tx.set_param(c.tag, "enabled", Value::Bool(true))?;
            tx.set_param(c.tag, "goal", Value::Link(None))
        })
        .unwrap();
    assert!(forge_rig::tags::tag_writes(c.doc.scene(), c.tag).is_empty());
    let _ = before;
}

#[test]
fn fk_blend_halfway_mixes_the_rotations() {
    let mut c = build(3, 100.0, DVec3::new(100.0, 100.0, 0.0), None, 0.0);
    c.doc
        .transact("fk", |tx| tx.set_param(c.tag, "ik_fk", Value::Float(1.0)))
        .unwrap();
    let s = forge_rig::tags::apply_rig_tags(c.doc.scene().clone());
    // full FK: nothing moves
    let tip = s.world_matrix(c.joints[2]).unwrap().w_axis.truncate();
    assert!((tip - DVec3::new(0.0, 200.0, 0.0)).length() < 1e-9);
    c.doc
        .transact("half", |tx| tx.set_param(c.tag, "ik_fk", Value::Float(0.5)))
        .unwrap();
    let s = forge_rig::tags::apply_rig_tags(c.doc.scene().clone());
    let j2 = s.world_matrix(c.joints[1]).unwrap().w_axis.truncate();
    // root bone turned half of the 90 degrees the IK asks for
    assert!(
        (j2 - DVec3::new(
            100.0 * 45f64.to_radians().sin(),
            100.0 * 45f64.to_radians().cos(),
            0.0
        ))
        .length()
            < 1e-6,
        "{j2:?}"
    );
}
