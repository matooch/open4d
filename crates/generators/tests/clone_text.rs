//! O31 clone channels and depth, with O48 layout shared by the Text spline.

use std::{collections::BTreeMap, path::Path, sync::Arc};

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::{Primitives, clone_text, evaluate};
use forge_parity_harness::{Samples, discover_cases};

fn document() -> Document {
    let mut registry = Registry::with_builtins();
    registry.install(&Primitives).unwrap();
    registry.install(&CloneToolset).unwrap();
    Document::new(Arc::new(registry))
}

// Only the numerical XY pivots are pinned: the reference default font has unknown advances.
const FONT_PIVOT_MISMATCHES: &[&str] = &["defaults", "depth"];

#[test]
fn o31_count_order_clone_channels_and_depth() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases = discover_cases(&root).unwrap();
    let cases: Vec<_> = cases
        .into_iter()
        .filter(|c| c.feature == "clone-text")
        .collect();
    assert_eq!(cases.len(), 2);
    let mut mismatched = Vec::new();
    for case in cases {
        let m = case.manifest().unwrap();
        let p = &m.params;
        let mut document = document();
        let depth = p["depth"].as_float().unwrap();
        let text = p["text_spline"].as_str().unwrap();
        let node = document
            .transact("O31", |tx| {
                let node = tx.create(clone_text::TYPE, None, None)?;
                tx.set_param(node, "text", Value::String(text.into()))?;
                tx.set_param(
                    node,
                    "height",
                    Value::Float(p["height"].as_float().unwrap()),
                )?;
                tx.set_param(node, "depth", Value::Float(depth))?;
                tx.set_param(
                    node,
                    "subdivision",
                    Value::Int(p["subdivision"].as_integer().unwrap()),
                )?;
                tx.set_param(
                    node,
                    "start_cap",
                    Value::Bool(p["start_cap"].as_bool().unwrap()),
                )?;
                tx.set_param(
                    node,
                    "end_cap",
                    Value::Bool(p["end_cap"].as_bool().unwrap()),
                )?;
                Ok(node)
            })
            .unwrap();
        let ctx = EvalCtx::default();
        let clones = clone_text::clone_data(document.scene(), node, &ctx)
            .unwrap()
            .unwrap();
        let Samples::Clones {
            clones: expected,
            tolerance,
        } = case.samples().unwrap().remove(0).1
        else {
            panic!("expected clone samples");
        };
        assert_eq!(clones.len(), expected.len(), "{}", case.name);
        assert!(
            clones
                .matrices
                .windows(2)
                .all(|m| m[0].w_axis.x < m[1].w_axis.x)
        );
        let mut max_xy_error: f64 = 0.0;
        for (i, sample) in expected.iter().enumerate() {
            let m = clones.matrices[i];
            assert_eq!(m.x_axis.truncate().to_array(), sample.x);
            assert_eq!(m.y_axis.truncate().to_array(), sample.y);
            assert_eq!(m.z_axis.truncate().to_array(), sample.z);
            assert_eq!(m.w_axis.z, sample.pos[2]);
            assert_eq!(clones.weights[i], sample.weight);
            assert_eq!(clones.colors[i], sample.color);
            assert_eq!(clones.visible[i], sample.visible);
            max_xy_error = max_xy_error
                .max((m.w_axis.x - sample.pos[0]).abs())
                .max((m.w_axis.y - sample.pos[1]).abs());
        }
        if max_xy_error > tolerance {
            mismatched.push(case.name.clone());
        }
        eprintln!("{}: max pivot error {max_xy_error:.6}", case.name);
        let mesh = evaluate(document.scene(), node, &ctx).unwrap().unwrap();
        let expected = case.expected_mesh().unwrap().unwrap();
        let z_bounds = |z: Vec<f64>| {
            [
                z.iter().copied().fold(f64::INFINITY, f64::min),
                z.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            ]
        };
        assert_eq!(
            z_bounds(mesh.points.iter().map(|p| p.z).collect()),
            z_bounds(expected.positions.iter().map(|p| p[2]).collect())
        );
        // Closed, consistently wound glyph rims and caps, including the letter counters.
        let mut edges = BTreeMap::<_, (u32, i32)>::new();
        for f in &mesh.polys {
            for (&a, &b) in f.iter().zip(f.iter().cycle().skip(1)) {
                assert_ne!(a, b);
                let (count, balance) = edges.entry((a.min(b), a.max(b))).or_default();
                *count += 1;
                *balance += if a < b { 1 } else { -1 };
            }
        }
        assert!(
            edges.values().all(|&v| v == (2, 0)),
            "unclosed {}",
            case.name
        );
    }
    mismatched.sort();
    assert_eq!(mismatched, FONT_PIVOT_MISMATCHES);
}

#[test]
fn groups_and_effectors_preserve_glyph_geometry_and_order() {
    let mut doc = document();
    let (text, plain) = doc
        .transact("Groups", |tx| {
            let text = tx.create(clone_text::TYPE, None, None)?;
            tx.set_param(text, "text", Value::String("AB C\nD".into()))?;
            let plain = tx.create("forge.effector.plain", None, None)?;
            Ok((text, plain))
        })
        .unwrap();
    let ctx = EvalCtx::default();
    let base = evaluate(doc.scene(), text, &ctx).unwrap().unwrap();
    for (group, count) in [("letters", 4), ("words", 3), ("lines", 2)] {
        doc.transact("Set grouping", |tx| {
            tx.set_param(text, "group", Value::Enum(group.into()))
        })
        .unwrap();
        let data = clone_text::clone_data(doc.scene(), text, &ctx)
            .unwrap()
            .unwrap();
        assert_eq!(data.index, (0..count).collect::<Vec<_>>());
        assert_eq!(data.child, data.index);
        let mesh = evaluate(doc.scene(), text, &ctx).unwrap().unwrap();
        assert_eq!(mesh.polys, base.polys);
        assert!(
            mesh.points
                .iter()
                .zip(&base.points)
                .all(|(a, b)| (*a - *b).length() < 1e-10)
        );
    }
    doc.transact("Plain", |tx| {
        tx.set_param(text, "effectors", Value::LinkList(vec![plain]))
    })
    .unwrap();
    let moved = evaluate(doc.scene(), text, &ctx).unwrap().unwrap();
    assert!(
        moved
            .points
            .iter()
            .zip(&base.points)
            .all(|(a, b)| (a.y - b.y - 100.0).abs() < 1e-10)
    );
}
