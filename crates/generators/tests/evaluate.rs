//! docs/specs/SUBDIVISION.md §1, §4: the generator node and scene evaluation.

use std::sync::Arc;

use forge_core::{Document, Registry, Value};
use forge_generators::{Primitives, evaluate};

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    Document::new(Arc::new(r))
}

#[test]
fn subdivision_consumes_its_first_mesh_child() {
    let mut d = doc();
    let (sub, cube) = d
        .transact("b", |tx| {
            let sub = tx.create("forge.subdivision", None, None)?;
            let null = tx.create("forge.null", Some(sub), None)?; // no mesh: skipped
            let cube = tx.create("forge.cube", Some(sub), None)?;
            let _ = null;
            Ok((sub, cube))
        })
        .unwrap();
    let m = evaluate(d.scene(), sub).unwrap().expect("subdivided cube");
    assert_eq!(
        (m.points.len(), m.polys.len()),
        (98, 96),
        "default viewport level 2"
    );

    // the child's transform places the input inside the generator
    d.transact("move", |tx| {
        tx.set_param(cube, "transform.pos", Value::Vec3([1000.0, 0.0, 0.0]))
    })
    .unwrap();
    let m = evaluate(d.scene(), sub).unwrap().unwrap();
    assert!(m.points.iter().all(|p| p.x > 900.0));

    d.transact("lvl", |tx| tx.set_param(sub, "level_view", Value::Int(0)))
        .unwrap();
    assert_eq!(evaluate(d.scene(), sub).unwrap().unwrap().points.len(), 8);
}

#[test]
fn nodes_without_geometry_evaluate_to_none() {
    let mut d = doc();
    let (sub, null) = d
        .transact("b", |tx| {
            Ok((
                tx.create("forge.subdivision", None, None)?,
                tx.create("forge.null", None, None)?,
            ))
        })
        .unwrap();
    assert!(evaluate(d.scene(), sub).unwrap().is_none());
    assert!(evaluate(d.scene(), null).unwrap().is_none());
    let cube = d
        .transact("c", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    assert_eq!(evaluate(d.scene(), cube).unwrap().unwrap().points.len(), 8);
}

/// docs/specs/BOOLE.md §1: A and B are the first two mesh children, placed by their
/// transforms and consumed.
#[test]
fn boole_consumes_its_first_two_mesh_children() {
    use forge_generators::is_consumed;
    let mut d = doc();
    let (boole, a, b, c) = d
        .transact("b", |tx| {
            let boole = tx.create("forge.boole", None, None)?;
            let a = tx.create("forge.cube", Some(boole), None)?;
            tx.create("forge.null", Some(boole), None)?; // no mesh: skipped
            let b = tx.create("forge.cube", Some(boole), None)?;
            let c = tx.create("forge.cube", Some(boole), None)?; // third: ignored
            tx.set_param(b, "transform.pos", Value::Vec3([50.0, 50.0, 50.0]))?;
            Ok((boole, a, b, c))
        })
        .unwrap();
    let m = evaluate(d.scene(), boole).unwrap().unwrap();
    assert_eq!((m.points.len(), m.polys.len()), (14, 12), "A subtract B");
    assert!(is_consumed(d.scene(), a) && is_consumed(d.scene(), b));
    assert!(!is_consumed(d.scene(), c));

    d.transact("mode", |tx| {
        tx.set_param(boole, "mode", Value::Enum("union".into()))
    })
    .unwrap();
    let m = evaluate(d.scene(), boole).unwrap().unwrap();
    assert_eq!(m.points.len(), 20);

    d.transact("hq", |tx| {
        tx.set_param(boole, "high_quality", Value::Bool(false))
    })
    .unwrap();
    assert!(matches!(
        evaluate(d.scene(), boole),
        Err(forge_generators::BuildError::Unsupported(_))
    ));
}

#[test]
fn boole_with_one_input_passes_it_through() {
    let mut d = doc();
    let boole = d
        .transact("b", |tx| {
            let boole = tx.create("forge.boole", None, None)?;
            tx.create("forge.cube", Some(boole), None)?;
            Ok(boole)
        })
        .unwrap();
    assert_eq!(evaluate(d.scene(), boole).unwrap().unwrap().points.len(), 8);
}

/// Curved input: a sphere minus a cube corner stays closed with plausible volume.
#[test]
fn boole_of_sphere_and_cube_is_closed() {
    use std::collections::BTreeMap;
    let sphere = forge_generators::build("forge.sphere", &BTreeMap::new()).unwrap();
    let mut cube = forge_generators::build("forge.cube", &BTreeMap::new()).unwrap();
    for p in &mut cube.points {
        *p += glam::DVec3::splat(60.0);
    }
    let vol = |m: &forge_geometry::PolyMesh| -> f64 {
        m.polys
            .iter()
            .map(|f| {
                let p = |i: usize| m.points[f[i] as usize];
                (1..f.len() - 1)
                    .map(|i| p(0).dot(p(i).cross(p(i + 1))) / 6.0)
                    .sum::<f64>()
            })
            .sum()
    };
    let he = |m: &forge_geometry::PolyMesh| forge_geometry::HalfEdgeMesh::build(m).unwrap();
    for (mode, hide) in [("a_minus_b", true), ("union", false), ("intersect", true)] {
        let v = BTreeMap::from([
            ("mode".to_string(), Value::Enum(mode.into())),
            ("hide_new_edges".to_string(), Value::Bool(hide)),
        ]);
        let r = forge_generators::boole(&sphere, &cube, &v).unwrap();
        r.validate().unwrap();
        assert!(he(&r).is_closed(), "{mode}");
        let (vs, vc, vr) = (vol(&sphere), vol(&cube), vol(&r));
        assert!(vr > 0.0 && vr < vs + vc, "{mode}: {vr}");
    }
    let parts = |mode: &str| {
        let v = BTreeMap::from([("mode".to_string(), Value::Enum(mode.into()))]);
        vol(&forge_generators::boole(&sphere, &cube, &v).unwrap())
    };
    let (i, s) = (parts("intersect"), parts("a_minus_b"));
    assert!(
        (i + s - vol(&sphere)).abs() < 1e-6 * vol(&sphere),
        "{i} + {s}"
    );
    let u = parts("union");
    assert!((u + i - vol(&sphere) - vol(&cube)).abs() < 1e-6 * vol(&cube));
}
