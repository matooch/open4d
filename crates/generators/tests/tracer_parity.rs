//! Parity: clone-toolset spec T1-T4, S1-S2 against tests/golden/tracer and
//! tests/golden/clone-spline. Every case exports the generator's spline through an Extrude
//! with its defaults.

use std::path::Path;
use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, Key, KeyTangent, NodeId, Registry, Track, Value};
use forge_generators::{BuildError, Primitives, evaluate, tracer::sampled_frames};
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};

/// Cases built but differing, with the reason (spec T4): the same frame-1 position reads
/// 10.001106 in `interval` and 10.000106 in `vertices`, so no single rule is within 1e-4 of both.
const KNOWN_MISMATCH: &[&str] = &["interval", "vertices"];
/// Cases whose options have no derived rules (spec S2).
const UNSUPPORTED: &[(&str, &str)] = &[];

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    Arc::new(reg)
}

fn to_mesh(m: &forge_geometry::PolyMesh) -> Mesh {
    export_weld(&Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    })
}

/// The Turtle tab of the case's spline-generator table (`turtle-2`) set on the generator.
fn set_turtle(
    tx: &mut forge_core::Tx,
    gen_: NodeId,
    t: &toml::map::Map<String, toml::Value>,
) -> Result<(), forge_core::EditError> {
    let num = |k: &str| {
        t[k].as_float()
            .or(t[k].as_integer().map(|i| i as f64))
            .unwrap()
    };
    tx.set_param(
        gen_,
        "premise",
        Value::String(t["premise"].as_str().unwrap().into()),
    )?;
    tx.set_param(
        gen_,
        "rules",
        Value::String(t["rules"].as_str().unwrap().into()),
    )?;
    tx.set_param(gen_, "growth", Value::Int(num("growth") as i64))?;
    tx.set_param(
        gen_,
        "default_angle",
        Value::Float(num("default_angle_deg").to_radians()),
    )?;
    tx.set_param(
        gen_,
        "default_movement",
        Value::Float(num("default_movement")),
    )?;
    tx.set_param(
        gen_,
        "tropism",
        Value::Float(num("tropism_percent") / 100.0),
    )?;
    tx.set_param(
        gen_,
        "move_multiplier",
        Value::Float(num("move_multiplier_percent") / 100.0),
    )?;
    tx.set_param(
        gen_,
        "randomness",
        Value::Float(num("randomness_percent") / 100.0),
    )?;
    Ok(())
}

/// An Extrude (defaults) over the case's spline generator.
fn build(
    doc: &mut Document,
    feature: &str,
    name: &str,
    params: &std::collections::BTreeMap<String, toml::Value>,
) -> NodeId {
    doc.transact("case", |tx| {
        let extrude = tx.create("forge.extrude", None, None)?;
        match feature {
            "tracer" => {
                let cube = tx.create("forge.cube", None, None)?;
                for a in ["size_x", "size_y", "size_z"] {
                    tx.set_param(cube, a, Value::Float(200.0))?;
                }
                let keys: Vec<(f64, f64)> = params["reference_key_x"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|k| {
                        let k = k.as_array().unwrap();
                        let v = |x: &toml::Value| {
                            x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap()
                        };
                        (v(&k[0]) / 30.0, v(&k[1]))
                    })
                    .collect();
                // Keys with straight-line (chord) tangents, as the observer's linear keys.
                let mut track = Track::new(keys.iter().map(|(t, v)| Key::new(*t, *v)).collect());
                for i in 0..keys.len() {
                    track.set_tangent(i, KeyTangent::chord());
                }
                tx.set_track(cube, "transform.pos.x", Some(track))?;
                let tracer = tx.create("forge.tracer", Some(extrude), None)?;
                tx.set_param(tracer, "trace_links", Value::LinkList(vec![cube]))?;
                tx.set_param(
                    tracer,
                    "sample_step",
                    Value::Int(params["sample_step"].as_integer().unwrap()),
                )?;
            }
            _ => {
                let gen_ = tx.create("forge.clone_spline", Some(extrude), None)?;
                let mode = match params["spline_mode_label"].as_str().unwrap() {
                    "Spline" => "spline",
                    "Turtle" => "turtle",
                    _ => "simple",
                };
                tx.set_param(gen_, "mode", Value::Enum(mode.into()))?;
                if let Some(t) = params
                    .values()
                    .find_map(|v| v.as_table().filter(|t| t.contains_key("premise")))
                {
                    set_turtle(tx, gen_, t)?;
                }
                if mode == "spline" {
                    let circle = tx.create("forge.circle_spline", None, None)?;
                    tx.set_param(gen_, "source", Value::Link(Some(circle)))?;
                }
            }
        }
        let _ = name;
        Ok(extrude)
    })
    .unwrap()
}

#[test]
fn tracer_and_clone_spline_cases_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut mismatched, mut unsupported) = (vec![], vec![], vec![]);
    for case in discover_cases(&root).unwrap() {
        if case.feature != "tracer" && case.feature != "clone-spline" {
            continue;
        }
        let m = case.manifest().unwrap();
        let mut doc = Document::new(reg.clone());
        let extrude = build(&mut doc, &case.feature, &case.name, &m.params);
        let frame = m
            .params
            .get("export_frame")
            .and_then(|v| v.as_integer())
            .unwrap_or(0);
        let ctx = EvalCtx::at_frame(frame, 30.0);
        let result = evaluate(doc.scene(), extrude, &ctx);
        if UNSUPPORTED.iter().any(|(n, _)| *n == case.name) {
            assert!(
                matches!(result, Err(BuildError::Unsupported(_)) | Ok(None)),
                "{}",
                case.name
            );
            unsupported.push(case.name.clone());
            continue;
        }
        let mesh = result.unwrap().expect("geometry");
        let want = export_weld(&case.expected_mesh().unwrap().unwrap());
        let r = compare_meshes(&want, &to_mesh(&mesh), &m.tolerance);
        if KNOWN_MISMATCH.contains(&case.name.as_str()) {
            // Same counts and face lists; only the reference's animation-solver noise differs.
            assert!(r.topology_match, "{}: {:?}", case.name, r.failures);
            assert!(r.max_position_error.unwrap() < 2e-3, "{}", case.name);
            assert!(!r.passed(), "{} matches now", case.name);
            mismatched.push(case.name.clone());
            continue;
        }
        assert!(
            r.passed(),
            "{}: {:?} (max {:?})",
            case.name,
            r.failures,
            r.max_position_error
        );
        matched.push(case.name.clone());
    }
    println!("matched {matched:?}");
    assert_eq!(unsupported.len(), UNSUPPORTED.len());
    assert_eq!(mismatched.len(), KNOWN_MISMATCH.len());
    assert_eq!(matched.len(), 4, "{matched:?}");
}

#[test]
fn sampled_frames_are_the_first_and_the_multiples_of_the_step() {
    assert_eq!(sampled_frames(0, 5), Vec::<i64>::new());
    assert_eq!(sampled_frames(30, 5), vec![1, 5, 10, 15, 20, 25, 30]);
    assert_eq!(sampled_frames(4, 1), vec![1, 2, 3, 4]);
}

#[test]
fn connect_all_joins_the_clones_of_a_matrix_and_other_modes_are_unsupported() {
    let reg = registry();
    let mut doc = Document::new(reg);
    let (tracer, extrude) = doc
        .transact("t", |tx| {
            let m = tx.create("forge.matrix", None, None)?;
            tx.set_param(m, "mode", Value::Enum("linear".into()))?;
            tx.set_param(m, "linear_count", Value::Int(3))?;
            let extrude = tx.create("forge.extrude", None, None)?;
            let t = tx.create("forge.tracer", Some(extrude), None)?;
            tx.set_param(t, "tracing_mode", Value::Enum("connect_all".into()))?;
            tx.set_param(t, "trace_links", Value::LinkList(vec![m]))?;
            Ok((t, extrude))
        })
        .unwrap();
    let loops = forge_generators::splines::sampled_loops(doc.scene(), tracer).unwrap();
    assert_eq!(loops.len(), 1);
    let ys: Vec<f64> = loops[0].0.iter().map(|p| p.y).collect();
    assert_eq!(ys, vec![0.0, 50.0, 100.0]);
    assert!(!loops[0].1);
    doc.transact("single", |tx| {
        tx.set_param(tracer, "tracing_mode", Value::Enum("trace_single".into()))
    })
    .unwrap();
    assert!(matches!(
        evaluate(doc.scene(), extrude, &EvalCtx::default()),
        Err(BuildError::Unsupported(_))
    ));
}
