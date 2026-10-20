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

/// GENERATORS2.md §1, §3: our own weld (the export weld hides it in the parity cases).
#[test]
fn symmetry_and_connect_weld_their_own_points() {
    use std::collections::BTreeMap;
    let mut cube = forge_generators::build("forge.cube", &BTreeMap::new()).unwrap();
    for p in &mut cube.points {
        p.x += 100.0; // -X face on the mirror plane
    }
    let weld = |on: bool, key: &str| BTreeMap::from([(key.to_string(), Value::Bool(on))]);
    let s = forge_generators::symmetry(&cube, &weld(true, "weld_points")).unwrap();
    assert_eq!((s.points.len(), s.polys.len()), (12, 12));
    let s = forge_generators::symmetry(&cube, &weld(false, "weld_points")).unwrap();
    assert_eq!((s.points.len(), s.polys.len()), (16, 12));

    let mut b = cube.clone();
    for p in &mut b.points {
        p.x += 200.0;
    }
    let pair = [cube.clone(), b];
    let c = forge_generators::connect(&pair, &weld(true, "weld")).unwrap();
    assert_eq!((c.points.len(), c.polys.len()), (12, 11));
    let c = forge_generators::connect(&pair, &weld(false, "weld")).unwrap();
    assert_eq!((c.points.len(), c.polys.len()), (16, 12));
}

/// GENERATORS2.md §0, §4: modeling generators consume their inputs; instances don't.
#[test]
fn modeling_generators_consume_inputs_and_instances_follow_their_reference() {
    use forge_generators::is_consumed;
    let mut d = doc();
    let (sym, cube, inst) = d
        .transact("b", |tx| {
            let sym = tx.create("forge.symmetry", None, None)?;
            let cube = tx.create("forge.cube", Some(sym), None)?;
            tx.set_param(cube, "transform.pos", Value::Vec3([150.0, 0.0, 0.0]))?;
            let inst = tx.create("forge.instance", None, None)?;
            tx.set_param(inst, "reference", Value::Link(Some(sym)))?;
            Ok((sym, cube, inst))
        })
        .unwrap();
    assert!(is_consumed(d.scene(), cube));
    assert!(!is_consumed(d.scene(), sym));
    let m = evaluate(d.scene(), inst).unwrap().unwrap();
    assert_eq!(m.points.len(), 16, "instance of the symmetry result");
    let empty = d
        .transact("e", |tx| tx.create("forge.instance", None, None))
        .unwrap();
    assert!(evaluate(d.scene(), empty).unwrap().is_none());
}

#[test]
fn instance_cycles_have_no_geometry() {
    let mut d = doc();
    let (a, b) = d
        .transact("c", |tx| {
            let a = tx.create("forge.instance", None, None)?;
            let b = tx.create("forge.instance", None, None)?;
            tx.set_param(a, "reference", Value::Link(Some(b)))?;
            tx.set_param(b, "reference", Value::Link(Some(a)))?;
            Ok((a, b))
        })
        .unwrap();
    assert!(evaluate(d.scene(), a).unwrap().is_none());
    assert!(evaluate(d.scene(), b).unwrap().is_none());
}

/// GENERATORS2.md §5–10: spline generators consume their spline children and build
/// valid meshes; closed extrusions and lofts are closed.
#[test]
fn spline_generators_build_closed_meshes() {
    use forge_generators::is_consumed;
    let mut d = doc();
    let (ex, circle, lathe, loft, sweep) = d
        .transact("b", |tx| {
            let ex = tx.create("forge.extrude", None, None)?;
            let circle = tx.create("forge.circle_spline", Some(ex), None)?;
            let lathe = tx.create("forge.lathe", None, None)?;
            let rect = tx.create("forge.rectangle_spline", Some(lathe), None)?;
            tx.set_param(rect, "width", Value::Float(100.0))?;
            tx.set_param(rect, "transform.pos", Value::Vec3([150.0, 0.0, 0.0]))?;
            tx.set_param(lathe, "angle", Value::Float(4.0))?;
            let loft = tx.create("forge.loft", None, None)?;
            for (r, y) in [(200.0, 0.0), (100.0, 200.0), (150.0, 400.0)] {
                let c = tx.create("forge.circle_spline", Some(loft), None)?;
                tx.set_param(c, "radius", Value::Float(r))?;
                tx.set_param(c, "plane", Value::Enum("xz".into()))?;
                tx.set_param(c, "transform.pos", Value::Vec3([0.0, y, 0.0]))?;
            }
            let sweep = tx.create("forge.sweep", None, None)?;
            let prof = tx.create("forge.circle_spline", Some(sweep), None)?;
            tx.set_param(prof, "radius", Value::Float(20.0))?;
            tx.create("forge.helix_spline", Some(sweep), None)?;
            Ok((ex, circle, lathe, loft, sweep))
        })
        .unwrap();
    assert!(is_consumed(d.scene(), circle));
    assert!(
        evaluate(d.scene(), circle).unwrap().is_none(),
        "splines have no mesh"
    );
    // the reference loft's sides are wound against its caps (as exported): valid only
    evaluate(d.scene(), loft)
        .unwrap()
        .unwrap()
        .validate()
        .unwrap();
    for id in [ex, lathe, sweep] {
        let m = evaluate(d.scene(), id).unwrap().unwrap();
        m.validate().unwrap();
        let he = forge_geometry::HalfEdgeMesh::build(&m).unwrap();
        assert!(he.is_closed(), "{:?}", d.scene().node(id).unwrap().type_id);
    }
}
