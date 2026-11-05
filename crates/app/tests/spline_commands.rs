//! The editable spline commands of docs/specs/SPLINES.md §5 headless: Make Editable on a
//! spline primitive, one undo step per command, merged gestures, argument checks, and the
//! extrude of an edited spline.

use forge_app::App;
use forge_command::CommandError;
use forge_core::{NodeId, Value};
use forge_generators::editable_spline::data_of;
use serde_json::json;

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn editable_circle(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.circle_spline"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

fn lens(app: &App, id: NodeId) -> Vec<usize> {
    data_of(app.doc.scene(), id)
        .unwrap()
        .segments
        .iter()
        .map(|s| s.len)
        .collect()
}

#[test]
fn make_editable_turns_a_circle_into_a_spline_object_in_one_step() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.circle_spline"}))
        .unwrap();
    let before = steps(&app);
    let out = app.run("mesh.make_editable", json!({})).unwrap();
    assert_eq!(steps(&app), before + 1);
    let id = app.selection.active.unwrap();
    assert_eq!(out["made"], json!([id.0]));
    assert_eq!(app.doc.scene().node(id).unwrap().type_id, "forge.spline");
    assert_eq!(lens(&app, id), [4]);
    app.run("edit.undo", json!({})).unwrap();
    assert!(app.doc.scene().node(id).is_none());
    assert_eq!(steps(&app), before);
}

#[test]
fn spline_commands_are_one_undo_step_each() {
    let mut app = App::new_default();
    let id = editable_circle(&mut app);
    let n = id.0;
    let cases = [
        (
            "spline.edit_points",
            json!({"node": n, "edits": [{"index": 0, "position": [250, 0, 0], "right": [0, 90, 0]}]}),
        ),
        (
            "spline.insert_point",
            json!({"node": n, "index": 1, "position": [150, 150, 0]}),
        ),
        ("spline.delete_points", json!({"node": n, "indices": [1]})),
        ("spline.set_closed", json!({"node": n, "closed": false})),
        ("spline.set_type", json!({"nodes": [n], "type": "akima"})),
    ];
    for (cmd, args) in cases {
        let before = steps(&app);
        app.run(cmd, args).unwrap_or_else(|e| panic!("{cmd}: {e}"));
        assert_eq!(steps(&app), before + 1, "{cmd}");
    }
    // everything undone: the circle again
    for _ in 0..5 {
        app.run("edit.undo", json!({})).unwrap();
    }
    let d = data_of(app.doc.scene(), id).unwrap();
    assert_eq!(d.points.len(), 4);
    assert_eq!(d.points[0].x, 200.0);
    assert!(d.segments[0].closed);
    assert_eq!(
        app.doc.scene().param(id, "type"),
        Some(&Value::Enum("bezier".into()))
    );
}

#[test]
fn a_gesture_merges_into_one_step() {
    let mut app = App::new_default();
    let id = editable_circle(&mut app);
    let before = steps(&app);
    for x in [210.0, 220.0, 230.0] {
        app.run(
            "spline.edit_points",
            json!({"node": id.0, "gesture": 7, "edits": [{"index": 0, "position": [x, 0, 0]}]}),
        )
        .unwrap();
    }
    assert_eq!(steps(&app), before + 1);
    assert_eq!(data_of(app.doc.scene(), id).unwrap().points[0].x, 230.0);
}

#[test]
fn create_makes_a_spline_from_points() {
    let mut app = App::new_default();
    let out = app
        .run(
            "spline.create",
            json!({"points": [[0, 0, 0], [100, 0, 0], [100, 100, 0]], "closed": true, "type": "linear", "name": "Tri"}),
        )
        .unwrap();
    let id = NodeId(out["node"].as_u64().unwrap());
    assert_eq!(app.doc.scene().node(id).unwrap().name, "Tri");
    assert_eq!(lens(&app, id), [3]);
    assert!(data_of(app.doc.scene(), id).unwrap().segments[0].closed);
    assert_eq!(
        app.doc.scene().param(id, "type"),
        Some(&Value::Enum("linear".into()))
    );
}

#[test]
fn refusals_change_nothing() {
    let mut app = App::new_default();
    let id = editable_circle(&mut app);
    let before = steps(&app);
    let bad = [
        ("spline.edit_points", json!({"node": id.0, "edits": []})),
        (
            "spline.edit_points",
            json!({"node": id.0, "edits": [{"index": 9, "position": [0, 0, 0]}]}),
        ),
        (
            "spline.edit_points",
            json!({"node": id.0, "edits": [{"index": 0}]}),
        ),
        (
            "spline.insert_point",
            json!({"node": id.0, "index": 9, "position": [0, 0, 0]}),
        ),
        (
            "spline.delete_points",
            json!({"node": id.0, "indices": [0, 1, 2]}),
        ),
        (
            "spline.set_closed",
            json!({"node": id.0, "segment": 3, "closed": true}),
        ),
        ("spline.create", json!({"points": [[0, 0, 0]]})),
        (
            "param.set",
            json!({"node": id.0, "param": "points", "value": []}),
        ),
    ];
    for (cmd, args) in bad {
        assert!(app.run(cmd, args).is_err(), "{cmd}");
    }
    assert_eq!(steps(&app), before);

    // a spline primitive is not editable yet
    app.run("object.create", json!({"type": "forge.rectangle_spline"}))
        .unwrap();
    let prim = app.selection.active.unwrap();
    let err = app
        .run(
            "spline.edit_points",
            json!({"node": prim.0, "edits": [{"index": 0, "position": [0, 0, 0]}]}),
        )
        .unwrap_err();
    assert!(matches!(err, CommandError::Failed(m) if m.contains("make it editable")));
    assert!(
        app.run(
            "spline.set_type",
            json!({"nodes": [prim.0], "type": "linear"})
        )
        .is_err()
    );
}

#[test]
fn an_edited_spline_feeds_extrude() {
    let mut app = App::new_default();
    let out = app
        .run(
            "spline.create",
            json!({"points": [[0, 0, 0], [100, 0, 0], [100, 100, 0], [0, 100, 0]], "closed": true, "type": "linear"}),
        )
        .unwrap();
    let spline = NodeId(out["node"].as_u64().unwrap());
    let ex = app
        .run("object.create", json!({"type": "forge.extrude"}))
        .unwrap();
    let ex = NodeId(ex["node"].as_u64().unwrap());
    app.run("object.reparent", json!({"node": spline.0, "parent": ex.0}))
        .unwrap();
    let count = |app: &App| {
        forge_generators::evaluate(app.doc.scene(), ex, &Default::default())
            .unwrap()
            .unwrap()
            .points
            .len()
    };
    let n = count(&app);
    assert!(n >= 8, "the square extrudes: {n}");
    app.run(
        "spline.insert_point",
        json!({"node": spline.0, "index": 1, "position": [50, 0, 0]}),
    )
    .unwrap();
    assert!(count(&app) > n, "the inserted point shows up in the mesh");
}

// ---- the spline tools (docs/specs/SPLINES.md section 8) ----

fn editable_rectangle(app: &mut App) -> NodeId {
    app.run("object.create", json!({"type": "forge.rectangle_spline"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    app.selection.active.unwrap()
}

#[test]
fn the_tools_are_one_undo_step_each_and_undo_restores() {
    let mut app = App::new_default();
    let id = editable_rectangle(&mut app);
    let original = data_of(app.doc.scene(), id).unwrap();
    let cases = [
        ("spline.reverse", json!({"node": id.0}), 4),
        (
            "spline.subdivide",
            json!({"node": id.0, "subdivisions": 2}),
            8,
        ),
        // the four corners; the midpoints are straight
        ("spline.chamfer", json!({"node": id.0, "radius": 40}), 12),
        ("spline.outline", json!({"node": id.0, "distance": 5}), 24),
    ];
    for (cmd, args, points) in cases {
        let before = steps(&app);
        app.run(cmd, args).unwrap_or_else(|e| panic!("{cmd}: {e}"));
        assert_eq!(steps(&app), before + 1, "{cmd}");
        assert_eq!(data_of(app.doc.scene(), id).unwrap().points.len(), points);
    }
    // the arcs of the chamfer need the Bezier type
    assert_eq!(
        app.doc.scene().param(id, "type"),
        Some(&Value::Enum("bezier".into()))
    );
    for _ in 0..4 {
        app.run("edit.undo", json!({})).unwrap();
    }
    assert_eq!(data_of(app.doc.scene(), id).unwrap(), original);
    assert_eq!(
        app.doc.scene().param(id, "type"),
        Some(&Value::Enum("linear".into()))
    );
}

#[test]
fn a_flat_chamfer_keeps_the_linear_type() {
    let mut app = App::new_default();
    let id = editable_rectangle(&mut app);
    app.run(
        "spline.chamfer",
        json!({"node": id.0, "radius": 40, "flat": true, "indices": [0]}),
    )
    .unwrap();
    assert_eq!(data_of(app.doc.scene(), id).unwrap().points.len(), 5);
    assert_eq!(
        app.doc.scene().param(id, "type"),
        Some(&Value::Enum("linear".into()))
    );
}

fn two_circles(app: &mut App) -> (NodeId, NodeId) {
    let a = editable_circle(app);
    let b = editable_circle(app);
    app.run(
        "param.set",
        json!({"node": b.0, "param": "transform.pos", "value": [150, 0, 0]}),
    )
    .unwrap();
    (a, b)
}

#[test]
fn a_boolean_replaces_its_inputs_in_one_step() {
    let mut app = App::new_default();
    let (a, b) = two_circles(&mut app);
    let before = steps(&app);
    let out = app
        .run("spline.boolean", json!({"op": "union", "a": a.0, "b": b.0}))
        .unwrap();
    assert_eq!(steps(&app), before + 1);
    let id = NodeId(out["node"].as_u64().unwrap());
    assert!(app.doc.scene().node(a).is_none() && app.doc.scene().node(b).is_none());
    assert_eq!(app.selection.active, Some(id));
    // two crossing points, four control points left of each circle's arc
    assert_eq!(lens(&app, id).len(), 1);
    assert_eq!(
        app.doc.scene().param(id, "intermediate"),
        Some(&Value::Enum("adaptive".into()))
    );
    app.run("edit.undo", json!({})).unwrap();
    assert!(app.doc.scene().node(id).is_none());
    assert_eq!(lens(&app, a), [4]);
    assert_eq!(lens(&app, b), [4]);
}

#[test]
fn keep_inputs_and_the_loop_counts_of_the_operations() {
    let mut app = App::new_default();
    let (a, b) = two_circles(&mut app);
    let loops = |app: &mut App, op: &str| {
        let out = app
            .run(
                "spline.boolean",
                json!({"op": op, "a": a.0, "b": b.0, "keep_inputs": true}),
            )
            .unwrap();
        let id = NodeId(out["node"].as_u64().unwrap());
        assert!(app.doc.scene().node(a).is_some() && app.doc.scene().node(b).is_some());
        lens(app, id).len()
    };
    assert_eq!(loops(&mut app, "union"), 1);
    assert_eq!(loops(&mut app, "and"), 1);
    assert_eq!(loops(&mut app, "subtract"), 1);
    assert_eq!(loops(&mut app, "or"), 2);
    assert_eq!(loops(&mut app, "intersect"), 3);
}

#[test]
fn tool_refusals_change_nothing() {
    let mut app = App::new_default();
    let (a, b) = two_circles(&mut app);
    let before = steps(&app);
    let bad = [
        ("spline.chamfer", json!({"node": a.0, "radius": 0})),
        ("spline.chamfer", json!({"node": a.0, "indices": [9]})),
        ("spline.outline", json!({"node": a.0, "distance": 0})),
        ("spline.subdivide", json!({"node": a.0, "subdivisions": 0})),
        ("spline.boolean", json!({"op": "union", "a": a.0, "b": a.0})),
        (
            "spline.boolean",
            json!({"op": "union", "a": a.0, "b": 9999}),
        ),
        ("spline.boolean", json!({"op": "xor", "a": a.0, "b": b.0})),
        ("spline.reverse", json!({"node": 9999})),
    ];
    for (cmd, args) in bad {
        assert!(app.run(cmd, args).is_err(), "{cmd}");
    }
    // a spline primitive must be made editable first
    app.run("object.create", json!({"type": "forge.circle_spline"}))
        .unwrap();
    let prim = app.selection.active.unwrap();
    let before = before + 1;
    let err = app
        .run(
            "spline.boolean",
            json!({"op": "union", "a": a.0, "b": prim.0}),
        )
        .unwrap_err();
    assert!(matches!(err, CommandError::Failed(m) if m.contains("editable")));
    assert_eq!(steps(&app), before);
    // an open spline cannot be a boolean operand
    app.run("spline.set_closed", json!({"node": b.0, "closed": false}))
        .unwrap();
    assert!(
        app.run("spline.boolean", json!({"op": "union", "a": a.0, "b": b.0}))
            .is_err()
    );
    assert_eq!(steps(&app), before + 1);
}
