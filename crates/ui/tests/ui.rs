//! docs/specs/UI.md §9: the egui shell, driven headlessly through accessibility labels.

mod common;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::Value;
use forge_ui::app::Editor;
use forge_ui::dock::{Area, Panel};

fn harness() -> Harness<'static, Editor> {
    let editor = Editor::new(common::state());
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor)
}

fn click(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

#[test]
fn create_select_edit_undo() {
    let mut h = harness();
    click(&mut h, "Create");
    click(&mut h, "Generator");
    click(&mut h, "Box");
    let id = h.state().state.selection.active.expect("box selected");
    assert_eq!(
        h.state().state.doc.scene().node(id).unwrap().type_id,
        "test.box"
    );

    // the attribute panel shows the selected node; toggle a checkbox param
    click(&mut h, "Enabled");
    assert_eq!(
        h.state().state.doc.scene().param(id, "enabled"),
        Some(&Value::Bool(false))
    );

    click(&mut h, "Edit");
    click(&mut h, "Undo");
    assert_eq!(
        h.state().state.doc.scene().param(id, "enabled"),
        Some(&Value::Bool(true))
    );
}

#[test]
fn window_menu_moves_and_closes_panels() {
    let mut h = harness();
    click(&mut h, "Window");
    click(&mut h, "Timeline panel");
    assert_eq!(h.state().state.dock.area_of(Panel::Timeline), None);
    click(&mut h, "Window");
    click(&mut h, "Timeline panel");
    assert_eq!(
        h.state().state.dock.area_of(Panel::Timeline),
        Some(Area::Bottom)
    );
    click(&mut h, "Window");
    click(&mut h, "Move Attributes");
    click(&mut h, "Attributes to left");
    assert_eq!(
        h.state().state.dock.area_of(Panel::Attributes),
        Some(Area::Left)
    );
    click(&mut h, "Window");
    click(&mut h, "Reset layout");
    assert_eq!(
        h.state().state.dock.area_of(Panel::Attributes),
        Some(Area::RightBottom)
    );
}

#[test]
fn viewport_without_gpu_shows_a_notice() {
    let h = harness();
    h.get_by_label("Viewport unavailable");
}
