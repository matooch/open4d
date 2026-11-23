//! Native behavioral tests for O50's unobserved tag cases and clone-cache playback.

use forge_app::App;
use forge_core::{EvalCtx, NodeId, Value, file};
use serde_json::json;

fn create(app: &mut App, ty: &str) -> NodeId {
    NodeId(
        app.run("object.create", json!({"type": ty})).unwrap()["node"]
            .as_u64()
            .unwrap(),
    )
}

fn set(app: &mut App, node: NodeId, param: &str, value: Value) {
    app.doc
        .transact("Set", |tx| tx.set_param(node, param, value))
        .unwrap();
}

fn cloner(app: &mut App, effector: &str) -> (NodeId, NodeId) {
    let cloner = create(app, "forge.cloner");
    let cube = create(app, "forge.cube");
    app.run(
        "object.reparent",
        json!({"node": cube.0, "parent": cloner.0}),
    )
    .unwrap();
    let effector = create(app, effector);
    set(app, cloner, "mode", Value::Enum("linear".into()));
    set(app, cloner, "linear_count", Value::Int(5));
    set(
        app,
        cloner,
        "transform_position",
        Value::Vec3([0.0, 0.0, 50.0]),
    );
    set(app, cloner, "effectors", Value::LinkList(vec![effector]));
    (cloner, effector)
}

fn y_positions(app: &App, cloner: NodeId) -> Vec<f64> {
    forge_generators::clone_data(app.doc.scene(), cloner, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .matrices
        .iter()
        .map(|m| m.w_axis.y)
        .collect()
}

#[test]
fn selection_and_weights_restrict_plain_without_changing_clone_indices() {
    let mut app = App::new_default();
    let (cloner, plain) = cloner(&mut app, "forge.effector.plain");
    set(&mut app, plain, "p", Value::Vec3([0.0, 100.0, 0.0]));
    let selection = app
        .doc
        .transact("Clone tags", |tx| {
            let selection = tx.attach_tag("forge.clone_selection", cloner, None)?;
            tx.rename(selection, "Picked")?;
            tx.set_param(selection, "indices", Value::IndexList(vec![1, 3, 99]))?;
            let weight = tx.attach_tag("forge.clone_weight", cloner, None)?;
            tx.set_param(
                weight,
                "weights",
                Value::FloatList(vec![0.0, 0.25, 0.5, 0.75, 1.0]),
            )?;
            Ok(selection)
        })
        .unwrap();
    set(&mut app, plain, "selection", Value::String("Picked".into()));
    let baseline = y_positions(&app, cloner);
    assert_eq!(baseline, [0.0, 100.0, 0.0, 100.0, 0.0]);
    set(&mut app, plain, "use_clone_weight", Value::Bool(true));
    assert_eq!(y_positions(&app, cloner), [0.0, 25.0, 0.0, 75.0, 0.0]);
    set(&mut app, selection, "indices", Value::IndexList(vec![]));
    assert_eq!(y_positions(&app, cloner), [0.0; 5]);
    set(
        &mut app,
        plain,
        "selection",
        Value::String("missing".into()),
    );
    assert_eq!(y_positions(&app, cloner), [0.0, 25.0, 50.0, 75.0, 100.0]);
    let data = forge_generators::clone_data(app.doc.scene(), cloner, &EvalCtx::default())
        .unwrap()
        .unwrap();
    assert_eq!(data.index, [0, 1, 2, 3, 4]);
    assert_eq!(data.weights, [0.0, 0.25, 0.5, 0.75, 1.0]);
}

#[test]
fn baked_frames_round_trip_and_replace_live_clone_data_and_mesh() {
    let mut app = App::new_default();
    let (cloner, time) = cloner(&mut app, "forge.effector.time");
    let fps = f64::from(app.doc.scene().settings().fps);
    let at = |scene: &forge_core::Scene, frame| {
        forge_generators::clone_data(scene, cloner, &EvalCtx::at_frame(frame, fps))
            .unwrap()
            .unwrap()
    };
    let f1 = at(app.doc.scene(), 1);
    let f2 = at(app.doc.scene(), 2);
    assert_ne!(f1.matrices, f2.matrices);
    let mesh = app.eval_mesh(cloner, 2).unwrap();
    let baked = app
        .run(
            "cloner.bake",
            json!({"node": cloner.0, "start": 1, "end": 2}),
        )
        .unwrap();
    let tag = NodeId(baked["tag"].as_u64().unwrap());
    assert!(app.doc.scene().contains(tag));
    app.run("edit.undo", json!({})).unwrap();
    assert!(!app.doc.scene().contains(tag));
    app.run("edit.redo", json!({})).unwrap();
    set(&mut app, time, "enabled", Value::Bool(false));
    assert_eq!(app.eval_mesh(cloner, 2).unwrap(), mesh);
    let draw_f1 = app.scheduled_draw(1).outputs[&cloner]
        .instances
        .clone()
        .unwrap();
    let draw_f2 = app.scheduled_draw(2).outputs[&cloner]
        .instances
        .clone()
        .unwrap();
    assert_eq!(draw_f1.clones, f1);
    assert_eq!(draw_f2.clones, f2);
    assert_ne!(at(app.doc.scene(), 3).matrices, f2.matrices);
    let saved = file::save_with(app.doc.scene(), &file::SaveOptions::default());
    assert!(
        saved
            .windows(b"%%FORGE-CHUNKS".len())
            .any(|w| w == b"%%FORGE-CHUNKS")
    );
    let (loaded, _) = file::load_bytes(app.doc.scene().registry().clone(), &saved).unwrap();
    assert_eq!(at(&loaded, 1), f1);
    assert_eq!(at(&loaded, 2), f2);
    app.run(
        "cloner.bake",
        json!({"node": cloner.0, "start": 1, "end": 2}),
    )
    .unwrap();
    assert_ne!(at(app.doc.scene(), 2), f2, "rebake samples live data");
}
