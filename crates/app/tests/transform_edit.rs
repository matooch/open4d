//! `transform.edit`: the Coordinates manager's command (docs/specs/COMMANDS.md §6,
//! UI.md §20): local/world rows, null axes, several objects, size, one undo step, errors.

use forge_app::App;
use forge_command::CommandError;
use forge_core::NodeId;
use glam::DVec3;
use serde_json::json;

fn close(a: DVec3, b: DVec3) {
    assert!((a - b).length() < 1e-9, "{a:?} != {b:?}");
}

fn make(app: &mut App, ty: &str, pos: [f64; 3]) -> NodeId {
    app.run("object.create", json!({"type": ty})).unwrap();
    let id = app.selection.active.unwrap();
    app.run("transform.set", json!({"node": id.0, "pos": pos}))
        .unwrap();
    id
}

fn local(app: &App, id: NodeId) -> forge_core::transform::Transform {
    app.doc.scene().local_transform(id).unwrap()
}

fn steps(app: &App) -> usize {
    app.doc.history().undo_labels().len()
}

fn two() -> (App, NodeId, NodeId) {
    let mut app = App::new_default();
    let a = make(&mut app, "forge.cube", [10.0, 20.0, 30.0]);
    let b = make(&mut app, "forge.cube", [100.0, 200.0, 300.0]);
    app.selection.nodes = [a, b].into_iter().collect();
    app.selection.active = Some(b);
    (app, a, b)
}

#[test]
fn local_rows_set_only_the_given_axes_of_every_selected_object() {
    let (mut app, a, b) = two();
    let before = steps(&app);
    app.run("transform.edit", json!({"pos": [null, 7.0, null]}))
        .unwrap();
    assert_eq!(steps(&app), before + 1, "two objects, one step");
    close(local(&app, a).pos, DVec3::new(10.0, 7.0, 30.0));
    close(local(&app, b).pos, DVec3::new(100.0, 7.0, 300.0));
    assert!(app.doc.undo());
    close(local(&app, a).pos, DVec3::new(10.0, 20.0, 30.0));
}

#[test]
fn rotation_is_in_degrees_and_scale_is_a_row() {
    let (mut app, a, _b) = two();
    app.run(
        "transform.edit",
        json!({"nodes": [a.0], "rot": [90.0, null, null], "scale": [2.0, 2.0, null]}),
    )
    .unwrap();
    let t = local(&app, a);
    close(t.rot, DVec3::new(std::f64::consts::FRAC_PI_2, 0.0, 0.0));
    close(t.scale, DVec3::new(2.0, 2.0, 1.0));
}

#[test]
fn world_rows_account_for_the_parent() {
    let mut app = App::new_default();
    let parent = make(&mut app, "forge.null", [100.0, 0.0, 0.0]);
    let child = make(&mut app, "forge.cube", [0.0, 0.0, 0.0]);
    app.run(
        "object.move",
        json!({"nodes": [child.0], "parent": parent.0, "index": 0}),
    )
    .ok();
    app.run(
        "transform.set",
        json!({"node": child.0, "pos": [0.0, 0.0, 0.0]}),
    )
    .unwrap();
    app.selection.nodes = [child].into_iter().collect();
    app.run(
        "transform.edit",
        json!({"space": "world", "pos": [150.0, null, null]}),
    )
    .unwrap();
    close(local(&app, child).pos, DVec3::new(50.0, 0.0, 0.0));
    let w = app.doc.scene().world_matrix(child).unwrap();
    close(w.w_axis.truncate(), DVec3::new(150.0, 0.0, 0.0));
}

#[test]
fn size_sets_the_scale_from_the_bounding_box() {
    let (mut app, a, b) = two();
    app.run("transform.edit", json!({"size": [400.0, null, null]}))
        .unwrap();
    // the default cube is 200 wide
    close(local(&app, a).scale, DVec3::new(2.0, 1.0, 1.0));
    close(local(&app, b).scale, DVec3::new(2.0, 1.0, 1.0));
}

#[test]
fn several_rows_are_one_step_and_a_gesture_merges() {
    let (mut app, a, _b) = two();
    let before = steps(&app);
    app.run(
        "transform.edit",
        json!({"pos": [1.0, 2.0, 3.0], "rot": [10.0, 0.0, 0.0], "scale": [1.5, 1.5, 1.5]}),
    )
    .unwrap();
    assert_eq!(steps(&app), before + 1);
    for x in [1.0, 2.0, 3.0] {
        app.run(
            "transform.edit",
            json!({"pos": [x, null, null], "gesture": 9}),
        )
        .unwrap();
    }
    assert_eq!(steps(&app), before + 2, "a dragged field merges");
    close(local(&app, a).pos, DVec3::new(3.0, 2.0, 3.0));
}

#[test]
fn errors_change_nothing() {
    let (mut app, a, _b) = two();
    let before = steps(&app);
    let p = local(&app, a).pos;
    for args in [
        json!({}),
        json!({"pos": [1.0, 2.0, 3.0], "space": "sky"}),
        json!({"scale": [1.0, 1.0, 1.0], "size": [1.0, 1.0, 1.0]}),
        json!({"nodes": [9999], "pos": [1.0, 2.0, 3.0]}),
    ] {
        let r = app.run("transform.edit", args);
        assert!(
            matches!(
                r,
                Err(CommandError::InvalidArgs(_) | CommandError::Failed(_))
            ),
            "{r:?}"
        );
    }
    assert_eq!(steps(&app), before);
    close(local(&app, a).pos, p);
}

#[test]
fn mesh_transform_gestures_merge_into_one_step() {
    let mut app = App::new_default();
    app.run("object.create", json!({"type": "forge.cube"}))
        .unwrap();
    app.run("mesh.make_editable", json!({})).unwrap();
    app.run("select.mode", json!({"mode": "points"})).unwrap();
    app.run("select.all", json!({})).unwrap();
    let before = steps(&app);
    for x in [1.0, 2.0, 3.0] {
        app.run(
            "mesh.transform",
            json!({"move": [x, 0.0, 0.0], "gesture": 5}),
        )
        .unwrap();
    }
    assert_eq!(steps(&app), before + 1);
    app.run("mesh.transform", json!({"move": [1.0, 0.0, 0.0]}))
        .unwrap();
    assert_eq!(steps(&app), before + 2, "no gesture, a step of its own");
    assert!(
        app.run(
            "mesh.transform",
            json!({"move": [1.0, 0.0, 0.0], "gesture": -1})
        )
        .is_err()
    );
}
