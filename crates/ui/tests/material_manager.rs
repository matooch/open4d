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
fn dragging_a_material_card_onto_an_object_creates_one_undoable_tag() {
    use egui::{Event, Modifiers, PointerButton, Rect, vec2};
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use forge_ui::material_manager::{MaterialPreviews, material_manager_ui};
    use forge_ui::object_manager::ObjectManager;

    let mut s = state();
    let cube = NodeId(
        execute(&mut s, "object.create", json!({"type": "forge.cube"})).unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    execute(
        &mut s,
        "object.rename",
        json!({"node": cube.0, "name": "Drop target"}),
    )
    .unwrap();
    let other = NodeId(
        execute(&mut s, "object.create", json!({"type": "forge.sphere"})).unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    let m = NodeId(
        execute(&mut s, "material.create", json!({"name": "Drag material"})).unwrap()["material"]
            .as_u64()
            .unwrap(),
    );
    let depth = s.doc.undo_depth();
    let original_tags = s.doc.scene().node(cube).unwrap().tags.clone();
    let other_tags = s.doc.scene().node(other).unwrap().tags.clone();
    let mut objects = ObjectManager::default();
    let mut previews = MaterialPreviews::default();
    let mut h = Harness::new_ui_state(
        move |ui, st| {
            let r = ui.available_rect_before_wrap();
            let a = Rect::from_min_size(r.min, vec2(170.0, r.height()));
            let b = Rect::from_min_max(r.min + vec2(185.0, 0.0), r.max);
            ui.scope_builder(
                egui::UiBuilder::new().id_salt("materials").max_rect(a),
                |ui| {
                    for (cmd, args) in material_manager_ui(ui, st, &mut previews) {
                        execute(st, cmd, args).unwrap();
                    }
                },
            );
            ui.scope_builder(
                egui::UiBuilder::new().id_salt("objects").max_rect(b),
                |ui| {
                    for (cmd, args) in objects.show(ui, st) {
                        execute(st, cmd, args).unwrap();
                    }
                },
            );
        },
        s,
    );
    h.run();
    let from = h.get_by_label("Drag material").rect().center();
    let to = h.get_by_label("Drop target").rect().center();
    h.hover_at(from);
    h.run();
    for pressed in [true, false] {
        let pos = if pressed { from } else { to };
        h.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run();
        if pressed {
            for k in 1..=4 {
                h.hover_at(from + (to - from) * (k as f32 / 4.0));
                h.run();
            }
        }
    }
    let s = h.state_mut();
    let scene = s.doc.scene();
    let tags = &scene.node(cube).unwrap().tags;
    let [added] = tags.strip_prefix(original_tags.as_slice()).unwrap() else {
        panic!("one new texture tag, got {tags:?}");
    };
    assert_eq!(scene.param(*added, "material"), Some(&Value::Link(Some(m))));
    assert_eq!(scene.node(other).unwrap().tags, other_tags);
    assert_eq!(s.selection.active, Some(other));
    assert_eq!(s.doc.undo_depth(), depth + 1);
    assert!(s.doc.undo());
    assert_eq!(s.doc.scene().node(cube).unwrap().tags, original_tags);
}

#[test]
fn cpu_thumbnail_tracks_material_colour() {
    use forge_ui::material_manager::render_preview;
    let mut s = state();
    let m = NodeId(
        execute(&mut s, "material.create", json!({})).unwrap()["material"]
            .as_u64()
            .unwrap(),
    );
    let mut chart = Vec::new();
    for color in [[0.9, 0.0, 0.0], [0.0, 0.0, 0.9]] {
        s.doc
            .transact("preview colour", |tx| {
                tx.set_param(m, "color.color", Value::Color(color))
            })
            .unwrap();
        let scene = s.doc.scene();
        let preview = render_preview(scene, m).unwrap();
        assert_eq!(preview.size, [64, 64]);
        assert_eq!(preview, render_preview(scene, m).unwrap());
        chart.push(preview.pixels[32 * 64 + 32]);
    }
    assert!(
        chart[0].r() > chart[0].b() + 40 && chart[1].b() > chart[1].r() + 40,
        "{chart:?}"
    );
}

#[test]
fn the_panel_is_in_the_window_menu_list() {
    assert!(Panel::ALL.contains(&Panel::Materials));
    assert_eq!(Panel::Materials.title(), "Materials");
}
