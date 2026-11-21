//! The Materials panel (docs/specs/MATERIAL.md M6, brief 194): rows follow the document's
//! material list, and the panel's commands are registry commands.

use std::sync::Arc;

use forge_core::{NodeId, Value};
use forge_ui::commands::execute;
use forge_ui::dock::Panel;
use forge_ui::material_manager::rows;
use forge_ui::state::EditorState;
use serde_json::json;

fn state() -> EditorState {
    EditorState::new(Arc::new(forge_app::default_registry().unwrap()))
}

#[test]
fn rows_list_the_materials_in_order_with_their_icon_colour() {
    let mut s = state();
    assert!(rows(&s).is_empty());
    let a = execute(&mut s, "material.create", json!({"name": "A"})).unwrap();
    let b = execute(&mut s, "material.create", json!({"name": "B", "index": 0})).unwrap();
    let (a, b) = (
        NodeId(a["material"].as_u64().unwrap()),
        NodeId(b["material"].as_u64().unwrap()),
    );
    let r = rows(&s);
    assert_eq!(r.iter().map(|x| x.0).collect::<Vec<_>>(), [b, a]);
    assert_eq!(r[0].1, "B");
    // the default icon colour (1, 0.9, 0.4)
    assert_eq!(r[0].2, [255, 230, 102]);
    execute(&mut s, "material.delete", json!({"materials": [b.0]})).unwrap();
    assert_eq!(rows(&s).len(), 1);
    assert!(s.doc.undo());
    assert_eq!(rows(&s).len(), 2);
}

#[test]
fn assign_adds_a_texture_tag_to_the_selected_objects() {
    let mut s = state();
    let cube = execute(&mut s, "object.create", json!({"type": "forge.cube"})).unwrap();
    let cube = NodeId(cube["node"].as_u64().unwrap());
    let m = execute(&mut s, "material.create", json!({})).unwrap();
    let m = m["material"].as_u64().unwrap();
    let r = execute(&mut s, "material.assign", json!({"material": m})).unwrap();
    let tag = NodeId(r["tags"][0].as_u64().unwrap());
    assert_eq!(s.doc.scene().node(tag).unwrap().host, Some(cube));
    assert_eq!(
        s.doc.scene().param(tag, "material"),
        Some(&Value::Link(Some(NodeId(m))))
    );
}

#[test]
fn the_panel_is_in_the_window_menu_list() {
    assert!(Panel::ALL.contains(&Panel::Materials));
    assert_eq!(Panel::Materials.title(), "Materials");
}
