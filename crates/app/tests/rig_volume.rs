//! Volume and rig nodes in the default editor app (docs/specs/RIG.md section 7, VOLUME.md 11.3,
//! brief 192): the registry evaluator hook reaches the mesher, the joint commands run through
//! the command registry and undo.

use forge_app::App;
use forge_core::{EvalCtx, NodeId, Value};
use serde_json::{Value as Json, json};

fn id(out: &Json) -> NodeId {
    NodeId(out["node"].as_u64().unwrap())
}

fn nodes(out: &Json) -> Vec<NodeId> {
    out["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| NodeId(v.as_u64().unwrap()))
        .collect()
}

#[test]
fn the_default_app_registers_the_volume_types() {
    let app = App::new_default();
    let r = app.doc.scene().registry();
    for t in [
        "forge.volume_builder",
        "forge.volume_mesher",
        "forge.volume_filter",
    ] {
        assert!(r.get(t).is_some(), "{t}");
    }
    assert!(r.evaluator("forge.volume_mesher").is_some());
}

#[test]
fn a_mesher_over_a_builder_over_a_cube_meshes_like_the_volume_crate() {
    let mut app = App::new_default();
    let mesher = id(&app
        .run("object.create", json!({"type": "forge.volume_mesher"}))
        .unwrap());
    let builder = id(&app
        .run("object.create", json!({"type": "forge.volume_builder"}))
        .unwrap());
    let cube = id(&app
        .run("object.create", json!({"type": "forge.cube"}))
        .unwrap());
    app.run(
        "object.reparent",
        json!({"node": builder.0, "parent": mesher.0}),
    )
    .unwrap();
    app.run(
        "object.reparent",
        json!({"node": cube.0, "parent": builder.0}),
    )
    .unwrap();
    app.doc
        .transact("size", |tx| {
            tx.set_param(builder, "voxel_size", Value::Float(25.0))
        })
        .unwrap();
    let expected = forge_volume::mesh_volume_node(app.doc.scene(), mesher, &EvalCtx::default())
        .unwrap()
        .expect("a mesh");
    assert!(!expected.polys.is_empty());
    let got = app.eval_mesh(mesher, 0).unwrap();
    // eval_mesh is in world space; the mesher sits at the origin
    assert_eq!(got.polys, expected.polys);
    assert_eq!(got.points.len(), expected.points.len());
    for (a, b) in got.points.iter().zip(&expected.points) {
        assert!((*a - *b).length() < 1e-9);
    }
}

#[test]
fn joint_commands_run_through_the_registry_and_undo() {
    let mut app = App::new_default();
    for c in ["joint.create_chain", "joint.align", "joint.mirror"] {
        assert!(app.commands.get(c).is_some(), "{c}");
    }
    let before = app.doc.scene().len();
    let out = app
        .run(
            "joint.create_chain",
            json!({"points": [[50.0, 0.0, 0.0], [50.0, 100.0, 0.0], [50.0, 100.0, 80.0]],
                   "prefix": "Arm_L_", "align": "z"}),
        )
        .unwrap();
    let chain = nodes(&out);
    assert_eq!(chain.len(), 3);
    assert_eq!(app.doc.scene().len(), before + 3);
    assert_eq!(app.selection.active, Some(chain[0]));
    assert_eq!(
        app.doc.scene().node(chain[1]).unwrap().parent,
        Some(chain[0])
    );

    let m = app
        .run(
            "joint.mirror",
            json!({"nodes": [chain[0].0], "plane": "yz"}),
        )
        .unwrap();
    let mirrored = nodes(&m);
    assert_eq!(mirrored.len(), 1);
    assert_eq!(app.doc.scene().len(), before + 6);
    let scene = app.doc.scene();
    assert_eq!(scene.node(mirrored[0]).unwrap().name, "Arm_R_1");
    let world = scene.world_matrix(mirrored[0]).unwrap().w_axis;
    assert!((world.x + 50.0).abs() < 1e-9 && world.y.abs() < 1e-9);

    app.run("joint.align", json!({"nodes": [chain[0].0], "axis": "y"}))
        .unwrap();
    // the first joint now points at the second: its local y runs along the bone
    let w = app.doc.scene().world_matrix(chain[0]).unwrap();
    let bone = app.doc.scene().world_matrix(chain[1]).unwrap().w_axis
        - app.doc.scene().world_matrix(chain[0]).unwrap().w_axis;
    let y = w.y_axis.truncate().normalize();
    assert!(y.dot(bone.truncate().normalize()) > 1.0 - 1e-9);

    // each command is one undo step
    for _ in 0..3 {
        app.run("edit.undo", json!({})).unwrap();
    }
    assert_eq!(app.doc.scene().len(), before);
    app.run("edit.redo", json!({})).unwrap();
    assert_eq!(app.doc.scene().len(), before + 3);
}

#[test]
fn joint_commands_refuse_bad_input_without_changing_the_scene() {
    let mut app = App::new_default();
    let cube = id(&app
        .run("object.create", json!({"type": "forge.cube"}))
        .unwrap());
    let rev = app.doc.scene().revision();
    assert!(
        app.run("joint.create_chain", json!({"points": []}))
            .is_err()
    );
    assert!(
        app.run(
            "joint.create_chain",
            json!({"points": [[0, 0, 0]], "parent": 9999})
        )
        .is_err()
    );
    assert!(app.run("joint.align", json!({"nodes": [cube.0]})).is_err());
    assert!(app.run("joint.mirror", json!({"nodes": [cube.0]})).is_err());
    assert!(app.run("joint.mirror", json!({})).is_err());
    assert_eq!(app.doc.scene().revision(), rev);
}
