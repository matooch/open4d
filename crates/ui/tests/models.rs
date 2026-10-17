//! docs/specs/UI.md §1–3, §6–8: UI-free models.

mod common;

use std::collections::BTreeSet;

use common::{create, state};
use forge_core::NodeId;
use forge_ui::commands::{Command, execute};
use forge_ui::dock::{Area, Dock, Panel};
use forge_ui::state::NodeSelection;
use forge_ui::theme::{Theme, contrast};
use forge_ui::timeline::Timeline;
use forge_ui::tree::tree_rows;
use forge_ui::viewport_panel::{NavAction, NavInput, nav_action};

#[test]
fn selection_click_toggle_range() {
    let ids: Vec<NodeId> = (1..=5).map(NodeId).collect();
    let mut s = NodeSelection::default();
    s.click(ids[1]);
    assert_eq!(s.nodes, BTreeSet::from([ids[1]]));
    s.toggle(ids[3]);
    assert_eq!(s.nodes, BTreeSet::from([ids[1], ids[3]]));
    assert_eq!(s.active, Some(ids[3]));
    s.toggle(ids[3]);
    assert_eq!(s.nodes, BTreeSet::from([ids[1]]));
    assert_eq!(s.active, Some(ids[1]));
    s.toggle(ids[1]);
    assert!(s.nodes.is_empty());
    assert_eq!(s.active, None);

    // range runs over visible row order, not id order
    let rows = [ids[4], ids[0], ids[2], ids[1], ids[3]];
    s.click(ids[0]);
    s.range(ids[1], &rows);
    assert_eq!(s.nodes, BTreeSet::from([ids[0], ids[2], ids[1]]));
    assert_eq!(s.active, Some(ids[1]));
    let mut fresh = NodeSelection::default();
    fresh.range(ids[2], &rows);
    assert_eq!(fresh.nodes, BTreeSet::from([ids[2]]));
}

#[test]
fn selection_drops_deleted_nodes() {
    let mut st = state();
    let a = create(&mut st, "forge.null");
    let b = create(&mut st, "forge.null");
    st.selection.click(a);
    st.selection.toggle(b);
    execute(&mut st, Command::DeleteSelected).unwrap();
    assert!(st.selection.nodes.is_empty());
    execute(&mut st, Command::Undo).unwrap();
    st.selection.click(b);
    execute(&mut st, Command::Undo).unwrap(); // removes b's creation
    assert!(st.selection.nodes.is_empty() && st.selection.active.is_none());
}

#[test]
fn default_dock_layout_is_ours() {
    let d = Dock::default();
    assert_eq!(d.tabs(Area::Center), [Panel::Viewport]);
    assert_eq!(d.tabs(Area::RightTop), [Panel::ObjectTree]);
    assert_eq!(d.tabs(Area::RightBottom), [Panel::Attributes]);
    assert_eq!(d.tabs(Area::Bottom), [Panel::Timeline]);
    assert!(d.tabs(Area::Left).is_empty());
    assert_eq!(d.area_of(Panel::Attributes), Some(Area::RightBottom));
}

#[test]
fn moving_and_closing_panels() {
    let mut d = Dock::default();
    d.move_panel(Panel::Attributes, Area::RightTop);
    assert_eq!(
        d.tabs(Area::RightTop),
        [Panel::ObjectTree, Panel::Attributes]
    );
    assert_eq!(d.active(Area::RightTop), Some(Panel::Attributes));
    assert!(d.tabs(Area::RightBottom).is_empty());
    assert_eq!(d.active(Area::RightBottom), None);

    d.set_active(Area::RightTop, 0);
    d.move_panel(Panel::Attributes, Area::Left);
    assert_eq!(
        d.active(Area::RightTop),
        Some(Panel::ObjectTree),
        "stays on the same panel"
    );
    d.set_active(Area::RightTop, 9);
    assert_eq!(d.active(Area::RightTop), Some(Panel::ObjectTree));

    d.move_panel(Panel::Attributes, Area::Left); // no-op
    assert_eq!(d.tabs(Area::Left), [Panel::Attributes]);

    d.close(Panel::Timeline);
    assert_eq!(d.area_of(Panel::Timeline), None);
    d.open(Panel::Timeline);
    assert_eq!(d.area_of(Panel::Timeline), Some(Area::Bottom));
    d.open(Panel::Attributes); // already open: stays where it is
    assert_eq!(d.area_of(Panel::Attributes), Some(Area::Left));
    // each panel appears once
    let all: Vec<Panel> = Area::ALL.iter().flat_map(|a| d.tabs(*a).to_vec()).collect();
    assert_eq!(all.len(), 4);
}

#[test]
fn closing_the_active_middle_tab_clamps() {
    let mut d = Dock::default();
    d.move_panel(Panel::ObjectTree, Area::Bottom);
    d.move_panel(Panel::Attributes, Area::Bottom); // [Timeline, ObjectTree, Attributes], active 2
    d.close(Panel::Attributes);
    assert_eq!(d.active(Area::Bottom), Some(Panel::ObjectTree));
}

#[test]
fn tree_rows_follow_hierarchy_and_expansion() {
    let mut st = state();
    let a = create(&mut st, "forge.null");
    let b = create(&mut st, "test.box");
    execute(&mut st, Command::Reparent(b, Some(a))).unwrap();
    st.selection.click(b);
    execute(&mut st, Command::Create("test.sel".into())).unwrap(); // tag on b
    let c = create(&mut st, "test.bend");

    let collapsed = tree_rows(st.doc.scene(), &BTreeSet::new());
    let ids: Vec<NodeId> = collapsed.iter().map(|r| r.id).collect();
    assert_eq!(ids, [a, c]);
    assert!(collapsed[0].has_children && !collapsed[0].expanded);

    let rows = tree_rows(st.doc.scene(), &BTreeSet::from([a]));
    let ids: Vec<NodeId> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, [a, b, c]);
    assert_eq!(rows[1].depth, 1);
    assert_eq!(rows[1].type_label, "Box");
    assert_eq!(
        rows[1].tags.len(),
        2,
        "smoothing tag from creation + Selection"
    );
    assert_eq!(rows[1].tags[0].1, "Smoothing");
    assert_eq!(rows[1].tags[1].1, "Selection");
    assert!(!rows[1].missing_type);
}

#[test]
fn timeline_placeholder() {
    let mut t = Timeline::default();
    assert_eq!(
        (t.start, t.end, t.current, t.fps, t.playing),
        (0, 90, 0, 30, false)
    );
    t.tick();
    assert_eq!(t.current, 0, "paused");
    t.playing = true;
    t.set_current(90);
    t.tick();
    assert_eq!(t.current, 0, "wraps");
    t.set_current(500);
    assert_eq!(t.current, 90);
    t.set_range(40, 10);
    assert_eq!((t.start, t.end, t.current), (10, 40, 40));
}

fn nav(
    primary: bool,
    middle: bool,
    secondary: bool,
    alt: bool,
    drag: [f32; 2],
    scroll: f32,
) -> NavAction {
    nav_action(&NavInput {
        primary,
        middle,
        secondary,
        alt,
        drag,
        scroll,
    })
}

#[test]
fn navigation_bindings() {
    assert_eq!(
        nav(true, false, false, true, [10.0, -4.0], 0.0),
        NavAction::Orbit(-0.1, -0.04)
    );
    assert_eq!(
        nav(false, true, false, false, [3.0, 2.0], 0.0),
        NavAction::Pan(3.0, 2.0)
    );
    assert_eq!(
        nav(false, true, false, true, [3.0, 2.0], 0.0),
        NavAction::Pan(3.0, 2.0)
    );
    let NavAction::Dolly(f) = nav(false, false, true, true, [100.0, 0.0], 0.0) else {
        panic!()
    };
    assert!((f - (-1.0f64).exp()).abs() < 1e-12);
    let NavAction::Dolly(z) = nav(true, false, false, true, [5.0, 5.0], 120.0) else {
        panic!("wheel wins")
    };
    assert!((z - 0.999f64.powf(120.0)).abs() < 1e-12 && z < 1.0);
    assert_eq!(
        nav(true, false, false, false, [5.0, 5.0], 0.0),
        NavAction::None
    );
    assert_eq!(
        nav(false, false, false, true, [0.0, 0.0], 0.0),
        NavAction::None
    );
}

#[test]
fn theme_meets_contrast_targets() {
    let t = Theme::default();
    assert!(contrast(t.text, t.panel) >= 7.0);
    assert!(contrast(t.text, t.window) >= 7.0);
    assert!(contrast(t.weak_text, t.panel) >= 4.5);
    assert!(contrast(t.accent_text, t.accent) >= 4.5);
    assert!((contrast(egui::Color32::WHITE, egui::Color32::BLACK) - 21.0).abs() < 1e-6);
}
