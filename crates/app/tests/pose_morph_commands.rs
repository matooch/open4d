//! The `pose_morph.*`, `tag.attach` and `weights.auto_bind` commands (docs/specs/RIG.md
//! sections 4 and 15.4). Pose Morph is own design (D20), parity-pending (O37).

use forge_app::App;
use forge_core::{EvalCtx, Interp, Key, NodeId, Value};
use forge_rig::pose_morph;
use serde_json::json;

fn editable_cylinder(app: &mut App) -> NodeId {
    let c = app
        .run("object.create", json!({"type": "forge.cylinder"}))
        .unwrap()["node"]
        .as_u64()
        .unwrap();
    let made = app
        .run("mesh.make_editable", json!({"nodes": [c]}))
        .unwrap();
    NodeId(made["made"][0].as_u64().unwrap())
}

fn top(app: &App, obj: NodeId) -> f64 {
    forge_generators::evaluate(app.doc.scene(), obj, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .points
        .iter()
        .map(|p| p.y)
        .fold(f64::MIN, f64::max)
}

#[test]
fn add_edit_clear_and_undo_through_commands() {
    let mut app = App::new_default();
    let obj = editable_cylinder(&mut app);
    let base_top = top(&app, obj);
    let pts: Vec<[f64; 3]> = match app.doc.scene().param(obj, "mesh") {
        Some(Value::Mesh(m)) => m
            .points
            .iter()
            .map(|p| {
                if p[1] > 0.0 {
                    [p[0], p[1] + 40.0, p[2]]
                } else {
                    *p
                }
            })
            .collect(),
        _ => panic!(),
    };
    let depth = app.doc.undo_depth();
    let out = app
        .run(
            "pose_morph.add_target",
            json!({"object": obj.0, "name": "Tall", "points": pts}),
        )
        .unwrap();
    assert_eq!(
        app.doc.undo_depth(),
        depth + 1,
        "tag, base and target are one step"
    );
    let tag = NodeId(out["tag"].as_u64().unwrap());
    assert_eq!(out["slot"], 0);
    // the strength is a percent parameter
    app.run(
        "param.set",
        json!({"node": tag.0, "param": "strength_0", "value": 50.0}),
    )
    .unwrap();
    assert!((top(&app, obj) - (base_top + 20.0)).abs() < 1e-9);
    // a second target reuses the tag and takes slot 1; update from the current points
    let out = app
        .run("pose_morph.add_target", json!({"object": obj.0}))
        .unwrap();
    assert_eq!(out["tag"].as_u64().unwrap(), tag.0);
    assert_eq!(out["slot"], 1);
    app.run("pose_morph.update_target", json!({"tag": tag.0, "slot": 1}))
        .unwrap();
    assert!(
        app.run("pose_morph.update_target", json!({"tag": tag.0, "slot": 7}))
            .is_err()
    );
    app.run("pose_morph.clear_target", json!({"tag": tag.0, "slot": 0}))
        .unwrap();
    assert_eq!(pose_morph::used_slots(app.doc.scene(), tag), vec![1]);
    assert!((top(&app, obj) - base_top).abs() < 1e-9);
    app.run("pose_morph.set_base", json!({"tag": tag.0}))
        .unwrap();
    assert!(app.doc.undo());
    assert!(app.doc.undo());
    assert_eq!(pose_morph::used_slots(app.doc.scene(), tag), vec![0, 1]);
    // not a tag
    assert!(
        app.run("pose_morph.clear_target", json!({"tag": obj.0, "slot": 0}))
            .is_err()
    );
}

#[test]
fn a_strength_key_set_through_the_command_is_a_percent_track() {
    let mut app = App::new_default();
    let obj = editable_cylinder(&mut app);
    let out = app
        .run("pose_morph.add_target", json!({"object": obj.0}))
        .unwrap();
    let tag = NodeId(out["tag"].as_u64().unwrap());
    app.doc
        .transact("key", |tx| {
            tx.set_key(
                tag,
                "strength_0",
                Key {
                    time: 1.0,
                    value: 1.0,
                    interp: Interp::Linear,
                },
            )
        })
        .unwrap();
    let ty = app.doc.scene().node_type(tag).unwrap().clone();
    assert!(ty.find_param("strength_0").unwrap().animatable);
}

#[test]
fn tag_attach_and_auto_bind_build_a_skinned_object() {
    let mut app = App::new_default();
    let obj = editable_cylinder(&mut app);
    let chain = app
        .run(
            "joint.create_chain",
            json!({"points": [[0, -50, 0], [0, 0, 0], [0, 50, 0]], "align": "y"}),
        )
        .unwrap();
    let joints: Vec<u64> = chain["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let out = app
        .run(
            "weights.auto_bind",
            json!({"object": obj.0, "joints": joints}),
        )
        .unwrap();
    let tag = NodeId(out["tag"].as_u64().unwrap());
    let scene = app.doc.scene();
    assert_eq!(forge_rig::weights::tag_joints(scene, tag).len(), 3);
    assert!(
        !forge_rig::weights::read_weights(scene, tag)
            .points()
            .next()
            .is_none()
    );
    assert!(
        scene
            .node(obj)
            .unwrap()
            .children
            .iter()
            .any(|c| scene.node(*c).unwrap().type_id == forge_rig::SKIN)
    );
    // binding twice keeps one tag and one skin
    app.run(
        "weights.auto_bind",
        json!({"object": obj.0, "joints": joints}),
    )
    .unwrap();
    let scene = app.doc.scene();
    assert_eq!(
        scene.node(obj).unwrap().children.len(),
        1,
        "one skin deformer"
    );
    assert!(
        app.run(
            "weights.auto_bind",
            json!({"object": obj.0, "joints": [obj.0]})
        )
        .is_err()
    );
    // tag.attach
    let ik = app
        .run(
            "tag.attach",
            json!({"type": "forge.ik_tag", "node": joints[0]}),
        )
        .unwrap();
    let ik = NodeId(ik["node"].as_u64().unwrap());
    assert_eq!(
        app.doc.scene().node(ik).unwrap().host,
        Some(NodeId(joints[0]))
    );
    assert!(
        app.run(
            "tag.attach",
            json!({"type": "forge.cube", "node": joints[0]})
        )
        .is_err()
    );
}
