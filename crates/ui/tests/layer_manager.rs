//! Layer manager (docs/specs/UI.md §22; brief 158): pure helpers (toggle/merge/assign
//! arguments, the Layer sub menu, row colours, manager filtering) and egui_kittest
//! interactions. Every edit is a `layer.*` registry command and one undo step.

mod common;

use std::collections::BTreeSet;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::layers::{self, LAYER};
use forge_core::visibility;
use forge_core::{Layer, NodeId, Value};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::dock::{Area, Dock, Panel};
use forge_ui::layer_manager::{Toggle, assign_command, merge_command, toggle_command};
use forge_ui::manipulate::pick_node;
use forge_ui::state::EditorState;
use forge_ui::tree::{filter_rows, layer_menu, tree_rows};
use forge_viewport::camera::Camera;
use serde_json::{Value as Json, json};

fn rename(s: &mut EditorState, id: NodeId, name: &str) {
    execute(s, "object.rename", json!({"node": id.0, "name": name})).unwrap();
}

/// Roots Alpha, Bravo (child "Bravo child"), Charlie; returns `[a, b, b1, c]`. Nothing is selected.
fn sample(s: &mut EditorState) -> [NodeId; 4] {
    let a = create(s, "forge.null");
    rename(s, a, "Alpha");
    let b = create(s, "forge.null");
    rename(s, b, "Bravo");
    let b1 = create(s, "forge.cube");
    rename(s, b1, "Bravo child");
    execute(s, "object.reparent", json!({"node": b1.0, "parent": b.0})).unwrap();
    let c = create(s, "forge.sphere");
    rename(s, c, "Charlie");
    s.expanded.insert(b);
    s.selection.clear();
    [a, b, b1, c]
}

fn new_layer(s: &mut EditorState, name: &str) -> NodeId {
    let r = execute(s, "layer.create", json!({"name": name})).unwrap();
    NodeId(r["layer"].as_u64().unwrap())
}

fn assign(s: &mut EditorState, layer: Option<NodeId>, nodes: &[NodeId]) {
    let nodes: Vec<u64> = nodes.iter().map(|n| n.0).collect();
    execute(
        s,
        "layer.assign",
        json!({"layer": layer.map(|l| l.0), "nodes": nodes}),
    )
    .unwrap();
}

fn layer_of(s: &EditorState, id: NodeId) -> Layer {
    s.doc.scene().layers().get(id).unwrap().clone()
}

// ---- pure ---------------------------------------------------------------------------

#[test]
fn toggle_commands_flip_the_clicked_value_for_the_row_or_the_selection() {
    let mut s = state();
    let (l1, l2, l3) = (
        new_layer(&mut s, "One"),
        new_layer(&mut s, "Two"),
        new_layer(&mut s, "Three"),
    );
    execute(&mut s, "layer.set", json!({"layer": l1.0, "view": false})).unwrap();
    let sel: BTreeSet<NodeId> = BTreeSet::from([l1, l2]);
    // a row outside the selection acts alone
    let (id, args) = toggle_command(&sel, &layer_of(&s, l3), Toggle::View);
    assert_eq!(
        (id, args),
        ("layer.set", json!({"layers": [l3.0], "view": false}))
    );
    // a selected row acts on every selected row, with the opposite of its own value
    let (_, args) = toggle_command(&sel, &layer_of(&s, l1), Toggle::View);
    assert_eq!(args, json!({"layers": [l1.0, l2.0], "view": true}));
    let (_, args) = toggle_command(&sel, &layer_of(&s, l2), Toggle::Locked);
    assert_eq!(args, json!({"layers": [l1.0, l2.0], "locked": true}));
    for t in Toggle::ALL {
        assert!(!t.key().is_empty() && t.letter().len() == 1);
        assert_eq!(t.label("X"), format!("{}: X", t.name()));
    }
    assert_eq!(
        Toggle::ALL.map(Toggle::letter),
        ["S", "V", "R", "M", "L", "A", "G", "D", "E"]
    );
}

#[test]
fn merge_and_assign_arguments() {
    let mut s = state();
    let [a, b, ..] = sample(&mut s);
    let (l1, l2, l3) = (
        new_layer(&mut s, "One"),
        new_layer(&mut s, "Two"),
        new_layer(&mut s, "Three"),
    );
    let sc = s.doc.scene();
    assert_eq!(merge_command(sc, &BTreeSet::from([l1])), None);
    assert_eq!(merge_command(sc, &BTreeSet::new()), None);
    let (id, args) = merge_command(sc, &BTreeSet::from([l3, l1])).unwrap();
    assert_eq!(id, "layer.merge");
    assert_eq!(
        args,
        json!({"layers": [l1.0, l3.0], "into": l1.0}),
        "into the first in table order"
    );
    let objects = BTreeSet::from([a, b]);
    assert_eq!(
        assign_command(&BTreeSet::from([l2]), &objects),
        Some(("layer.assign", json!({"layer": l2.0, "nodes": [a.0, b.0]})))
    );
    assert_eq!(assign_command(&BTreeSet::from([l1, l2]), &objects), None);
    assert_eq!(assign_command(&BTreeSet::new(), &objects), None);
    assert_eq!(
        assign_command(&BTreeSet::from([l1]), &BTreeSet::new()),
        None
    );
}

#[test]
fn the_layer_sub_menu_lists_none_every_layer_and_new() {
    let mut s = state();
    let [a, b, ..] = sample(&mut s);
    assert!(layer_menu(s.doc.scene(), &BTreeSet::new()).is_empty());
    let (l1, l2) = (new_layer(&mut s, "One"), new_layer(&mut s, "Two"));
    assign(&mut s, Some(l2), &[a, b]);
    let items = layer_menu(s.doc.scene(), &BTreeSet::from([a, b]));
    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(
        labels,
        ["No layer", "One", "Two", "New layer from selection"]
    );
    let checked: Vec<bool> = items.iter().map(|i| i.checked).collect();
    assert_eq!(checked, [false, false, true, false]);
    assert_eq!(items[1].args, json!({"layer": l1.0, "nodes": [a.0, b.0]}));
    assert_eq!(items[0].args["layer"], Json::Null);
    assert_eq!(items[3].command, "layer.create");
    for i in &items {
        assert!(s.app.commands.get(i.command).is_some());
    }
    // a mixed selection checks nothing
    let items = layer_menu(
        s.doc.scene(),
        &BTreeSet::from([a, s.doc.scene().roots()[2]]),
    );
    assert!(items.iter().all(|i| !i.checked || i.label == "No layer"));
}

#[test]
fn rows_carry_the_layer_and_manager_off_omits_them() {
    let mut s = state();
    let [a, b, b1, c] = sample(&mut s);
    let l = new_layer(&mut s, "Props");
    execute(
        &mut s,
        "layer.set",
        json!({"layer": l.0, "color": [1.0, 0.0, 0.5]}),
    )
    .unwrap();
    assign(&mut s, Some(l), &[b, c]);
    let rows = tree_rows(s.doc.scene(), &s.expanded);
    let find = |name: &str| rows.iter().find(|r| r.name == name).unwrap();
    assert_eq!(find("Alpha").layer, None);
    assert_eq!(
        find("Bravo").layer,
        Some((l, "Props".to_string(), [1.0, 0.0, 0.5]))
    );
    assert_eq!(find("Bravo child").layer, None, "children do not inherit");
    // the manager toggle omits the row, not its children
    execute(&mut s, "layer.set", json!({"layer": l.0, "manager": false})).unwrap();
    let rows = tree_rows(s.doc.scene(), &s.expanded);
    let shown: Vec<(&str, usize)> = rows.iter().map(|r| (r.name.as_str(), r.depth)).collect();
    assert_eq!(shown, [("Alpha", 0), ("Bravo child", 0)]);
    // also in search results
    let hits = filter_rows(s.doc.scene(), "bravo");
    let names: Vec<&str> = hits.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Bravo child"]);
    let _ = (a, b1);
    execute(&mut s, "edit.undo", json!({})).unwrap();
    assert_eq!(tree_rows(s.doc.scene(), &s.expanded).len(), 4);
}

// ---- viewport: hiding and locking ---------------------------------------------------

#[test]
fn view_off_removes_the_object_from_the_viewport_items_and_undo_brings_it_back() {
    use forge_ui::viewport_panel::scene_draw_items_at;
    let mut s = state();
    let c = create(&mut s, "forge.cube");
    let d = create(&mut s, "forge.sphere");
    let l = new_layer(&mut s, "Props");
    assign(&mut s, Some(l), &[c]);
    let ids = |s: &EditorState| -> Vec<NodeId> {
        scene_draw_items_at(s.doc.scene(), &Default::default(), &Default::default())
            .iter()
            .map(|i| i.id())
            .collect()
    };
    assert_eq!(ids(&s), vec![c, d]);
    execute(&mut s, "layer.set", json!({"layer": l.0, "view": false})).unwrap();
    assert_eq!(ids(&s), vec![d]);
    execute(&mut s, "edit.undo", json!({})).unwrap();
    assert_eq!(ids(&s), vec![c, d]);
}

#[test]
fn a_locked_layer_cannot_be_picked_in_the_viewport() {
    let mut s = state();
    let cube = create(&mut s, "forge.cube");
    let l = new_layer(&mut s, "Props");
    assign(&mut s, Some(l), &[cube]);
    let cam = Camera {
        distance: 1000.0,
        ..EditorState::default_camera()
    };
    let (w, h) = (800.0, 600.0);
    let pick = |s: &EditorState| pick_node(s.doc.scene(), &cam, w / 2.0, h / 2.0, w, h);
    assert_eq!(pick(&s), Some(cube));
    execute(&mut s, "layer.set", json!({"layer": l.0, "locked": true})).unwrap();
    assert_eq!(pick(&s), None);
    // a click selects nothing, though the object is still there and selectable by name
    s.selection.clear();
    forge_ui::components::viewport_click(&mut s, [w / 2.0, h / 2.0], [w, h], Default::default())
        .unwrap();
    assert!(s.selection.nodes.is_empty());
    s.selection.click(cube);
    assert!(s.selection.nodes.contains(&cube));
    execute(&mut s, "layer.set", json!({"layer": l.0, "locked": false})).unwrap();
    assert_eq!(pick(&s), Some(cube));
}

#[test]
fn a_locked_polygon_object_is_not_a_component_target() {
    let mut s = state();
    let cube = create(&mut s, "forge.cube");
    execute(&mut s, "mesh.make_editable", json!({"nodes": [cube.0]})).unwrap();
    let poly = s.selection.active.unwrap();
    execute(&mut s, "select.mode", json!({"mode": "points"})).unwrap();
    assert_eq!(forge_ui::components::targets(&s), vec![poly]);
    let l = new_layer(&mut s, "Props");
    assign(&mut s, Some(l), &[poly]);
    execute(&mut s, "layer.set", json!({"layer": l.0, "locked": true})).unwrap();
    assert!(forge_ui::components::targets(&s).is_empty());
}

// ---- egui_kittest ---------------------------------------------------------------------

fn editor_with(st: EditorState) -> Harness<'static, Editor> {
    let mut editor = Editor::new(st);
    editor.state.dock.open(Panel::Layers);
    let mut h = Harness::builder()
        .with_size([1400.0, 1800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    h
}

fn harness() -> Harness<'static, Editor> {
    let mut st = state();
    sample(&mut st);
    editor_with(st)
}

fn st<'a>(h: &'a Harness<'_, Editor>) -> &'a EditorState {
    &h.state().state
}

fn click(h: &mut Harness<'_, Editor>, label: &str) {
    h.get_by_label(label).click();
    h.run();
}

fn undo(h: &mut Harness<'_, Editor>) {
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
}

fn steps(h: &Harness<'_, Editor>) -> usize {
    st(h).doc.undo_depth()
}

fn node_named(h: &Harness<'_, Editor>, name: &str) -> NodeId {
    let sc = st(h).doc.scene();
    sc.roots()
        .iter()
        .flat_map(|r| std::iter::once(*r).chain(sc.descendants(*r)))
        .find(|id| sc.node(*id).unwrap().name == name)
        .unwrap()
}

#[test]
fn the_panel_is_closed_by_default_and_opens_from_the_window_menu() {
    assert_eq!(Dock::default().area_of(Panel::Layers), None);
    assert_eq!(Panel::Layers.title(), "Layers");
    let editor = Editor::new(state());
    let mut h = Harness::builder()
        .with_size([1400.0, 900.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    assert!(h.query_by_label("New layer").is_none());
    click(&mut h, "Window");
    click(&mut h, "Layers panel");
    assert_eq!(st(&h).dock.area_of(Panel::Layers), Some(Area::RightBottom));
    assert_eq!(st(&h).dock.active(Area::RightBottom), Some(Panel::Layers));
    h.get_by_label("New layer");
    h.get_by_label("No layers");
    click(&mut h, "Window");
    click(&mut h, "Reset layout");
    assert_eq!(st(&h).dock.area_of(Panel::Layers), None);
}

#[test]
fn new_layer_creates_a_row_with_nine_toggles_and_one_undo_step() {
    let mut h = harness();
    let n = steps(&h);
    click(&mut h, "New layer");
    assert_eq!(steps(&h), n + 1);
    let layers = st(&h).doc.scene().layers().clone();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers.iter().next().unwrap().name, "Layer 1");
    h.get_by_label("Layer Layer 1");
    for t in Toggle::ALL {
        h.get_by_label(&t.label("Layer 1"));
    }
    undo(&mut h);
    assert!(st(&h).doc.scene().layers().is_empty());
    h.get_by_label("No layers");
}

#[test]
fn toggle_cells_run_layer_set_with_one_undo_step_each() {
    let mut h = harness();
    click(&mut h, "New layer");
    let id = st(&h).doc.scene().layers().iter().next().unwrap().id;
    for t in Toggle::ALL {
        let before = steps(&h);
        let on = t.get(&layer_of(st(&h), id));
        click(&mut h, &t.label("Layer 1"));
        assert_eq!(steps(&h), before + 1, "{}", t.name());
        assert_eq!(t.get(&layer_of(st(&h), id)), !on, "{}", t.name());
    }
    // all nine flipped; undo them one by one
    for t in Toggle::ALL.iter().rev() {
        let on = t.get(&layer_of(st(&h), id));
        undo(&mut h);
        assert_eq!(t.get(&layer_of(st(&h), id)), !on);
    }
}

#[test]
fn toggling_view_hides_the_object_and_the_object_manager_shows_the_colour() {
    let mut h = harness();
    let c = node_named(&h, "Charlie");
    click(&mut h, "New layer");
    let l = st(&h).doc.scene().layers().iter().next().unwrap().id;
    h.get_by_label("Charlie").click();
    h.run();
    click(&mut h, "Layer Layer 1");
    click(&mut h, "Assign selection");
    assert_eq!(
        st(&h).doc.scene().param(c, LAYER),
        Some(&Value::Link(Some(l)))
    );
    h.get_by_label("Layer: Layer 1");
    assert!(visibility::editor_visible(st(&h).doc.scene(), c));
    click(&mut h, "View: Layer 1");
    assert!(!visibility::editor_visible(st(&h).doc.scene(), c));
    assert!(!forge_app::schedule::drawn_nodes(st(&h).doc.scene()).contains(&c));
    click(&mut h, "View: Layer 1");
    assert!(forge_app::schedule::drawn_nodes(st(&h).doc.scene()).contains(&c));
    // manager off: the row goes
    click(&mut h, "Manager: Layer 1");
    assert!(h.query_by_label("Charlie").is_none());
    click(&mut h, "Manager: Layer 1");
    h.get_by_label("Charlie");
}

#[test]
fn a_toggle_on_a_selected_row_applies_to_every_selected_row() {
    let mut h = harness();
    click(&mut h, "New layer");
    click(&mut h, "New layer");
    click(&mut h, "New layer");
    h.get_by_label("Layer Layer 1").click();
    h.run();
    h.get_by_label("Layer Layer 2")
        .click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    let n = steps(&h);
    click(&mut h, "Locked: Layer 1");
    assert_eq!(steps(&h), n + 1, "one command for both rows");
    let l: Vec<bool> = st(&h)
        .doc
        .scene()
        .layers()
        .iter()
        .map(|l| l.locked)
        .collect();
    assert_eq!(l, [true, true, false]);
    // an unselected row acts alone
    click(&mut h, "Solo: Layer 3");
    let l: Vec<bool> = st(&h).doc.scene().layers().iter().map(|l| l.solo).collect();
    assert_eq!(l, [false, false, true]);
}

#[test]
fn double_click_renames_enter_commits_and_escape_cancels() {
    let mut h = harness();
    click(&mut h, "New layer");
    let double = |h: &mut Harness<'_, Editor>| {
        // a pause so the earlier click is not part of a triple click
        for _ in 0..8 {
            h.step();
        }
        h.ctx
            .options_mut(|o| o.input_options.max_double_click_delay = 1.0);
        h.get_by_label("Layer Layer 1").click();
        h.step();
        h.get_by_label("Layer Layer 1").click();
        h.step();
        h.run();
    };
    double(&mut h);
    let id = st(&h).doc.scene().layers().iter().next().unwrap().id;
    // cancelled
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Rename layer")
        .type_text("Zzz");
    h.run();
    h.key_press(egui::Key::Escape);
    h.run();
    assert_eq!(layer_of(st(&h), id).name, "Layer 1");
    // committed
    double(&mut h);
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Rename layer")
        .type_text("Walls");
    h.run();
    let n = steps(&h);
    h.key_press(egui::Key::Enter);
    h.run();
    assert_eq!(steps(&h), n + 1);
    assert!(
        layer_of(st(&h), id).name.starts_with("Layer 1")
            && layer_of(st(&h), id).name.ends_with("Walls")
    );
}

#[test]
fn delete_merge_assign_and_remove_buttons_with_undo() {
    let mut h = harness();
    let (a, b, c) = (
        node_named(&h, "Alpha"),
        node_named(&h, "Bravo"),
        node_named(&h, "Charlie"),
    );
    click(&mut h, "New layer");
    click(&mut h, "New layer");
    let ids: Vec<NodeId> = st(&h).doc.scene().layers().iter().map(|l| l.id).collect();
    let (l1, l2) = (ids[0], ids[1]);
    // disabled without a precondition
    let n = steps(&h);
    for b in ["Delete", "Merge", "Assign selection", "Remove from layer"] {
        click(&mut h, b);
    }
    assert_eq!(steps(&h), n, "disabled buttons run nothing");
    // assign A and B to layer 1
    h.get_by_label("Alpha").click();
    h.run();
    h.get_by_label("Bravo")
        .click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    click(&mut h, "Layer Layer 1");
    let n = steps(&h);
    click(&mut h, "Assign selection");
    assert_eq!(steps(&h), n + 1);
    for x in [a, b] {
        assert_eq!(
            st(&h).doc.scene().param(x, LAYER),
            Some(&Value::Link(Some(l1)))
        );
    }
    // C to layer 2 through the same buttons
    h.get_by_label("Charlie").click();
    h.run();
    click(&mut h, "Layer Layer 2");
    click(&mut h, "Assign selection");
    assert_eq!(layers::layer_id(st(&h).doc.scene(), c), Some(l2));
    // merge both rows into the first
    h.get_by_label("Layer Layer 1").click();
    h.run();
    h.get_by_label("Layer Layer 2")
        .click_modifiers(egui::Modifiers::COMMAND);
    h.run();
    let n = steps(&h);
    click(&mut h, "Merge");
    assert_eq!(steps(&h), n + 1);
    assert_eq!(st(&h).doc.scene().layers().len(), 1);
    assert_eq!(layers::layer_id(st(&h).doc.scene(), c), Some(l1));
    undo(&mut h);
    assert_eq!(st(&h).doc.scene().layers().len(), 2);
    assert_eq!(layers::layer_id(st(&h).doc.scene(), c), Some(l2));
    // remove C from its layer
    h.get_by_label("Charlie").click();
    h.run();
    click(&mut h, "Remove from layer");
    assert_eq!(layers::layer_id(st(&h).doc.scene(), c), None);
    undo(&mut h);
    // delete layer 1: A and B are released
    h.get_by_label("Layer Layer 1").click();
    h.run();
    let n = steps(&h);
    click(&mut h, "Delete");
    assert_eq!(steps(&h), n + 1);
    assert_eq!(st(&h).doc.scene().layers().len(), 1);
    assert_eq!(layers::layer_id(st(&h).doc.scene(), a), None);
    undo(&mut h);
    assert_eq!(layers::layer_id(st(&h).doc.scene(), a), Some(l1));
}

#[test]
fn the_object_context_menu_assigns_layers() {
    let mut st0 = state();
    let [_, _, b1, _] = sample(&mut st0);
    let l = new_layer(&mut st0, "Props");
    let mut h = editor_with(st0);
    h.get_by_label("Bravo child").click_secondary();
    h.run();
    h.get_by_label("Layer").click();
    h.run();
    let n = steps(&h);
    h.get_by_label("Props").click();
    h.run();
    assert_eq!(steps(&h), n + 1);
    assert_eq!(
        st(&h).doc.scene().param(b1, LAYER),
        Some(&Value::Link(Some(l)))
    );
    h.get_by_label("Layer: Props");
    h.get_by_label("Bravo child").click_secondary();
    h.run();
    h.get_by_label("Layer").click();
    h.run();
    h.get_by_label("No layer").click();
    h.run();
    assert_eq!(layers::layer_id(st(&h).doc.scene(), b1), None);
    undo(&mut h);
    assert_eq!(layers::layer_id(st(&h).doc.scene(), b1), Some(l));
}

#[test]
fn layers_survive_save_and_open() {
    let mut s = state();
    let [_, _, b1, _] = sample(&mut s);
    let l = new_layer(&mut s, "Props");
    assign(&mut s, Some(l), &[b1]);
    execute(
        &mut s,
        "layer.set",
        json!({"layer": l.0, "view": false, "generators": false, "color": [0.2, 0.4, 0.6]}),
    )
    .unwrap();
    let path = std::env::temp_dir().join(format!("forge-layers-{}.forge", std::process::id()));
    let p = path.to_str().unwrap();
    execute(&mut s, "file.save_as", json!({"path": p})).unwrap();
    let mut s2 = state();
    execute(&mut s2, "file.open", json!({"path": p})).unwrap();
    std::fs::remove_file(&path).ok();
    assert_eq!(s2.doc.scene().layers(), s.doc.scene().layers());
    assert_eq!(s2.doc.scene().param(b1, LAYER), Some(&Value::Link(Some(l))));
    assert!(!visibility::editor_visible(s2.doc.scene(), b1));
    let h = editor_with(s2);
    h.get_by_label("Layer Props");
}
