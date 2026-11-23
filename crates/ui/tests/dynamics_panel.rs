use egui_kittest::{Harness, kittest::Queryable};
use forge_ui::app::Editor;
use forge_ui::state::EditorState;
use std::sync::Arc;

#[test]
fn settings_window_reads_defaults_without_edits_and_creates_settings_on_change() {
    let state = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    let mut h = Harness::builder()
        .with_size([1280.0, 900.0])
        .build_ui_state(
            |ui, editor: &mut Editor| editor.show(ui, None),
            Editor::new(state),
        );
    h.run();
    let before = h.state().state.doc.undo_depth();
    h.get_by_label("Dynamics").click();
    h.run();
    h.get_by_label("Dynamics Settings").click();
    h.run();
    assert_eq!(h.state().state.doc.undo_depth(), before);
    h.get_by_label("Simulation Enabled").click();
    h.run();
    let doc = &h.state().state.doc;
    assert_eq!(doc.undo_depth(), before + 1);
    let settings = forge_app::dynamics::DynamicsSettings::of_scene(doc.scene());
    assert!(!settings.enabled);
    assert_eq!(settings.steps_per_frame, 2);
}
