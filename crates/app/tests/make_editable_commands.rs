//! docs/specs/EDITABLE.md §1.4 through the registered commands: one undo step each and the
//! selection afterwards (observation O3 "Undo" and "Selection after").

use forge_app::App;
use forge_core::NodeId;
use serde_json::json;

fn create(app: &mut App, ty: &str) -> NodeId {
    let out = app.run("object.create", json!({"type": ty})).unwrap();
    NodeId(out["node"].as_u64().unwrap())
}

fn select(app: &mut App, id: NodeId) {
    app.selection.click(id);
}

#[test]
fn current_state_to_object_keeps_the_original_selected_in_one_undo_step() {
    let mut app = App::new_default();
    let cube = create(&mut app, "forge.cube");
    select(&mut app, cube);
    let before = forge_core::file::save(app.doc.scene());
    let steps = app.doc.history().undo_labels().len();
    app.run("mesh.current_state_to_object", json!({})).unwrap();
    assert_eq!(app.doc.history().undo_labels().len(), steps + 1);
    assert_eq!(app.doc.scene().roots().len(), 2);
    assert_eq!(
        app.selection.active,
        Some(cube),
        "the original stays active"
    );
    assert_eq!(app.selection.nodes.clone(), [cube].into());
    assert!(app.doc.undo());
    assert_eq!(forge_core::file::save(app.doc.scene()), before);
}

#[test]
fn make_editable_selects_the_converted_object_and_not_the_caps() {
    let mut app = App::new_default();
    let extrude = create(&mut app, "forge.extrude");
    app.doc
        .transact("child", |tx| {
            tx.create("forge.rectangle_spline", Some(extrude), None)
        })
        .unwrap();
    select(&mut app, extrude);
    let before = forge_core::file::save(app.doc.scene());
    let steps = app.doc.history().undo_labels().len();
    app.run("mesh.make_editable", json!({})).unwrap();
    assert_eq!(app.doc.history().undo_labels().len(), steps + 1);
    let new = app.selection.active.unwrap();
    assert_ne!(new, extrude);
    assert_eq!(app.selection.nodes.clone(), [new].into());
    let scene = app.doc.scene();
    let names: Vec<_> = scene
        .node(new)
        .unwrap()
        .children
        .iter()
        .map(|c| scene.node(*c).unwrap().name.clone())
        .collect();
    assert_eq!(names, ["Cap 1", "Cap 2"]);
    assert!(app.doc.undo());
    assert_eq!(forge_core::file::save(app.doc.scene()), before);
}
