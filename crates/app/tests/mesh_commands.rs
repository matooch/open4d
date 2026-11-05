//! The mesh commands of docs/specs/MODELING.md §4 headless: one undo step each, selection
//! defaults, the Split object, Center Axis and the argument checks.

use forge_app::App;
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use serde_json::{Value as Json, json};

const COMMANDS: &[&str] = &[
    "mesh.subdivide",
    "mesh.triangulate",
    "mesh.untriangulate",
    "mesh.reverse_normals",
    "mesh.optimize",
    "mesh.weld",
    "mesh.melt",
    "mesh.dissolve",
    "mesh.split",
    "mesh.disconnect",
    "mesh.connect_points",
    "mesh.mirror",
    "mesh.center_axis",
];

fn editable_cube(app: &mut App, segments: [i64; 3]) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let cube = app.selection.active.unwrap();
    for (axis, n) in ["x", "y", "z"].iter().zip(segments) {
        app.run(
            "param.set",
            json!({"node": cube.0, "param": format!("segments_{axis}"), "value": n}),
        )
        .unwrap();
    }
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

fn mesh_of(app: &App, id: NodeId) -> forge_geometry::PolyMesh {
    match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m),
        _ => panic!("no mesh"),
    }
}

fn counts(app: &App, id: NodeId) -> (usize, usize) {
    let m = mesh_of(app, id);
    (m.points.len(), m.polys.len())
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn pos(app: &App, id: NodeId) -> [f64; 3] {
    match app.doc.scene().param(id, "transform.pos") {
        Some(Value::Vec3(v)) => *v,
        _ => panic!("no position"),
    }
}

/// (command, arguments, expected (points, polygons)) on the default editable cube.
fn cases() -> Vec<(&'static str, Json, (usize, usize))> {
    vec![
        ("mesh.subdivide", json!({}), (26, 24)),
        ("mesh.triangulate", json!({}), (8, 12)),
        ("mesh.reverse_normals", json!({"polys": [4]}), (8, 6)),
        ("mesh.optimize", json!({}), (8, 6)),
        ("mesh.weld", json!({"points": [0, 1]}), (7, 6)),
        ("mesh.dissolve", json!({"edges": [[0, 1]]}), (8, 5)),
        ("mesh.split", json!({"polys": [4]}), (8, 6)),
        ("mesh.disconnect", json!({"polys": [4]}), (12, 6)),
        ("mesh.connect_points", json!({"points": [0, 3]}), (8, 7)),
        ("mesh.mirror", json!({}), (8, 6)),
        ("mesh.center_axis", json!({}), (8, 6)),
    ]
}

#[test]
fn every_command_is_one_undo_step() {
    for (command, args, expected) in cases() {
        let mut app = App::new_default();
        let cube = editable_cube(&mut app, [1, 1, 1]);
        let before = steps(&app);
        let source = mesh_of(&app, cube);
        app.run(command, args)
            .unwrap_or_else(|e| panic!("{command}: {e}"));
        assert_eq!(counts(&app, cube), expected, "{command}");
        assert_eq!(steps(&app), before + 1, "{command}");
        app.run("edit.undo", json!({})).unwrap();
        assert_eq!(steps(&app), before, "{command}");
        assert_eq!(
            mesh_of(&app, cube),
            source,
            "{command}: undo restores the mesh"
        );
        if command == "mesh.split" {
            // the new object went with the same step
            assert_eq!(
                app.doc.scene().roots().len(),
                1,
                "split undo removes the copy"
            );
        }
    }
}

#[test]
fn melt_merges_the_selected_polygons() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [2, 1, 1]);
    let out = app.run("mesh.melt", json!({"polys": [6, 7]})).unwrap();
    assert_eq!(counts(&app, cube), (12, 9));
    // the merged polygon is selected: stored selection and result agree
    assert_eq!(out["selected"], json!([6]));
    match app.doc.scene().param(cube, "selection.polys") {
        Some(Value::IndexList(l)) => assert_eq!(l, &vec![6]),
        _ => panic!("selection"),
    }
}

#[test]
fn split_adds_a_named_object_and_selects_it() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    app.run("object.rename", json!({"node": cube.0, "name": "Cube"}))
        .unwrap();
    let out = app.run("mesh.split", json!({"polys": [4]})).unwrap();
    let new = NodeId(out["created"].as_u64().unwrap());
    assert_eq!(app.doc.scene().node(new).unwrap().name, "Cube.1");
    assert_eq!(counts(&app, cube), (8, 6), "the source keeps every polygon");
    assert_eq!(counts(&app, new), (4, 1));
    assert_eq!(app.selection.active, Some(new));
    // a second split takes the next free name
    app.run("mesh.split", json!({"node": cube.0, "polys": [0]}))
        .unwrap();
    let names: Vec<String> = app
        .doc
        .scene()
        .roots()
        .iter()
        .map(|r| app.doc.scene().node(*r).unwrap().name.clone())
        .collect();
    assert!(names.contains(&"Cube.2".to_string()), "{names:?}");
}

#[test]
fn center_axis_keeps_the_world_and_children_in_place() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    // move every point by (50, 0, 0) through a mirror-free route: a child and the mesh edit
    let mut moved = mesh_of(&app, cube);
    for p in &mut moved.points {
        p.x += 50.0;
    }
    app.doc
        .transact("move", |tx| {
            tx.set_param(
                cube,
                "mesh",
                Value::Mesh(std::sync::Arc::new(forge_generators::from_poly_mesh(
                    &moved,
                ))),
            )
        })
        .unwrap();
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    let child = app.selection.active.unwrap();
    app.run(
        "object.reparent",
        json!({"node": child.0, "parent": cube.0}),
    )
    .unwrap();
    let out = app
        .run("mesh.center_axis", json!({"node": cube.0}))
        .unwrap();
    assert_eq!(out["center"], json!([50.0, 0.0, 0.0]));
    assert_eq!(pos(&app, cube), [50.0, 0.0, 0.0]);
    assert_eq!(pos(&app, child), [-50.0, 0.0, 0.0]);
    let m = mesh_of(&app, cube);
    assert_eq!(m.points[0].to_array(), [-100.0, -100.0, -100.0]);
    // world positions are unchanged: position + point == old point
    assert_eq!(pos(&app, cube)[0] + m.points[0].x, -50.0);
}

#[test]
fn errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    let before = steps(&app);
    let failed = |r: Result<Json, CommandError>| matches!(r, Err(CommandError::Failed(_)));
    assert!(failed(app.run("mesh.weld", json!({}))), "no points");
    assert!(
        failed(app.run("mesh.weld", json!({"points": [1]}))),
        "one point"
    );
    assert!(failed(app.run("mesh.weld", json!({"points": [0, 99]}))));
    assert!(failed(app.run("mesh.melt", json!({}))), "no polygons");
    assert!(failed(app.run("mesh.split", json!({}))), "no polygons");
    assert!(failed(app.run("mesh.disconnect", json!({"polys": [99]}))));
    assert!(failed(
        app.run("mesh.connect_points", json!({"points": [0, 1]}))
    ));
    assert!(failed(
        app.run("mesh.connect_points", json!({"edges": [[0, 1]]}))
    ));
    assert!(failed(app.run(
        "mesh.mirror",
        json!({"settings": {"plane_normal": [0.0, 0.0, 0.0]}})
    )));
    assert!(failed(app.run("mesh.subdivide", json!({"polys": [99]}))));
    for bad in [
        json!({"settings": {"nope": 1}}),
        json!({"settings": {"subdivisions": "two"}}),
        json!({"surprise": 1}),
    ] {
        assert!(matches!(
            app.run("mesh.subdivide", bad),
            Err(CommandError::InvalidArgs(_))
        ));
    }
    assert!(matches!(
        app.run("mesh.mirror", json!({"settings": {"plane_point": [1, 2]}})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert_eq!(steps(&app), before);
    assert_eq!(counts(&app, cube), (8, 6));
    // not a polygon object
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    assert!(failed(app.run("mesh.triangulate", json!({}))));
}

#[test]
fn mirror_with_duplicate_and_settings() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    // shift the plane off the cube: the copy is a second cube, nothing is welded
    let out = app
        .run(
            "mesh.mirror",
            json!({"settings": {"plane_point": [200.0, 0.0, 0.0], "duplicate": true}}),
        )
        .unwrap();
    assert_eq!(counts(&app, cube), (16, 12));
    assert_eq!(out["selected"], json!([6, 7, 8, 9, 10, 11]));
    // the default plane is x = 0: points on it (none here) would weld
    let m = mesh_of(&app, cube);
    assert_eq!(m.points[8].x, 500.0);
}

#[test]
fn schemas_and_samples_are_valid() {
    let app = App::new_default();
    for id in COMMANDS {
        let h = app.commands.get(id).unwrap_or_else(|| panic!("{id}"));
        let schema = h.args_schema();
        schema::validate(&schema, &h.sample_args()).unwrap();
        assert!(schema::validate(&schema, &json!({"settings": {"nope": 1}})).is_err());
        assert!(h.undoable(), "{id}");
    }
    let mirror = app.commands.get("mesh.mirror").unwrap().args_schema();
    let normal = &mirror["properties"]["settings"]["properties"]["plane_normal"];
    assert_eq!(normal["default"], json!([1.0, 0.0, 0.0]));
    let subdivide = app.commands.get("mesh.subdivide").unwrap().args_schema();
    assert_eq!(
        subdivide["properties"]["settings"]["properties"]["subdivisions"]["default"],
        1
    );
}
