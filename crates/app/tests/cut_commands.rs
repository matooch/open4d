//! mesh.loop_cut, mesh.knife and mesh.slide headless (docs/specs/MODELING.md §5.4): one undo
//! step each, stored selection, argument errors, schemas.

use forge_app::App;
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use serde_json::{Value as Json, json};

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

fn set(app: &App, id: NodeId, key: &str) -> Vec<u32> {
    match app.doc.scene().param(id, key) {
        Some(Value::IndexList(l)) => l.clone(),
        _ => panic!("selection"),
    }
}

fn failed(r: Result<Json, CommandError>) -> bool {
    matches!(r, Err(CommandError::Failed(_)))
}

#[test]
fn loop_cut_by_edge_id_and_point_pair() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    let before = steps(&app);
    let m = mesh_of(&app, cube);
    let e = m.edge_ids()[&forge_geometry::Edge::new(0, 2)];
    let out = app
        .run("mesh.loop_cut", json!({"edge": e, "offsets": [0.5]}))
        .unwrap();
    assert_eq!(counts(&app, cube), (12, 10));
    assert_eq!(steps(&app), before + 1);
    assert_eq!(out["cut_edges"].as_array().unwrap().len(), 4);
    // the new edges are the stored edge selection; points and polygons are cleared
    assert_eq!(set(&app, cube, "selection.edges").len(), 4);
    assert!(set(&app, cube, "selection.polys").is_empty());
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(counts(&app, cube), (8, 6));
    assert!(set(&app, cube, "selection.edges").is_empty());
    // point pair and count
    app.run("mesh.loop_cut", json!({"edge": [0, 2], "count": 3}))
        .unwrap();
    assert_eq!(counts(&app, cube), (20, 18));
}

#[test]
fn loop_cut_errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    let before = steps(&app);
    for args in [
        json!({"edge": 0, "offsets": [0.0]}),
        json!({"edge": 0, "offsets": [0.5, 0.5]}),
        json!({"edge": 99, "offsets": [0.5]}),
        json!({"edge": [0, 7], "offsets": [0.5]}),
    ] {
        assert!(failed(app.run("mesh.loop_cut", args)));
    }
    for args in [
        json!({}),
        json!({"edge": 0, "offsets": [0.5], "count": 2}),
        json!({"edge": 0, "count": 65}),
        json!({"edge": 0, "bogus": 1}),
        json!({"edge": "x"}),
    ] {
        assert!(matches!(
            app.run("mesh.loop_cut", args),
            Err(CommandError::InvalidArgs(_))
        ));
    }
    assert_eq!(steps(&app), before);
    assert_eq!(counts(&app, cube), (8, 6));
    // not a polygon object
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    assert!(failed(app.run("mesh.loop_cut", json!({"edge": 0}))));
}

#[test]
fn knife_modes() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    let before = steps(&app);
    let out = app
        .run(
            "mesh.knife",
            json!({"mode": "plane", "point": [0, 0, 0], "normal": [1, 0, 0]}),
        )
        .unwrap();
    assert_eq!(counts(&app, cube), (12, 10));
    assert_eq!(out["cut_edges"].as_array().unwrap().len(), 4);
    assert_eq!(steps(&app), before + 1);
    app.run("edit.undo", json!({})).unwrap();
    // line mode, parallel view, all faces
    app.run(
        "mesh.knife",
        json!({"mode": "line", "from": [0, -500, 0], "to": [0, 500, 0],
               "direction": [0, 0, 1], "visible_only": false}),
    )
    .unwrap();
    assert_eq!(counts(&app, cube), (12, 10));
    app.run("edit.undo", json!({})).unwrap();
    // loop mode is a single loop cut
    app.run(
        "mesh.knife",
        json!({"mode": "loop", "edge": [0, 2], "offset": 0.25}),
    )
    .unwrap();
    assert_eq!(counts(&app, cube), (12, 10));
    app.run("edit.undo", json!({})).unwrap();
    // restricted to one polygon
    app.run(
        "mesh.knife",
        json!({"mode": "plane", "point": [0, 0, 0], "normal": [1, 0, 0], "polys": [4]}),
    )
    .unwrap();
    assert_eq!(counts(&app, cube), (10, 7));
}

#[test]
fn knife_errors() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [1, 1, 1]);
    let before = steps(&app);
    // misses the mesh
    assert!(failed(app.run(
        "mesh.knife",
        json!({"mode": "plane", "point": [900, 0, 0], "normal": [1, 0, 0]})
    )));
    for args in [
        json!({}),
        json!({"mode": "spin"}),
        json!({"mode": "plane", "point": [0, 0, 0]}),
        json!({"mode": "plane", "point": [0, 0], "normal": [1, 0, 0]}),
        json!({"mode": "line", "from": [0, 0, 0], "to": [0, 1, 0]}),
        json!({"mode": "line", "from": [0, 0, 0], "to": [0, 1, 0],
               "eye": [0, 0, -9], "direction": [0, 0, 1]}),
        json!({"mode": "loop"}),
        json!({"mode": "plane", "extra": 1}),
    ] {
        assert!(
            matches!(
                app.run("mesh.knife", args.clone()),
                Err(CommandError::InvalidArgs(_))
            ),
            "{args}"
        );
    }
    assert_eq!(steps(&app), before);
    assert_eq!(counts(&app, cube), (8, 6));
}

#[test]
fn slide_moves_points_and_keeps_the_selection() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app, [2, 1, 1]);
    let m = mesh_of(&app, cube);
    // the middle top edge of the 2/1/1 cube slides along the top polygons' rails
    let ids = m.edge_ids();
    let (top, mid): (Vec<_>, Vec<_>) = m
        .edges()
        .into_iter()
        .partition(|e| m.points[e.0 as usize].y < 99.0 || m.points[e.1 as usize].y < 99.0);
    assert!(!top.is_empty() && !mid.is_empty());
    let middle = mid
        .iter()
        .find(|e| m.points[e.0 as usize].x.abs() < 1e-9 && m.points[e.1 as usize].x.abs() < 1e-9)
        .copied()
        .expect("middle top edge");
    let id = ids[&middle];
    let before = steps(&app);
    app.run(
        "mesh.slide",
        json!({"edges": [id], "settings": {"amount": 0.5}}),
    )
    .unwrap();
    assert_eq!(steps(&app), before + 1);
    assert_eq!(counts(&app, cube), (m.points.len(), m.polys.len()));
    let moved = mesh_of(&app, cube);
    let a = moved.points[middle.0 as usize].x;
    assert!(a.abs() > 1.0, "the edge left x = 0: {a}");
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(mesh_of(&app, cube), m);
    // the edge selection is kept after a slide (indices stay valid)
    app.run("select.mode", json!({"mode": "edges"})).ok();
    app.doc
        .transact("sel", |tx| {
            tx.set_param(cube, "selection.edges", Value::IndexList(vec![id]))
        })
        .unwrap();
    app.run("mesh.slide", json!({"settings": {"amount": -0.25}}))
        .unwrap();
    assert_eq!(set(&app, cube, "selection.edges"), vec![id]);
    // errors
    assert!(failed(app.run(
        "mesh.slide",
        json!({"edges": [id], "settings": {"amount": 0}})
    )));
    assert!(failed(app.run(
        "mesh.slide",
        json!({"polys": [0], "settings": {"amount": 0.5}})
    )));
}

#[test]
fn schemas_and_samples() {
    let app = App::new_default();
    for id in ["mesh.loop_cut", "mesh.knife", "mesh.slide"] {
        let h = app.commands.get(id).unwrap_or_else(|| panic!("{id}"));
        let s = h.args_schema();
        schema::validate(&s, &h.sample_args()).unwrap();
        assert!(schema::validate(&s, &json!({"bogus": 1})).is_err(), "{id}");
        assert!(h.undoable());
    }
    let h = app.commands.get("mesh.loop_cut").unwrap();
    assert!(schema::validate(&h.args_schema(), &json!({"edge": [1, 2], "count": 2})).is_ok());
    let h = app.commands.get("mesh.knife").unwrap();
    assert!(schema::validate(&h.args_schema(), &json!({"mode": "spin"})).is_err());
}
