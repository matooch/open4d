//! The Null's display in the editor viewport overlay (docs/specs/SCENE-OBJECTS.md S9, brief 046).

use std::collections::BTreeSet;

use forge_app::App;
use forge_core::{EvalCtx, NodeId};
use forge_ui::scene_overlay::SceneOverlay;
use forge_viewport::camera::{AxisView, Camera};
use forge_viewport::render::Palette;
use forge_viewport::scene_view::OBJECT_COLOR;
use serde_json::json;

const SIZE: [f64; 2] = [640.0, 480.0];

fn null_with(app: &mut App, params: &[(&str, serde_json::Value)]) -> NodeId {
    let id = NodeId(
        app.run("object.create", json!({"type": "forge.null"}))
            .unwrap()["node"]
            .as_u64()
            .unwrap(),
    );
    for (k, v) in params {
        app.run("param.set", json!({"node": id.0, "param": k, "value": v}))
            .unwrap();
    }
    id
}

fn overlay(app: &App, sel: &BTreeSet<NodeId>) -> SceneOverlay {
    SceneOverlay::collect(app.doc.scene(), sel, &EvalCtx::default())
}

fn front() -> Camera {
    let mut c = Camera::default();
    c.look_along(AxisView::PosZ);
    c
}

#[test]
fn a_rectangle_null_draws_four_lines_in_the_object_colour() {
    let mut app = App::new_default();
    null_with(
        &mut app,
        &[("shape", json!("rectangle")), ("radius", json!(100.0))],
    );
    let lines = overlay(&app, &BTreeSet::new()).lines(&Palette::default(), &front(), SIZE);
    assert_eq!(lines.len(), 4);
    assert!(lines.iter().all(|l| l.color == OBJECT_COLOR));
    let half = 100.0 / 2f64.sqrt();
    for l in &lines {
        for p in [l.a, l.b] {
            assert!(
                (p.x.abs() - half).abs() < 1e-9 && (p.y.abs() - half).abs() < 1e-9,
                "{p:?}"
            );
        }
    }
}

#[test]
fn custom_colour_and_selection_colour() {
    let mut app = App::new_default();
    let n = null_with(
        &mut app,
        &[
            ("shape", json!("locator")),
            ("display_color", json!("custom")),
            ("color", json!([0.25, 0.5, 0.75])),
        ],
    );
    let pal = Palette::default();
    let lines = overlay(&app, &BTreeSet::new()).lines(&pal, &front(), SIZE);
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|l| l.color == [0.25, 0.5, 0.75]));
    let sel: BTreeSet<NodeId> = [n].into();
    let lines = overlay(&app, &sel).lines(&pal, &front(), SIZE);
    assert!(lines.iter().all(|l| l.color == pal.selection));
}

#[test]
fn the_default_dot_is_a_small_cross_and_none_draws_nothing() {
    let mut app = App::new_default();
    let n = null_with(&mut app, &[]);
    let lines = overlay(&app, &BTreeSet::new()).lines(&Palette::default(), &front(), SIZE);
    assert_eq!(lines.len(), 2);
    for l in &lines {
        assert!(l.a.length() < 5.0 && l.b.length() < 5.0);
    }
    app.run(
        "param.set",
        json!({"node": n.0, "param": "shape", "value": "none"}),
    )
    .unwrap();
    assert!(
        overlay(&app, &BTreeSet::new())
            .lines(&Palette::default(), &front(), SIZE)
            .is_empty()
    );
}

#[test]
fn a_camera_oriented_circle_faces_the_viewport_camera() {
    let mut app = App::new_default();
    null_with(
        &mut app,
        &[("shape", json!("circle")), ("radius", json!(100.0))],
    );
    let o = overlay(&app, &BTreeSet::new());
    let mut top = Camera::default();
    top.look_along(AxisView::PosY);
    for cam in [front(), top] {
        let lines = o.lines(&Palette::default(), &cam, SIZE);
        assert_eq!(lines.len(), 64);
        let n = cam.forward_world();
        for l in &lines {
            assert!(l.a.dot(n).abs() < 1e-9, "in the view plane");
            assert!((l.a.length() - 100.0).abs() < 1e-9);
        }
    }
}

#[test]
fn a_fixed_orientation_moves_with_the_null_and_hidden_nulls_draw_nothing() {
    let mut app = App::new_default();
    let n = null_with(
        &mut app,
        &[
            ("shape", json!("circle")),
            ("orientation", json!("y")),
            ("radius", json!(50.0)),
            ("transform.pos", json!([100.0, 20.0, 0.0])),
        ],
    );
    let lines = overlay(&app, &BTreeSet::new()).lines(&Palette::default(), &front(), SIZE);
    assert_eq!(lines.len(), 64);
    // the XZ plane through the Null's position
    assert!(lines.iter().all(|l| (l.a.y - 20.0).abs() < 1e-9));
    assert!(lines.iter().any(|l| (l.a.x - 150.0).abs() < 1e-9));
    app.run(
        "param.set",
        json!({"node": n.0, "param": "object.editor_vis", "value": "off"}),
    )
    .unwrap();
    assert!(
        overlay(&app, &BTreeSet::new())
            .lines(&Palette::default(), &front(), SIZE)
            .is_empty()
    );
}
