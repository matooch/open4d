//! Parity: docs/specs/RIG.md §11 against tests/golden/constraints (O36), plus unit tests.

use std::path::Path;
use std::sync::Arc;

use forge_core::{Document, LinkRow, NodeId, Registry, Value};
use forge_parity_harness::discover_cases;
use forge_parity_harness::samples::Samples;
use forge_rig::{CONSTRAINT_TAG, Rig, constraint};
use glam::DVec3;

/// Cases that build but differ (none: every recorded case matches).
const KNOWN_MISMATCH: &[&str] = &[];

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Rig).unwrap();
    Arc::new(reg)
}

struct Rigging {
    doc: Document,
    cube: NodeId,
    driver: NodeId,
    tag: NodeId,
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

fn mode_of(case: &str) -> &'static str {
    match case {
        "aim" | "aim-up" => "aim",
        "parent" => "parent",
        _ => "psr",
    }
}

fn build(case: &str, p: &std::collections::BTreeMap<String, toml::Value>) -> Rigging {
    let mut doc = Document::new(registry());
    let flag = |k: &str| p.get(k).and_then(toml::Value::as_bool);
    let (cube, driver, tag) = doc
        .transact("rig", |tx| {
            let driver = tx.create("forge.null", None, None)?;
            let cube = tx.create("forge.null", None, None)?;
            tx.set_param(
                cube,
                "transform.pos",
                Value::Vec3(vec3(&p["object_start_pos"]).to_array()),
            )?;
            let tag = tx.attach_tag(CONSTRAINT_TAG, cube, None)?;
            tx.set_param(tag, "mode", Value::Enum(mode_of(case).into()))?;
            tx.set_param(
                tag,
                "targets",
                Value::LinkRows(vec![LinkRow::new(driver, "normal")]),
            )?;
            tx.set_param(
                tag,
                "position",
                Value::Bool(flag("position").unwrap_or(true)),
            )?;
            tx.set_param(
                tag,
                "rotation",
                Value::Bool(flag("rotation").unwrap_or(false)),
            )?;
            tx.set_param(tag, "scale", Value::Bool(flag("scale").unwrap_or(false)))?;
            tx.set_param(
                tag,
                "maintain_offset",
                Value::Bool(flag("maintain_original").unwrap_or(false)),
            )?;
            if let Some(w) = p.get("weight") {
                tx.set_param(
                    tag,
                    "targets",
                    Value::LinkRows(vec![LinkRow {
                        strength: num(w),
                        ..LinkRow::new(driver, "normal")
                    }]),
                )?;
            }
            if let Some(u) = p.get("up_target_pos") {
                let up = tx.create("forge.null", None, None)?;
                tx.set_param(up, "transform.pos", Value::Vec3(vec3(u).to_array()))?;
                tx.set_param(tag, "up_target", Value::Link(Some(up)))?;
            }
            Ok((cube, driver, tag))
        })
        .unwrap();
    Rigging {
        doc,
        cube,
        driver,
        tag,
    }
}

/// How the driver of a frame is found.
#[derive(Clone, Copy, PartialEq)]
enum Driver {
    /// Eased from the frame-0 to the frame-30 key (smoothstep; the recorded curve follows it
    /// to 1.3e-4 degrees, the reference's own noise).
    Smoothstep,
    /// The curve the reference recorded for its driver: the position and H tracks of
    /// `psr-all` (its object copies the driver exactly).
    Recorded,
}

fn recorded_driver() -> &'static [(f64, f64, f64)] {
    static CURVE: std::sync::OnceLock<Vec<(f64, f64, f64)>> = std::sync::OnceLock::new();
    CURVE.get_or_init(|| {
        let cases = discover_cases(Path::new("../../tests/golden")).unwrap();
        let case = cases
            .iter()
            .find(|c| c.feature == "constraints" && c.name == "psr-all")
            .unwrap();
        let tracks = case.samples().unwrap();
        let get = |param: &str| -> Vec<f64> {
            tracks
                .iter()
                .find_map(|(_, s)| match s {
                    Samples::Track {
                        param: p, samples, ..
                    } if p == param => Some(samples.iter().map(|(_, v)| *v).collect()),
                    _ => None,
                })
                .unwrap()
        };
        let (x, y, h) = (get("pos_x"), get("pos_y"), get("rot_h"));
        (0..x.len()).map(|i| (x[i], y[i], h[i])).collect()
    })
}

fn driver_at(driver: Driver, frame: f64) -> (DVec3, f64) {
    match driver {
        Driver::Smoothstep => {
            let t = (frame / 30.0).clamp(0.0, 1.0);
            let s = t * t * (3.0 - 2.0 * t);
            (DVec3::new(200.0, 100.0, 0.0) * s, 90f64.to_radians() * s)
        }
        Driver::Recorded => {
            let (x, y, h) = recorded_driver()[frame as usize];
            (DVec3::new(x, y, 0.0), h.to_radians())
        }
    }
}

fn scene_at(r: &Rigging, driver: Driver, frame: f64) -> forge_core::Scene {
    let (pos, h) = driver_at(driver, frame);
    let mut s = r.doc.scene().clone();
    s.apply_params([
        (
            r.driver,
            "transform.pos".to_string(),
            Value::Vec3(pos.to_array()),
        ),
        (
            r.driver,
            "transform.rot".to_string(),
            Value::Vec3([h, 0.0, 0.0]),
        ),
    ]);
    s
}

fn angle_diff(a: f64, b: f64) -> f64 {
    ((a - b + 180.0).rem_euclid(360.0) - 180.0).abs()
}

#[test]
fn constraint_cases() {
    run_cases(Driver::Recorded, 0.0);
}

/// The same cases with an ideal driver curve stay inside the reference's noise.
#[test]
fn constraint_cases_with_an_ideal_driver_stay_in_the_noise_envelope() {
    run_cases(Driver::Smoothstep, 1e-3);
}

/// `envelope` 0: every sample within its recorded tolerance; otherwise within `envelope`.
fn run_cases(driver: Driver, envelope: f64) {
    let cases = discover_cases(Path::new("../../tests/golden")).unwrap();
    let mut seen = 0;
    for case in cases.iter().filter(|c| c.feature == "constraints") {
        seen += 1;
        let m = case.manifest().unwrap();
        let rig = build(&case.name, &m.params);
        // activation: the offset is captured at frame 0
        let s0 = scene_at(&rig, driver, 0.0);
        let mut doc_scene = rig.doc.scene().clone();
        doc_scene.apply_params(constraint::capture_offset(&s0, rig.tag));
        let rig = Rigging {
            doc: Document::from_scene(doc_scene),
            ..rig
        };
        let mut worst: f64 = 0.0;
        for (name, samples) in case.samples().unwrap() {
            let Samples::Track {
                param,
                samples,
                tolerance,
                ..
            } = samples
            else {
                panic!("track expected in {name}");
            };
            for (frame, want) in samples {
                let s = forge_rig::tags::apply_rig_tags(scene_at(&rig, driver, frame));
                let w = s.world_matrix(rig.cube).unwrap();
                let hpb = forge_core::transform::hpb_from_rotation(glam::DMat3::from_cols(
                    w.x_axis.truncate().normalize(),
                    w.y_axis.truncate().normalize(),
                    w.z_axis.truncate().normalize(),
                ));
                let got = match param.as_str() {
                    "pos_x" => w.w_axis.x,
                    "pos_y" => w.w_axis.y,
                    "pos_z" => w.w_axis.z,
                    "rot_h" => hpb.x.to_degrees(),
                    "rot_p" => hpb.y.to_degrees(),
                    "rot_b" => hpb.z.to_degrees(),
                    other => panic!("param {other}"),
                };
                let err = if param.starts_with("rot") {
                    angle_diff(got, want)
                } else {
                    (got - want).abs()
                };
                worst = worst.max(err);
                let limit = if envelope > 0.0 { envelope } else { tolerance };
                if !KNOWN_MISMATCH.contains(&case.name.as_str()) {
                    assert!(
                        err <= limit,
                        "{} {param} frame {frame}: {got} vs {want}",
                        case.name
                    );
                }
            }
        }
        println!("{}: worst error {worst:e}", case.name);
    }
    assert_eq!(seen, 7);
}

fn two_objects(mode: &str) -> (Document, NodeId, NodeId, NodeId) {
    let mut doc = Document::new(registry());
    let (a, b, tag) = doc
        .transact("two", |tx| {
            let a = tx.create("forge.null", None, None)?;
            let b = tx.create("forge.null", None, None)?;
            let tag = tx.attach_tag(CONSTRAINT_TAG, b, None)?;
            tx.set_param(tag, "mode", Value::Enum(mode.into()))?;
            tx.set_param(
                tag,
                "targets",
                Value::LinkRows(vec![LinkRow::new(a, "normal")]),
            )?;
            Ok((a, b, tag))
        })
        .unwrap();
    (doc, a, b, tag)
}

#[test]
fn two_targets_blend_by_weight() {
    let mut doc = Document::new(registry());
    let (b, tag) = doc
        .transact("t", |tx| {
            let t1 = tx.create("forge.null", None, None)?;
            let t2 = tx.create("forge.null", None, None)?;
            tx.set_param(t2, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]))?;
            let b = tx.create("forge.null", None, None)?;
            let tag = tx.attach_tag(CONSTRAINT_TAG, b, None)?;
            tx.set_param(
                tag,
                "targets",
                Value::LinkRows(vec![
                    LinkRow {
                        strength: 0.75,
                        ..LinkRow::new(t1, "normal")
                    },
                    LinkRow {
                        strength: 0.25,
                        ..LinkRow::new(t2, "normal")
                    },
                ]),
            )?;
            Ok((b, tag))
        })
        .unwrap();
    let _ = tag;
    let s = forge_rig::tags::apply_rig_tags(doc.scene().clone());
    assert!((s.world_matrix(b).unwrap().w_axis.x - 25.0).abs() < 1e-12);
}

#[test]
fn strength_mixes_with_the_unconstrained_value() {
    let (mut doc, a, b, tag) = two_objects("psr");
    doc.transact("set", |tx| {
        tx.set_param(a, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]))?;
        tx.set_param(tag, "strength", Value::Float(0.5))
    })
    .unwrap();
    let s = forge_rig::tags::apply_rig_tags(doc.scene().clone());
    assert!((s.world_matrix(b).unwrap().w_axis.x - 50.0).abs() < 1e-12);
}

#[test]
fn clamp_limits_the_distance() {
    let (mut doc, _a, b, tag) = two_objects("clamp");
    doc.transact("set", |tx| {
        tx.set_param(b, "transform.pos", Value::Vec3([300.0, 0.0, 0.0]))?;
        tx.set_param(tag, "clamp_max", Value::Float(100.0))
    })
    .unwrap();
    let s = forge_rig::tags::apply_rig_tags(doc.scene().clone());
    assert!((s.world_matrix(b).unwrap().w_axis.x - 100.0).abs() < 1e-12);
    doc.transact("min", |tx| {
        tx.set_param(b, "transform.pos", Value::Vec3([20.0, 0.0, 0.0]))?;
        tx.set_param(tag, "clamp_max", Value::Float(0.0))?;
        tx.set_param(tag, "clamp_min", Value::Float(50.0))
    })
    .unwrap();
    let s = forge_rig::tags::apply_rig_tags(doc.scene().clone());
    assert!((s.world_matrix(b).unwrap().w_axis.x - 50.0).abs() < 1e-12);
}

#[test]
fn aim_other_axis_points_it_at_the_target() {
    let (mut doc, a, b, tag) = two_objects("aim");
    doc.transact("set", |tx| {
        tx.set_param(a, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]))?;
        tx.set_param(tag, "aim_axis", Value::Enum("-x".into()))
    })
    .unwrap();
    let s = forge_rig::tags::apply_rig_tags(doc.scene().clone());
    let w = s.world_matrix(b).unwrap();
    assert!((-w.x_axis.truncate() - DVec3::Y).length() < 1e-9);
}

#[test]
fn a_child_object_is_constrained_in_world_space() {
    let mut doc = Document::new(registry());
    let (child, tag) = doc
        .transact("t", |tx| {
            let target = tx.create("forge.null", None, None)?;
            tx.set_param(target, "transform.pos", Value::Vec3([10.0, 20.0, 30.0]))?;
            let parent = tx.create("forge.null", None, None)?;
            tx.set_param(parent, "transform.pos", Value::Vec3([100.0, 0.0, 0.0]))?;
            tx.set_param(parent, "transform.rot", Value::Vec3([1.0, 0.3, 0.0]))?;
            let child = tx.create("forge.null", Some(parent), None)?;
            let tag = tx.attach_tag(CONSTRAINT_TAG, child, None)?;
            tx.set_param(
                tag,
                "targets",
                Value::LinkRows(vec![LinkRow::new(target, "normal")]),
            )?;
            Ok((child, tag))
        })
        .unwrap();
    let _ = tag;
    let s = forge_rig::tags::apply_rig_tags(doc.scene().clone());
    let p = s.world_matrix(child).unwrap().w_axis.truncate();
    assert!((p - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-9, "{p:?}");
}
