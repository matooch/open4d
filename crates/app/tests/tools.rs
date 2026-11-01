//! mesh.extrude, mesh.extrude_inner and mesh.bevel headless (docs/specs/MODELING.md §0.1, §4).

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

#[test]
fn bevel_edges_by_point_pair_are_one_undo_step() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    let out = app
        .run(
            "mesh.bevel",
            json!({"edges": [[1, 3]], "settings": {"offset": 20.0, "subdivision": 2}}),
        )
        .unwrap();
    // bevel-edge-sub: 14 points, 9 polygons; the 3 strip polygons are selected
    assert_eq!(
        (out["points"].as_u64(), out["polys"].as_u64()),
        (Some(14), Some(9))
    );
    assert_eq!(out["selected"], json!([6, 7, 8]));
    assert_eq!(mesh_of(&app, cube), (14, 9));
    assert_eq!(list(&app, cube, "selection.polys"), vec![6, 7, 8]);
    assert!(list(&app, cube, "selection.edges").is_empty());
    assert_eq!(steps(&app), before + 1);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(mesh_of(&app, cube), (8, 6));
    assert_eq!(steps(&app), before);
    // the same edge by its id in the mesh's edge list
    let id = match app.doc.scene().param(cube, "mesh") {
        Some(Value::Mesh(m)) => {
            forge_generators::to_poly_mesh(m).edge_ids()[&forge_geometry::Edge::new(1, 3)]
        }
        _ => panic!("mesh"),
    };
    app.run(
        "mesh.bevel",
        json!({"edges": [id], "settings": {"offset": 20.0}}),
    )
    .unwrap();
    assert_eq!(mesh_of(&app, cube), (10, 7));
}

#[test]
fn bevel_uses_the_stored_selection_and_edge_ids() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    // stored polygon selection: polygon mode, the default Extrusion lifts by 10
    app.doc
        .transact("sel", |tx| {
            tx.set_param(cube, "selection.polys", Value::IndexList(vec![4]))
        })
        .unwrap();
    app.run("mesh.bevel", json!({"settings": {"offset": 20.0}}))
        .unwrap();
    assert_eq!(mesh_of(&app, cube), (12, 10));
    // an explicit argument replaces the stored sets: edge ids work as well as pairs
    let e = match app.doc.scene().param(cube, "mesh") {
        Some(Value::Mesh(m)) => forge_generators::to_poly_mesh(m)
            .edge_ids()
            .get(&forge_geometry::Edge::new(1, 3))
            .copied()
            .unwrap(),
        _ => panic!("mesh"),
    };
    // after the polygon bevel, points 1 and 3 are used by four polygons: refused, unchanged
    let r = app.run("mesh.bevel", json!({"edges": [e]}));
    assert!(matches!(r, Err(CommandError::Failed(_))), "{r:?}");
    assert_eq!(mesh_of(&app, cube), (12, 10));
}

#[test]
fn bevel_errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    let failed = |r: Result<Json, CommandError>| matches!(r, Err(CommandError::Failed(_)));
    assert!(failed(app.run("mesh.bevel", json!({}))), "nothing selected");
    assert!(
        failed(app.run("mesh.bevel", json!({"edges": [[0, 5]]}))),
        "not an edge"
    );
    assert!(failed(app.run("mesh.bevel", json!({"edges": [999]}))));
    assert!(
        failed(app.run("mesh.bevel", json!({"points": [1]}))),
        "point mode"
    );
    assert!(failed(app.run(
        "mesh.bevel",
        json!({"edges": [[1, 3]], "settings": {"shape": "user"}})
    )));
    assert!(matches!(
        app.run(
            "mesh.bevel",
            json!({"edges": [[1, 3]], "settings": {"shape": "sharp"}})
        ),
        Err(CommandError::InvalidArgs(_))
    ));
    assert!(matches!(
        app.run("mesh.bevel", json!({"edges": [[1.5, 3]]})),
        Err(CommandError::InvalidArgs(_)) | Err(CommandError::Failed(_))
    ));
    assert!(matches!(
        app.run("mesh.extrude", json!({"edges": [[1, 3]]})),
        Err(CommandError::InvalidArgs(_))
    ));
    assert_eq!(steps(&app), before);
    assert_eq!(mesh_of(&app, cube), (8, 6));
}

#[test]
fn bevel_schema_lists_enums_and_components() {
    let app = App::new_default();
    let h = app.commands.get("mesh.bevel").unwrap();
    let schema = h.args_schema();
    schema::validate(&schema, &h.sample_args()).unwrap();
    let mode = &schema["properties"]["settings"]["properties"]["bevel_mode"];
    assert_eq!(mode["enum"], json!(["chamfer", "solid"]));
    assert_eq!(mode["default"], "chamfer");
    assert_eq!(
        schema["properties"]["settings"]["properties"]["offset"]["default"],
        10.0
    );
    schema::validate(&schema, &json!({"edges": [[1, 3], 5], "points": [0]})).unwrap();
    assert!(schema::validate(&schema, &json!({"settings": {"bevel_mode": "sharp"}})).is_err());
}
