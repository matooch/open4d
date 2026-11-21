//! Selection tags, vertex maps and restriction tags through the command registry
//! (docs/specs/EDITABLE.md section 7, brief 027). The four selection and vertex map cases of
//! O54 (`tests/golden/tags-2/`) are rebuilt from `mesh.obj` and `selection.json` / `case.toml`
//! and compared exactly (level A: indices; weights to the six decimals the observation gives).

use std::path::Path;

use forge_app::App;
use forge_app::select::EditMode;
use forge_core::tags;
use forge_core::{NodeId, Value};
use forge_generators::from_poly_mesh;
use forge_geometry::PolyMesh;
use forge_parity_harness::{GoldenCase, discover_cases, parse_obj};
use glam::DVec3;
use serde_json::json;

/// The O54 cases of this brief; the other five (look-at-camera, protection, display-tag,
/// compositing, user-data) belong to briefs 106 and 110.
const CASES: &[&str] = &[
    "point-selection",
    "polygon-selection",
    "edge-selection",
    "vertex-map-tag",
];
/// Cases whose options we do not run (none).
const UNSUPPORTED: &[(&str, &str)] = &[];
/// Cases that run but differ (none).
const KNOWN_MISMATCH: &[(&str, &str)] = &[];

fn cases() -> Vec<GoldenCase> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    discover_cases(&root)
        .unwrap()
        .into_iter()
        .filter(|c| c.feature == "tags-2" && CASES.contains(&c.name.as_str()))
        .collect()
}

fn golden_mesh(case: &GoldenCase) -> PolyMesh {
    let src = std::fs::read_to_string(case.dir.join("mesh.obj")).unwrap();
    let g = parse_obj(&src).unwrap();
    PolyMesh {
        points: g.positions.iter().map(|p| DVec3::from_array(*p)).collect(),
        polys: g.faces,
    }
}

fn object_with(app: &mut App, mesh: &PolyMesh) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    let id = app.selection.active.unwrap();
    app.doc
        .transact("Golden mesh", |tx| {
            tx.set_param(id, "mesh", Value::Mesh(from_poly_mesh(mesh).into()))
        })
        .unwrap();
    id
}

fn run_selection_case(case: &GoldenCase) -> Result<(), String> {
    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(case.dir.join("selection.json")).unwrap())
            .unwrap();
    let want: Vec<u32> = expected["indices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_u64().unwrap() as u32)
        .collect();
    let (kind, param, mode, tag_type) = match expected["kind"].as_str().unwrap() {
        "point" => (
            "points",
            "selection.points",
            EditMode::Points,
            tags::POINT_SELECTION,
        ),
        "edge" => (
            "edges",
            "selection.edges",
            EditMode::Edges,
            tags::EDGE_SELECTION,
        ),
        "polygon" => (
            "polys",
            "selection.polys",
            EditMode::Polys,
            tags::POLYGON_SELECTION,
        ),
        k => return Err(format!("unknown kind {k}")),
    };
    let mesh = golden_mesh(case);
    let mut app = App::new_default();
    let id = object_with(&mut app, &mesh);
    app.doc
        .transact("Select", |tx| {
            tx.set_param(id, param, Value::IndexList(want.clone()))
        })
        .unwrap();
    app.mode = mode;
    let r = app
        .run("select.store", json!({}))
        .map_err(|e| e.to_string())?;
    let tag = NodeId(r["tag"].as_u64().unwrap());
    let scene = app.doc.scene();
    let n = scene.node(tag).unwrap();
    if n.type_id != tag_type || n.host != Some(id) {
        return Err(format!("wrong tag {} on {:?}", n.type_id, n.host));
    }
    let stored = tags::selection_indices(scene, tag).unwrap().1.to_vec();
    if stored != want {
        return Err(format!("stored {stored:?}, want {want:?}"));
    }
    // clear the object's set, restore from the tag and read it back
    app.doc
        .transact("Clear", |tx| {
            tx.set_param(id, param, Value::IndexList(Vec::new()))
        })
        .unwrap();
    app.mode = EditMode::Model;
    let r = app
        .run("select.restore", json!({"tag": tag.0}))
        .map_err(|e| e.to_string())?;
    if r["kind"] != kind || r["count"] != want.len() {
        return Err(format!("restore reported {r}"));
    }
    if app.mode != mode {
        return Err("restore did not switch to the kind's mode".into());
    }
    match app.doc.scene().param(id, param) {
        Some(Value::IndexList(l)) if *l == want => Ok(()),
        other => Err(format!("restored {other:?}, want {want:?}")),
    }
}

fn run_vertex_map_case(case: &GoldenCase) -> Result<(), String> {
    let manifest = case.manifest()?;
    let vm = manifest.params["vertex_map"].as_table().unwrap();
    let count = vm["point_count"].as_integer().unwrap() as usize;
    let first: Vec<f64> = vm["first_weights"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_float().unwrap())
        .collect();
    let mesh = golden_mesh(case);
    if mesh.points.len() != count {
        return Err(format!(
            "mesh has {} points, case says {count}",
            mesh.points.len()
        ));
    }
    let mut app = App::new_default();
    let id = object_with(&mut app, &mesh);
    // weight = point index / point count, as the observation scripted it
    let weights: Vec<f64> = (0..count).map(|i| i as f64 / count as f64).collect();
    let r = app
        .run(
            "vertexmap.create",
            json!({"weights": weights, "name": "Vertex Map"}),
        )
        .map_err(|e| e.to_string())?;
    let tag = NodeId(r["tag"].as_u64().unwrap());
    let stored = tags::vertex_map_weights(app.doc.scene(), tag).unwrap();
    if stored.len() != count {
        return Err(format!("{} weights stored", stored.len()));
    }
    for (i, want) in first.iter().enumerate() {
        // the observation prints six decimals
        if (stored[i] - want).abs() > 5.1e-7 {
            return Err(format!("weight {i}: {} vs {want}", stored[i]));
        }
    }
    if app.doc.scene().node(tag).unwrap().host != Some(id) {
        return Err("the tag is not on the object".into());
    }
    Ok(())
}

#[test]
fn o54_selection_and_vertex_map_cases() {
    let cases = cases();
    assert_eq!(
        cases.len(),
        CASES.len(),
        "every case of this brief is on disk"
    );
    let mut failures = Vec::new();
    for c in &cases {
        let key = c.name.as_str();
        let result = if c.name == "vertex-map-tag" {
            run_vertex_map_case(c)
        } else {
            run_selection_case(c)
        };
        let pinned = UNSUPPORTED
            .iter()
            .chain(KNOWN_MISMATCH)
            .any(|(n, _)| *n == key);
        match (result, pinned) {
            (Ok(()), false) | (Err(_), true) => {}
            (Ok(()), true) => failures.push(format!("{key}: pinned but now matches")),
            (Err(e), false) => failures.push(format!("{key}: {e}")),
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

fn editable_cube(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

#[test]
fn store_and_restore_are_one_undo_step_each_and_restore_ops_combine() {
    let mut app = App::new_default();
    let id = editable_cube(&mut app);
    app.doc
        .transact("Select", |tx| {
            tx.set_param(id, "selection.points", Value::IndexList(vec![1, 2, 3]))
        })
        .unwrap();
    // Model mode and no kind: refused, nothing changes
    assert!(app.run("select.store", json!({})).is_err());
    let depth = app.doc.undo_depth();
    let r = app
        .run("select.store", json!({"kind": "points", "name": "Top"}))
        .unwrap();
    assert_eq!(app.doc.undo_depth(), depth + 1);
    let tag = r["tag"].as_u64().unwrap();
    assert_eq!(app.doc.scene().node(NodeId(tag)).unwrap().name, "Top");
    app.doc
        .transact("Select", |tx| {
            tx.set_param(id, "selection.points", Value::IndexList(vec![3, 9]))
        })
        .unwrap();
    app.run("select.restore", json!({"tag": tag, "op": "add"}))
        .unwrap();
    assert_eq!(
        app.doc.scene().param(id, "selection.points"),
        Some(&Value::IndexList(vec![1, 2, 3, 9]))
    );
    app.run("select.restore", json!({"tag": tag, "op": "subtract"}))
        .unwrap();
    assert_eq!(
        app.doc.scene().param(id, "selection.points"),
        Some(&Value::IndexList(vec![9]))
    );
    // restoring what is already selected writes no step
    app.run("select.restore", json!({"tag": tag})).unwrap();
    let depth = app.doc.undo_depth();
    app.run("select.restore", json!({"tag": tag})).unwrap();
    assert_eq!(app.doc.undo_depth(), depth);
    // a tag that is not a selection
    assert!(app.run("select.restore", json!({"tag": id.0})).is_err());
}

#[test]
fn vertex_maps_can_be_set_by_points_and_clamp() {
    let mut app = App::new_default();
    let id = editable_cube(&mut app);
    let n = match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => m.points.len(),
        _ => unreachable!(),
    };
    let r = app.run("vertexmap.create", json!({"value": 0.25})).unwrap();
    let tag = r["tag"].as_u64().unwrap();
    let w = |app: &App| {
        tags::vertex_map_weights(app.doc.scene(), NodeId(tag))
            .unwrap()
            .to_vec()
    };
    assert_eq!(w(&app), vec![0.25; n]);
    app.run(
        "vertexmap.set",
        json!({"tag": tag, "points": [0, 2], "value": 5.0}),
    )
    .unwrap();
    assert_eq!(&w(&app)[..3], &[1.0, 0.25, 1.0]);
    assert!(app.run("vertexmap.set", json!({"tag": tag})).is_err());
    assert!(
        app.run("vertexmap.create", json!({"weights": vec![0.0; n + 1]}))
            .is_err()
    );
}

#[test]
fn restriction_command_limits_a_deformer_to_a_vertex_map() {
    let mut app = App::new_default();
    let id = editable_cube(&mut app);
    app.run("object.create", json!({"type": "forge.bend"}))
        .unwrap();
    let bend = app.selection.active.unwrap();
    app.run("object.reparent", json!({"node": bend.0, "parent": id.0}))
        .unwrap();
    app.run(
        "vertexmap.create",
        json!({"object": id.0, "name": "Mask", "value": 0.0}),
    )
    .unwrap();
    let r = app
        .run(
            "restriction.set",
            json!({"deformer": bend.0, "selection": "Mask"}),
        )
        .unwrap();
    let tag = NodeId(r["tag"].as_u64().unwrap());
    // a second call updates the same tag
    let r2 = app
        .run(
            "restriction.set",
            json!({"deformer": bend.0, "selection": "Mask", "invert": true}),
        )
        .unwrap();
    assert_eq!(r2["tag"].as_u64().unwrap(), tag.0);
    let w = tags::restriction_weights(app.doc.scene(), bend, id, 3).unwrap();
    assert_eq!(w, vec![1.0; 3], "inverted all-zero map");
    assert!(
        app.run(
            "restriction.set",
            json!({"deformer": id.0, "selection": "Mask"})
        )
        .is_err()
    );
}

#[test]
fn a_topology_change_clears_stored_selection_tags_and_vertex_maps() {
    let mut app = App::new_default();
    let id = editable_cube(&mut app);
    app.run("vertexmap.create", json!({})).unwrap();
    app.run("select.store", json!({"kind": "polys"})).unwrap();
    let tags_of = |app: &App| app.doc.scene().node(id).unwrap().tags.len();
    // the editable cube already carries its smoothing tag
    let (with, without) = (tags_of(&app), tags_of(&app) - 2);
    assert!(without <= 1);
    // a move keeps the topology
    app.doc
        .transact("Move", |tx| {
            let Some(Value::Mesh(m)) = tx.scene().param(id, "mesh").cloned() else {
                unreachable!()
            };
            let mut m = (*m).clone();
            m.points[0][0] += 1.0;
            forge_generators::clear_stale_tags(tx, id, &m)?;
            tx.set_param(id, "mesh", Value::Mesh(m.into()))
        })
        .unwrap();
    assert_eq!(tags_of(&app), with);
    // a loop cut changes it
    app.doc
        .transact("Cut", |tx| {
            let Some(Value::Mesh(m)) = tx.scene().param(id, "mesh").cloned() else {
                unreachable!()
            };
            let mut m = (*m).clone();
            m.polys.pop();
            let removed = forge_generators::clear_stale_tags(tx, id, &m)?;
            assert_eq!(removed, 2);
            tx.set_param(id, "mesh", Value::Mesh(m.into()))
        })
        .unwrap();
    assert_eq!(tags_of(&app), without);
    app.doc.undo();
    assert_eq!(tags_of(&app), with, "undo brings the tags back");
}
