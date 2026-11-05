//! docs/specs/MODELING.md §9.7: the arrangement tools in the editor, without a GPU.

mod common;

use serde_json::json;

use common::{create, state};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{NodeId, Value};
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::state::EditorState;
use forge_ui::tools::{activate_tool, apply_tool, arrange_tools, tool_preview};

fn steps(st: &EditorState) -> usize {
    st.doc.history().undo_labels().len()
}

fn pos(st: &EditorState, id: NodeId) -> [f64; 3] {
    match st.doc.scene().param(id, "transform.pos") {
        Some(Value::Vec3(v)) => *v,
        _ => panic!("pos"),
    }
}

#[test]
fn the_arrangement_tools_are_listed_and_activate() {
    let ids: Vec<_> = arrange_tools().iter().map(|t| t.id()).collect();
    assert_eq!(
        ids,
        ["array", "duplicate", "arrange", "randomize", "transfer"]
    );
    let mut st = state();
    for id in ids {
        assert!(activate_tool(&mut st, id), "{id}");
    }
}

#[test]
fn apply_duplicate_is_one_undo_step() {
    let mut st = state();
    let a = create(&mut st, "forge.cube");
    assert!(activate_tool(&mut st, "duplicate"));
    st.tool
        .as_mut()
        .unwrap()
        .set("copies", Value::Int(2))
        .unwrap();
    st.tool
        .as_mut()
        .unwrap()
        .set("offset", Value::Vec3([100.0, 0.0, 0.0]))
        .unwrap();
    assert!(
        tool_preview(&st).is_none(),
        "object commands have no preview"
    );
    let before = steps(&st);
    let out = apply_tool(&mut st).unwrap();
    assert_eq!(out["count"], 2);
    assert_eq!(steps(&st), before + 1);
    assert_eq!(st.doc.scene().roots().len(), 3);
    assert_eq!(pos(&st, a), [0.0; 3]);
    // the tool stays active with its values
    assert_eq!(st.tool.as_ref().unwrap().values().int("copies"), 2);
    execute(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(st.doc.scene().roots().len(), 1);
}

#[test]
fn apply_transfer_uses_the_other_selected_object() {
    let mut st = state();
    let a = create(&mut st, "forge.cube");
    let b = create(&mut st, "forge.cube");
    execute(
        &mut st,
        "transform.set",
        json!({"node": a.0, "pos": [7.0, 8.0, 9.0]}),
    )
    .unwrap();
    st.selection.nodes = [a, b].into_iter().collect();
    st.selection.active = Some(a);
    activate_tool(&mut st, "transfer");
    apply_tool(&mut st).unwrap();
    assert_eq!(pos(&st, b), [7.0, 8.0, 9.0]);
    // nothing to transfer to: the command refuses
    st.selection.click(a);
    assert!(apply_tool(&mut st).is_err());
}

#[test]
fn apply_array_on_an_editable_cube_previews_and_applies() {
    let mut st = state();
    create(&mut st, "forge.cube");
    execute(&mut st, "mesh.make_editable", json!({})).unwrap();
    let id = st.selection.active.unwrap();
    st.doc
        .transact("sel", |tx| {
            tx.set_param(id, "selection.polys", Value::IndexList(vec![4]))
        })
        .unwrap();
    activate_tool(&mut st, "array");
    let before = steps(&st);
    let (_, preview) = tool_preview(&st).unwrap();
    assert_eq!((preview.points.len(), preview.polys.len()), (16, 8));
    assert_eq!(steps(&st), before, "the preview does not write");
    apply_tool(&mut st).unwrap();
    assert_eq!(steps(&st), before + 1);
    match st.doc.scene().param(id, "mesh") {
        Some(Value::Mesh(m)) => assert_eq!((m.points.len(), m.polys.len()), (16, 8)),
        _ => panic!("mesh"),
    }
}

#[test]
fn the_mesh_menu_lists_them_and_the_panel_shows_the_settings() {
    let mut st = state();
    create(&mut st, "forge.cube");
    let a = st.selection.active.unwrap();
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), Editor::new(st));
    h.run();
    h.get_by_label("Mesh").click();
    h.run();
    h.get_by_label("Array");
    h.get_by_label("Arrange");
    h.get_by_label("Randomize");
    h.get_by_label("Transfer");
    h.get_by_label("Duplicate").click();
    h.run();
    assert!(h.query_all_by_label("Copies").count() > 0);
    assert!(h.query_all_by_label("Offset").count() > 0);
    h.get_by_label("Apply").click();
    h.run();
    assert_eq!(h.state().state.doc.scene().roots().len(), 2);
    assert_eq!(pos(&h.state().state, a), [0.0; 3]);
    h.get_by_label("Done").click();
    h.run();
    assert!(h.state().state.tool.is_none());
}
