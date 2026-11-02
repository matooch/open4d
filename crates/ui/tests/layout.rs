//! docs/specs/UI.md §11: the default layout, command bar and tool palette (brief 147).

mod common;

use std::collections::BTreeSet;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_ui::app::Editor;
use forge_ui::dock::{Area, Dock, Panel};
use forge_ui::layout::{LAYOUT_VERSION, Layout, all_buttons, command_bar, tool_palette};

fn harness() -> Harness<'static, Editor> {
    let editor = Editor::new(common::state());
    Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor)
}

fn rect(h: &Harness<'_, Editor>, label: &str) -> egui::Rect {
    h.get_by_label(label).rect()
}

#[test]
fn default_layout_places_the_panels() {
    let h = harness();
    // one widget of each panel: command bar, palette, viewport, the managers' tab
    // headers, the timeline strip
    let bar = rect(&h, "<- Undo");
    let palette = rect(&h, "^^ Extrude");
    let viewport = rect(&h, "Viewport unavailable");
    let objects = rect(&h, "Objects");
    let attributes = rect(&h, "Attributes");
    let timeline = rect(&h, "Timeline");
    let menu = rect(&h, "File");
    let status = rect(&h, "Ready");

    assert!(
        menu.center().y < bar.center().y,
        "command bar below the menus"
    );
    assert!(bar.center().y < palette.center().y, "palette below the bar");
    assert!(bar.center().y < viewport.center().y);
    assert!(
        palette.max.x < viewport.min.x,
        "palette left of the viewport"
    );
    assert!(
        viewport.max.x < objects.min.x,
        "managers right of the viewport"
    );
    assert!(viewport.max.x < attributes.min.x);
    assert!(objects.min.y < attributes.min.y, "objects above attributes");
    assert!(
        (objects.min.x - attributes.min.x).abs() < 20.0,
        "one right column"
    );
    assert!(viewport.center().y < timeline.min.y, "timeline strip below");
    assert!(timeline.min.y < status.min.y, "status bar last");
    // the timeline strip spans under the palette and the viewport, not under the managers
    assert!(timeline.min.x < viewport.min.x + 400.0);
    h.get_by_label("Coordinates");
}

#[test]
fn standard_layout_is_the_dock_default() {
    let l = Layout::standard();
    assert_eq!(l.dock, Dock::default());
    assert_eq!(l.version, LAYOUT_VERSION);
    assert_eq!(l.dock.tabs(Area::Top), [Panel::CommandBar]);
    assert_eq!(l.dock.tabs(Area::Left), [Panel::ToolPalette]);
    assert_eq!(harness().state().layout(), l);
}

#[test]
fn layout_round_trips_through_json() {
    let mut l = Layout::standard();
    l.dock.move_panel(Panel::Script, Area::RightBottom);
    l.dock.move_panel(Panel::Coordinates, Area::Left);
    l.sizes.left_width = 99.5;
    let text = l.to_json();
    assert_eq!(Layout::from_json(&text).unwrap(), l);
    // applying a loaded layout changes the editor
    let mut h = harness();
    h.state_mut().set_layout(Layout::from_json(&text).unwrap());
    h.run();
    assert_eq!(h.state().layout(), l);
    assert_eq!(
        h.state().state.dock.area_of(Panel::Script),
        Some(Area::RightBottom)
    );
}

#[test]
fn bad_layouts_are_rejected() {
    assert!(Layout::from_json("{").is_err());
    let text = Layout::standard().to_json();
    let wrong_version = text.replace("\"version\": 1", "\"version\": 99");
    assert!(
        Layout::from_json(&wrong_version)
            .unwrap_err()
            .contains("version")
    );
    // the viewport listed in two areas
    let doubled = text.replacen("\"CommandBar\"", "\"Viewport\"", 1);
    assert!(Layout::from_json(&doubled).is_err());
}

#[test]
fn every_button_maps_to_a_registry_command() {
    let st = common::state();
    let commands = st.app.commands.clone();
    let buttons = all_buttons();
    assert!(buttons.len() >= 25, "{} buttons", buttons.len());
    let mut texts = BTreeSet::new();
    for b in &buttons {
        let handler = commands
            .get(b.command)
            .unwrap_or_else(|| panic!("`{}` ({}) is not a registry command", b.command, b.label));
        assert!(!b.label.is_empty() && !b.tip.is_empty() && !b.glyph.is_empty());
        assert!(b.args.is_object(), "{}: arguments are an object", b.label);
        let _ = handler;
        assert!(texts.insert(b.text()), "duplicate button text {}", b.text());
    }
    assert!(!command_bar().is_empty() && !tool_palette().is_empty());
    // create buttons name registered node types
    for b in buttons.iter().filter(|b| b.command == "object.create") {
        let ty = b.args["type"].as_str().unwrap();
        assert!(
            st.doc.scene().registry().get(ty).is_some(),
            "unknown node type {ty}"
        );
    }
}

#[test]
fn buttons_run_their_commands() {
    let mut h = harness();
    h.get_by_label("# Cube").click();
    h.run();
    let id = h.state().state.selection.active.expect("cube created");
    assert_eq!(
        h.state().state.doc.scene().node(id).unwrap().type_id,
        "forge.cube"
    );
    h.get_by_label("<- Undo").click();
    h.run();
    assert!(h.state().state.selection.nodes.is_empty());
    h.get_by_label("-> Redo").click();
    h.run();
    let len = h.state().state.doc.scene().len();
    assert!(len >= 1);

    // the mode buttons run select.mode
    h.get_by_label("\\ Pol").click();
    h.run();
    assert_eq!(h.state().state.mode, forge_ui::components::EditMode::Polys);
    // a failing command reports in the status bar and changes nothing
    h.get_by_label("^^ Extrude").click();
    h.run();
    assert!(!h.state().state.status.is_empty());
    assert_eq!(h.state().state.doc.scene().len(), len);
}

#[test]
fn reset_layout_restores_the_standard_layout() {
    let mut h = harness();
    h.state_mut().state.dock.close(Panel::ToolPalette);
    h.run();
    h.get_by_label("Window").click();
    h.run();
    h.get_by_label("Reset layout").click();
    h.run();
    assert_eq!(h.state().layout(), Layout::standard());
}
