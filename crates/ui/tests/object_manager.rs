//! Object manager (docs/specs/UI.md §13; brief 149): pure tree logic (drop targets,
//! ranges, filter, context items) and egui_kittest interactions. Every edit is a registry
//! command and one undo step.

mod common;

use std::collections::BTreeSet;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_command::NodeSelection;
use forge_core::visibility::{self, Vis};
use forge_core::{NodeId, Scene, Value};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::state::EditorState;
use forge_ui::tree::{
    DropTarget, DropZone, click_selection, context_menu, drop_target, drop_zone, filter_rows,
    menu_target, moved_set, tree_rows,
};
use serde_json::json;

/// Roots `A`, `B` (children `B1`, `B2`), `C`; returns `[A, B, B1, B2, C]`.
fn sample(s: &mut EditorState) -> [NodeId; 5] {
    let mk = |s: &mut EditorState, name: &str, ty: &str| {
        let id = create(s, ty);
        execute(s, "object.rename", json!({"node": id.0, "name": name})).unwrap();
        id
    };
    let a = mk(s, "A", "forge.null");
    let b = mk(s, "B", "forge.null");
    let b1 = mk(s, "B1", "test.box");
    let b2 = mk(s, "B2", "test.box");
    let c = mk(s, "C", "forge.null");
    for k in [b1, b2] {
        execute(s, "object.reparent", json!({"node": k.0, "parent": b.0})).unwrap();
    }
    s.expanded.insert(b);
    [a, b, b1, b2, c]
}

fn names(s: &EditorState) -> Vec<String> {
    tree_rows(s.doc.scene(), &s.expanded)
        .into_iter()
        .map(|r| format!("{}{}", "-".repeat(r.depth), r.name))
        .collect()
}

fn rows(s: &EditorState) -> Vec<forge_ui::tree::TreeRow> {
    tree_rows(s.doc.scene(), &s.expanded)
}

// ---- pure: drop targets -------------------------------------------------------------

#[test]
fn drop_zone_thresholds() {
    assert_eq!(drop_zone(0.0), DropZone::Before);
    assert_eq!(drop_zone(0.24), DropZone::Before);
    assert_eq!(drop_zone(0.5), DropZone::Into);
    assert_eq!(drop_zone(0.76), DropZone::After);
    assert_eq!(drop_zone(1.0), DropZone::After);
}

#[test]
fn drop_targets_for_every_zone() {
    let mut s = state();
    let [a, b, b1, b2, c] = sample(&mut s);
    let r = rows(&s);
    let sc = s.doc.scene();
    let t = |h, z, m: &[NodeId]| drop_target(sc, &r, h, z, m);
    // into: last child of the row
    assert_eq!(
        t(b, DropZone::Into, &[a]),
        Some(DropTarget {
            parent: Some(b),
            index: None
        })
    );
    // before / after among roots, counting without the moved node
    assert_eq!(
        t(a, DropZone::Before, &[c]),
        Some(DropTarget {
            parent: None,
            index: Some(0)
        })
    );
    assert_eq!(
        t(b, DropZone::Before, &[a]),
        Some(DropTarget {
            parent: None,
            index: Some(0)
        }),
        "A is removed first, so B sits at 0"
    );
    assert_eq!(
        t(c, DropZone::After, &[a]),
        Some(DropTarget {
            parent: None,
            index: Some(2)
        })
    );
    // after an expanded row with children = first child
    assert_eq!(
        t(b, DropZone::After, &[c]),
        Some(DropTarget {
            parent: Some(b),
            index: Some(0)
        })
    );
    // before / after a child
    assert_eq!(
        t(b2, DropZone::Before, &[a]),
        Some(DropTarget {
            parent: Some(b),
            index: Some(1)
        })
    );
    assert_eq!(
        t(b1, DropZone::After, &[c]),
        Some(DropTarget {
            parent: Some(b),
            index: Some(1)
        })
    );
    // a collapsed row with children: after = next sibling
    let collapsed = tree_rows(sc, &BTreeSet::new());
    assert_eq!(
        drop_target(sc, &collapsed, b, DropZone::After, &[a]),
        Some(DropTarget {
            parent: None,
            index: Some(1)
        })
    );
}

#[test]
fn invalid_drops_are_refused() {
    let mut s = state();
    let [a, b, b1, b2, c] = sample(&mut s);
    let r = rows(&s);
    let sc = s.doc.scene();
    for z in [DropZone::Before, DropZone::Into, DropZone::After] {
        assert_eq!(drop_target(sc, &r, b, z, &[b]), None, "onto itself");
        assert_eq!(drop_target(sc, &r, b1, z, &[b]), None, "into own subtree");
        assert!(drop_target(sc, &r, a, z, &[a, b1, c]).is_none());
    }
    // a child of a moved node may not be a target either way
    assert_eq!(drop_target(sc, &r, b2, DropZone::Into, &[b]), None);
    // moving a child next to its own parent is fine
    assert!(drop_target(sc, &r, b, DropZone::After, &[b1]).is_some());
    let _ = a;
}

#[test]
fn moved_set_is_selection_or_dragged_row_in_visible_order() {
    let mut s = state();
    let [a, b, b1, _, c] = sample(&mut s);
    let r = rows(&s);
    let sc = s.doc.scene();
    let sel = BTreeSet::from([c, a, b1]);
    assert_eq!(moved_set(sc, &r, &sel, c), vec![a, b1, c]);
    // dragging an unselected row drags only that row
    assert_eq!(moved_set(sc, &r, &sel, b), vec![b]);
    // nested selections: the child goes with its parent
    let sel = BTreeSet::from([b, b1]);
    assert_eq!(moved_set(sc, &r, &sel, b), vec![b]);
}

// ---- pure: selection ranges ---------------------------------------------------------

#[test]
fn click_shift_and_command_selection() {
    let mut s = state();
    let [a, b, b1, b2, c] = sample(&mut s);
    let order: Vec<NodeId> = rows(&s).iter().map(|r| r.id).collect();
    assert_eq!(order, [a, b, b1, b2, c]);
    let mut sel = NodeSelection::default();
    click_selection(&mut sel, &order, b, false, false);
    assert_eq!(sel.nodes, BTreeSet::from([b]));
    click_selection(&mut sel, &order, c, true, false);
    assert_eq!(sel.nodes, BTreeSet::from([b, b1, b2, c]), "range b..c");
    assert_eq!(sel.active, Some(c));
    // the anchor stays: shift to a re-ranges from b
    click_selection(&mut sel, &order, a, true, false);
    assert_eq!(sel.nodes, BTreeSet::from([a, b]));
    // command toggles and moves the anchor
    click_selection(&mut sel, &order, c, false, true);
    assert_eq!(sel.nodes, BTreeSet::from([a, b, c]));
    click_selection(&mut sel, &order, b, false, true);
    assert_eq!(sel.nodes, BTreeSet::from([a, c]));
    // shift without an anchor selects just the row
    let mut none = NodeSelection::default();
    click_selection(&mut none, &order, b2, true, false);
    assert_eq!(none.nodes, BTreeSet::from([b2]));
    // plain click replaces
    click_selection(&mut sel, &order, a, false, false);
    assert_eq!(sel.nodes, BTreeSet::from([a]));
}

#[test]
fn right_click_selects_an_unselected_row_only() {
    let mut s = state();
    let [a, b, _, _, c] = sample(&mut s);
    let mut sel = NodeSelection::default();
    sel.click(a);
    sel.toggle(b);
    menu_target(&mut sel, b);
    assert_eq!(sel.nodes, BTreeSet::from([a, b]), "kept");
    menu_target(&mut sel, c);
    assert_eq!(sel.nodes, BTreeSet::from([c]), "replaced");
}

// ---- pure: filter -------------------------------------------------------------------

#[test]
fn filter_matches_name_type_and_tag_with_ancestors() {
    let mut s = state();
    let [a, b, b1, b2, c] = sample(&mut s);
    execute(
        &mut s,
        "object.rename",
        json!({"node": b2.0, "name": "Wheel"}),
    )
    .unwrap();
    let sc = s.doc.scene();
    let f = filter_rows(sc, "whe");
    let ids: Vec<_> = f
        .iter()
        .map(|r| (r.id, r.depth, r.dim, r.expanded))
        .collect();
    assert_eq!(ids, [(b, 0, true, true), (b2, 1, false, false)]);
    // type label ("Box" = B1 and Wheel), tag label ("Smoothing" is on generators)
    let f = filter_rows(sc, "  BOX ");
    assert_eq!(f.iter().map(|r| r.id).collect::<Vec<_>>(), [b, b1, b2]);
    assert_eq!(
        filter_rows(sc, "smoothing")
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        [b, b1, b2],
        "tag labels match the host"
    );
    // empty query = the normal tree for the given expansion
    assert!(
        filter_rows(sc, "").is_empty(),
        "empty query is not a filter result"
    );
    assert!(filter_rows(sc, "zzz").is_empty());
    let _ = (a, c);
}

// ---- pure: context menu -------------------------------------------------------------

#[test]
fn context_menu_items_are_top_level_registry_commands() {
    let mut s = state();
    let [a, b, b1, ..] = sample(&mut s);
    let items = context_menu(s.doc.scene(), &BTreeSet::from([b1]));
    let labels: Vec<&str> = items.iter().map(|i| i.label).collect();
    assert_eq!(
        labels,
        [
            "Rename",
            "Group Objects",
            "Make Editable",
            "Hide",
            "Disable",
            "Move to Root",
            "Delete"
        ]
    );
    for i in &items {
        if let Some(c) = i.command {
            assert!(s.app.commands.get(c).is_some(), "{c} is registered");
        }
    }
    assert_eq!(items[0].command, None);
    // several selected: no Rename; a null only: no Make Editable
    let items = context_menu(s.doc.scene(), &BTreeSet::from([a, b]));
    let labels: Vec<&str> = items.iter().map(|i| i.label).collect();
    assert_eq!(
        labels,
        ["Group Objects", "Hide", "Disable", "Move to Root", "Delete"]
    );
    assert_eq!(items[0].args["nodes"], json!([a.0, b.0]));
}

#[test]
fn context_menu_toggles_labels_with_state() {
    let mut s = state();
    let [a, ..] = sample(&mut s);
    execute(
        &mut s,
        "object.visibility",
        json!({"nodes": [a.0], "editor": "off", "render": "off"}),
    )
    .unwrap();
    execute(
        &mut s,
        "object.enable",
        json!({"nodes": [a.0], "enabled": false}),
    )
    .unwrap();
    let items = context_menu(s.doc.scene(), &BTreeSet::from([a]));
    let labels: Vec<&str> = items.iter().map(|i| i.label).collect();
    assert!(labels.contains(&"Show") && labels.contains(&"Enable"));
    let show = items.iter().find(|i| i.label == "Show").unwrap();
    assert_eq!(show.args["editor"], "default");
}

// ---- core: visibility ---------------------------------------------------------------

fn vis_of(sc: &Scene, id: NodeId) -> (bool, bool, bool) {
    (
        visibility::editor_visible(sc, id),
        visibility::render_visible(sc, id),
        visibility::enabled(sc, id),
    )
}

#[test]
fn visibility_resolves_down_the_hierarchy() {
    let mut s = state();
    let [_, b, b1, b2, c] = sample(&mut s);
    assert_eq!(vis_of(s.doc.scene(), b1), (true, true, true));
    execute(
        &mut s,
        "object.visibility",
        json!({"nodes": [b.0], "editor": "off"}),
    )
    .unwrap();
    assert_eq!(
        vis_of(s.doc.scene(), b1),
        (false, true, true),
        "inherits off"
    );
    execute(
        &mut s,
        "object.visibility",
        json!({"nodes": [b2.0], "editor": "on"}),
    )
    .unwrap();
    assert!(vis_of(s.doc.scene(), b2).0, "forced on under an off parent");
    assert!(!vis_of(s.doc.scene(), b1).0);
    execute(
        &mut s,
        "object.enable",
        json!({"nodes": [b.0], "enabled": false}),
    )
    .unwrap();
    assert_eq!(
        vis_of(s.doc.scene(), b2),
        (false, false, false),
        "disabled ancestor"
    );
    assert_eq!(visibility::own(s.doc.scene(), c).0, Vis::Default);
    assert_eq!(Vis::Default.next(), Vis::Off);
    assert_eq!(Vis::Off.next(), Vis::On);
    assert_eq!(Vis::On.next(), Vis::Default);
}

#[test]
fn viewport_meshes_skip_hidden_and_disabled_objects() {
    let mut s = state();
    let cube = create(&mut s, "forge.cube");
    let sel = BTreeSet::new();
    let n = forge_ui::viewport_panel::scene_meshes(s.doc.scene(), &sel).len();
    assert_eq!(n, 1);
    execute(
        &mut s,
        "object.visibility",
        json!({"nodes": [cube.0], "editor": "off"}),
    )
    .unwrap();
    assert_eq!(
        forge_ui::viewport_panel::scene_meshes(s.doc.scene(), &sel).len(),
        0
    );
    execute(
        &mut s,
        "object.visibility",
        json!({"nodes": [cube.0], "editor": "default"}),
    )
    .unwrap();
    execute(
        &mut s,
        "object.enable",
        json!({"nodes": [cube.0], "enabled": false}),
    )
    .unwrap();
    assert_eq!(
        forge_ui::viewport_panel::scene_meshes(s.doc.scene(), &sel).len(),
        0
    );
    execute(&mut s, "edit.undo", json!({})).unwrap();
    assert_eq!(
        forge_ui::viewport_panel::scene_meshes(s.doc.scene(), &sel).len(),
        1
    );
}

// ---- commands -----------------------------------------------------------------------

#[test]
fn move_command_reorders_reparents_and_undoes_in_one_step() {
    let mut s = state();
    let [a, b, b1, b2, c] = sample(&mut s);
    let before = s.doc.history().undo_labels().len();
    execute(
        &mut s,
        "object.move",
        json!({"nodes": [a.0, c.0], "parent": b.0, "index": 1}),
    )
    .unwrap();
    assert_eq!(s.doc.history().undo_labels().len(), before + 1);
    let kids = s.doc.scene().node(b).unwrap().children.clone();
    assert_eq!(kids, [b1, a, c, b2]);
    execute(&mut s, "edit.undo", json!({})).unwrap();
    assert_eq!(s.doc.scene().roots(), [a, b, c]);
    assert_eq!(s.doc.scene().node(b).unwrap().children, [b1, b2]);
    // reorder among roots: C to the front (index counts without C)
    execute(&mut s, "object.move", json!({"nodes": [c.0], "index": 0})).unwrap();
    assert_eq!(s.doc.scene().roots(), [c, a, b]);
    // a cycle fails and changes nothing
    let rev = s.doc.scene().revision();
    assert!(
        execute(
            &mut s,
            "object.move",
            json!({"nodes": [b.0], "parent": b1.0})
        )
        .is_err()
    );
    assert_eq!(s.doc.scene().revision(), rev);
    // nested nodes move with their parent only
    execute(
        &mut s,
        "object.move",
        json!({"nodes": [b.0, b1.0], "parent": a.0}),
    )
    .unwrap();
    assert_eq!(s.doc.scene().node(a).unwrap().children, [b]);
    assert_eq!(s.doc.scene().node(b).unwrap().children, [b1, b2]);
}

#[test]
fn group_command_wraps_nodes_in_a_null_at_the_first_position() {
    let mut s = state();
    let [a, b, _, _, c] = sample(&mut s);
    let out = execute(
        &mut s,
        "object.group",
        json!({"nodes": [b.0, c.0], "name": "G"}),
    )
    .unwrap();
    let g = NodeId(out["node"].as_u64().unwrap());
    let sc = s.doc.scene();
    assert_eq!(sc.roots(), [a, g]);
    assert_eq!(sc.node(g).unwrap().children, [b, c]);
    assert_eq!(sc.node(g).unwrap().name, "G");
    assert_eq!(sc.node(g).unwrap().type_id, "forge.null");
    assert_eq!(s.selection.nodes, BTreeSet::from([g]));
    execute(&mut s, "edit.undo", json!({})).unwrap();
    assert_eq!(s.doc.scene().roots(), [a, b, c]);
    assert!(!s.doc.scene().contains(g));
}

#[test]
fn visibility_and_enable_commands_validate_and_undo() {
    let mut s = state();
    let [a, ..] = sample(&mut s);
    assert!(execute(&mut s, "object.visibility", json!({"nodes": [a.0]})).is_err());
    assert!(
        execute(
            &mut s,
            "object.visibility",
            json!({"nodes": [a.0], "editor": "blue"})
        )
        .is_err()
    );
    execute(
        &mut s,
        "object.visibility",
        json!({"nodes": [a.0], "render": "off"}),
    )
    .unwrap();
    assert_eq!(
        s.doc.scene().param(a, "object.render_vis"),
        Some(&Value::Enum("off".into()))
    );
    assert_eq!(
        s.doc.scene().param(a, "object.editor_vis"),
        Some(&Value::Enum("default".into()))
    );
    execute(&mut s, "edit.undo", json!({})).unwrap();
    assert_eq!(visibility::own(s.doc.scene(), a).1, Vis::Default);
}

// ---- egui_kittest -------------------------------------------------------------------

fn harness() -> Harness<'static, Editor> {
    let mut st = state();
    sample(&mut st);
    st.selection.clear();
    let editor = Editor::new(st);
    let mut h = Harness::builder()
        .with_size([1280.0, 1800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    h
}

fn st<'a>(h: &'a Harness<'_, Editor>) -> &'a EditorState {
    &h.state().state
}

fn node_named(h: &Harness<'_, Editor>, name: &str) -> NodeId {
    let sc = st(h).doc.scene();
    sc.roots()
        .iter()
        .flat_map(|r| std::iter::once(*r).chain(sc.descendants(*r)))
        .find(|id| sc.node(*id).unwrap().name == name)
        .unwrap()
}

fn double_click(h: &mut Harness<'_, Editor>, label: &str) {
    h.ctx
        .options_mut(|o| o.input_options.max_double_click_delay = 5.0);
    h.get_by_label(label).click();
    h.step();
    h.get_by_label(label).click();
    h.step();
    h.run();
}

fn row_names(h: &Harness<'_, Editor>) -> Vec<String> {
    names(st(h))
}

/// Drags the row `from` to a point `frac` (0 top .. 1 bottom) down the row `to`.
fn drag_row(h: &mut Harness<'_, Editor>, from: &str, to: &str, frac: f32) {
    let src = h.get_by_label(from).rect().center();
    let r = h.get_by_label(to).rect();
    let dst = egui::pos2(r.center().x, r.top() + r.height() * frac);
    h.hover_at(src);
    h.run();
    h.drag_at(src);
    h.run();
    for k in 1..=4 {
        h.hover_at(src.lerp(dst, k as f32 / 4.0));
        h.run();
    }
    h.drop_at(dst);
    h.run();
}

#[test]
fn drag_onto_a_row_reparents_and_undo_restores() {
    let mut h = harness();
    drag_row(&mut h, "C", "A", 0.5);
    let (a, c) = (node_named(&h, "A"), node_named(&h, "C"));
    assert_eq!(st(&h).doc.scene().node(c).unwrap().parent, Some(a));
    assert_eq!(st(&h).doc.history().undo_labels().last(), Some(&"Move"));
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
    assert_eq!(st(&h).doc.scene().node(c).unwrap().parent, None);
    assert_eq!(row_names(&h), ["A", "B", "-B1", "-B2", "C"]);
}

#[test]
fn drag_to_a_row_edge_reorders() {
    let mut h = harness();
    drag_row(&mut h, "C", "A", 0.05);
    assert_eq!(row_names(&h), ["C", "A", "B", "-B1", "-B2"]);
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
    assert_eq!(row_names(&h), ["A", "B", "-B1", "-B2", "C"]);
    // below the last row's bottom edge: after C = root end
    drag_row(&mut h, "A", "C", 0.95);
    assert_eq!(row_names(&h), ["B", "-B1", "-B2", "C", "A"]);
}

#[test]
fn dragging_a_selected_row_moves_the_whole_selection() {
    let mut h = harness();
    h.get_by_label("A").click();
    h.run();
    h.get_by_label("C")
        .click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    drag_row(&mut h, "C", "B2", 0.5);
    let b2 = node_named(&h, "B2");
    let kids = &st(&h).doc.scene().node(b2).unwrap().children;
    assert_eq!(kids, &[node_named(&h, "A"), node_named(&h, "C")]);
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    assert_eq!(st(&h).doc.scene().roots().len(), 3, "one undo step");
}

#[test]
fn click_shift_and_command_in_the_panel() {
    let mut h = harness();
    h.get_by_label("A").click();
    h.run();
    h.get_by_label("C").click_modifiers(egui::Modifiers::SHIFT);
    h.run();
    assert_eq!(st(&h).selection.nodes.len(), 5, "A, B, B1, B2, C");
}

#[test]
fn dots_and_tick_run_commands_and_undo() {
    let mut h = harness();
    let a = node_named(&h, "A");
    let param = |h: &Harness<'_, Editor>, p: &str| st(h).doc.scene().param(a, p).cloned();
    h.get_by_label("Editor visibility: A").click();
    h.run();
    assert_eq!(
        param(&h, "object.editor_vis"),
        Some(Value::Enum("off".into()))
    );
    h.get_by_label("Editor visibility: A").click();
    h.run();
    assert_eq!(
        param(&h, "object.editor_vis"),
        Some(Value::Enum("on".into()))
    );
    h.get_by_label("Render visibility: A").click();
    h.run();
    assert_eq!(
        param(&h, "object.render_vis"),
        Some(Value::Enum("off".into()))
    );
    h.get_by_label("Enabled: A").click();
    h.run();
    assert_eq!(param(&h, "object.enabled"), Some(Value::Bool(false)));
    for (p, want) in [
        ("object.enabled", Value::Bool(true)),
        ("object.render_vis", Value::Enum("default".into())),
        ("object.editor_vis", Value::Enum("off".into())),
    ] {
        execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
        h.run();
        assert_eq!(param(&h, p), Some(want), "undo of {p}");
    }
}

#[test]
fn a_dot_on_a_selected_row_changes_the_whole_selection() {
    let mut h = harness();
    h.get_by_label("A").click();
    h.run();
    h.get_by_label("C")
        .click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    h.get_by_label("Enabled: C").click();
    h.run();
    for n in ["A", "C"] {
        assert!(
            !visibility::enabled(st(&h).doc.scene(), node_named(&h, n)),
            "{n}"
        );
    }
    assert!(visibility::enabled(st(&h).doc.scene(), node_named(&h, "B")));
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    assert!(visibility::enabled(st(&h).doc.scene(), node_named(&h, "A")));
}

#[test]
fn double_click_renames_in_place_with_undo() {
    let mut h = harness();
    let a = node_named(&h, "A");
    double_click(&mut h, "A");
    h.get_by_label("Rename").type_text("xyz");
    h.run();
    h.key_press(egui::Key::Enter);
    h.run();
    let name = st(&h).doc.scene().node(a).unwrap().name.clone();
    assert!(name.contains("xyz") && name.contains('A'), "{name}");
    assert_eq!(st(&h).doc.history().undo_labels().last(), Some(&"Rename"));
    assert!(h.query_by_label(&name).is_some(), "row shows the new name");
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    assert_eq!(st(&h).doc.scene().node(a).unwrap().name, "A");
}

#[test]
fn escape_cancels_a_rename() {
    let mut h = harness();
    let a = node_named(&h, "A");
    double_click(&mut h, "A");
    h.get_by_label("Rename").type_text("zed");
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(st(&h).doc.scene().node(a).unwrap().name, "A");
    assert!(h.query_by_label("A").is_some(), "back to a plain row");
}

#[test]
fn search_filters_rows_and_clears() {
    let mut h = harness();
    let search = h.get_by_label("Find");
    search.focus();
    h.run();
    h.get_by_label("Find").set_text_value_accesskit("B2");
    h.run();
    assert!(h.query_by_label("B2").is_some());
    assert!(
        h.query_by_label("C").is_none(),
        "non-matching rows are hidden"
    );
    assert!(h.query_by_label("B1").is_none());
    assert!(h.query_by_label("B").is_some(), "ancestor stays");
    h.get_by_label("Find").set_text_value_accesskit("");
    h.run();
    assert!(h.query_by_label("C").is_some());
}

#[test]
fn context_menu_group_and_delete_run_commands_with_undo() {
    let mut h = harness();
    h.get_by_label("A").click();
    h.run();
    h.get_by_label("C")
        .click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    h.get_by_label("C").click_secondary();
    h.run();
    h.get_by_label("Group Objects").click();
    h.run();
    let g = *st(&h).selection.nodes.iter().next().unwrap();
    assert_eq!(st(&h).doc.scene().node(g).unwrap().children.len(), 2);
    assert_eq!(st(&h).doc.scene().roots().len(), 2);
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
    assert_eq!(st(&h).doc.scene().roots().len(), 3);

    h.get_by_label("B").click_secondary();
    h.run();
    h.get_by_label("Delete").click();
    h.run();
    assert_eq!(row_names(&h), ["A", "C"]);
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
    assert_eq!(st(&h).doc.scene().roots().len(), 3);
}

#[test]
fn context_menu_hide_disable_and_move_to_root() {
    let mut h = harness();
    h.get_by_label("B1").click_secondary();
    h.run();
    h.get_by_label("Hide").click();
    h.run();
    let b1 = node_named(&h, "B1");
    assert!(!visibility::editor_visible(st(&h).doc.scene(), b1));
    assert!(!visibility::render_visible(st(&h).doc.scene(), b1));
    h.get_by_label("B1").click_secondary();
    h.run();
    h.get_by_label("Move to Root").click();
    h.run();
    assert_eq!(st(&h).doc.scene().node(b1).unwrap().parent, None);
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    assert_eq!(
        st(&h).doc.scene().node(b1).unwrap().parent,
        Some(node_named(&h, "B"))
    );
}

#[test]
fn dragging_a_row_onto_an_attribute_link_field_sets_the_link() {
    let mut h = harness();
    let b1 = node_named(&h, "B1");
    h.get_by_label("B1").click();
    h.run();
    // the Target link row of the selected Box is in the attribute manager
    let target = node_named(&h, "A");
    let src = h.get_by_label("A").rect().center();
    let dst = h
        .get_by_role_and_label(egui::accesskit::Role::ComboBox, "Target")
        .rect()
        .center();
    h.hover_at(src);
    h.run();
    h.drag_at(src);
    h.run();
    for k in 1..=4 {
        h.hover_at(src.lerp(dst, k as f32 / 4.0));
        h.run();
    }
    // release without removing the cursor, so the target sees the pointer on that frame
    h.event(egui::Event::PointerButton {
        pos: dst,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    h.run();
    assert_eq!(
        st(&h).doc.scene().param(b1, "target"),
        Some(&Value::Link(Some(target)))
    );
}
