//! Parity: docs/specs/RIG.md §3 and §8 against tests/golden/rigging (O34) and the
//! skinned-arm case of tests/golden/exchange (O44).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, EvalCtx, Interp, Key, NodeId, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_modifiers::Deformers;
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};
use forge_rig::weights::{WeightMap, bind};
use forge_rig::{Rig, tools};
use glam::DVec3;

/// Cases not derived (RIG.md §8).
const UNSUPPORTED: &[&str] = &["rigging/j2-bend-dq"];

/// Frames of the skinned-arm case whose recorded bounding box carries the app's own noise.
const KNOWN_MISMATCH_FRAMES: &[i32] = &[15, 30];

/// Cases we build but that differ, pinned so a fix shows up (RIG.md §8).
const KNOWN_MISMATCH: &[&str] = &[
    "rigging/j2-bend-45",
    "rigging/j2-bend-90",
    "rigging/j1-twist-90",
    "rigging/j2-scale",
];

/// The reference's own numeric noise in the posed cases (RIG.md §8): a pinned case must stay
/// inside this envelope, so a real error (a wrong rule moves points by units) still fails.
/// This is a guard on the pins, not a parity tolerance.
const NOISE_ENVELOPE: f64 = 2e-3;

pub fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    reg.install(&Rig).unwrap();
    Arc::new(reg)
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

/// The O34 rig: the editable cylinder (radius 20, height 300, 12 x 8 segments, +Y), the chain
/// `joints` (world positions, in the order given), a weight tag on the cylinder with the
/// "explicit-linear" rule, bound at rest, and a skin deformer under the cylinder.
pub struct Rig3 {
    pub doc: Document,
    pub cylinder: NodeId,
    pub joints: Vec<NodeId>,
    pub tag: NodeId,
    pub skin: NodeId,
}

/// J1 weight = clamp(-y/150), J3 weight = clamp(y/150), J2 = 1 - J1 - J3 (O34).
fn explicit_linear(y: f64) -> [f64; 3] {
    let j1 = (-y / 150.0).clamp(0.0, 1.0);
    let j3 = (y / 150.0).clamp(0.0, 1.0);
    [j1, 1.0 - j1 - j3, j3]
}

pub fn build_rig(reg: &Arc<Registry>, positions: &[DVec3], names: &[String]) -> Rig3 {
    build_rig_with(reg, positions, names, None)
}

/// `recorded`: the weight triples of the case (4 decimals); `None` uses the exact rule.
pub fn build_rig_with(
    reg: &Arc<Registry>,
    positions: &[DVec3],
    names: &[String],
    recorded: Option<Vec<Vec<f64>>>,
) -> Rig3 {
    let mut doc = Document::new(reg.clone());
    let (cylinder, joints, tag, skin) = doc
        .transact("rig", |tx| {
            let cyl = tx.create("forge.cylinder", None, None)?;
            tx.set_param(cyl, "radius", Value::Float(20.0))?;
            tx.set_param(cyl, "height", Value::Float(300.0))?;
            tx.set_param(cyl, "height_segments", Value::Int(12))?;
            tx.set_param(cyl, "rotation_segments", Value::Int(8))?;
            let joints = tools::create_chain(tx, None, positions, "J", None)?;
            for (j, n) in joints.iter().zip(names) {
                tx.rename(*j, n)?;
            }
            let tag = tx.attach_tag(forge_rig::WEIGHT_TAG, cyl, None)?;
            tx.set_param(tag, "joints", Value::LinkList(joints.clone()))?;
            let skin = tx.create(forge_rig::SKIN, Some(cyl), None)?;
            Ok((cyl, joints, tag, skin))
        })
        .unwrap();
    // The weights come from the point heights of the unskinned mesh.
    let mesh = evaluate(doc.scene(), cylinder, &EvalCtx::default())
        .unwrap()
        .unwrap();
    let dense: Vec<Vec<f64>> = recorded.unwrap_or_else(|| {
        mesh.points
            .iter()
            .map(|p| explicit_linear(p.y).to_vec())
            .collect()
    });
    doc.transact("weights", |tx| {
        tx.set_param(tag, "weights", WeightMap::from_dense(&dense).to_value())?;
        bind(tx, tag)
    })
    .unwrap();
    Rig3 {
        doc,
        cylinder,
        joints,
        tag,
        skin,
    }
}

pub enum Pose {
    None,
    Rotation { joint: usize, axis: usize, deg: f64 },
    Scale { joint: usize, scale: DVec3 },
}

/// `none`, `J2 rotation B 45 deg`, `J2 scale (1, 1.5, 1)` (the `pose` strings of O34).
pub fn parse_pose(s: &str) -> Pose {
    if s == "none" {
        return Pose::None;
    }
    let joint = s[1..2].parse::<usize>().unwrap() - 1;
    let rest = &s[3..];
    if let Some(r) = rest.strip_prefix("rotation ") {
        let axis = match &r[..1] {
            "H" => 0,
            "P" => 1,
            "B" => 2,
            other => panic!("rotation channel {other}"),
        };
        let deg = r[2..].trim_end_matches(" deg").parse::<f64>().unwrap();
        return Pose::Rotation { joint, axis, deg };
    }
    if let Some(r) = rest.strip_prefix("scale (") {
        let v: Vec<f64> = r
            .trim_end_matches(')')
            .split(',')
            .map(|x| x.trim().parse().unwrap())
            .collect();
        return Pose::Scale {
            joint,
            scale: DVec3::new(v[0], v[1], v[2]),
        };
    }
    panic!("pose {s:?}");
}

pub fn apply_pose(rig: &mut Rig3, pose: &Pose) {
    let (j, key, value) = match pose {
        Pose::None => return,
        Pose::Rotation { joint, axis, deg } => {
            let mut r = [0.0; 3];
            r[*axis] = deg.to_radians();
            (rig.joints[*joint], "transform.rot", Value::Vec3(r))
        }
        Pose::Scale { joint, scale } => (
            rig.joints[*joint],
            "transform.scale",
            Value::Vec3(scale.to_array()),
        ),
    };
    rig.doc
        .transact("pose", |tx| tx.set_param(j, key, value))
        .unwrap();
}

pub fn skinned_export(scene: &forge_core::Scene, cylinder: NodeId) -> Mesh {
    let mesh = evaluate(scene, cylinder, &EvalCtx::default())
        .unwrap()
        .unwrap();
    export_weld(&Mesh {
        positions: mesh.points.iter().map(|p| p.to_array()).collect(),
        faces: mesh.polys.clone(),
    })
}

fn bbox(points: &[DVec3]) -> (DVec3, DVec3) {
    points.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    )
}

#[test]
fn rigging_matches_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    let mut unsupported = BTreeSet::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "rigging" {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let m = case.manifest().unwrap();
        let p = &m.params;
        let skin_type = p["skin_type"].as_str().unwrap();
        if skin_type != "Linear" {
            println!("{key}: unsupported (skin type {skin_type})");
            unsupported.insert(key);
            continue;
        }
        let positions: Vec<DVec3> = p["joint_positions"]
            .as_array()
            .unwrap()
            .iter()
            .map(vec3)
            .collect();
        let names: Vec<String> = p["joint_chain"]
            .as_array()
            .unwrap()
            .iter()
            .map(|n| n.as_str().unwrap().to_string())
            .collect();
        // The exact rule must agree with the triples the case recorded, to the 4 decimals
        // they were recorded with.
        let recorded: Vec<Vec<f64>> = p["weights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_array().unwrap().iter().map(num).collect())
            .collect();
        assert_eq!(recorded.len(), 106);
        // The skin uses the exact rule: the 4-decimal copy is worse (3.9e-3 against 9e-4).
        let mut rig = build_rig(&reg, &positions, &names);
        let exact = &rig;
        let exact_w = forge_rig::weights::read_weights(exact.doc.scene(), exact.tag);
        for (i, row) in recorded.iter().enumerate() {
            for (slot, w) in row.iter().enumerate() {
                let ours = exact_w
                    .get(i as u32)
                    .iter()
                    .find(|(s, _)| *s as usize == slot)
                    .map_or(0.0, |(_, w)| *w);
                assert!(
                    (ours - w).abs() < 5e-5,
                    "{key}: weight {i}/{slot}: rule {ours} vs recorded {w}"
                );
            }
        }
        apply_pose(&mut rig, &parse_pose(p["pose"].as_str().unwrap()));
        let ours = skinned_export(rig.doc.scene(), rig.cylinder);
        let expected = export_weld(&case.expected_mesh().unwrap().unwrap());
        let r = compare_meshes(&expected, &ours, &m.tolerance);
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !r.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            let worst = r.max_position_error.expect("same vertex count");
            assert!(
                worst < NOISE_ENVELOPE,
                "{key}: error {worst:e} left the envelope"
            );
            println!("{key}: known mismatch, max error {worst:e}");
            continue;
        }
        assert!(
            r.passed(),
            "{key}: {:?} (max error {:?})",
            r.failures,
            r.max_position_error
        );
        matched.push(key);
    }
    println!("matched exactly: {}", matched.join(", "));
    let want: BTreeSet<String> = UNSUPPORTED.iter().map(|s| s.to_string()).collect();
    assert_eq!(unsupported, want, "unsupported set changed");
    assert_eq!(matched, ["rigging/bind-pose"]);
    assert_eq!(KNOWN_MISMATCH.len(), 4);
}

/// exchange/skinned-arm: the frame-0 export is the bind pose; J2 is keyed B 0 deg at frame 0
/// and 90 deg at frame 30 (linear), and the posed bounding boxes of frames 15 and 30 are in
/// the case's `posed_samples`.
#[test]
fn skinned_arm_animation_matches_the_reference() {
    let reg = registry();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/exchange/skinned-arm");
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(dir.join("case.toml")).unwrap()).unwrap();
    let params = &manifest["params"];
    let tol = manifest
        .get("tolerance")
        .and_then(|t| t.get("position"))
        .map_or(1e-4, num);
    let positions = [
        DVec3::new(0.0, -150.0, 0.0),
        DVec3::ZERO,
        DVec3::new(0.0, 150.0, 0.0),
    ];
    let names = ["J1", "J2", "J3"].map(String::from);
    let mut rig = build_rig(&reg, &positions, &names);
    let j2 = rig.joints[1];
    rig.doc
        .transact("keys", |tx| {
            let key = |t: f64, deg: f64| Key {
                time: t,
                value: deg.to_radians(),
                interp: Interp::Linear,
            };
            tx.set_key(j2, "transform.rot.z", key(0.0, 0.0))?;
            tx.set_key(j2, "transform.rot.z", key(1.0, 90.0))
        })
        .unwrap();
    let samples = &params["posed_samples"];
    for (frame, name) in [(0, "f0_bbox"), (15, "f15_bbox"), (30, "f30_bbox")] {
        let scene = forge_anim::scene_at(rig.doc.scene(), f64::from(frame) / 30.0);
        let mesh = evaluate(&scene, rig.cylinder, &EvalCtx::default())
            .unwrap()
            .unwrap();
        let (lo, hi) = bbox(&mesh.points);
        let want = samples[name].as_array().unwrap();
        let (wlo, whi) = (vec3(&want[0]), vec3(&want[1]));
        let err = (lo - wlo)
            .abs()
            .max_element()
            .max((hi - whi).abs().max_element());
        println!("frame {frame}: bbox error {err:e}");
        if KNOWN_MISMATCH_FRAMES.contains(&frame) {
            // the sample carries 4e-4 of the app's own noise (RIG.md §8): pinned
            assert!(err > tol, "frame {frame} matches now: unpin it");
        } else {
            assert!(
                err <= tol,
                "frame {frame}: {lo:?} {hi:?} vs {wlo:?} {whi:?}"
            );
        }
        // The whole mesh at the keyed frames is the O34 pose with the same angle.
        let pose_case = match frame {
            15 => Some("j2-bend-45"),
            30 => Some("j2-bend-90"),
            _ => None,
        };
        if let Some(c) = pose_case {
            let golden =
                std::fs::read_to_string(dir.join("../../rigging").join(c).join("mesh.obj"))
                    .unwrap();
            let expected = export_weld(&forge_parity_harness::parse_obj(&golden).unwrap());
            let ours = skinned_export(&scene, rig.cylinder);
            let r = compare_meshes(
                &expected,
                &ours,
                &forge_parity_harness::Tolerance::default(),
            );
            let worst = r.max_position_error.expect("same vertex count");
            assert!(worst < NOISE_ENVELOPE, "frame {frame} vs {c}: {worst:e}");
        }
    }
    let bind_mesh = {
        let expected = forge_parity_harness::parse_obj(
            &std::fs::read_to_string(dir.join("mesh.obj")).unwrap(),
        )
        .unwrap();
        export_weld(&expected)
    };
    let ours = skinned_export(rig.doc.scene(), rig.cylinder);
    let r = compare_meshes(
        &bind_mesh,
        &ours,
        &forge_parity_harness::Tolerance::default(),
    );
    assert!(r.passed(), "{:?}", r.failures);
}
