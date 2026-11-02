//! Node editor (docs/specs/UI.md §17; brief 109): egui_kittest on the full editor with the
//! expression tag, driving the canvas with pointer events. Every edit is a `graph.*` command.

use std::sync::Arc;

use egui::{Modifiers, PointerButton, Pos2, pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Value};
use forge_nodes::{Graph, GraphNodeId, TAG, load_graph};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::dock::{Area, Panel};
use forge_ui::node_canvas::{PortAddr, Side};
use forge_ui::state::EditorState;
use serde_json::{Value as Json, json};

struct Fixture {
    cube: NodeId,
    tag: NodeId,
}

fn state() -> (EditorState, Fixture) {
    let mut s = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    execute(&mut s, "object.create", json!({"type": "forge.cube"})).unwrap();
    let cube = s.selection.active.unwrap();
    let out = execute(&mut s, "object.create", json!({"type": TAG})).unwrap();
    let tag = NodeId(out["node"].as_u64().unwrap());
    s.selection.click(cube);
    (s, Fixture { cube, tag })
}

fn harness(mut s: EditorState) -> Harness<'static, Editor> {
    s.dock.reveal(Panel::NodeEditor);
    let mut h = Harness::builder()
        .with_size([1700.0, 900.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(s));
    h.run();
    h
}

fn add(s: &mut EditorState, tag: NodeId, ty: &str, pos: [f64; 2], values: Json) -> u64 {
    execute(
        s,
        "graph.add_node",
        json!({"tag": tag.0, "type": ty, "pos": pos, "values": values}),
    )
    .unwrap()["node"]
        .as_u64()
        .unwrap()
}

fn canvas<'a>(h: &'a Harness<'a, Editor>) -> &'a forge_ui::node_canvas::CanvasState {
    &h.state().state.node_editor.canvas
}

fn graph(h: &Harness<'_, Editor>, tag: NodeId) -> Graph {
    load_graph(h.state().state.doc.scene(), tag).unwrap()
}

fn steps(h: &Harness<'_, Editor>) -> usize {
    h.state().state.doc.history().undo_labels().len()
}

fn undo(h: &mut Harness<'_, Editor>) {
    execute(&mut h.state_mut().state, "edit.undo", json!({})).unwrap();
    h.run();
}

fn redo(h: &mut Harness<'_, Editor>) {
    execute(&mut h.state_mut().state, "edit.redo", json!({})).unwrap();
    h.run();
}

/// Screen position of a port.
fn port(h: &Harness<'_, Editor>, node: u64, id: &str, side: Side) -> Pos2 {
    let c = canvas(h);
    c.layout
        .port_screen(&c.view, c.rect.min, &PortAddr::new(node, id, side))
        .unwrap_or_else(|| panic!("no port {node}.{id}"))
}

/// A point on the title bar of a node (not on a port).
fn title(h: &Harness<'_, Editor>, node: u64) -> Pos2 {
    let c = canvas(h);
    let r = c.layout.node_screen(&c.view, c.rect.min, node).unwrap();
    r.min + vec2(100.0 * c.view.zoom, 12.0 * c.view.zoom)
}

/// A point on the canvas at a world position.
fn at(h: &Harness<'_, Editor>, x: f32, y: f32) -> Pos2 {
    let c = canvas(h);
    c.view.to_screen(c.rect.min, pos2(x, y))
}

fn press(h: &mut Harness<'_, Editor>, pos: Pos2, button: PointerButton, mods: Modifiers) {
    h.event(egui::Event::PointerButton {
        pos,
        button,
        pressed: true,
        modifiers: mods,
    });
    h.run();
}

fn release(h: &mut Harness<'_, Editor>, pos: Pos2, button: PointerButton, mods: Modifiers) {
    h.event(egui::Event::PointerButton {
        pos,
        button,
        pressed: false,
        modifiers: mods,
    });
    h.run();
}

fn drag_with(h: &mut Harness<'_, Editor>, from: Pos2, to: Pos2, mods: Modifiers) {
    h.event(egui::Event::ModifiersChanged(mods));
    h.hover_at(from);
    h.run();
    press(h, from, PointerButton::Primary, mods);
    for k in 1..=5 {
        h.hover_at(from + (to - from) * (k as f32 / 5.0));
        h.run();
    }
    release(h, to, PointerButton::Primary, mods);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

fn drag(h: &mut Harness<'_, Editor>, from: Pos2, to: Pos2) {
    drag_with(h, from, to, Modifiers::NONE);
}

type PortSpec<'a> = (u64, &'a str, Side);

fn drag_ports(h: &mut Harness<'_, Editor>, from: PortSpec<'_>, to: PortSpec<'_>) {
    let (a, b) = (port(h, from.0, from.1, from.2), port(h, to.0, to.1, to.2));
    drag(h, a, b);
}

fn right_click_title(h: &mut Harness<'_, Editor>, node: u64) {
    let p = title(h, node);
    secondary_click(h, p);
}

fn click_title(h: &mut Harness<'_, Editor>, node: u64) {
    let p = title(h, node);
    click_at(h, p);
}

fn click_world(h: &mut Harness<'_, Editor>, x: f32, y: f32) {
    let p = at(h, x, y);
    click_at(h, p);
}

/// Drags between two world positions.
fn drag_world(h: &mut Harness<'_, Editor>, from: (f32, f32), to: (f32, f32)) {
    let (a, b) = (at(h, from.0, from.1), at(h, to.0, to.1));
    drag(h, a, b);
}

fn click_at(h: &mut Harness<'_, Editor>, pos: Pos2) {
    h.hover_at(pos);
    h.run();
    press(h, pos, PointerButton::Primary, Modifiers::NONE);
    release(h, pos, PointerButton::Primary, Modifiers::NONE);
}

fn secondary_click(h: &mut Harness<'_, Editor>, pos: Pos2) {
    h.hover_at(pos);
    h.run();
    press(h, pos, PointerButton::Secondary, Modifiers::NONE);
    release(h, pos, PointerButton::Secondary, Modifiers::NONE);
}

fn double_click(h: &mut Harness<'_, Editor>, pos: Pos2) {
    h.ctx
        .options_mut(|o| o.input_options.max_double_click_delay = 2.0);
    h.hover_at(pos);
    h.run();
    for _ in 0..2 {
        press(h, pos, PointerButton::Primary, Modifiers::NONE);
        release(h, pos, PointerButton::Primary, Modifiers::NONE);
    }
}

/// Adds a node through the search menu opened by a right-click at `pos`.
fn add_via_menu(h: &mut Harness<'_, Editor>, pos: Pos2, query: &str, entry: &str) {
    secondary_click(h, pos);
    let search = h.get_by_label("Search nodes");
    search.focus();
    h.run();
    h.get_by_label("Search nodes").type_text(query);
    h.run();
    h.get_by_label_contains(entry).click();
    h.run();
}

/// Two math nodes with ids (1, 2) far enough apart to drag between.
fn two_adds(s: &mut EditorState, f: &Fixture) -> (u64, u64) {
    let a = add(s, f.tag, "math.add", [0.0, 0.0], json!({}));
    let b = add(s, f.tag, "math.mul", [400.0, 40.0], json!({}));
    (a, b)
}

#[test]
fn the_panel_opens_from_the_window_menu_and_by_double_clicking_a_tag() {
    let (mut s, f) = state();
    // A second cube with its own tag, so the double-click has to pick the right one.
    execute(&mut s, "object.create", json!({"type": "forge.sphere"})).unwrap();
    let sphere = s.selection.active.unwrap();
    let out = execute(&mut s, "object.create", json!({"type": TAG})).unwrap();
    let tag2 = NodeId(out["node"].as_u64().unwrap());
    assert_ne!(tag2, f.tag);
    let _ = (sphere, f.cube);
    assert_eq!(s.dock.area_of(Panel::NodeEditor), None);
    assert_eq!(Panel::NodeEditor.title(), "Node Editor");
    let mut h = Harness::builder()
        .with_size([1700.0, 900.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(s));
    h.run();
    assert!(h.query_by_label("Expression tag").is_none());

    // Window > Node Editor panel
    h.get_by_label("Window").click();
    h.run();
    h.get_by_label("Node Editor panel").click();
    h.run();
    assert_eq!(
        h.state().state.dock.area_of(Panel::NodeEditor),
        Some(Area::Center)
    );
    assert_eq!(
        h.state().state.dock.active(Area::Center),
        Some(Panel::NodeEditor)
    );
    h.get_by_label("Expression tag");
    h.get_by_label("Node canvas");

    // Back to the viewport tab, then double-click the second tag in the object manager.
    h.state_mut().state.dock.set_active(Area::Center, 0);
    h.run();
    assert!(h.query_by_label("Node canvas").is_none());
    assert_eq!(h.get_all_by_label("[Expression]").count(), 2);
    let second = h
        .get_all_by_label("[Expression]")
        .nth(1)
        .unwrap()
        .rect()
        .center();
    double_click(&mut h, second);
    assert_eq!(
        h.state().state.dock.active(Area::Center),
        Some(Panel::NodeEditor)
    );
    assert_eq!(h.state().state.node_editor.tag, Some(tag2));
    h.get_by_label("Node canvas");
}

#[test]
fn a_tag_can_be_added_and_chosen_in_the_panel() {
    let (mut s, f) = state();
    execute(&mut s, "edit.undo", json!({})).unwrap(); // the tag is gone again
    assert!(s.doc.scene().node(f.tag).is_none());
    let mut h = harness(s);
    h.get_by_label("Add expression tag").click();
    h.run();
    let tags = forge_ui::node_editor::expression_tags(h.state().state.doc.scene());
    assert_eq!(tags.len(), 1);
    assert_eq!(h.state().state.node_editor.tag, Some(tags[0].0));
    assert_eq!(tags[0].1, "Cube / Expression");
}

#[test]
fn add_a_node_through_the_search_menu_and_undo() {
    let (s, f) = state();
    let mut h = harness(s);
    let before = steps(&h);
    let spot = at(&h, 120.0, 80.0);
    secondary_click(&mut h, spot);
    h.get_by_label("Search nodes");
    // Everything is listed until a query narrows it.
    let all = h.query_all_by_label_contains("(").count();
    assert!(all > 20, "{all}");
    h.get_by_label("Search nodes").focus();
    h.run();
    h.get_by_label("Search nodes").type_text("noise");
    h.run();
    h.get_by_label_contains("(math.noise)");
    assert!(h.query_by_label_contains("(math.add)").is_none());
    h.get_by_label("Search nodes").type_text("zzz");
    h.run();
    h.get_by_label("No match");
    // Fix the query and choose.
    h.get_by_label("Search nodes").focus();
    h.key_press_modifiers(Modifiers::COMMAND, egui::Key::A);
    h.get_by_label("Search nodes").type_text("math.add");
    h.run();
    h.get_by_label_contains("(math.add)").click();
    h.run();

    let g = graph(&h, f.tag);
    assert_eq!(g.len(), 1);
    let n = g.nodes().next().unwrap();
    assert_eq!(n.type_id, "math.add");
    assert!(
        (n.pos[0] - 120.0).abs() < 1.0 && (n.pos[1] - 80.0).abs() < 1.0,
        "{:?}",
        n.pos
    );
    assert_eq!(steps(&h), before + 1);
    assert!(
        h.query_by_label("Search nodes").is_none(),
        "the menu closed"
    );
    assert_eq!(canvas(&h).selection.len(), 1, "the new node is selected");
    undo(&mut h);
    assert!(graph(&h, f.tag).is_empty());
    redo(&mut h);
    assert_eq!(graph(&h, f.tag).len(), 1);
}

#[test]
fn double_click_on_empty_canvas_opens_the_menu_and_escape_closes_it() {
    let (s, _f) = state();
    let mut h = harness(s);
    let spot = at(&h, 50.0, 50.0);
    double_click(&mut h, spot);
    h.get_by_label("Search nodes");
    h.key_press(egui::Key::Escape);
    h.run();
    assert!(h.query_by_label("Search nodes").is_none());
}

#[test]
fn connect_two_nodes_by_dragging_one_undo_step() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    let mut h = harness(s);
    let before = steps(&h);
    let from = port(&h, a, "out", Side::Output);
    let to = port(&h, b, "a", Side::Input);
    drag(&mut h, from, to);
    let g = graph(&h, f.tag);
    assert_eq!(g.links().len(), 1);
    let l = &g.links()[0];
    assert_eq!((l.from.node.0, l.from.port.as_str()), (a, "out"));
    assert_eq!((l.to.node.0, l.to.port.as_str()), (b, "a"));
    assert_eq!(steps(&h), before + 1);
    assert!(canvas(&h).notice.is_none());
    undo(&mut h);
    assert!(graph(&h, f.tag).links().is_empty());
    redo(&mut h);
    assert_eq!(graph(&h, f.tag).links().len(), 1);
}

#[test]
fn a_wire_can_also_start_on_a_free_input() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    let mut h = harness(s);
    drag_ports(&mut h, (b, "b", Side::Input), (a, "out", Side::Output));
    let g = graph(&h, f.tag);
    assert_eq!(g.links().len(), 1);
    assert_eq!(g.links()[0].to.port, "b");
    assert_eq!(g.links()[0].from.node.0, a);
}

#[test]
fn a_refused_connection_changes_nothing_and_says_why() {
    let (mut s, f) = state();
    let a = add(&mut s, f.tag, "math.add", [0.0, 0.0], json!({}));
    let get = add(&mut s, f.tag, "object.get_param", [400.0, 0.0], json!({}));
    let mut h = harness(s);
    let before = steps(&h);
    let doc_rev = h.state().state.doc.scene().revision();
    // float -> node link: no conversion
    drag_ports(
        &mut h,
        (a, "out", Side::Output),
        (get, "object", Side::Input),
    );
    assert!(graph(&h, f.tag).links().is_empty());
    assert_eq!(steps(&h), before);
    assert_eq!(h.state().state.doc.scene().revision(), doc_rev);
    let notice = canvas(&h).notice.clone().expect("the reason is shown");
    assert!(notice.contains("cannot connect float to node"), "{notice}");
    assert_eq!(h.state().state.status, notice, "and in the status bar");

    // A cycle is refused too.
    let b = add_node_chain(&mut h, f.tag);
    let notice = {
        drag_ports(&mut h, (b.1, "out", Side::Output), (b.0, "a", Side::Input));
        canvas(&h).notice.clone().expect("cycle reason")
    };
    assert!(notice.contains("cycle"), "{notice}");
    assert_eq!(graph(&h, f.tag).links().len(), 1);
}

#[test]
fn while_a_wire_is_dragged_the_port_under_the_pointer_shows_the_verdict() {
    let (mut s, f) = state();
    let a = add(&mut s, f.tag, "math.add", [0.0, 0.0], json!({}));
    let get = add(&mut s, f.tag, "object.get_param", [400.0, 0.0], json!({}));
    let b = add(&mut s, f.tag, "math.mul", [400.0, 200.0], json!({}));
    let mut h = harness(s);
    let from = port(&h, a, "out", Side::Output);
    h.hover_at(from);
    h.run();
    press(&mut h, from, PointerButton::Primary, Modifiers::NONE);
    // Over a Node input: refused, with the reason.
    let bad = port(&h, get, "object", Side::Input);
    for k in 1..=4 {
        h.hover_at(from + (bad - from) * (k as f32 / 4.0));
        h.run();
    }
    let (addr, verdict) = canvas(&h).wire_hover.clone().expect("a port is hovered");
    assert_eq!((addr.node, addr.port.as_str()), (get, "object"));
    assert!(
        verdict
            .unwrap_err()
            .contains("cannot connect float to node")
    );
    // Over a Float input: allowed.
    let good = port(&h, b, "a", Side::Input);
    for k in 1..=4 {
        h.hover_at(bad + (good - bad) * (k as f32 / 4.0));
        h.run();
    }
    let (addr, verdict) = canvas(&h).wire_hover.clone().expect("a port is hovered");
    assert_eq!((addr.node, addr.port.as_str()), (b, "a"));
    assert_eq!(verdict, Ok(()));
    // Over an output (the wrong side) or nothing: no verdict.
    h.hover_at(at(&h, 250.0, 500.0));
    h.run();
    assert!(canvas(&h).wire_hover.is_none());
    // Escape cancels the wire; releasing then changes nothing.
    h.key_press(egui::Key::Escape);
    h.run();
    release(&mut h, good, PointerButton::Primary, Modifiers::NONE);
    assert!(graph(&h, f.tag).links().is_empty());
}

/// Two more nodes connected `x.out -> y.a`; returns (x, y).
fn add_node_chain(h: &mut Harness<'_, Editor>, tag: NodeId) -> (u64, u64) {
    let s = &mut h.state_mut().state;
    let x = add(s, tag, "math.sub", [0.0, 300.0], json!({}));
    let y = add(s, tag, "math.div", [400.0, 300.0], json!({}));
    execute(
        s,
        "graph.connect",
        json!({"tag": tag.0, "from_node": x, "from_port": "out", "to_node": y, "to_port": "a"}),
    )
    .unwrap();
    h.run();
    (x, y)
}

#[test]
fn dragging_a_node_moves_it_in_one_undo_step_and_saves_the_position() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    let mut h = harness(s);
    let before = steps(&h);
    let start = title(&h, a);
    drag(&mut h, start, start + vec2(120.0, 60.0));
    let g = graph(&h, f.tag);
    let p = g.node(GraphNodeId(a)).unwrap().pos;
    assert!(
        (p[0] - 120.0).abs() < 1.0 && (p[1] - 60.0).abs() < 1.0,
        "{p:?}"
    );
    assert_eq!(
        g.node(GraphNodeId(b)).unwrap().pos,
        [400.0, 40.0],
        "others stay"
    );
    assert_eq!(steps(&h), before + 1, "a whole drag is one step");
    // The position is in the saved graph text.
    let text = match h.state().state.doc.scene().param(f.tag, "graph") {
        Some(Value::String(t)) => t.clone(),
        other => panic!("{other:?}"),
    };
    assert!(text.contains("\"pos\""), "{text}");
    undo(&mut h);
    assert_eq!(
        graph(&h, f.tag).node(GraphNodeId(a)).unwrap().pos,
        [0.0, 0.0]
    );
    redo(&mut h);
    assert_eq!(graph(&h, f.tag).node(GraphNodeId(a)).unwrap().pos, p);
    // A second drag is another step.
    let start = title(&h, a);
    drag(&mut h, start, start + vec2(0.0, 40.0));
    assert_eq!(steps(&h), before + 2);
}

#[test]
fn dragging_a_selected_node_moves_the_whole_selection() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    let mut h = harness(s);
    click_title(&mut h, a);
    let tb = title(&h, b);
    drag_with(&mut h, tb, tb + vec2(10.0, 0.0), Modifiers::SHIFT);
    assert_eq!(canvas(&h).selection.len(), 2);
    let before = steps(&h);
    let start = title(&h, a);
    drag(&mut h, start, start + vec2(0.0, 100.0));
    let g = graph(&h, f.tag);
    assert!((g.node(GraphNodeId(a)).unwrap().pos[1] - 100.0).abs() < 1.0);
    assert!((g.node(GraphNodeId(b)).unwrap().pos[1] - 140.0).abs() < 1.0);
    assert_eq!(steps(&h), before + 1);
}

#[test]
fn picking_up_a_link_and_dropping_it_on_empty_canvas_disconnects() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    execute(
        &mut s,
        "graph.connect",
        json!({"tag": f.tag.0, "from_node": a, "from_port": "out", "to_node": b, "to_port": "a"}),
    )
    .unwrap();
    let mut h = harness(s);
    let before = steps(&h);
    let input = port(&h, b, "a", Side::Input);
    let empty = at(&h, 300.0, 400.0);
    drag(&mut h, input, empty);
    assert!(graph(&h, f.tag).links().is_empty());
    assert_eq!(steps(&h), before + 1);
    undo(&mut h);
    assert_eq!(graph(&h, f.tag).links().len(), 1);
}

#[test]
fn moving_a_link_to_another_input_is_one_step_and_dropping_it_back_is_none() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    execute(
        &mut s,
        "graph.connect",
        json!({"tag": f.tag.0, "from_node": a, "from_port": "out", "to_node": b, "to_port": "a"}),
    )
    .unwrap();
    let mut h = harness(s);
    let before = steps(&h);
    // Back onto the same input: nothing.
    let input = port(&h, b, "a", Side::Input);
    drag(&mut h, input, input + vec2(3.0, 0.0));
    assert_eq!(steps(&h), before);
    assert_eq!(graph(&h, f.tag).links().len(), 1);
    // To the other input.
    drag_ports(&mut h, (b, "a", Side::Input), (b, "b", Side::Input));
    let g = graph(&h, f.tag);
    assert_eq!(g.links().len(), 1);
    assert_eq!(g.links()[0].to.port, "b");
    assert_eq!(steps(&h), before + 1);
    undo(&mut h);
    assert_eq!(graph(&h, f.tag).links()[0].to.port, "a");
}

#[test]
fn a_context_menu_disconnects_a_link_and_deletes_nodes() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    execute(
        &mut s,
        "graph.connect",
        json!({"tag": f.tag.0, "from_node": a, "from_port": "out", "to_node": b, "to_port": "a"}),
    )
    .unwrap();
    let mut h = harness(s);
    // On the link: halfway along its curve.
    let (p0, p1) = (
        port(&h, a, "out", Side::Output),
        port(&h, b, "a", Side::Input),
    );
    let mid = forge_ui::node_canvas::curve(p0, p1, canvas(&h).view.zoom)[12];
    let before = steps(&h);
    secondary_click(&mut h, mid);
    h.get_by_label("Disconnect").click();
    h.run();
    assert!(graph(&h, f.tag).links().is_empty());
    assert_eq!(steps(&h), before + 1);
    // On a node: Delete node.
    right_click_title(&mut h, b);
    h.get_by_label("Delete node").click();
    h.run();
    assert_eq!(graph(&h, f.tag).len(), 1);
    assert_eq!(steps(&h), before + 2);
}

#[test]
fn the_node_menu_disconnects_one_input() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    execute(
        &mut s,
        "graph.connect",
        json!({"tag": f.tag.0, "from_node": a, "from_port": "out", "to_node": b, "to_port": "b"}),
    )
    .unwrap();
    let mut h = harness(s);
    right_click_title(&mut h, b);
    h.get_by_label("Disconnect B").click();
    h.run();
    assert!(graph(&h, f.tag).links().is_empty());
}

#[test]
fn box_select_then_delete_removes_the_nodes_and_their_links_in_one_step() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    let c = add(&mut s, f.tag, "math.sub", [0.0, 500.0], json!({}));
    execute(
        &mut s,
        "graph.connect",
        json!({"tag": f.tag.0, "from_node": a, "from_port": "out", "to_node": b, "to_port": "a"}),
    )
    .unwrap();
    let mut h = harness(s);
    // A rectangle over the first two nodes only.
    drag_world(&mut h, (-30.0, -30.0), (640.0, 220.0));
    let sel: Vec<u64> = canvas(&h).selection.iter().copied().collect();
    assert_eq!(sel, vec![a, b]);
    // Clicking empty canvas clears; a click on a node selects only it.
    click_world(&mut h, 800.0, 450.0);
    assert!(canvas(&h).selection.is_empty());
    drag_world(&mut h, (-30.0, -30.0), (640.0, 220.0));
    let before = steps(&h);
    let objects_before = h.state().state.doc.scene().len();
    h.hover_at(at(&h, 800.0, 450.0));
    h.run();
    h.key_press(egui::Key::Delete);
    h.run();
    let g = graph(&h, f.tag);
    assert_eq!(g.len(), 1);
    assert!(g.node(GraphNodeId(c)).is_some());
    assert!(g.links().is_empty());
    assert_eq!(steps(&h), before + 1);
    assert_eq!(
        h.state().state.doc.scene().len(),
        objects_before,
        "Delete over the canvas does not delete objects"
    );
    assert!(h.state().state.doc.scene().node(f.cube).is_some());
    undo(&mut h);
    let g = graph(&h, f.tag);
    assert_eq!((g.len(), g.links().len()), (3, 1));
}

#[test]
fn shift_click_toggles_and_a_plain_click_replaces_the_selection() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    let _ = f;
    let mut h = harness(s);
    click_title(&mut h, a);
    assert_eq!(
        canvas(&h).selection.iter().copied().collect::<Vec<_>>(),
        vec![a]
    );
    let pos = title(&h, b);
    h.event(egui::Event::ModifiersChanged(Modifiers::SHIFT));
    h.hover_at(pos);
    h.run();
    press(&mut h, pos, PointerButton::Primary, Modifiers::SHIFT);
    release(&mut h, pos, PointerButton::Primary, Modifiers::SHIFT);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run();
    assert_eq!(canvas(&h).selection.len(), 2);
    click_title(&mut h, b);
    assert_eq!(
        canvas(&h).selection.iter().copied().collect::<Vec<_>>(),
        vec![b]
    );
}

#[test]
fn pan_and_zoom_move_the_view_but_not_the_document() {
    let (mut s, f) = state();
    let (a, _b) = two_adds(&mut s, &f);
    let mut h = harness(s);
    let before = steps(&h);
    let rev = h.state().state.doc.scene().revision();
    let pan0 = canvas(&h).view.pan;
    // Middle-drag pans.
    let from = at(&h, 300.0, 400.0);
    h.hover_at(from);
    h.run();
    press(&mut h, from, PointerButton::Middle, Modifiers::NONE);
    for k in 1..=4 {
        h.hover_at(from + vec2(10.0 * k as f32, 5.0 * k as f32));
        h.run();
    }
    release(
        &mut h,
        from + vec2(40.0, 20.0),
        PointerButton::Middle,
        Modifiers::NONE,
    );
    let moved = canvas(&h).view.pan - pan0;
    assert!(
        (moved.x - 40.0).abs() < 8.0 && (moved.y - 20.0).abs() < 8.0,
        "{moved:?}"
    );
    // The wheel zooms about the pointer (near the first node, so it stays in view).
    let p = at(&h, 100.0, 12.0);
    let world_before = canvas(&h).view.to_world(canvas(&h).rect.min, p);
    h.hover_at(p);
    h.run();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: vec2(0.0, 200.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    });
    for _ in 0..10 {
        h.run();
    }
    let c = canvas(&h);
    assert!(c.view.zoom > 1.2, "{}", c.view.zoom);
    let world_after = c.view.to_world(c.rect.min, p);
    assert!(world_before.distance(world_after) < 1.0);
    // Nodes are still hit where they are drawn.
    let t = title(&h, a);
    let g_before = graph(&h, f.tag);
    drag(&mut h, t, t + vec2(100.0, 0.0));
    let moved = graph(&h, f.tag).node(GraphNodeId(a)).unwrap().pos[0]
        - g_before.node(GraphNodeId(a)).unwrap().pos[0];
    assert!(
        (moved - f64::from(100.0 / canvas(&h).view.zoom)).abs() < 1.0,
        "{moved}"
    );
    assert_eq!(steps(&h), before + 1, "only the node move was an edit");
    assert_ne!(h.state().state.doc.scene().revision(), rev);
}

#[test]
fn editing_a_constant_runs_set_value_and_undoes() {
    let (mut s, f) = state();
    let a = add(&mut s, f.tag, "const.float", [0.0, 0.0], json!({}));
    let mut h = harness(s);
    let before = steps(&h);
    h.get_by_label(&format!("Float #{a} Value"))
        .set_value_accesskit(2.5);
    h.run();
    let g = graph(&h, f.tag);
    assert_eq!(
        g.node(GraphNodeId(a)).unwrap().values.get("value"),
        Some(&forge_nodes::PortValue::Float(2.5))
    );
    assert_eq!(steps(&h), before + 1);
    undo(&mut h);
    assert!(
        graph(&h, f.tag)
            .node(GraphNodeId(a))
            .unwrap()
            .values
            .is_empty()
    );
}

#[test]
fn a_linked_input_has_no_editor_and_a_free_one_does() {
    let (mut s, f) = state();
    let (a, b) = two_adds(&mut s, &f);
    execute(
        &mut s,
        "graph.connect",
        json!({"tag": f.tag.0, "from_node": a, "from_port": "out", "to_node": b, "to_port": "a"}),
    )
    .unwrap();
    let h = harness(s);
    assert!(h.query_by_label(&format!("Multiply #{b} A")).is_none());
    h.get_by_label(&format!("Multiply #{b} B"));
    h.get_by_label(&format!("Add #{a} A"));
}

#[test]
fn live_values_follow_the_graph_and_the_timeline() {
    let (mut s, f) = state();
    let c = add(
        &mut s,
        f.tag,
        "const.float",
        [0.0, 0.0],
        json!({"value": 2.5}),
    );
    let m = add(&mut s, f.tag, "math.mul", [300.0, 0.0], json!({"b": 4.0}));
    let t = add(&mut s, f.tag, "time.now", [0.0, 200.0], json!({}));
    execute(
        &mut s,
        "graph.connect",
        json!({"tag": f.tag.0, "from_node": c, "from_port": "out", "to_node": m, "to_port": "a"}),
    )
    .unwrap();
    let mut h = harness(s);
    let ne = |h: &Harness<'_, Editor>, n: u64, p: &str| {
        h.state().state.node_editor.live_value(n, p).cloned()
    };
    assert_eq!(ne(&h, c, "out"), Some(json!(2.5)));
    assert_eq!(ne(&h, m, "out"), Some(json!(10.0)));
    assert!(h.state().state.node_editor.eval_error().is_none());
    // An edit shows up at once.
    h.get_by_label(&format!("Float #{c} Value"))
        .set_value_accesskit(3.0);
    h.run();
    assert_eq!(ne(&h, m, "out"), Some(json!(12.0)));
    // And time moves the values.
    execute(
        &mut h.state_mut().state,
        "timeline.set_frame",
        json!({"frame": 30}),
    )
    .unwrap();
    h.run();
    let fps = f64::from(h.state().state.doc.scene().settings().fps);
    let secs = ne(&h, t, "seconds").unwrap().as_f64().unwrap();
    assert!((secs - 30.0 / fps).abs() < 1e-9, "{secs}");
}

#[test]
fn a_failing_graph_shows_its_error_and_no_values() {
    let (mut s, f) = state();
    // The Rhai node fails on a syntax error.
    let r = add(
        &mut s,
        f.tag,
        "script.rhai",
        [0.0, 0.0],
        json!({"source": "let x = ;"}),
    );
    let h = harness(s);
    let err = h
        .state()
        .state
        .node_editor
        .eval_error()
        .expect("an error")
        .to_string();
    assert!(err.contains(&format!("graph node {r}")), "{err}");
    assert!(
        h.state()
            .state
            .node_editor
            .live_value(r, "result")
            .is_none()
    );
}

#[test]
fn an_expression_built_in_the_editor_drives_the_scene_and_undo_takes_it_back() {
    let (s, f) = state();
    let mut h = harness(s);
    // Three nodes through the add menu.
    let spot = at(&h, 0.0, 0.0);
    add_via_menu(&mut h, spot, "const.float", "(const.float)");
    let spot = at(&h, 320.0, 0.0);
    add_via_menu(&mut h, spot, "vector.compose", "(vector.compose)");
    let spot = at(&h, 640.0, 0.0);
    add_via_menu(&mut h, spot, "object.set_param", "(object.set_param)");
    let ids: Vec<(u64, String)> = graph(&h, f.tag)
        .nodes()
        .map(|n| (n.id.0, n.type_id.clone()))
        .collect();
    assert_eq!(ids.len(), 3, "{ids:?}");
    let id_of = |ty: &str| ids.iter().find(|(_, t)| t == ty).unwrap().0;
    let (c, v, w) = (
        id_of("const.float"),
        id_of("vector.compose"),
        id_of("object.set_param"),
    );

    // Constants typed into the nodes.
    h.get_by_label(&format!("Float #{c} Value"))
        .set_value_accesskit(42.0);
    h.run();
    // The text box: typing commits when it loses focus (Enter), as one step.
    let pname = format!("#{w} Param");
    let steps0 = steps(&h);
    for text in ["transform.rot", "transform.pos"] {
        h.get_by_label_contains(&pname).focus();
        h.run();
        h.key_press_modifiers(Modifiers::COMMAND, egui::Key::A);
        h.run();
        h.get_by_label_contains(&pname).type_text(text);
        h.run();
        assert_eq!(
            steps(&h),
            steps0 + if text == "transform.rot" { 0 } else { 1 },
            "{text}"
        );
        h.key_press(egui::Key::Enter);
        h.run();
        let g = graph(&h, f.tag);
        assert_eq!(
            g.node(GraphNodeId(w)).unwrap().values.get("param"),
            Some(&forge_nodes::PortValue::String(text.into()))
        );
    }
    assert_eq!(steps(&h), steps0 + 2, "one step per committed text");

    // Not connected yet: the cube stays at the origin.
    let pos_at = |h: &mut Harness<'_, Editor>, frame: i64| -> [f64; 3] {
        let scene = h.state_mut().state.frame(frame);
        match scene.param(f.cube, "transform.pos") {
            Some(Value::Vec3(p)) => *p,
            other => panic!("{other:?}"),
        }
    };
    assert_eq!(pos_at(&mut h, 0), [0.0; 3]);

    // Wire const -> x and compose -> value by dragging.
    drag_ports(&mut h, (c, "out", Side::Output), (v, "x", Side::Input));
    assert_eq!(
        pos_at(&mut h, 0),
        [0.0; 3],
        "the write node is not connected yet"
    );
    let before_last = steps(&h);
    drag_ports(&mut h, (v, "out", Side::Output), (w, "value", Side::Input));
    assert_eq!(steps(&h), before_last + 1);
    assert_eq!(pos_at(&mut h, 0), [42.0, 0.0, 0.0]);
    assert_eq!(pos_at(&mut h, 20), [42.0, 0.0, 0.0]);
    // The stored scene is untouched; only the evaluated one has the expression's output.
    match h.state().state.doc.scene().param(f.cube, "transform.pos") {
        Some(Value::Vec3(p)) => assert_eq!(*p, [0.0; 3]),
        other => panic!("{other:?}"),
    }
    // Live values show the same number the scene got.
    assert_eq!(
        h.state().state.node_editor.live_value(v, "out"),
        Some(&json!([42.0, 0.0, 0.0]))
    );
    // Undo of the last connection takes the effect away.
    undo(&mut h);
    assert_eq!(pos_at(&mut h, 0), [0.0; 3]);
    redo(&mut h);
    assert_eq!(pos_at(&mut h, 0), [42.0, 0.0, 0.0]);
}
