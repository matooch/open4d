//! docs/specs/EDITABLE.md §1: polygon object, Make Editable, Current State to Object.

use std::path::Path;
use std::sync::Arc;

mod common;

use common::{resolved_params, typed_params};
use forge_core::{Document, NodeId, Registry, Value, file};
use forge_generators::{
    EditableError, POLYGON, Primitives, current_state_to_object, evaluate, make_editable,
};
use forge_modifiers::Deformers;
use forge_parity_harness::{Mesh, Tolerance, compare_geometric, compare_meshes, discover_cases};

/// Same list as `parity.rs`: the cut faces are triangulated with an underived method.
const CUT_GEOMETRIC: &[&str] = &["cone/slice", "torus/slice", "capsule/slice"];

const FEATURES: &[&str] = &[
    "plane", "cube", "sphere", "cylinder", "cone", "disc", "torus", "tube", "capsule", "pyramid",
];

fn registry() -> Arc<Registry> {
    let mut r = Registry::with_builtins();
    r.install(&Primitives).unwrap();
    r.install(&Deformers).unwrap();
    Arc::new(r)
}

fn doc() -> Document {
    Document::new(registry())
}

fn as_mesh(m: &forge_geometry::PolyMesh) -> Mesh {
    Mesh {
        positions: m.points.iter().map(|p| p.to_array()).collect(),
        faces: m.polys.clone(),
    }
}

fn check_cut_geometric(key: &str, expected: &Mesh, ours: &Mesh, tol: &Tolerance) {
    let prefix = Mesh {
        positions: expected.positions.clone(),
        faces: Vec::new(),
    };
    let same_points = compare_meshes(
        &prefix,
        &Mesh {
            faces: Vec::new(),
            ..ours.clone()
        },
        tol,
    );
    assert!(same_points.passed(), "{key}: {:?}", same_points.failures);
    assert_eq!(expected.faces.len(), ours.faces.len(), "{key}: face count");
    let first_diff = expected
        .faces
        .iter()
        .zip(&ours.faces)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.faces.len());
    for m in [expected, ours] {
        assert!(
            m.faces[first_diff..].iter().all(|f| f.len() == 3),
            "{key}: faces differ before the cut (face {first_diff})"
        );
    }
    let r = compare_geometric(expected, ours, tol);
    assert!(r.passed(), "{key}: {:?}", r.failures);
}

#[test]
fn every_primitive_golden_case_survives_make_editable() {
    let reg = registry();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut checked = 0;
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let manifest = case.manifest().unwrap();
        let type_id = format!("forge.{}", case.feature);
        let values = typed_params(&reg, &type_id, &resolved_params(&case, &manifest));
        let mut d = Document::new(reg.clone());
        let src = d
            .transact("create", |tx| {
                let id = tx.create(&type_id, None, None)?;
                for (k, v) in &values {
                    tx.set_param(id, k, v.clone())?;
                }
                Ok(id)
            })
            .unwrap();
        let before = evaluate(d.scene(), src);
        let new = d.transact("make editable", |tx| {
            make_editable(tx, src).map_err(|e| match e {
                EditableError::Edit(e) => e,
                other => panic!("{key}: {other}"),
            })
        });
        let Ok(before) = before else {
            // an observed option we do not build: Make Editable reports it, edits nothing
            assert!(new.is_err() || d.scene().contains(src), "{key}");
            continue;
        };
        let new = new.unwrap();
        let before = before.expect("primitive has a mesh");
        assert_eq!(d.scene().node(new).unwrap().type_id, POLYGON, "{key}");
        let after = evaluate(d.scene(), new).unwrap().unwrap();
        assert_eq!(before, after, "{key}: mesh changed by Make Editable");

        let expected = case.expected_mesh().unwrap().unwrap();
        let ours = as_mesh(&after);
        if CUT_GEOMETRIC.contains(&key.as_str()) {
            check_cut_geometric(&key, &expected, &ours, &manifest.tolerance);
        } else {
            let report = compare_meshes(&expected, &ours, &manifest.tolerance);
            assert!(
                report.passed(),
                "{key}: {:?} (max error {:?})",
                report.failures,
                report.max_position_error
            );
        }
        checked += 1;
    }
    println!("checked {checked} cases");
    assert!(checked >= 75, "only {checked} cases checked");
}

/// A cube (4/8/4 segments, tall) with a bend child, a smoothing tag, a transform and a name.
fn bent_cube(d: &mut Document) -> (NodeId, NodeId) {
    d.transact("build", |tx| {
        let cube = tx.create("forge.cube", None, None)?;
        tx.rename(cube, "My Cube")?;
        tx.set_param(cube, "size_y", Value::Float(400.0))?;
        tx.set_param(cube, "segments_x", Value::Int(4))?;
        tx.set_param(cube, "segments_y", Value::Int(8))?;
        tx.set_param(cube, "segments_z", Value::Int(4))?;
        tx.set_param(cube, "transform.pos", Value::Vec3([10.0, 20.0, 30.0]))?;
        tx.attach_tag(forge_core::smoothing::TAG, cube, None)?;
        let bend = tx.create("forge.bend", Some(cube), None)?;
        tx.set_param(bend, "strength", Value::Float(90f64.to_radians()))?;
        Ok((cube, bend))
    })
    .unwrap()
}

#[test]
fn make_editable_keeps_the_deformer_as_a_child_and_still_applies_it() {
    let mut d = doc();
    // a second root before and after, so the tree position is checked
    let (first, last) = d
        .transact("roots", |tx| {
            let a = tx.create("forge.null", None, None)?;
            Ok((a, tx.create("forge.null", None, None)?))
        })
        .unwrap();
    let (cube, bend) = {
        let (c, b) = bent_cube(&mut d);
        // move between the two nulls
        d.transact("move", |tx| tx.move_node(c, None, Some(1)))
            .unwrap();
        (c, b)
    };
    let deformed = evaluate(d.scene(), cube).unwrap().unwrap();
    let undeformed = {
        let node = d.scene().node(cube).unwrap();
        forge_generators::build(&node.type_id, &node.params).unwrap()
    };
    assert_ne!(deformed, undeformed);

    let new = d
        .transact("make editable", |tx| Ok(make_editable(tx, cube).unwrap()))
        .unwrap();
    let s = d.scene();
    assert!(!s.contains(cube));
    assert_eq!(s.roots(), [first, new, last], "same tree position");
    let n = s.node(new).unwrap();
    assert_eq!(n.type_id, POLYGON);
    assert_eq!(n.name, "My Cube");
    assert_eq!(
        s.param(new, "transform.pos"),
        Some(&Value::Vec3([10.0, 20.0, 30.0]))
    );
    assert_eq!(n.tags.len(), 1);
    assert_eq!(
        s.node(n.tags[0]).unwrap().type_id,
        forge_core::smoothing::TAG
    );
    assert_eq!(n.children, [bend], "deformer child kept");
    assert_eq!(s.node(bend).unwrap().parent, Some(new));
    // stored mesh is the undeformed one; evaluate applies the kept bend to it
    let Some(Value::Mesh(stored)) = s.param(new, "mesh") else {
        panic!("mesh param")
    };
    assert_eq!(forge_generators::to_poly_mesh(stored), undeformed);
    assert_eq!(evaluate(s, new).unwrap().unwrap(), deformed);
}

#[test]
fn current_state_to_object_bakes_the_deformer_and_keeps_the_source() {
    let mut d = doc();
    let (cube, bend) = bent_cube(&mut d);
    let tail = d
        .transact("tail", |tx| tx.create("forge.null", None, None))
        .unwrap();
    let deformed = evaluate(d.scene(), cube).unwrap().unwrap();
    let before = file::save(d.scene());

    let new = d
        .transact("csto", |tx| Ok(current_state_to_object(tx, cube).unwrap()))
        .unwrap();
    let s = d.scene();
    assert_eq!(s.roots(), [cube, new, tail], "directly after the source");
    assert_eq!(s.node(cube).unwrap().children, [bend], "source untouched");
    let n = s.node(new).unwrap();
    assert_eq!((n.type_id.as_str(), n.name.as_str()), (POLYGON, "My Cube"));
    assert!(n.children.is_empty());
    assert_eq!(n.tags.len(), 1);
    assert_eq!(
        s.param(new, "transform.pos"),
        Some(&Value::Vec3([10.0, 20.0, 30.0]))
    );
    assert_eq!(
        evaluate(s, new).unwrap().unwrap(),
        deformed,
        "bend baked in"
    );

    d.undo();
    assert_eq!(file::save(d.scene()), before);
}

#[test]
fn one_undo_restores_the_original() {
    let mut d = doc();
    let (cube, _) = bent_cube(&mut d);
    let before = file::save(d.scene());
    let steps = d.history().undo_labels().len();
    d.transact("Make Editable", |tx| Ok(make_editable(tx, cube).unwrap()))
        .unwrap();
    assert_eq!(d.history().undo_labels().len(), steps + 1);
    assert_ne!(file::save(d.scene()), before);
    assert!(d.undo());
    assert_eq!(file::save(d.scene()), before);
    assert!(d.redo());
    assert!(!d.scene().contains(cube));
}

#[test]
fn consumed_children_are_removed_and_other_children_kept() {
    let mut d = doc();
    let (sub, input, bend, extra) = d
        .transact("build", |tx| {
            let sub = tx.create("forge.subdivision", None, None)?;
            let input = tx.create("forge.cube", Some(sub), None)?;
            let bend = tx.create("forge.bend", Some(sub), None)?;
            let extra = tx.create("forge.null", Some(sub), None)?;
            Ok((sub, input, bend, extra))
        })
        .unwrap();
    let expected = evaluate(d.scene(), sub).unwrap().unwrap();
    let new = d
        .transact("make editable", |tx| Ok(make_editable(tx, sub).unwrap()))
        .unwrap();
    let s = d.scene();
    assert!(!s.contains(sub) && !s.contains(input), "input consumed");
    assert_eq!(s.node(new).unwrap().children, [bend, extra]);
    // the stored mesh is the subdivided cube before the bend
    let stored = forge_generators::to_poly_mesh(match s.param(new, "mesh") {
        Some(Value::Mesh(m)) => m,
        _ => panic!("mesh param"),
    });
    assert_eq!(stored.points.len(), expected.points.len());
    assert_eq!(stored.polys, expected.polys);
}

#[test]
fn nodes_without_a_mesh_are_reported_and_edit_nothing() {
    let mut d = doc();
    let (null, circle, bend) = d
        .transact("build", |tx| {
            Ok((
                tx.create("forge.null", None, None)?,
                tx.create("forge.circle_spline", None, None)?,
                tx.create("forge.bend", None, None)?,
            ))
        })
        .unwrap();
    let before = file::save(d.scene());
    for id in [null, circle, bend] {
        let r = d.transact("x", |tx| Ok(make_editable(tx, id)));
        assert!(matches!(r, Ok(Err(EditableError::NoMesh(_)))), "{id}");
        let r = d.transact("x", |tx| Ok(current_state_to_object(tx, id)));
        assert!(matches!(r, Ok(Err(EditableError::NoMesh(_)))), "{id}");
    }
    assert_eq!(file::save(d.scene()), before);
    assert!(d.history().undo_labels().len() == 1, "nothing recorded");
}

#[test]
fn a_polygon_object_is_already_editable() {
    let mut d = doc();
    let cube = d
        .transact("c", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    let poly = d
        .transact("m", |tx| Ok(make_editable(tx, cube).unwrap()))
        .unwrap();
    let before = file::save(d.scene());
    let again = d
        .transact("m", |tx| Ok(make_editable(tx, poly).unwrap()))
        .unwrap();
    assert_eq!(again, poly);
    assert_eq!(file::save(d.scene()), before);
    assert_eq!(d.history().undo_labels().len(), 2, "no new step");
}

#[test]
fn polygon_object_saves_and_loads_with_its_mesh() {
    let mut d = doc();
    let (cube, _) = bent_cube(&mut d);
    let new = d
        .transact("m", |tx| Ok(make_editable(tx, cube).unwrap()))
        .unwrap();
    let text = file::save(d.scene());
    let (scene, report) = file::load(registry(), &text).unwrap();
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    assert_eq!(file::save(&scene), text);
    assert_eq!(
        evaluate(&scene, new).unwrap(),
        evaluate(d.scene(), new).unwrap()
    );
}

#[test]
fn polygon_object_is_hidden_from_the_create_menu_flag() {
    let reg = registry();
    assert!(!reg.get(POLYGON).unwrap().in_create_menu);
    assert!(reg.get("forge.cube").unwrap().in_create_menu);
}

#[test]
fn a_parent_null_passes_its_deformer_to_a_polygon_object() {
    let mut d = doc();
    let (null, cube, bend) = d
        .transact("build", |tx| {
            let null = tx.create("forge.null", None, None)?;
            let cube = tx.create("forge.cube", Some(null), None)?;
            let bend = tx.create("forge.bend", Some(null), None)?;
            tx.set_param(bend, "strength", Value::Float(1.0))?;
            Ok((null, cube, bend))
        })
        .unwrap();
    let _ = (null, bend);
    let deformed = evaluate(d.scene(), cube).unwrap().unwrap();
    let poly = d
        .transact("m", |tx| Ok(make_editable(tx, cube).unwrap()))
        .unwrap();
    assert_eq!(evaluate(d.scene(), poly).unwrap().unwrap(), deformed);
}

#[test]
fn current_state_to_object_with_a_sibling_deformer_is_not_deformed_twice() {
    let mut d = doc();
    let (null, cube, bend) = d
        .transact("build", |tx| {
            let null = tx.create("forge.null", None, None)?;
            tx.set_param(null, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]))?;
            let cube = tx.create("forge.cube", Some(null), None)?;
            tx.set_param(cube, "transform.pos", Value::Vec3([50.0, 0.0, 0.0]))?;
            tx.set_param(cube, "size_y", Value::Float(400.0))?;
            tx.set_param(cube, "segments_x", Value::Int(4))?;
            tx.set_param(cube, "segments_y", Value::Int(8))?;
            tx.set_param(cube, "segments_z", Value::Int(4))?;
            let bend = tx.create("forge.bend", Some(null), None)?;
            tx.set_param(bend, "strength", Value::Float(90f64.to_radians()))?;
            Ok((null, cube, bend))
        })
        .unwrap();
    let tail = d
        .transact("tail", |tx| tx.create("forge.null", None, None))
        .unwrap();
    let world = |d: &Document, id: NodeId| {
        let m = d.scene().world_matrix(id).unwrap();
        let mesh = evaluate(d.scene(), id).unwrap().unwrap();
        mesh.points
            .iter()
            .map(|p| m.transform_point3(*p))
            .collect::<Vec<_>>()
    };
    let expected = world(&d, cube);
    let before = file::save(d.scene());

    let new = d
        .transact("csto", |tx| Ok(current_state_to_object(tx, cube).unwrap()))
        .unwrap();
    let s = d.scene();
    assert_eq!(s.roots(), [null, new, tail], "directly after the parent");
    assert_eq!(s.node(null).unwrap().children, [cube, bend], "untouched");
    assert_eq!(s.node(new).unwrap().name, "Cube");
    assert_eq!(
        s.param(new, "transform.pos"),
        Some(&Value::Vec3([0.0, 100.0, 0.0])),
        "the parent's transform"
    );
    let got = world(&d, new);
    assert_eq!(got.len(), expected.len());
    for (a, b) in got.iter().zip(&expected) {
        assert!(a.distance(*b) < 1e-9, "{a:?} vs {b:?}");
    }
    // evaluating the new object applies no deformer: stored points are what it yields
    let Some(Value::Mesh(stored)) = s.param(new, "mesh") else {
        panic!("mesh param")
    };
    assert_eq!(
        forge_generators::to_poly_mesh(stored),
        evaluate(s, new).unwrap().unwrap()
    );
    d.undo();
    assert_eq!(file::save(d.scene()), before);
}

#[test]
fn make_editable_redirects_links_to_the_new_object() {
    let mut d = doc();
    let (cube, inst) = d
        .transact("build", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            let inst = tx.create("forge.instance", None, None)?;
            tx.set_param(inst, "reference", Value::Link(Some(cube)))?;
            Ok((cube, inst))
        })
        .unwrap();
    let before = evaluate(d.scene(), inst).unwrap().unwrap();
    assert!(!before.points.is_empty());
    let new = d
        .transact("m", |tx| Ok(make_editable(tx, cube).unwrap()))
        .unwrap();
    assert_eq!(
        d.scene().param(inst, "reference"),
        Some(&Value::Link(Some(new)))
    );
    assert_eq!(evaluate(d.scene(), inst).unwrap().unwrap(), before);
    d.undo();
    assert_eq!(
        d.scene().param(inst, "reference"),
        Some(&Value::Link(Some(cube)))
    );
}
