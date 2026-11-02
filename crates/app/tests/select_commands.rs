//! The selection commands of docs/specs/EDITABLE.md §2.8 headless: one undo step each, no
//! step when nothing changes, argument and mode checks.

use forge_app::App;
use forge_app::select::{EditMode, selection_of};
use forge_core::NodeId;
use serde_json::json;

fn editable_cube(app: &mut App, seg: i64) -> NodeId {
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    let cube = app.selection.active.unwrap();
    for axis in ["x", "y", "z"] {
        let args = json!({"node": cube.0, "param": format!("segments_{axis}"), "value": seg});
        app.run("param.set", args).unwrap();
    }
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn sel(app: &App, id: NodeId, mode: EditMode) -> Vec<u32> {
    selection_of(app.doc.scene(), id, mode).iter().collect()
}

fn mode(app: &mut App, m: &str) {
    app.run("select.mode", json!({ "mode": m })).unwrap();
}

#[test]
fn convert_keeps_the_source_switches_mode_and_is_one_step() {
    let mut app = App::new_default();
    let id = editable_cube(&mut app, 2);
    mode(&mut app, "polys");
    app.doc
        .transact("Start", |tx| {
            tx.set_param(
                id,
                "selection.polys",
                forge_core::Value::IndexList(vec![16, 17, 18, 19]),
            )
        })
        .unwrap();
    let before = steps(&app);
    app.run("select.convert", json!({"to": "points"})).unwrap();
    assert_eq!(steps(&app), before + 1);
    assert_eq!(app.mode, EditMode::Points);
    assert_eq!(sel(&app, id, EditMode::Points).len(), 9);
    assert_eq!(sel(&app, id, EditMode::Polys), [16, 17, 18, 19]);
    // Back to polygons strictly gives exactly the same four; nothing changes, no step.
    mode(&mut app, "points");
    let before = steps(&app);
    app.run("select.convert", json!({"to": "polys", "mode": "strict"}))
        .unwrap();
    assert_eq!(steps(&app), before);
    mode(&mut app, "points");
    // Tolerant takes every polygon touching a selected point.
    app.run("select.convert", json!({"to": "polys", "mode": "tolerant"}))
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    assert_eq!(sel(&app, id, EditMode::Polys).len(), 4 + 4 * 2);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(sel(&app, id, EditMode::Polys), [16, 17, 18, 19]);
}

#[test]
fn connected_loop_and_ring_run_headless() {
    let mut app = App::new_default();
    let id = editable_cube(&mut app, 2);
    mode(&mut app, "polys");
    app.doc
        .transact("Start", |tx| {
            tx.set_param(id, "selection.polys", forge_core::Value::IndexList(vec![0]))
        })
        .unwrap();
    // The cube is one piece.
    app.run("select.connected", json!({})).unwrap();
    assert_eq!(sel(&app, id, EditMode::Polys).len(), 24);
    // Strips on a closed cube: a polygon loop goes round 8 faces (2/2/2 sides).
    app.run("select.none", json!({})).unwrap();
    app.doc
        .transact("Start", |tx| {
            tx.set_param(id, "selection.polys", forge_core::Value::IndexList(vec![0]))
        })
        .unwrap();
    let before = steps(&app);
    app.run("select.loop", json!({})).unwrap();
    assert_eq!(steps(&app), before + 1);
    let a = sel(&app, id, EditMode::Polys);
    assert_eq!(a.len(), 8);
    app.run("edit.undo", json!({})).unwrap();
    app.run("select.ring", json!({})).unwrap();
    let b = sel(&app, id, EditMode::Polys);
    assert_eq!(b.len(), 8);
    assert_ne!(a, b);
    // Edge mode: the loop through one edge.
    mode(&mut app, "edges");
    app.run("select.none", json!({})).unwrap();
    app.doc
        .transact("Start", |tx| {
            tx.set_param(id, "selection.edges", forge_core::Value::IndexList(vec![0]))
        })
        .unwrap();
    app.run("select.ring", json!({})).unwrap();
    assert!(sel(&app, id, EditMode::Edges).len() > 1);
}

#[test]
fn argument_and_mode_checks() {
    let mut app = App::new_default();
    let id = editable_cube(&mut app, 1);
    // Model mode: nothing happens, no error, no step.
    mode(&mut app, "model");
    let before = steps(&app);
    for cmd in ["select.connected", "select.loop", "select.ring"] {
        app.run(cmd, json!({})).unwrap();
    }
    app.run("select.convert", json!({"to": "points"})).unwrap();
    assert_eq!(steps(&app), before);
    assert!(app.run("select.convert", json!({"to": "model"})).is_err());
    assert!(app.run("select.convert", json!({})).is_err());
    assert!(
        app.run("select.convert", json!({"to": "edges", "x": 1}))
            .is_err()
    );
    // Points mode: loop and ring need edges or polygons.
    mode(&mut app, "points");
    assert!(app.run("select.loop", json!({})).is_err());
    assert!(app.run("select.ring", json!({})).is_err());
    // A non-polygon active object: component commands fail with the usual message.
    app.selection.active = None;
    assert!(app.run("select.connected", json!({})).is_err());
    let _ = id;
}
