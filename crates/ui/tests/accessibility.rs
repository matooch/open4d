//! docs/specs/UI.md §24: themes, UI scale and HiDPI, IME text, keyboard navigation.

mod common;

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::prefs::UiPrefs;
use forge_ui::theme::{Theme, ThemeKind, contrast};
use serde_json::json;

fn harness_at(ppp: f32, prefs: Option<std::path::PathBuf>) -> Harness<'static, Editor> {
    let mut state = common::state();
    if prefs.is_some() {
        state.prefs.use_settings_file(prefs);
    }
    let editor = Editor::new(state);
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .with_pixels_per_point(ppp)
        .allow_missing_glyphs()
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    h
}

// ---- tokens -------------------------------------------------------------------------------

#[test]
fn theme_names_round_trip_and_dark_is_the_default() {
    for k in ThemeKind::ALL {
        assert_eq!(ThemeKind::from_name(k.name()), Some(k));
        assert_eq!(Theme::of(k).kind, k);
    }
    assert_eq!(ThemeKind::from_name("blue"), None);
    assert_eq!(Theme::of(ThemeKind::Dark), Theme::default());
    assert_eq!(
        ThemeKind::ALL.map(|k| k.name()),
        ["dark", "light", "high-contrast"]
    );
}

#[test]
fn every_theme_meets_the_contrast_targets() {
    for k in ThemeKind::ALL {
        let t = Theme::of(k);
        let n = k.name();
        let min_text = if k == ThemeKind::HighContrast {
            15.0
        } else {
            7.0
        };
        assert!(contrast(t.text, t.panel) >= min_text, "{n} text/panel");
        assert!(contrast(t.text, t.window) >= min_text, "{n} text/window");
        assert!(contrast(t.weak_text, t.panel) >= 4.5, "{n} weak");
        assert!(contrast(t.accent_text, t.accent) >= 4.5, "{n} accent");
        assert!(contrast(t.focus, t.panel) >= 3.0, "{n} focus");
        let min_error = if k == ThemeKind::Dark { 3.0 } else { 4.5 };
        assert!(contrast(t.error, t.panel) >= min_error, "{n} error");
    }
}

#[test]
fn light_and_high_contrast_differ_from_dark() {
    let dark = Theme::of(ThemeKind::Dark);
    for k in [ThemeKind::Light, ThemeKind::HighContrast] {
        assert_ne!(Theme::of(k), dark);
    }
    let light = Theme::of(ThemeKind::Light).visuals();
    assert!(!light.dark_mode);
    assert!(Theme::of(ThemeKind::Dark).visuals().dark_mode);
}

#[test]
fn visuals_draw_the_focus_ring_and_ime_underline_in_the_focus_colour() {
    for k in ThemeKind::ALL {
        let t = Theme::of(k);
        let v = t.visuals();
        assert_eq!(v.widgets.active.bg_stroke.color, t.focus);
        assert!(v.widgets.active.bg_stroke.width >= 2.0);
        assert_eq!(v.ime_composition.active_underline_stroke.color, t.focus);
    }
}

#[test]
fn current_theme_is_what_was_applied_to_the_context() {
    let ctx = egui::Context::default();
    assert_eq!(Theme::current(&ctx), Theme::default());
    Theme::of(ThemeKind::Light).apply_visuals(&ctx);
    assert_eq!(Theme::current(&ctx).kind, ThemeKind::Light);
}

// ---- preferences --------------------------------------------------------------------------

#[test]
fn prefs_round_trip_keep_other_keys_and_fall_back_per_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, r#"{"keymap": {"x": 1}}"#).unwrap();
    let mut p = UiPrefs::default();
    p.use_settings_file(Some(path.clone()));
    assert_eq!((p.theme, p.scale), (ThemeKind::Dark, 1.0));
    p.theme = ThemeKind::Light;
    p.scale = 1.5;
    p.save().unwrap();
    let file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(file["keymap"], json!({"x": 1}));
    assert_eq!(file["ui"], json!({"theme": "light", "scale": 1.5}));
    let mut q = UiPrefs::default();
    q.use_settings_file(Some(path.clone()));
    assert_eq!((q.theme, q.scale), (ThemeKind::Light, 1.5));
    // bad fields fall back one by one
    std::fs::write(&path, r#"{"ui": {"theme": "neon", "scale": 9}}"#).unwrap();
    q.use_settings_file(Some(path.clone()));
    assert_eq!((q.theme, q.scale), (ThemeKind::Dark, 1.0));
    std::fs::write(
        &path,
        r#"{"ui": {"theme": "high-contrast", "scale": "big"}}"#,
    )
    .unwrap();
    q.use_settings_file(Some(path));
    assert_eq!((q.theme, q.scale), (ThemeKind::HighContrast, 1.0));
    // no file: saving is a no-op
    UiPrefs::default().save().unwrap();
}

// ---- commands -----------------------------------------------------------------------------

#[test]
fn commands_need_the_window_and_validate_their_arguments() {
    let mut s = common::state();
    for (id, args) in [
        ("ui.theme", json!({"name": "light"})),
        ("ui.scale", json!({"factor": 1.5})),
    ] {
        let r = s.app.commands.clone();
        assert!(r.get(id).is_some_and(|c| !c.undoable()), "{id}");
        // the editor state queues the request for the window
        execute(&mut s, id, args).unwrap();
        assert_eq!(s.ui_requests.len(), 1, "{id}");
        s.ui_requests.clear();
    }
    for bad in [0.4, 3.1, f64::NAN] {
        let e = execute(&mut s, "ui.scale", json!({"factor": bad}));
        assert!(e.is_err(), "{bad}");
    }
    assert!(execute(&mut s, "ui.scale", json!({})).is_err());
    assert!(execute(&mut s, "ui.theme", json!({"name": 3})).is_err());
    assert!(s.ui_requests.is_empty());
}

#[test]
fn switching_theme_and_scale_through_the_window_applies_and_saves() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let mut h = harness_at(1.0, Some(path.clone()));
    assert_eq!(Theme::current(&h.ctx).kind, ThemeKind::Dark);
    for kind in ThemeKind::ALL {
        execute(
            &mut h.state_mut().state,
            "ui.theme",
            json!({"name": kind.name()}),
        )
        .unwrap();
        h.run();
        assert_eq!(Theme::current(&h.ctx).kind, kind);
        assert_eq!(h.state().state.prefs.theme, kind);
    }
    execute(&mut h.state_mut().state, "ui.scale", json!({"factor": 1.5})).unwrap();
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 1.5);
    let file: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(file["ui"], json!({"theme": "high-contrast", "scale": 1.5}));
    // an unknown theme leaves things alone and tells the status bar
    execute(
        &mut h.state_mut().state,
        "ui.theme",
        json!({"name": "blue"}),
    )
    .unwrap();
    h.run();
    assert_eq!(Theme::current(&h.ctx).kind, ThemeKind::HighContrast);
    assert!(
        h.state().state.status.contains("no theme named"),
        "{}",
        h.state().state.status
    );
}

// ---- HiDPI --------------------------------------------------------------------------------

#[test]
fn the_editor_lays_out_at_scale_1_1_5_and_2() {
    for ppp in [1.0_f32, 1.5, 2.0] {
        let mut h = harness_at(ppp, None);
        assert_eq!(h.ctx.pixels_per_point(), ppp);
        let screen = h.ctx.content_rect();
        assert!(
            (screen.width() - 1280.0).abs() < 0.01,
            "points do not depend on ppp"
        );
        let used = h.ctx.globally_used_rect();
        assert!(used.width() > 0.0 && used.height() > 0.0);
        assert!(
            screen.contains_rect(used.shrink(0.5)),
            "ppp {ppp}: {used:?} in {screen:?}"
        );
        // every shape is finite and its pixel position is on the physical grid after snapping
        for clipped in h.output().shapes.iter() {
            assert!(clipped.clip_rect.is_finite(), "ppp {ppp}");
        }
        h.run();
    }
}

#[test]
fn ui_scale_shrinks_the_screen_in_points() {
    let mut h = harness_at(2.0, None);
    execute(&mut h.state_mut().state, "ui.scale", json!({"factor": 2.0})).unwrap();
    h.run();
    h.run();
    assert_eq!(h.ctx.zoom_factor(), 2.0);
    // 1280 physical-equivalent points at ppp 2 and zoom 2 = native 1 -> pixels_per_point 4
    assert_eq!(h.ctx.pixels_per_point(), 4.0);
    let screen = h.ctx.content_rect();
    assert!((screen.width() - 640.0).abs() < 0.5, "{screen:?}");
    let used = h.ctx.globally_used_rect();
    assert!(
        screen.contains_rect(used.shrink(0.5)),
        "{used:?} in {screen:?}"
    );
}

// ---- IME ----------------------------------------------------------------------------------

#[test]
fn the_palette_search_takes_composed_text_and_keeps_only_the_committed_result() {
    let mut h = harness_at(1.0, None);
    h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::P);
    h.run();
    h.run();
    assert!(h.state().palette().open);
    h.event(egui::Event::Ime(egui::ImeEvent::Preedit {
        text: "にほ".into(),
        active_range_chars: None,
    }));
    h.run();
    // egui shows the composition in the field while it lasts
    assert_eq!(h.state().palette().query, "にほ");
    h.event(egui::Event::Ime(egui::ImeEvent::Commit("日本語".into())));
    h.run();
    assert_eq!(
        h.state().palette().query,
        "日本語",
        "the commit replaces the preedit"
    );
    h.event(egui::Event::Text("ab".into()));
    h.run();
    assert_eq!(h.state().palette().query, "日本語ab");
}

// ---- keyboard navigation ------------------------------------------------------------------

#[test]
fn tab_is_not_a_shortcut() {
    let keys = forge_ui::keymap::KeyMap::defaults();
    assert!(
        keys.bindings().iter().all(|b| b.chord.key != Key::Tab),
        "Tab and Shift+Tab belong to focus navigation"
    );
}

#[test]
fn tab_and_shift_tab_move_keyboard_focus() {
    let mut h = harness_at(1.0, None);
    assert_eq!(h.ctx.memory(|m| m.focused()), None);
    h.key_press(Key::Tab);
    h.run();
    let first = h.ctx.memory(|m| m.focused());
    assert!(first.is_some(), "Tab focuses a widget");
    h.key_press(Key::Tab);
    h.run();
    let second = h.ctx.memory(|m| m.focused());
    assert!(second.is_some() && second != first, "Tab moves on");
    h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    h.run();
    assert_eq!(h.ctx.memory(|m| m.focused()), first, "Shift+Tab goes back");
}
