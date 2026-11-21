//! Parity and unit tests for the Fracture generator (clone-toolset spec F1-F3) against
//! tests/golden/fracture.

use std::path::Path;
use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, NodeId, Registry, Tx, Value};
use forge_generators::{BuildError, Primitives, evaluate, from_poly_mesh};
use forge_geometry::PolyMesh;
use forge_parity_harness::{Mesh, compare_meshes, discover_cases, export_weld};

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    Arc::new(reg)
}

fn weld(m: &PolyMesh) -> Mesh {
    export_weld(&Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    })
}

/// A polygon object child holding `mesh` under a new Fracture.
fn fracture_of(
    tx: &mut Tx,
    mesh: &PolyMesh,
    mode: &str,
) -> Result<(NodeId, NodeId), forge_core::EditError> {
    let f = tx.create("forge.fracture", None, None)?;
    tx.set_param(f, "mode", Value::Enum(mode.into()))?;
    let poly = tx.create("forge.polygon", Some(f), None)?;
    tx.set_param(poly, "mesh", Value::Mesh(Arc::new(from_poly_mesh(mesh))))?;
    Ok((f, poly))
}

#[test]
fn explode_segments_cases_match_the_reference() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut matched = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "fracture" {
            continue;
        }
        let m = case.manifest().unwrap();
        // The input is the 24-point, 6-quad cube of the case: the reference output equals it.
        let want = case.expected_mesh().unwrap().unwrap();
        let input = PolyMesh {
            points: want
                .positions
                .iter()
                .map(|p| glam::DVec3::from_array(*p))
                .collect(),
            polys: want.faces.clone(),
        };
        let mut doc = Document::new(reg.clone());
        let (f, _) = doc
            .transact("case", |tx| fracture_of(tx, &input, "explode_segments"))
            .unwrap();
        // A Plain effector as a child of the Fracture (case `plain`) changes nothing.
        if case.name == "plain" {
            doc.transact("effector", |tx| {
                let e = tx.create("forge.effector.plain", Some(f), None)?;
                tx.set_param(e, "position", Value::Bool(true))
            })
            .unwrap();
        }
        let mesh = evaluate(doc.scene(), f, &EvalCtx::default())
            .unwrap()
            .unwrap();
        let r = compare_meshes(&export_weld(&want), &weld(&mesh), &m.tolerance);
        assert!(r.passed(), "{}: {:?}", case.name, r.failures);
        matched.push(case.name);
    }
    assert_eq!(matched.len(), 2, "{matched:?}");
}

#[test]
fn islands_become_fragments_and_listed_effectors_move_them() {
    let reg = registry();
    let mut doc = Document::new(reg);
    // Two separate triangles sharing no point.
    let mesh = PolyMesh {
        points: vec![
            [0.0, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [100.0, 0.0, 0.0],
            [110.0, 0.0, 0.0],
            [100.0, 10.0, 0.0],
        ]
        .into_iter()
        .map(glam::DVec3::from_array)
        .collect(),
        polys: vec![vec![0, 1, 2], vec![3, 4, 5]],
    };
    let (f, e) = doc
        .transact("f", |tx| {
            let (f, _) = fracture_of(tx, &mesh, "explode_segments")?;
            let e = tx.create("forge.effector.plain", None, None)?;
            Ok((f, e))
        })
        .unwrap();
    let ctx = EvalCtx::default();
    let plain = evaluate(doc.scene(), f, &ctx).unwrap().unwrap();
    assert_eq!(plain.points, mesh.points);
    doc.transact("m", |tx| {
        tx.set_param(f, "effectors", Value::LinkList(vec![e]))
    })
    .unwrap();
    let moved = evaluate(doc.scene(), f, &ctx).unwrap().unwrap();
    assert_eq!(moved.polys, mesh.polys);
    for (a, b) in moved.points.iter().zip(&mesh.points) {
        assert!((a.y - b.y - 100.0).abs() < 1e-9 && (a.x - b.x).abs() < 1e-9);
    }
    doc.transact("c", |tx| {
        tx.set_param(f, "mode", Value::Enum("explode_segments_connect".into()))
    })
    .unwrap();
    assert!(matches!(
        evaluate(doc.scene(), f, &ctx),
        Err(BuildError::Unsupported(_))
    ));
}
