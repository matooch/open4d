//! Parity: docs/specs/GENERATORS3.md §3 against tests/golden/metaball (O20).

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;

mod common;

use common::{Table, apply, resolved_params};
use forge_core::{Document, EvalCtx, Registry, Value};
use forge_generators::{Primitives, evaluate};
use forge_parity_harness::{GoldenCase, Mesh, compare_volume_mesh, discover_cases, export_weld};

const FEATURE: &str = "metaball";

/// The case whose reference mesh is not closed (Euler 12, boundary edges), so its topology
/// cannot equal a closed iso-surface mesh; its geometry is still checked below.
const KNOWN_MISMATCH: &[&str] = &["metaball/editor-sub"];

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    Arc::new(reg)
}

fn inputs(params: &Table) -> Vec<(&String, &Table)> {
    let mut v: Vec<_> = params
        .iter()
        .filter_map(|(k, v)| v.as_table().map(|t| (k, t)))
        .collect();
    v.sort_by_key(|(k, _)| k.trim_start_matches("input").parse::<u32>().unwrap_or(1));
    v
}

fn type_of(t: &Table) -> String {
    format!("forge.{}", t["type"].as_str().unwrap())
}

/// Builds the case and returns (mesh, lattice spacing).
fn build(case: &GoldenCase) -> (Mesh, f64) {
    let reg = registry();
    let m = case.manifest().unwrap();
    let params = resolved_params(case, &m);
    let gen_type = "forge.metaball".to_string();
    let mut own = Table::new();
    for (k, v) in &params {
        // The scripted form records the hull as a percentage; our percent params store a
        // fraction.
        if k == "hull_value" {
            own.insert(k.clone(), toml::Value::Float(v.as_float().unwrap() / 100.0));
        } else {
            own.insert(k.clone(), v.clone());
        }
    }
    let voxel = own["editor_subdiv"].as_float().unwrap();
    let mut doc = Document::new(reg.clone());
    let root = doc
        .transact("case", |tx| {
            let root = tx.create(&gen_type, None, None)?;
            apply(tx, &reg, root, &gen_type, &own);
            for (_, t) in inputs(&params) {
                let ty = type_of(t);
                let child = tx.create(&ty, Some(root), None)?;
                // The scripted form renames the sphere's own Type row to avoid the child's
                // `type` key.
                let mut child_params = t.clone();
                if let Some(kind) = child_params.remove("sphere_type") {
                    child_params.insert("type".into(), kind);
                }
                apply(tx, &reg, child, &ty, &child_params);
            }
            Ok(root)
        })
        .unwrap();
    let mesh = evaluate(doc.scene(), root, &EvalCtx::default())
        .expect("case builds")
        .expect("geometry");
    let world = doc.scene().local_transform(root).unwrap().matrix();
    let out = Mesh {
        positions: mesh
            .points
            .iter()
            .map(|p| world.transform_point3(*p).to_array())
            .collect(),
        faces: mesh.polys.clone(),
    };
    (out, voxel)
}

#[test]
fn metaball_cases_match_the_reference_geometrically() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let (mut matched, mut pinned) = (BTreeSet::new(), BTreeSet::new());
    let mut failed: Vec<String> = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != FEATURE {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let expected = export_weld(&case.expected_mesh().unwrap().unwrap());
        let (ours, voxel) = build(&case);
        let ours = export_weld(&ours);
        let r = compare_volume_mesh(&expected, &ours, voxel);
        println!(
            "{key}: passed {} haus {:.2}/{:.2} area {:?} volume {:?} euler {:?} closed {:?}",
            r.passed(),
            r.hausdorff_expected_to_actual,
            r.hausdorff_actual_to_expected,
            r.area,
            r.volume,
            r.euler,
            r.closed
        );
        if KNOWN_MISMATCH.contains(&key.as_str()) {
            assert!(
                !r.passed(),
                "{key} matches now: remove it from KNOWN_MISMATCH"
            );
            // The geometry still has to match; only the topology may differ.
            let only_topology = r
                .failures
                .iter()
                .all(|f| f.contains("euler") || f.contains("closed"));
            assert!(only_topology, "{key}: {:#?}", r.failures);
            pinned.insert(key);
            continue;
        }
        if !r.passed() {
            failed.push(format!("{key}: {:#?}", r.failures));
            continue;
        }
        matched.insert(key);
    }
    assert!(failed.is_empty(), "unlisted mismatches: {failed:#?}");
    assert_eq!(
        matched,
        BTreeSet::from([
            "metaball/hull-50".to_string(),
            "metaball/two-spheres".to_string()
        ])
    );
    assert_eq!(pinned, BTreeSet::from(["metaball/editor-sub".to_string()]));
}

#[test]
fn unsupported_children_are_stored_options_only() {
    // Exponential Falloff and Accurate Normals are stored; the default field is used.
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let reg = Arc::new(reg);
    let mut doc = Document::new(reg.clone());
    let (root, child) = doc
        .transact("case", |tx| {
            let root = tx.create("forge.metaball", None, None)?;
            let child = tx.create("forge.sphere", Some(root), None)?;
            tx.set_param(child, "radius", Value::Float(100.0))?;
            Ok((root, child))
        })
        .unwrap();
    let a = evaluate(doc.scene(), root, &EvalCtx::default())
        .unwrap()
        .unwrap();
    doc.transact("options", |tx| {
        tx.set_param(root, "exponential_falloff", Value::Bool(true))?;
        tx.set_param(root, "accurate_normals", Value::Bool(true))
    })
    .unwrap();
    let b = evaluate(doc.scene(), root, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_eq!(a.points, b.points);
    assert_eq!(a.polys, b.polys);
    let _ = child;
}

#[test]
fn no_sources_or_zero_hull_give_no_geometry() {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    let reg = Arc::new(reg);
    let mut doc = Document::new(reg.clone());
    let root = doc
        .transact("case", |tx| tx.create("forge.metaball", None, None))
        .unwrap();
    assert!(
        evaluate(doc.scene(), root, &EvalCtx::default())
            .unwrap()
            .is_none()
    );
    doc.transact("hull", |tx| {
        tx.set_param(root, "hull_value", Value::Float(0.0))
    })
    .unwrap();
    doc.transact("child", |tx| tx.create("forge.sphere", Some(root), None))
        .unwrap();
    assert!(
        evaluate(doc.scene(), root, &EvalCtx::default())
            .unwrap()
            .is_none()
    );
    assert!(
        doc.transact("bad", |tx| tx.set_param(
            root,
            "editor_subdiv",
            Value::Float(f64::NAN)
        ))
        .is_err()
    );
}
