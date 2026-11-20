//! The `weights.*` commands (docs/specs/RIG.md section 9.3).

use forge_app::App;
use forge_core::{EvalCtx, NodeId, Value};
use forge_rig::WeightMap;
use forge_rig::weights::{find_weight_tag, read_weights};
use serde_json::json;

fn rigged() -> (App, Vec<NodeId>, NodeId, NodeId) {
    let mut app = App::new_default();
    let out = app
        .run(
            "joint.create_chain",
            json!({"points": [[0, 0, 0], [0, 100, 0], [0, 200, 0]], "align": "y"}),
        )
        .unwrap();
    let chain: Vec<NodeId> = out["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| NodeId(v.as_u64().unwrap()))
        .collect();
    let cube = NodeId(
        app.run("object.create", json!({"type": "forge.cube"}))
            .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    let n = forge_generators::evaluate(app.doc.scene(), cube, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .points
        .len();
    let rows: Vec<Vec<f64>> = vec![vec![1.0, 0.0]; n];
    let tag = app
        .doc
        .transact("tag", |tx| {
            let tag = tx.attach_tag("forge.weight_tag", cube, None)?;
            tx.set_param(tag, "joints", Value::LinkList(vec![chain[0], chain[1]]))?;
            tx.set_param(tag, "weights", WeightMap::from_dense(&rows).to_value())?;
            Ok(tag)
        })
        .unwrap();
    (app, chain, cube, tag)
}

fn w(app: &App, tag: NodeId, p: u32, slot: u32) -> f64 {
    read_weights(app.doc.scene(), tag)
        .get(p)
        .iter()
        .find(|(s, _)| *s == slot)
        .map_or(0.0, |(_, w)| *w)
}

#[test]
fn a_paint_stroke_is_one_undo_step() {
    let (mut app, chain, cube, tag) = rigged();
    let out = app
        .run(
            "weights.paint",
            json!({"object": cube.0, "joint": chain[1].0, "center": [0, 0, 0],
                   "radius": 1e6, "strength": 0.25, "falloff": "constant"}),
        )
        .unwrap();
    assert_eq!(out["points"], 8);
    for p in 0..8 {
        assert!((w(&app, tag, p, 1) - 0.25).abs() < 1e-12);
        assert!((w(&app, tag, p, 0) - 0.75).abs() < 1e-12);
    }
    assert!(app.doc.undo());
    assert_eq!(w(&app, tag, 0, 1), 0.0);
    assert_eq!(w(&app, tag, 0, 0), 1.0);
}

#[test]
fn tools_normalise_prune_smooth_and_errors() {
    let (mut app, chain, cube, tag) = rigged();
    app.run(
        "weights.paint",
        json!({"object": cube.0, "joint": chain[1].0, "center": [0, 0, 0],
               "radius": 1e6, "strength": 0.004, "falloff": "constant"}),
    )
    .unwrap();
    app.run(
        "weights.prune",
        json!({"object": cube.0, "threshold": 0.01}),
    )
    .unwrap();
    assert_eq!(w(&app, tag, 0, 1), 0.0);
    app.run("weights.normalise", json!({"object": cube.0}))
        .unwrap();
    assert!((w(&app, tag, 0, 0) - 1.0).abs() < 1e-12);
    app.run("weights.smooth", json!({"object": cube.0, "iterations": 2}))
        .unwrap();
    assert!((w(&app, tag, 3, 0) - 1.0).abs() < 1e-12);
    // a joint outside the tag, a missing tag and a bad radius change nothing
    let before = app.doc.scene().len();
    assert!(
        app.run(
            "weights.paint",
            json!({"object": cube.0, "joint": chain[2].0, "center": [0, 0, 0], "radius": 10.0}),
        )
        .is_err()
    );
    assert!(
        app.run(
            "weights.paint",
            json!({"object": cube.0, "joint": chain[1].0, "center": [0, 0, 0], "radius": 0.0}),
        )
        .is_err()
    );
    assert!(
        app.run("weights.normalise", json!({"object": chain[0].0}))
            .is_err()
    );
    assert_eq!(app.doc.scene().len(), before);
    assert!(find_weight_tag(app.doc.scene(), cube).is_some());
}

#[test]
fn mirror_reports_the_copies() {
    let (mut app, _chain, cube, _tag) = rigged();
    let out = app
        .run("weights.mirror", json!({"object": cube.0}))
        .unwrap();
    assert_eq!(out["copied"], 4);
    assert_eq!(out["unmatched"], 0);
}
