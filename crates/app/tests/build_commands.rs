//! mesh.bridge, mesh.close_hole, mesh.stitch and mesh.pen headless (docs/specs/MODELING.md §6.6):
//! one undo step, stored selection, argument errors, schemas.

use std::collections::HashMap;

use forge_app::App;
use forge_command::{CommandError, schema};
use forge_core::{NodeId, Value};
use forge_geometry::Edge;
use forge_geometry::mesh::poly_edges;
use serde_json::{Value as Json, json};

fn editable_cube(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
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

fn invalid(r: Result<Json, CommandError>) -> bool {
    matches!(r, Err(CommandError::InvalidArgs(_)))
}

/// Ids of the edges used by exactly one polygon.
fn border_edges(app: &App, id: NodeId) -> Vec<u32> {
    let m = mesh_of(app, id);
    let mut uses: HashMap<Edge, u32> = HashMap::new();
    for p in &m.polys {
        for e in poly_edges(p) {
            *uses.entry(e).or_default() += 1;
        }
    }
    let ids = m.edge_ids();
    let mut out: Vec<u32> = uses
        .iter()
        .filter(|(_, n)| **n == 1)
        .map(|(e, _)| ids[e])
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn close_hole_by_edge_id_and_point_pair() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    // an opened face: its copy keeps the cube's face but the cube is open there
    app.run("mesh.disconnect", json!({"polys": [4]})).unwrap();
    assert_eq!(counts(&app, cube), (12, 6));
    let before = steps(&app);
    let m = mesh_of(&app, cube);
    // a border edge that belongs to the hole, not to the loose polygon
    let border = border_edges(&app, cube);
    assert_eq!(border.len(), 8);
    let hole = border
        .iter()
        .copied()
        .find(|e| {
            let ed = m.edges()[*e as usize];
            !m.polys[4].contains(&ed.0)
        })
        .expect("a hole edge");
    let out = app.run("mesh.close_hole", json!({"edge": hole})).unwrap();
    assert_eq!(counts(&app, cube), (12, 7));
    assert_eq!(out["new_polys"], json!([6]));
    assert_eq!(steps(&app), before + 1);
    assert_eq!(set(&app, cube, "selection.polys"), vec![6]);
    assert!(set(&app, cube, "selection.edges").is_empty());
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(counts(&app, cube), (12, 6));
    // the same by point pair
    let e = m.edges()[hole as usize];
    app.run("mesh.close_hole", json!({"edge": [e.0, e.1]}))
        .unwrap();
    assert_eq!(counts(&app, cube), (12, 7));
}

#[test]
fn close_hole_errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    // a closed cube has no border
    assert!(failed(app.run("mesh.close_hole", json!({"edge": 0}))));
    assert!(failed(app.run("mesh.close_hole", json!({"edge": 999}))));
    assert!(failed(app.run("mesh.close_hole", json!({"edge": [0, 7]}))));
    assert!(invalid(app.run("mesh.close_hole", json!({}))));
    assert!(invalid(
        app.run("mesh.close_hole", json!({"edge": 0, "bogus": 1}))
    ));
    assert_eq!(steps(&app), before);
    assert_eq!(counts(&app, cube), (8, 6));
}

#[test]
fn stitch_sews_a_disconnected_polygon_back() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let original = mesh_of(&app, cube);
    app.run("mesh.disconnect", json!({"polys": [4]})).unwrap();
    let m = mesh_of(&app, cube);
    // pair the copy's corners with the original points of the polygon
    let pairs: Vec<Json> = original.polys[4]
        .iter()
        .zip(&m.polys[4])
        .map(|(a, b)| json!([a, b]))
        .collect();
    let before = steps(&app);
    let out = app
        .run("mesh.stitch", json!({"points": pairs, "position": "first"}))
        .unwrap();
    assert_eq!(counts(&app, cube), (8, 6));
    assert_eq!(steps(&app), before + 1);
    assert_eq!(out["merged"].as_array().unwrap().len(), 4);
    assert_eq!(set(&app, cube, "selection.points").len(), 4);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(counts(&app, cube), (12, 6));
    // sewing by border edges: the loose polygon's four edges to the hole's four edges
    let border = border_edges(&app, cube);
    let loose: Vec<u32> = border
        .iter()
        .copied()
        .filter(|e| {
            let ed = m.edges()[*e as usize];
            m.polys[4].contains(&ed.0) && m.polys[4].contains(&ed.1)
        })
        .collect();
    let hole: Vec<u32> = border
        .iter()
        .copied()
        .filter(|e| !loose.contains(e))
        .collect();
    assert_eq!((loose.len(), hole.len()), (4, 4));
    // an edge of the loose polygon and the hole edge between the same positions
    let pos = |p: u32| m.points[p as usize];
    let mut pairs = Vec::new();
    for l in &loose {
        let le = m.edges()[*l as usize];
        let h = hole
            .iter()
            .find(|h| {
                let he = m.edges()[**h as usize];
                let same = |a: u32, b: u32| (pos(a) - pos(b)).length() < 1e-9;
                (same(le.0, he.0) && same(le.1, he.1)) || (same(le.0, he.1) && same(le.1, he.0))
            })
            .unwrap();
        pairs.push(json!([l, h]));
    }
    app.run("mesh.stitch", json!({"edges": pairs})).unwrap();
    assert_eq!(counts(&app, cube), (8, 6));
}

#[test]
fn stitch_errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    assert!(failed(app.run("mesh.stitch", json!({"points": [[3, 3]]}))));
    assert!(failed(app.run("mesh.stitch", json!({"points": [[0, 99]]}))));
    assert!(failed(app.run("mesh.stitch", json!({"edges": [[0, 1]]}))));
    for args in [
        json!({}),
        json!({"points": [[0, 1]], "edges": [[0, 1]]}),
        json!({"points": [[0]]}),
        json!({"points": [[0, 1]], "position": "left"}),
        json!({"points": [[0, 1]], "extra": 1}),
    ] {
        assert!(invalid(app.run("mesh.stitch", args.clone())), "{args}");
    }
    assert_eq!(steps(&app), before);
    assert_eq!(counts(&app, cube), (8, 6));
}

#[test]
fn bridge_polygons_and_edges() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    // two opposite faces are disconnected so that they have their own points
    app.run("mesh.disconnect", json!({"polys": [0, 2]}))
        .unwrap();
    let (points, polys) = counts(&app, cube);
    assert_eq!((points, polys), (16, 6));
    let before = steps(&app);
    let out = app
        .run("mesh.bridge", json!({"polys": [0, 2], "segments": 2}))
        .unwrap();
    // two polygons removed, four points per ring in between, 2 x 4 new quads
    assert_eq!(counts(&app, cube), (16 + 4, 6 - 2 + 8));
    assert_eq!(out["new_polys"].as_array().unwrap().len(), 8);
    assert_eq!(out["new_points"].as_array().unwrap().len(), 4);
    assert_eq!(steps(&app), before + 1);
    assert_eq!(set(&app, cube, "selection.polys").len(), 8);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(counts(&app, cube), (16, 6));
    // by edges: the borders of the two holes in the cube's body
    let border = border_edges(&app, cube);
    let m = mesh_of(&app, cube);
    // pick one hole edge near z = -100 and one near z = +100 (the cube is 200 wide)
    let at_z = |z: f64| {
        border
            .iter()
            .copied()
            .find(|e| {
                let ed = m.edges()[*e as usize];
                let used = [ed.0, ed.1];
                used.iter()
                    .all(|p| (m.points[*p as usize].z - z).abs() < 1e-6)
                    && !m.polys[0].contains(&ed.0)
                    && !m.polys[2].contains(&ed.0)
            })
            .unwrap()
    };
    let (a, b) = (at_z(-100.0), at_z(100.0));
    let out = app.run("mesh.bridge", json!({"edges": [a, b]})).unwrap();
    assert_eq!(out["new_polys"].as_array().unwrap().len(), 4);
    assert_eq!(counts(&app, cube), (16, 10));
}

#[test]
fn bridge_errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    assert!(failed(app.run("mesh.bridge", json!({"polys": [0, 0]}))));
    assert!(failed(app.run("mesh.bridge", json!({"polys": [0, 99]}))));
    // neighbouring polygons share points
    assert!(failed(app.run("mesh.bridge", json!({"polys": [0, 1]}))));
    assert!(failed(app.run("mesh.bridge", json!({"edges": [0, 1]}))));
    for args in [
        json!({}),
        json!({"polys": [0, 1], "edges": [0, 1]}),
        json!({"polys": [0]}),
        json!({"polys": [0, 1], "segments": 0}),
        json!({"polys": [0, 1], "segments": 65}),
        json!({"polys": [0, "x"]}),
        json!({"polys": [0, 1], "bogus": 1}),
    ] {
        assert!(invalid(app.run("mesh.bridge", args.clone())), "{args}");
    }
    assert_eq!(steps(&app), before);
    assert_eq!(counts(&app, cube), (8, 6));
}

#[test]
fn pen_adds_points_and_polygons() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    let out = app
        .run(
            "mesh.pen",
            json!({"corners": [{"point": 0}, {"point": 1}, {"pos": [-300.0, 0.0, 0.0]}]}),
        )
        .unwrap();
    assert_eq!(counts(&app, cube), (9, 7));
    assert_eq!(out["new_polys"], json!([6]));
    assert_eq!(out["new_points"], json!([8]));
    assert_eq!(steps(&app), before + 1);
    assert_eq!(set(&app, cube, "selection.polys"), vec![6]);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(counts(&app, cube), (8, 6));
    // one corner adds a point and selects it
    app.run("mesh.pen", json!({"corners": [{"pos": [1.0, 2.0, 3.0]}]}))
        .unwrap();
    assert_eq!(counts(&app, cube), (9, 6));
    assert_eq!(set(&app, cube, "selection.points"), vec![8]);
}

#[test]
fn pen_errors_change_nothing() {
    let mut app = App::new_default();
    let cube = editable_cube(&mut app);
    let before = steps(&app);
    assert!(failed(
        app.run("mesh.pen", json!({"corners": [{"point": 0}]}))
    ));
    assert!(failed(app.run(
        "mesh.pen",
        json!({"corners": [{"point": 0}, {"point": 1}]})
    )));
    assert!(failed(app.run(
        "mesh.pen",
        json!({"corners": [{"point": 0}, {"point": 1}, {"point": 99}]})
    )));
    assert!(failed(app.run(
        "mesh.pen",
        json!({"corners": [{"point": 0}, {"point": 1}, {"point": 0}]})
    )));
    for args in [
        json!({}),
        json!({"corners": [{}]}),
        json!({"corners": [{"point": 0, "pos": [0, 0, 0]}]}),
        json!({"corners": [{"pos": [0, 0]}]}),
        json!({"corners": [{"point": -1}]}),
    ] {
        assert!(invalid(app.run("mesh.pen", args.clone())), "{args}");
    }
    assert_eq!(steps(&app), before);
    assert_eq!(counts(&app, cube), (8, 6));
    // not a polygon object
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    assert!(failed(
        app.run("mesh.pen", json!({"corners": [{"pos": [0, 0, 0]}]}))
    ));
}

#[test]
fn schemas_and_samples() {
    let app = App::new_default();
    for id in ["mesh.bridge", "mesh.close_hole", "mesh.stitch", "mesh.pen"] {
        let h = app.commands.get(id).unwrap_or_else(|| panic!("{id}"));
        let s = h.args_schema();
        schema::validate(&s, &h.sample_args()).unwrap();
        assert!(schema::validate(&s, &json!({"bogus": 1})).is_err(), "{id}");
        assert!(h.undoable());
    }
    let h = app.commands.get("mesh.pen").unwrap();
    assert!(
        schema::validate(
            &h.args_schema(),
            &json!({"corners": [{"point": 1}, {"pos": [0, 1, 2]}]})
        )
        .is_ok()
    );
    assert!(schema::validate(&h.args_schema(), &json!({"corners": [{"x": 1}]})).is_err());
}
