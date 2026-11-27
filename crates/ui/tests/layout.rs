//! docs/specs/UI.md §11: the default layout, command bar and tool palette (brief 147).

use std::collections::BTreeSet;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_command::ModeArg;
use forge_ui::app::Editor;
use forge_ui::dock::{Area, Dock, Panel};
use forge_ui::layout::{
    LAYOUT_VERSION, Layout, Sizes, WINDOW, all_buttons, command_bar, regions, tool_palette,
};
use forge_ui::theme::BUTTONS;
use forge_ui::tools::help_text;
use forge_viewport::gizmo::GizmoMode;

fn harness() -> Harness<'static, Editor> {
    let editor = Editor::new(forge_ui::state::EditorState::new(std::sync::Arc::new(
        forge_app::default_registry().unwrap(),
    )));
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
    let bar = rect(&h, "Undo (toolbar)");
    let palette = rect(&h, "Extrude (tool rail)");
    let viewport = rect(&h, "Viewport unavailable");
    let objects = rect(&h, "Objects");
    let attributes = rect(&h, "Attributes");
    let timeline = rect(&h, "Timeline");
    let menu = rect(&h, "File");
    let status = rect(&h, &help_text(GizmoMode::Translate, ModeArg::Model));

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
    assert!(
        Layout::standard()
            .dock
            .tabs(Area::Bottom)
            .contains(&Panel::Materials)
    );
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
    let st = harness();
    let st = &st.state().state;
    let commands = st.app.commands.clone();
    let buttons = all_buttons();
    assert!(buttons.len() >= 25, "{} buttons", buttons.len());
    let mut labels = BTreeSet::new();
    let atlas = forge_ui::icons::IconAtlas::build(1.0);
    for b in &buttons {
        let handler = commands
            .get(b.command)
            .unwrap_or_else(|| panic!("`{}` ({}) is not a registry command", b.command, b.label));
        assert!(!b.label.is_empty() && !b.tip.is_empty());
        assert!(atlas.rect(b.icon).is_some(), "missing icon for {}", b.label);
        assert!(b.args.is_object(), "{}: arguments are an object", b.label);
        let _ = handler;
        assert!(labels.insert(b.label), "duplicate button label {}", b.label);
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
    h.get_by_label("Cube (toolbar)").click();
    h.run();
    let id = h.state().state.selection.active.expect("cube created");
    assert_eq!(
        h.state().state.doc.scene().node(id).unwrap().type_id,
        "forge.cube"
    );
    h.get_by_label("Undo (toolbar)").click();
    h.run();
    assert!(h.state().state.selection.nodes.is_empty());
    h.get_by_label("Redo (toolbar)").click();
    h.run();
    let len = h.state().state.doc.scene().len();
    assert!(len >= 1);

    // the mode buttons run select.mode
    h.get_by_label("Polygon Mode (tool rail)").click();
    h.run();
    assert_eq!(h.state().state.mode, forge_ui::components::EditMode::Polys);
    // a failing command reports in the status bar and changes nothing
    h.get_by_label("Extrude (tool rail)").click();
    h.run();
    assert!(!h.state().state.status.is_empty());
    assert_eq!(h.state().state.doc.scene().len(), len);
}

#[test]
fn the_transform_rail_and_cloner_buttons_use_the_editor_commands() {
    let mut h = harness();
    let move_rect = rect(&h, "Move (tool rail)");
    let rotate_rect = rect(&h, "Rotate (tool rail)");
    assert_eq!(move_rect.center().x, rotate_rect.center().x);
    assert!(move_rect.max.y < rotate_rect.min.y);
    h.get_by_label("Rotate (tool rail)").click();
    h.run();
    assert_eq!(h.state().state.gizmo_mode, GizmoMode::Rotate);

    h.get_by_label("Cloner (toolbar)").click();
    h.run();
    let cloner = h.state().state.selection.active.unwrap();
    let doc = &h.state().state.doc;
    assert_eq!(
        doc.scene().node(cloner).unwrap().type_id,
        forge_clone_engine::CLONER
    );
    assert_eq!(doc.history().undo_labels().len(), 1);
    h.get_by_label("Clone tools").click();
    h.run();
    h.get_by_label("Cloner from Selection");
}

#[test]
fn animation_editors_share_the_playhead_and_expand_below_the_viewport() {
    let mut h = harness();
    let compact = rect(&h, "Frame strip");
    assert!(compact.max.x < rect(&h, "Attributes").min.x);
    h.get_by_label("Next Frame").click();
    h.run();
    let frame = h.state().state.timeline.current;
    assert_eq!(frame, 1);

    h.get_by_label("Dope Sheet").click();
    h.run();
    let sheet = rect(&h, "Dope sheet");
    assert!(sheet.height() > compact.height());
    assert_eq!(h.state().state.timeline.current, frame);
    h.get_by_label("F-Curve").click();
    h.run();
    h.run();
    let ruler = rect(&h, "Curve frame ruler");
    for _ in 0..3 {
        h.run();
        assert_eq!(rect(&h, "Curve frame ruler"), ruler);
    }
    assert_eq!(h.state().state.timeline.current, frame);
    let time = forge_ui::fcurve::graph_state(&h.ctx).tview;
    let pos = egui::pos2(time.to_x(15.0, ruler.left()), ruler.center().y);
    h.hover_at(pos);
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            pressed,
            button: egui::PointerButton::Primary,
            modifiers: egui::Modifiers::NONE,
        });
        h.run();
    }
    assert_eq!(h.state().state.timeline.current, 15);
    h.get_by_label("Timeline").click();
    h.run();
    assert_eq!(rect(&h, "Frame strip").height(), compact.height());
    assert!(rect(&h, "Frame strip").min.y > sheet.min.y);
    assert_eq!(h.state().state.timeline.current, 15);
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

// ---- observed window metrics (brief 233, G10 item 1; UI.md §29.1) -----------------------

#[test]
fn the_reference_window_regions_at_1920_by_1080() {
    // rows: title bar 28, document / layout row 24, command bar 42, viewport menu row 20,
    // timeline 92, status bar 24
    assert_eq!(
        (
            WINDOW.title_bar,
            WINDOW.document_row,
            WINDOW.command_bar,
            WINDOW.viewport_menu,
            WINDOW.timeline,
            WINDOW.status_bar
        ),
        (28.0, 24.0, 42.0, 20.0, 92.0, 24.0)
    );
    // columns: tool strip 43, object palette 43, managers 402
    assert_eq!(
        (WINDOW.tool_strip, WINDOW.object_palette, WINDOW.managers),
        (43.0, 43.0, 402.0)
    );
    let r = regions(1920.0, 1080.0);
    // y 0-28 and 28-52, then 52-94 and 94-114
    assert_eq!((r.title_bar.1, r.title_bar.3), (0.0, 28.0));
    assert_eq!((r.document_row.1, r.document_row.3), (28.0, 52.0));
    assert_eq!((r.command_bar.1, r.command_bar.3), (52.0, 94.0));
    assert_eq!((r.viewport_menu.1, r.viewport_menu.3), (94.0, 114.0));
    // the tool strip: x 0-43; the viewport canvas x 43-1472, y 114-962 (1429 x 848)
    assert_eq!((r.tool_strip.0, r.tool_strip.2), (0.0, 43.0));
    assert_eq!(r.viewport, (43.0, 114.0, 1472.0, 962.0));
    assert_eq!(r.viewport.2 - r.viewport.0, 1429.0);
    assert_eq!(r.viewport.3 - r.viewport.1, 848.0);
    // the object palette x 1473-1516 (43 wide), the managers x 1517-1919 (402 wide)
    assert_eq!((r.object_palette.0, r.object_palette.2), (1473.0, 1516.0));
    assert_eq!((r.managers.0, r.managers.2), (1517.0, 1919.0));
    // the timeline y 963-1055 under the viewport, the status bar y 1055-1079
    assert_eq!((r.timeline.1, r.timeline.3), (963.0, 1055.0));
    assert_eq!((r.timeline.0, r.timeline.2), (43.0, 1473.0));
    assert_eq!((r.status_bar.1, r.status_bar.3), (1055.0, 1079.0));
    // the split between Objects and Attributes sits near y 500 (47 % of the column's height
    // measured from the window top to the status bar: 0.47 * 1055 = 496)
    assert!((WINDOW.manager_split * r.status_bar.1 - 500.0).abs() < 10.0);
}

#[test]
fn the_default_layout_sizes_and_button_metrics_are_the_observed_ones() {
    let s = Sizes::default();
    assert_eq!(s.right_width, 402.0);
    assert_eq!(s.right_top_fraction, 0.47);
    assert_eq!(s.bottom_height, 92.0);
    assert_eq!(s.left_width, WINDOW.tool_strip);
    assert_eq!(Layout::standard().sizes, s);
    // command bar buttons about 27 x 26, palette buttons 34 x 34
    assert_eq!(BUTTONS.command, [27.0, 26.0]);
    assert_eq!(BUTTONS.palette, [34.0, 34.0]);
}

#[test]
fn bars_and_buttons_have_the_observed_heights_in_the_window() {
    let h = harness();
    // every command bar and palette button is at least as large as the metrics say
    let undo = rect(&h, "Undo (toolbar)");
    assert!(undo.width() >= BUTTONS.command[0] && undo.height() >= BUTTONS.command[1]);
    let extrude = rect(&h, "Extrude (tool rail)");
    assert!(extrude.width() >= BUTTONS.palette[0] && extrude.height() >= BUTTONS.palette[1]);
    // the status bar is 24 high: its text sits within the last 24 points of the window
    let text = help_text(GizmoMode::Translate, ModeArg::Model);
    let status = rect(&h, &text);
    assert!(status.center().y > 800.0 - WINDOW.status_bar);
    // the managers column is 402 wide (the tab header sits one frame margin inside it)
    let objects = rect(&h, "Objects");
    assert!(
        (objects.min.x - (1280.0 - WINDOW.managers)).abs() <= 12.0,
        "{objects:?}"
    );
}

#[test]
fn the_status_bar_shows_the_active_tools_help() {
    let move_help = help_text(GizmoMode::Translate, ModeArg::Model);
    assert!(move_help.starts_with("Move:"), "{move_help}");
    assert!(help_text(GizmoMode::Rotate, ModeArg::Points).starts_with("Rotate:"));
    assert!(help_text(GizmoMode::Scale, ModeArg::Edges).starts_with("Scale:"));
    assert!(help_text(GizmoMode::Translate, ModeArg::Polys).contains("polygons"));
    let mut h = harness();
    h.get_by_label(&move_help);
    // choosing the rotate tool changes the line; an error replaces it until the next success
    forge_ui::commands::execute(
        &mut h.state_mut().state,
        "view.gizmo_mode",
        serde_json::json!({"mode": "rotate"}),
    )
    .unwrap();
    h.run();
    h.get_by_label(&help_text(GizmoMode::Rotate, ModeArg::Model));
}

#[test]
fn the_timeline_ruler_labels_every_five_frames_at_the_reference_width() {
    use forge_ui::timeline::ruler_step;
    // the strip under the viewport is about 1413 points wide (1429 minus its 8 point margins)
    // for the empty document's 0 - 90 project: labels every 5 frames (observed, G10 A7)
    assert_eq!(ruler_step(90.0, 1429.0 - 16.0), 5);
    // narrower strips label less often, wider ones more often
    assert_eq!(ruler_step(90.0, 600.0), 10);
    assert_eq!(ruler_step(90.0, 2000.0), 2);
    assert_eq!(ruler_step(10.0, 1000.0), 1);
    // labels are never closer than 40 points
    for width in [200.0_f32, 700.0, 1413.0, 2500.0] {
        let step = ruler_step(90.0, width);
        assert!(step as f32 / 90.0 * width >= 40.0);
    }
}

#[test]
fn dragging_a_panel_tab_to_the_viewport_edge_creates_a_side_dock() {
    use egui::{Event, Modifiers, PointerButton};

    let mut h = harness();
    h.run();
    let start = rect(&h, "Attributes").center();
    let view = h.state().view_rect();
    let end = view.left_center() + egui::vec2(14.0, 0.0);
    h.hover_at(start);
    h.run();
    h.event(Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run();
    for i in 1..=5 {
        h.hover_at(start.lerp(end, i as f32 / 5.0));
        h.run();
    }
    h.event(Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    assert_eq!(
        h.state().state.dock.area_of(Panel::Attributes),
        Some(Area::Left)
    );
}

#[test]
fn object_shortcuts_duplicate_beside_source_and_paste_under_selected_parent() {
    use egui::{Key, Modifiers};
    use forge_core::NodeId;
    use forge_ui::commands::execute;
    use serde_json::json;

    let mut h = harness();
    let add = |h: &mut Harness<'_, Editor>, ty| {
        NodeId(
            execute(
                &mut h.state_mut().state,
                "object.create",
                json!({"type": ty}),
            )
            .unwrap()["node"]
                .as_u64()
                .unwrap(),
        )
    };
    let parent = add(&mut h, "forge.null");
    let source = add(&mut h, "forge.cube");
    execute(
        &mut h.state_mut().state,
        "object.reparent",
        json!({"node": source.0, "parent": parent.0}),
    )
    .unwrap();
    h.key_press_modifiers(Modifiers::COMMAND, Key::D);
    h.run();
    let duplicate = h.state().state.selection.active.unwrap();
    assert_ne!(source, duplicate);
    assert_eq!(
        h.state().state.doc.scene().node(parent).unwrap().children,
        [source, duplicate]
    );

    h.event(egui::Event::Copy);
    h.run();
    let target = add(&mut h, "forge.null");
    let before = h.state().state.doc.undo_depth();
    h.event(egui::Event::Paste("Forge3D scene objects".into()));
    h.run();
    let pasted = h.state().state.selection.active.unwrap();
    assert_ne!(duplicate, pasted);
    let scene = h.state().state.doc.scene();
    assert_eq!(scene.node(pasted).unwrap().parent, Some(target));
    assert_eq!(h.state().state.doc.undo_depth(), before + 1);
}

#[test]
fn a_viewport_size_handle_changes_the_primitive_in_one_undo_step() {
    use egui::{Event, Modifiers, PointerButton};
    use forge_core::{NodeId, Value};
    use forge_ui::commands::execute;
    use glam::DVec3;
    use serde_json::json;

    let mut h = harness();
    let cube = NodeId(
        execute(
            &mut h.state_mut().state,
            "object.create",
            json!({"type": "forge.cube"}),
        )
        .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    h.run();
    let rect = h.state().view_rect();
    let camera = h.state().state.camera;
    let project = |x| {
        let p = camera
            .project(
                DVec3::X * x,
                f64::from(rect.width()),
                f64::from(rect.height()),
            )
            .unwrap();
        rect.min + egui::vec2(p.x as f32, p.y as f32)
    };
    let start = project(100.0);
    let end = project(145.0);
    let before = h.state().state.doc.undo_depth();
    h.hover_at(start);
    h.run();
    h.event(Event::PointerButton {
        pos: start,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    h.run();
    for i in 1..=4 {
        h.hover_at(start.lerp(end, i as f32 / 4.0));
        h.run();
    }
    h.event(Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    h.run();
    let doc = &mut h.state_mut().state.doc;
    assert!(
        matches!(doc.scene().param(cube, "size_x"), Some(Value::Float(x)) if (*x - 290.0).abs() < 3.0)
    );
    assert_eq!(doc.undo_depth(), before + 1);
    assert!(doc.undo());
    assert_eq!(
        doc.scene().param(cube, "size_x"),
        Some(&Value::Float(200.0))
    );
}

#[test]
fn the_lights_menu_creates_an_area_light_in_one_undo_step() {
    use forge_core::Value;

    let mut h = harness();
    let before = h.state().state.doc.undo_depth();
    h.get_by_label("Lights").click();
    h.run();
    h.get_by_label("Area light").click();
    h.run();
    let state = &h.state().state;
    let id = state.selection.active.unwrap();
    assert_eq!(
        state.doc.scene().param(id, "type"),
        Some(&Value::Enum("area".into()))
    );
    assert_eq!(state.doc.undo_depth(), before + 1);
}
