//! UI.md §24.2a: per-user preferences survive editor restarts.

use std::path::Path;
use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::RenderFilter;
use forge_rpc::RpcSettings;
use forge_ui::app::Editor;
use forge_ui::prefs::{DistanceUnit, UiPrefs};
use forge_ui::state::EditorState;
use forge_ui::theme::ThemeKind;
use serde_json::json;

fn fresh(path: &Path) -> EditorState {
    let mut st = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    st.prefs.use_settings_file(Some(path.to_path_buf()));
    st
}

#[test]
fn persisted_values_reload_in_a_fresh_editor_and_keep_other_keys() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, r#"{"keymap":{"installed":true},"rpc":{"other":5}}"#).unwrap();
    let mut editor = fresh(&path);
    let p = &mut editor.prefs;
    assert_eq!(p.distance, DistanceUnit::Centimetre);
    assert_eq!(
        (p.render_defaults.width, p.render_defaults.height),
        (1280, 720)
    );
    assert_eq!(p.render_defaults.filter, RenderFilter::Cubic);
    assert_eq!(p.render_defaults.filter_width, 2.0);
    assert!(p.render_defaults.default_light);
    assert!(!p.use_render_defaults);
    p.theme = ThemeKind::Light;
    p.scale = 1.5;
    p.distance = DistanceUnit::Metre;
    p.autosave.enabled = false;
    p.autosave.interval_seconds = 85;
    p.use_render_defaults = true;
    p.render_defaults.width = 1024;
    p.render_defaults.samples = 12;
    p.rpc.port = 8447;
    p.rpc.token = Some("test-token-not-a-real-credential".into());
    p.save_all().unwrap();
    let restart = fresh(&path);
    assert_eq!(restart.prefs, *p);
    assert_eq!(restart.prefs.distance.to_display(125.0), 1.25);
    assert_eq!(restart.prefs.distance.to_centimetres(1.25), 125.0);
    assert!(!format!("{:?}", restart.prefs).contains("test-token"));
    let file: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(file["keymap"], json!({"installed":true}));
    assert_eq!(file["rpc"]["other"], 5);
    assert_eq!(RpcSettings::resolve(&|_| None, &file), p.rpc);
    assert_eq!(
        RpcSettings::resolve(
            &|key| (key == "FORGE_RPC_PORT").then(|| "9981".into()),
            &file
        )
        .port,
        9981
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o077,
            0
        );
    }
    // A rejected save must not replace the persisted settings.
    p.render_defaults.height = 0;
    assert!(p.save_all().is_err());
    assert_eq!(fresh(&path).prefs.render_defaults.height, 720);
}

#[test]
fn preferences_window_applies_or_discards_without_editing_the_scene() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let editor = Editor::new(fresh(&path));
    let mut h = Harness::builder()
        .with_size([1350.0, 900.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    let revision = h.state().state.doc.scene().revision();
    let open = |h: &mut Harness<'_, Editor>| {
        h.get_by_label("Edit").click();
        h.run();
        h.get_by_label("Preferences").click();
        h.run();
    };
    open(&mut h);
    h.get_by_label("Enable autosave").click();
    h.run();
    h.get_by_label("Cancel").click();
    h.run();
    assert!(h.state().state.prefs.autosave.enabled);
    assert!(!path.exists());
    open(&mut h);
    h.get_by_label("Enable autosave").click();
    h.run();
    h.get_by_label("Apply preferences").click();
    h.run();
    assert!(!h.state().state.autosave.settings.enabled);
    assert!(!fresh(&path).prefs.autosave.enabled);
    assert_eq!(h.state().state.doc.scene().revision(), revision);
    assert!(UiPrefs::default().autosave.enabled);
}
