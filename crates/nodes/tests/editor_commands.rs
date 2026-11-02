//! docs/specs/NODES.md section 6, "Editor additions" (brief 109): node positions, `graph.move`,
//! multi-node remove, link moves and gesture merging.

use forge_app::App;
use forge_core::NodeId;
use forge_nodes::{Graph, GraphNodeId, TAG, load_graph};
use serde_json::{Value as Json, json};

fn tag_app() -> (App, NodeId) {
    let mut app = App::new_default();
    let cube = app
        .run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.selection.click(NodeId(cube["node"].as_u64().unwrap()));
    let tag = app.run("object.create", json!({"type": TAG})).unwrap();
    (app, NodeId(tag["node"].as_u64().unwrap()))
}

fn add(app: &mut App, tag: NodeId, ty: &str, extra: Json) -> u64 {
    let mut args = json!({"tag": tag.0, "type": ty});
    for (k, v) in extra.as_object().unwrap() {
        args[k] = v.clone();
    }
    app.run("graph.add_node", args).unwrap()["node"]
        .as_u64()
        .unwrap()
}

fn graph(app: &App, tag: NodeId) -> Graph {
    load_graph(app.doc.scene(), tag).unwrap()
}

fn depth(app: &App) -> usize {
    app.doc.undo_depth()
}

#[test]
fn positions_round_trip_and_unpositioned_graphs_keep_their_text() {
    let (mut app, tag) = tag_app();
    let a = add(&mut app, tag, "math.add", json!({}));
    let text_before = graph(&app, tag).to_json();
    assert!(!text_before.contains("pos"), "{text_before}");
    let b = add(&mut app, tag, "math.mul", json!({"pos": [12.5, -40.0]}));
    let g = graph(&app, tag);
    assert_eq!(g.node(GraphNodeId(b)).unwrap().pos, [12.5, -40.0]);
    assert_eq!(g.node(GraphNodeId(a)).unwrap().pos, [0.0, 0.0]);
    let again = Graph::from_json(&g.to_json()).unwrap();
    assert_eq!(again, g);
    let got = app.run("graph.get", json!({"tag": tag.0})).unwrap();
    let nodes = got["nodes"].as_array().unwrap();
    assert_eq!(nodes[1]["pos"], json!([12.5, -40.0]));
    assert_eq!(nodes[0]["pos"], json!([0.0, 0.0]));
}

#[test]
fn move_sets_several_positions_in_one_step_and_undoes() {
    let (mut app, tag) = tag_app();
    let a = add(&mut app, tag, "math.add", json!({}));
    let b = add(&mut app, tag, "math.mul", json!({}));
    let d0 = depth(&app);
    let out = app
        .run(
            "graph.move",
            json!({"tag": tag.0, "moves": [{"node": a, "x": 1.0, "y": 2.0}, {"node": b, "x": 3.0, "y": 4.0}]}),
        )
        .unwrap();
    assert_eq!(out["moved"], 2);
    assert_eq!(depth(&app), d0 + 1);
    let g = graph(&app, tag);
    assert_eq!(g.node(GraphNodeId(a)).unwrap().pos, [1.0, 2.0]);
    assert_eq!(g.node(GraphNodeId(b)).unwrap().pos, [3.0, 4.0]);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(
        graph(&app, tag).node(GraphNodeId(a)).unwrap().pos,
        [0.0, 0.0]
    );
}

#[test]
fn moves_of_one_gesture_merge_into_one_step_and_a_new_gesture_does_not() {
    let (mut app, tag) = tag_app();
    let a = add(&mut app, tag, "math.add", json!({}));
    let d0 = depth(&app);
    for (i, g) in [(1, 7), (2, 7), (3, 7), (4, 8)] {
        app.run(
            "graph.move",
            json!({"tag": tag.0, "moves": [{"node": a, "x": f64::from(i), "y": 0.0}], "gesture": g}),
        )
        .unwrap();
    }
    assert_eq!(depth(&app), d0 + 2);
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(
        graph(&app, tag).node(GraphNodeId(a)).unwrap().pos,
        [3.0, 0.0]
    );
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(
        graph(&app, tag).node(GraphNodeId(a)).unwrap().pos,
        [0.0, 0.0]
    );
}

#[test]
fn a_bad_move_changes_nothing() {
    let (mut app, tag) = tag_app();
    let a = add(&mut app, tag, "math.add", json!({}));
    let d0 = depth(&app);
    let before = graph(&app, tag);
    let e = app.run(
        "graph.move",
        json!({"tag": tag.0, "moves": [{"node": a, "x": 5.0, "y": 5.0}, {"node": 99, "x": 1.0, "y": 1.0}]}),
    );
    assert!(e.is_err());
    assert_eq!(graph(&app, tag), before);
    assert_eq!(depth(&app), d0);
    // JSON cannot carry a non-finite number, and the model refuses one too.
    assert!(
        app.run(
            "graph.move",
            json!({"tag": tag.0, "moves": [{"node": a, "x": null, "y": 1.0}]})
        )
        .is_err()
    );
    let mut g = before;
    assert!(g.set_position(GraphNodeId(a), [f64::NAN, 0.0]).is_err());
}

#[test]
fn remove_takes_a_list_in_one_step_and_needs_a_node() {
    let (mut app, tag) = tag_app();
    let a = add(&mut app, tag, "math.add", json!({}));
    let b = add(&mut app, tag, "math.mul", json!({}));
    let c = add(&mut app, tag, "math.sub", json!({}));
    app.run(
        "graph.connect",
        json!({"tag": tag.0, "from_node": a, "from_port": "out", "to_node": b, "to_port": "a"}),
    )
    .unwrap();
    app.run(
        "graph.connect",
        json!({"tag": tag.0, "from_node": b, "from_port": "out", "to_node": c, "to_port": "a"}),
    )
    .unwrap();
    let d0 = depth(&app);
    let out = app
        .run("graph.remove", json!({"tag": tag.0, "nodes": [a, b]}))
        .unwrap();
    assert_eq!(out["removed_links"], 2);
    assert_eq!(depth(&app), d0 + 1);
    assert_eq!(graph(&app, tag).len(), 1);
    assert!(app.run("graph.remove", json!({"tag": tag.0})).is_err());
    assert!(
        app.run("graph.remove", json!({"tag": tag.0, "nodes": [c, 99]}))
            .is_err()
    );
    assert_eq!(graph(&app, tag).len(), 1, "all or none");
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(graph(&app, tag).len(), 3);
    assert_eq!(graph(&app, tag).links().len(), 2);
}

#[test]
fn connect_can_move_a_link_between_inputs_in_one_step() {
    let (mut app, tag) = tag_app();
    let src = add(&mut app, tag, "math.add", json!({}));
    let dst = add(&mut app, tag, "math.mul", json!({}));
    app.run(
        "graph.connect",
        json!({"tag": tag.0, "from_node": src, "from_port": "out", "to_node": dst, "to_port": "a"}),
    )
    .unwrap();
    let d0 = depth(&app);
    app.run(
        "graph.connect",
        json!({"tag": tag.0, "from_node": src, "from_port": "out", "to_node": dst, "to_port": "b",
               "release_node": dst, "release_port": "a"}),
    )
    .unwrap();
    assert_eq!(depth(&app), d0 + 1);
    let g = graph(&app, tag);
    assert_eq!(g.links().len(), 1);
    assert_eq!(g.links()[0].to.port, "b");
    app.run("edit.undo", json!({})).unwrap();
    assert_eq!(graph(&app, tag).links()[0].to.port, "a");

    // A refused connection keeps the released link.
    let text = add(&mut app, tag, "convert.to_string", json!({}));
    let d1 = depth(&app);
    let bad = app.run(
        "graph.connect",
        json!({"tag": tag.0, "from_node": text, "from_port": "out", "to_node": dst, "to_port": "b",
               "release_node": dst, "release_port": "a"}),
    );
    assert!(bad.is_err());
    assert_eq!(depth(&app), d1);
    assert_eq!(graph(&app, tag).links()[0].to.port, "a");
    // Only one of the two release fields is an error.
    assert!(
        app.run(
            "graph.connect",
            json!({"tag": tag.0, "from_node": src, "from_port": "out", "to_node": dst, "to_port": "b",
                   "release_node": dst}),
        )
        .is_err()
    );
}

#[test]
fn set_value_gestures_merge_per_port() {
    let (mut app, tag) = tag_app();
    let a = add(&mut app, tag, "math.add", json!({}));
    let d0 = depth(&app);
    for v in [1.0, 2.0, 3.0] {
        app.run(
            "graph.set_value",
            json!({"tag": tag.0, "node": a, "port": "a", "value": v, "gesture": 5}),
        )
        .unwrap();
    }
    assert_eq!(depth(&app), d0 + 1);
    app.run(
        "graph.set_value",
        json!({"tag": tag.0, "node": a, "port": "b", "value": 9.0, "gesture": 5}),
    )
    .unwrap();
    assert_eq!(depth(&app), d0 + 2, "another port is another step");
    app.run("edit.undo", json!({})).unwrap();
    app.run("edit.undo", json!({})).unwrap();
    assert!(
        graph(&app, tag)
            .node(GraphNodeId(a))
            .unwrap()
            .values
            .is_empty()
    );
}
