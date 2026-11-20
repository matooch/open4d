//! The Weight Manager panel and a paint stroke (docs/specs/RIG.md section 9.4, brief 102).

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_app::App;
use forge_core::{EvalCtx, NodeId, Value};
use forge_rig::WeightMap;
use forge_rig::weights::{find_weight_tag, read_weights};
use forge_ui::app::Editor;
use forge_ui::dock::Panel;
use forge_ui::state::EditorState;
use forge_ui::weight_manager::{end_stroke, paint_dab};
use serde_json::json;

fn rigged() -> (EditorState, Vec<NodeId>, NodeId) {
    let mut app = App::new_default();
    let out = app
        .run(
            "joint.create_chain",
            json!({"points": [[0, 0, 0], [0, 100, 0], [0, 200, 0]], "align": "y", "prefix": "Bone"}),
        )
        .unwrap();
    let chain: Vec<NodeId> = out["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| NodeId(v.as_u64().unwrap()))
        .collect();
    let cube = NodeId(
        app.run("object.create", json!({"type": "forge.cube"}))
            .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    let n = forge_generators::evaluate(app.doc.scene(), cube, &EvalCtx::default())
        .unwrap()
        .unwrap()
        .points
        .len();
    app.doc
        .transact("tag", |tx| {
            let tag = tx.attach_tag("forge.weight_tag", cube, None)?;
            tx.set_param(tag, "joints", Value::LinkList(vec![chain[0], chain[1]]))?;
            tx.set_param(
                tag,
                "weights",
                WeightMap::from_dense(&vec![vec![1.0, 0.0]; n]).to_value(),
            )
        })
        .unwrap();
    let mut st = EditorState::with_app(app);
    st.selection.click(cube);
    (st, chain, cube)
}

fn weight(st: &EditorState, cube: NodeId, p: u32, slot: u32) -> f64 {
    let tag = find_weight_tag(st.doc.scene(), cube).unwrap();
    read_weights(st.doc.scene(), tag)
        .get(p)
        .iter()
        .find(|(s, _)| *s == slot)
        .map_or(0.0, |(_, w)| *w)
}

fn harness(st: EditorState) -> Harness<'static, Editor> {
    let mut editor = Editor::new(st);
    editor.state.dock.open(Panel::WeightManager);
    let mut h = Harness::builder()
        .with_size([1400.0, 1800.0])
        .build_ui_state(|ui, editor: &mut Editor| editor.show(ui, None), editor);
    h.run();
    h
}

#[test]
fn the_panel_lists_the_tag_joints_and_selects_one() {
    let (st, chain, _) = rigged();
    let mut h = harness(st);
    h.get_by_label("Bone1 (8)");
    h.get_by_label("Bone2 (0)").click();
    h.run();
    assert_eq!(h.state().state.weight_panel.joint, Some(chain[1]));
}

#[test]
fn a_paint_stroke_over_several_dabs_is_one_undo_step() {
    let (mut st, chain, cube) = rigged();
    st.weight_panel.joint = Some(chain[1]);
    st.weight_panel.radius = 1e6;
    st.weight_panel.strength = 0.25;
    st.weight_panel.falloff = 2; // constant
    let before = st.doc.undo_depth();
    let size = [640.0, 480.0];
    // the default camera looks at the cube: the centre of the view hits it
    assert!(paint_dab(&mut st, [320.0, 240.0], size).unwrap());
    assert!(paint_dab(&mut st, [330.0, 240.0], size).unwrap());
    end_stroke(&mut st);
    assert_eq!(st.doc.undo_depth(), before + 1);
    assert!((weight(&st, cube, 0, 1) - 0.5).abs() < 1e-12);
    assert!((weight(&st, cube, 0, 0) - 0.5).abs() < 1e-12);
    assert!(st.doc.undo());
    assert_eq!(weight(&st, cube, 0, 1), 0.0);
    // a ray that misses the cube paints nothing
    assert!(!paint_dab(&mut st, [1.0, 1.0], size).unwrap());
}

#[test]
fn panel_buttons_run_the_weight_commands() {
    let (mut st, chain, cube) = rigged();
    st.weight_panel.joint = Some(chain[1]);
    let mut h = harness(st);
    h.get_by_label("Normalise").click();
    h.run();
    assert!((weight(&h.state().state, cube, 0, 0) - 1.0).abs() < 1e-12);
    h.get_by_label("Smooth").click();
    h.run();
    assert!((weight(&h.state().state, cube, 3, 0) - 1.0).abs() < 1e-12);
    h.get_by_label("Paint").click();
    h.run();
    assert!(h.state().state.weight_panel.painting);
}
