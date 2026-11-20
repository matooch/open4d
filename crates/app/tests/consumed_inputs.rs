//! Consumed-input rules of the volume builder and mesher (docs/specs/CORE.md section 2,
//! VOLUME.md section 6.5, brief 048).

use forge_app::App;
use forge_core::{NodeId, Value};
use forge_generators::is_consumed;
use serde_json::{Value as Json, json};

fn make(app: &mut App, ty: &str, parent: Option<NodeId>) -> NodeId {
    let out: Json = app.run("object.create", json!({"type": ty})).unwrap();
    let id = NodeId(out["node"].as_u64().unwrap());
    if let Some(p) = parent {
        app.run("object.reparent", json!({"node": id.0, "parent": p.0}))
            .unwrap();
    }
    id
}

#[test]
fn builder_children_and_a_builder_under_a_mesher_are_consumed() {
    let mut app = App::new_default();
    let mesher = make(&mut app, "forge.volume_mesher", None);
    let builder = make(&mut app, "forge.volume_builder", Some(mesher));
    let cube = make(&mut app, "forge.cube", Some(builder));
    let filter = make(&mut app, "forge.volume_filter", Some(builder));
    let null = make(&mut app, "forge.null", Some(builder));
    let held = make(&mut app, "forge.sphere", Some(null));
    let s = app.doc.scene();
    for n in [builder, cube, filter, null, held] {
        assert!(is_consumed(s, n), "{n:?}");
    }
    assert!(!is_consumed(s, mesher));
}

#[test]
fn a_plain_null_keeps_its_children_drawn() {
    let mut app = App::new_default();
    let null = make(&mut app, "forge.null", None);
    let cube = make(&mut app, "forge.cube", Some(null));
    assert!(!is_consumed(app.doc.scene(), cube));
    assert!(!is_consumed(app.doc.scene(), null));
}

#[test]
fn nested_generators_under_a_builder_are_expanded_first() {
    let mut app = App::new_default();
    let mesher = make(&mut app, "forge.volume_mesher", None);
    let builder = make(&mut app, "forge.volume_builder", Some(mesher));
    let cloner = make(&mut app, "forge.cloner", Some(builder));
    let cube = make(&mut app, "forge.cube", Some(cloner));
    let sub = make(&mut app, "forge.subdivision", Some(cloner));
    let inner = make(&mut app, "forge.cube", Some(sub));
    let s = app.doc.scene();
    // the cloner is a builder input; its prototype and the subdivision input stay consumed
    assert!(is_consumed(s, cloner));
    assert!(is_consumed(s, cube));
    assert!(is_consumed(s, sub));
    assert!(is_consumed(s, inner));
    app.doc
        .transact("size", |tx| {
            tx.set_param(builder, "voxel_size", Value::Float(30.0))
        })
        .unwrap();
    let m = app.eval_mesh(mesher, 0).unwrap();
    assert!(!m.polys.is_empty());
}
