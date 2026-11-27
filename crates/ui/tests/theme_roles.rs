//! docs/specs/UI.md §24.1 (brief 234, owner note N11): colour roles instead of one accent.
//! Pure tests, no GPU and no window.

use egui::Color32;
use forge_ui::theme::{Cue, Role, Theme, ThemeKind, contrast, role_audit};

/// Hue in degrees (0..360) of an sRGB colour.
fn hue(c: Color32) -> f64 {
    let (r, g, b) = (
        f64::from(c.r()) / 255.0,
        f64::from(c.g()) / 255.0,
        f64::from(c.b()) / 255.0,
    );
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    if d < 1e-9 {
        return 0.0;
    }
    let h = if max == r {
        ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h * 60.0
}

fn hue_gap(a: Color32, b: Color32) -> f64 {
    let d = (hue(a) - hue(b)).abs();
    d.min(360.0 - d)
}

fn hex(c: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b())
}

#[test]
fn the_dark_tokens_are_the_proposals() {
    let t = Theme::of(ThemeKind::Dark);
    let expect = [
        (t.window, "#171B21"),
        (t.panel, "#20262E"),
        (t.viewport, "#252C35"),
        (t.input, "#181D24"),
        (t.divider, "#343D49"),
        (t.text, "#E8EDF2"),
        (t.weak_text, "#A7B2C0"),
        (t.state, "#73D6BC"),
        (t.selection, "#F3B36B"),
        (t.record, "#F17A80"),
    ];
    for (c, want) in expect {
        assert_eq!(hex(c), want);
    }
}

#[test]
fn roles_share_their_colours_in_every_theme() {
    for k in ThemeKind::ALL {
        let t = Theme::of(k);
        let n = k.name();
        assert_eq!(t.time, t.state, "{n}: the playhead is mint like the state");
        assert_eq!(t.key, t.time, "{n}: key diamonds use the time colour");
        assert_eq!(t.record, t.error, "{n}: recording and errors share the red");
        for role in Role::ALL {
            let want = match role {
                Role::State => t.state,
                Role::Time => t.time,
                Role::Selection => t.selection,
                Role::Record => t.record,
                Role::Focus => t.focus,
                Role::Component => t.component_selection,
            };
            assert_eq!(t.role_colour(role), want, "{n} {role:?}");
        }
    }
}

#[test]
fn every_role_colour_is_readable_on_every_surface() {
    for k in ThemeKind::ALL {
        let t = Theme::of(k);
        let n = k.name();
        let text_min = if k == ThemeKind::HighContrast {
            15.0
        } else {
            7.0
        };
        for (surface, name) in [
            (t.panel, "panel"),
            (t.window, "window"),
            (t.viewport, "viewport"),
            (t.input, "input"),
        ] {
            assert!(
                contrast(t.text, surface) >= text_min,
                "{n}: text on {name}: {:.2}",
                contrast(t.text, surface)
            );
        }
        for (surface, name) in [(t.panel, "panel"), (t.viewport, "viewport")] {
            assert!(
                contrast(t.weak_text, surface) >= 4.5,
                "{n}: weak text on {name}"
            );
        }
        // text drawn in a role colour (amber names, mint labels, red errors) needs 4.5:1
        for (c, role) in [
            (t.state, "state"),
            (t.selection, "selection"),
            (t.record, "record"),
            (t.modified, "modified"),
        ] {
            for (surface, name) in [
                (t.panel, "panel"),
                (t.viewport, "viewport"),
                (t.input, "input"),
            ] {
                assert!(
                    contrast(c, surface) >= 4.5,
                    "{n}: {role} on {name}: {:.2}",
                    contrast(c, surface)
                );
            }
        }
        assert!(contrast(t.state_text, t.state) >= 4.5, "{n}: state text");
        assert!(
            contrast(t.selection_text, t.selection) >= 4.5,
            "{n}: selection text"
        );
        assert!(contrast(t.focus, t.panel) >= 3.0, "{n}: focus");
        assert!(
            contrast(t.component_selection, t.viewport) >= 3.0,
            "{n}: component"
        );
    }
}

#[test]
fn the_roles_are_told_apart_by_hue() {
    for k in ThemeKind::ALL {
        let t = Theme::of(k);
        let n = k.name();
        // amber (selection) is far from mint (state), red (record) and the draft violet
        for (other, name) in [
            (t.state, "state"),
            (t.record, "record"),
            (t.modified, "modified"),
        ] {
            assert!(
                hue_gap(t.selection, other) >= 30.0,
                "{n}: selection vs {name}: {:.0}",
                hue_gap(t.selection, other)
            );
        }
        // the key diamond: a draft is not amber (it would read as selected) and not the key hue
        assert!(hue_gap(t.modified, t.selection) >= 30.0, "{n}");
        assert!(hue_gap(t.modified, t.key) >= 30.0, "{n}");
        assert!(hue_gap(t.modified, t.record) >= 30.0, "{n}");
        // component selection is not the amber of an object
        assert!(hue_gap(t.component_selection, t.selection) >= 60.0, "{n}");
        assert!(hue_gap(t.component_selection, t.state) >= 20.0, "{n}");
    }
}

#[test]
fn the_light_and_high_contrast_roles_keep_the_dark_hues() {
    let dark = Theme::of(ThemeKind::Dark);
    for k in [ThemeKind::Light, ThemeKind::HighContrast] {
        let t = Theme::of(k);
        for (a, b, name) in [
            (t.state, dark.state, "state"),
            (t.selection, dark.selection, "selection"),
            (t.record, dark.record, "record"),
            (t.modified, dark.modified, "modified"),
        ] {
            assert!(hue_gap(a, b) <= 12.0, "{}: {name} hue drifted", k.name());
        }
    }
}

#[test]
fn visuals_use_the_state_colour_for_modes_and_the_input_colour_for_fields() {
    for k in ThemeKind::ALL {
        let t = Theme::of(k);
        let v = t.visuals();
        assert_eq!(v.selection.bg_fill, t.state, "{}", k.name());
        assert_eq!(v.selection.stroke.color, t.state_text, "{}", k.name());
        assert_eq!(v.hyperlink_color, t.state);
        assert_eq!(v.extreme_bg_color, t.input);
        assert_eq!(v.widgets.noninteractive.bg_stroke.color, t.divider);
        assert_eq!(v.error_fg_color, t.error);
    }
}

#[test]
fn a_selection_scope_draws_the_selection_colour_and_restores_the_state_colour() {
    let ctx = egui::Context::default();
    let t = Theme::of(ThemeKind::Dark);
    t.apply_visuals(&ctx);
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        t.selection_scope(ui, |ui| {
            assert_eq!(ui.visuals().selection.bg_fill, t.selection);
            assert_eq!(ui.visuals().selection.stroke.color, t.selection_text);
        });
        assert_eq!(ui.visuals().selection.bg_fill, t.state);
    });
    out.textures_delta.clear();
}

#[test]
fn every_role_lists_its_widgets_and_none_depends_on_colour_alone() {
    let audit = role_audit();
    for role in Role::ALL {
        let uses: Vec<_> = audit.iter().filter(|u| u.role == role).collect();
        assert!(!uses.is_empty(), "{role:?} has no widget in the audit");
        for u in uses {
            assert!(!u.widget.is_empty());
        }
    }
    // the cues are the greyscale story: each is a named non-colour cue (the type has no
    // "colour only" case), and amber, which is as bright as mint in the dark theme, always
    // has a shape cue
    for u in audit.iter().filter(|u| u.role == Role::Selection) {
        assert!(
            matches!(u.cue, Cue::Outline | Cue::Size | Cue::Line | Cue::Fill),
            "{}",
            u.widget
        );
    }
    // the audit has no duplicate widget lines
    for (i, a) in audit.iter().enumerate() {
        assert!(
            audit[i + 1..].iter().all(|b| b.widget != a.widget),
            "duplicate audit line: {}",
            a.widget
        );
    }
}

/// Every `.rs` file under a directory of the crates, recursively.
fn sources(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap().flatten() {
        let p = e.path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_plain_accent_is_left_in_the_ui_or_the_viewport() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut files = Vec::new();
    sources(&root.join("ui/src"), &mut files);
    sources(&root.join("viewport/src"), &mut files);
    assert!(files.len() > 40, "found {} sources", files.len());
    for f in files {
        let text = std::fs::read_to_string(&f).unwrap();
        for (i, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or("");
            for needle in [".accent", "accent:", "accent_text", "ACCENT"] {
                assert!(
                    !code.contains(needle),
                    "{}:{}: `{needle}` (use a role token, UI.md §24.1)",
                    f.display(),
                    i + 1
                );
            }
        }
    }
}
