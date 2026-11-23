//! O21 object-mode distributions (clone-toolset spec C2).

use std::path::Path;
use std::sync::Arc;

use forge_clone_engine::{CloneData, CloneToolset};
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{Primitives, clone_data};
use forge_parity_harness::{Samples, compare_samples, samples::Clone as Sample};
use glam::DVec3;

fn scene(values: &toml::Value) -> (Document, NodeId, NodeId) {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let (cloner, source) = doc
        .transact("O21 setup", |tx| {
            let source_type = match values["object"].as_str().unwrap() {
                "Cube" => "forge.cube",
                _ => "forge.sphere",
            };
            let source = tx.create(source_type, None, None)?;
            if source_type == "forge.sphere" {
                tx.set_param(source, "segments", Value::Int(8))?;
            }
            let cloner = tx.create("forge.cloner", None, None)?;
            tx.create("forge.cube", Some(cloner), None)?;
            tx.set_param(cloner, "mode", Value::Enum("object".into()))?;
            tx.set_param(cloner, "object", Value::Link(Some(source)))?;
            let dist = values["distribution"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .replace(' ', "_");
            tx.set_param(cloner, "distribution", Value::Enum(dist))?;
            if let Some(v) = values.get("align_clone").and_then(toml::Value::as_bool) {
                tx.set_param(cloner, "align_clone", Value::Bool(v))?;
            }
            for (key, param) in [("seed", "object_seed"), ("count", "object_count")] {
                if let Some(v) = values.get(key).and_then(toml::Value::as_integer) {
                    tx.set_param(cloner, param, Value::Int(v))?;
                }
            }
            Ok((cloner, source))
        })
        .unwrap();
    (doc, cloner, source)
}

fn case(name: &str) -> (Document, NodeId, NodeId, Samples) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/cloner")
        .join(name);
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(dir.join("case.toml")).unwrap()).unwrap();
    let samples = forge_parity_harness::samples::parse_samples(
        &std::fs::read_to_string(dir.join("samples.json")).unwrap(),
    )
    .unwrap();
    let (doc, cloner, source) = scene(&manifest["params"]);
    (doc, cloner, source, samples)
}

fn data(doc: &Document, cloner: NodeId) -> CloneData {
    clone_data(doc.scene(), cloner, &EvalCtx::default())
        .unwrap()
        .unwrap()
}

fn samples(data: &CloneData, tolerance: f64) -> Samples {
    Samples::Clones {
        tolerance,
        clones: data
            .matrices
            .iter()
            .enumerate()
            .map(|(i, m)| Sample {
                pos: m.w_axis.truncate().to_array(),
                x: m.x_axis.truncate().to_array(),
                y: m.y_axis.truncate().to_array(),
                z: m.z_axis.truncate().to_array(),
                color: data.colors[i],
                weight: data.weights[i],
                visible: data.visible[i],
            })
            .collect(),
    }
}

#[test]
fn deterministic_object_modes() {
    let mut failures = Vec::new();
    for name in [
        "object-axis",
        "object-vertex-align-off",
        "object-vertex",
        "object-edge",
        "object-polycenter",
    ] {
        let (doc, cloner, _, expected) = case(name);
        let actual = samples(&data(&doc, cloner), expected.tolerance());
        let r = compare_samples(&expected, &actual);
        if !r.passed() {
            failures.push(name);
            let (
                Samples::Clones {
                    clones: a,
                    tolerance,
                },
                Samples::Clones { clones: b, .. },
            ) = (&expected, &actual)
            else {
                panic!("expected clones")
            };
            assert_eq!(a.len(), b.len(), "{name}");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                for (a, b) in [(a.pos, b.pos), (a.z, b.z), (a.color, b.color)] {
                    assert!(
                        DVec3::from_array(a).abs_diff_eq(DVec3::from_array(b), *tolerance),
                        "{name}: non-roll mismatch on clone {i}"
                    );
                }
                assert_eq!((a.visible, a.weight), (b.visible, b.weight));
                let (x, y, z) = (
                    DVec3::from_array(b.x),
                    DVec3::from_array(b.y),
                    DVec3::from_array(b.z),
                );
                assert!(
                    x.cross(y).abs_diff_eq(z, *tolerance),
                    "{name}: invalid frame at clone {i}"
                );
            }
        }
    }
    assert_eq!(failures, ["object-vertex", "object-polycenter"]);
}

#[test]
fn object_seeded_distributions() {
    for name in ["object-surface", "object-surface-seed", "object-volume"] {
        let (mut doc, cloner, _, expected) = case(name);
        let Samples::Clones { clones, .. } = expected else {
            panic!("expected clones")
        };
        let a = data(&doc, cloner);
        assert_eq!(a.len(), clones.len());
        assert!(a.visible.iter().all(|v| *v));
        assert_eq!(a.matrices, data(&doc, cloner).matrices);
        let s = |points: Vec<DVec3>| {
            let mean = points.iter().copied().sum::<DVec3>() / points.len() as f64;
            let var =
                points.iter().map(|p| (*p - mean).powf(2.0)).sum::<DVec3>() / points.len() as f64;
            (
                mean,
                var,
                points.iter().copied().reduce(DVec3::min).unwrap(),
                points.iter().copied().reduce(DVec3::max).unwrap(),
            )
        };
        let (m1, v1, lo1, hi1) = s(a.matrices.iter().map(|m| m.w_axis.truncate()).collect());
        let (m2, v2, lo2, hi2) = s(clones.iter().map(|c| DVec3::from_array(c.pos)).collect());
        assert!(
            (m1 - m2)
                .abs()
                .cmple(((v1 + v2) / a.len() as f64).sqrt() * 3.5)
                .all(),
            "{name}: means {m1} {m2}"
        );
        assert!(
            v1.cmpge(v2 * 0.25).all() && v1.cmple(v2 * 4.0).all(),
            "{name}: variances {v1} {v2}"
        );
        let extent = (hi2 - lo2) * 0.45;
        assert!(
            (lo1 - lo2).abs().cmple(extent).all() && (hi1 - hi2).abs().cmple(extent).all(),
            "{name}: bounds {lo1} {hi1} vs {lo2} {hi2}"
        );
        doc.transact("Change seed", |tx| {
            tx.set_param(cloner, "object_seed", Value::Int(2718))
        })
        .unwrap();
        assert_ne!(a.matrices, data(&doc, cloner).matrices);
    }
}

#[test]
fn named_point_selection_preserves_source_order() {
    let (mut doc, cloner, source, _) = case("object-vertex-align-off");
    doc.transact("Restrict points", |tx| {
        let tag = tx.attach_tag(forge_core::tags::POINT_SELECTION, source, None)?;
        tx.rename(tag, "poles")?;
        tx.set_param(tag, "indices", Value::IndexList(vec![0, 1]))?;
        tx.set_param(cloner, "selection", Value::String("poles".into()))
    })
    .unwrap();
    let d = data(&doc, cloner);
    assert_eq!(d.len(), 2);
    assert_eq!(d.matrices[0].w_axis.y, -100.0);
    assert_eq!(d.matrices[1].w_axis.y, 100.0);
}
