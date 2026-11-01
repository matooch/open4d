//! mesh.extrude and mesh.extrude_inner headless (docs/specs/MODELING.md §0.1, §4).

use forge_app::App;
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use serde_json::{Value as Json, json};

fn editable_cube(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

fn mesh_of(app: &App, id: NodeId) -> (usize, usize) {
    match app.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => (m.points.len(), m.polys.len()),
        _ => panic!("no mesh"),
    }
}

fn list(app: &App, id: NodeId, param: &str) -> Vec<u32> {
    match app.doc.scene().param(id, param) {
        Some(Value::IndexList(l)) => l.clone(),
        _ => panic!("no list"),
    }
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

#[test]
fn extrude_is_one_undo_step_with_selection() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    let out = app
        .run(
            "mesh.extrude",
            json!({"polys": [4], "settings": {"offset": 50.0}}),
        )
        .unwrap();
    assert_eq!(out["points"], 12);
    assert_eq!(out["polys"], 10);
    assert_eq!(out["selected"], json!([4]));
    assert_eq!(mesh_of(&app, cube), (12, 10));
    assert_eq!(list(&app, cube, "selection.polys"), vec![4]);
    assert_eq!(steps(&app), before + 1);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(mesh_of(&app, cube), (8, 6));
    assert!(list(&app, cube, "selection.polys").is_empty());
    app.run("edit.redo", json!({})).unwrap();
    assert_eq!(mesh_of(&app, cube), (12, 10));
}

#[test]
fn selection_defaults_to_the_stored_polygons_and_node_can_be_named() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    app.doc
        .transact("sel", |tx| {
            tx.set_param(cube, "selection.polys", Value::IndexList(vec![4]))
        })
        .unwrap();
    // another object is active, the cube is named
    app.run("object.create", json!({"type": "forge.null"}))
        .unwrap();
    app.run(
        "mesh.extrude_inner",
        json!({"node": cube.0, "settings": {"offset": 20.0}}),
    )
    .unwrap();
    assert_eq!(mesh_of(&app, cube), (12, 10));
    // the stored selection is now the cap, stale point and edge sets are cleared
    assert_eq!(list(&app, cube, "selection.polys"), vec![4]);
    assert!(list(&app, cube, "selection.points").is_empty());
    // a second run on the same selection works on the new mesh
    app.run("mesh.extrude", json!({"node": cube.0})).unwrap();
    assert_eq!(mesh_of(&app, cube), (16, 14));
}

#[test]
fn errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    let failed = |r: Result<Json, CommandError>| matches!(r, Err(CommandError::Failed(_)));
    // no polygons selected
    assert!(failed(app.run("mesh.extrude", json!({}))));
    // out of range polygon
    assert!(failed(app.run("mesh.extrude", json!({"polys": [99]}))));
    // an option without parity data
    assert!(failed(app.run(
        "mesh.extrude",
        json!({"polys": [4], "settings": {"create_caps": true}})
    )));
    // not a polygon object
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    assert!(failed(app.run("mesh.extrude", json!({"polys": [4]}))));
    let steps_after_create = steps(&app);
    // bad arguments
    for bad in [
        json!({"settings": {"nope": 1}}),
        json!({"settings": {"offset": "far"}}),
        json!({"surprise": 1}),
        json!({"polys": ["a"]}),
    ] {
        assert!(matches!(
            app.run("mesh.extrude", bad),
            Err(CommandError::InvalidArgs(_))
        ),);
    }
    assert_eq!(steps(&app), steps_after_create);
    assert!(before < steps_after_create);
    assert_eq!(mesh_of(&app, cube), (8, 6));
}

#[test]
fn schemas_and_samples_are_valid() {
    let app = App::new_default();
    for id in ["mesh.extrude", "mesh.extrude_inner"] {
        let h = app.commands.get(id).unwrap();
        let schema = h.args_schema();
        schema::validate(&schema, &h.sample_args()).unwrap();
        // the schema lists the observed settings with their defaults
        let offset = &schema["properties"]["settings"]["properties"]["offset"];
        assert_eq!(offset["default"], 5.0);
        // an unknown setting is refused by the schema too
        assert!(schema::validate(&schema, &json!({"settings": {"nope": 1}})).is_err());
    }
}
