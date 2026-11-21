//! Camera, light, environment, floor and sky objects in the editor
//! (docs/specs/SCENE-OBJECTS.md S7, brief 047).

mod common;

use common::{create, state};
use forge_ui::commands::{SCENE_TYPES, execute, scene_menu};
use forge_viewport::scene_view::{Lens, look_through};
use glam::DVec3;
use serde_json::json;

#[test]
fn the_create_menu_lists_the_five_scene_objects() {
    let st = state();
    let labels: Vec<String> = scene_menu(st.doc.scene().registry())
        .into_iter()
        .map(|e| e.label)
        .collect();
    assert_eq!(labels, ["Camera", "Light", "Environment", "Floor", "Sky"]);
    for t in SCENE_TYPES {
        assert!(st.doc.scene().registry().get(t).is_some(), "{t}");
    }
}

#[test]
fn look_through_camera_sets_the_viewport_camera() {
    let mut st = state();
    let c = create(&mut st, "forge.camera");
    for (p, v) in [
        ("transform.pos", json!([30.0, 40.0, -600.0])),
        ("transform.rot", json!([-15.0, 5.0, 0.0])),
        ("fov_horizontal", json!(90.0)),
    ] {
        execute(
            &mut st,
            "param.set",
            json!({"node": c.0, "param": p, "value": v}),
        )
        .unwrap();
    }
    execute(
        &mut st,
        "view.look_through_camera",
        json!({"aspect": 4.0 / 3.0}),
    )
    .unwrap();
    // the same projection as the library function (18 mm lens on the 36 mm gate)
    let data = forge_generators::scene_objects::camera_data(st.doc.scene(), c).unwrap();
    let want = look_through(
        data.world,
        Lens::Perspective {
            focal_mm: 18.0,
            sensor_mm: 36.0,
        },
        4.0 / 3.0,
        2000.0,
    );
    for q in [DVec3::ZERO, DVec3::new(100.0, -50.0, 50.0)] {
        let (a, b) = (
            st.camera.project(q, 320.0, 240.0).unwrap(),
            want.project(q, 320.0, 240.0).unwrap(),
        );
        assert!((a - b).length() < 1e-6, "{a:?} vs {b:?}");
    }
    // a scene without camera is refused
    let mut empty = state();
    assert!(execute(&mut empty, "view.look_through_camera", json!({})).is_err());
}

#[test]
fn the_render_settings_command_stores_and_resets() {
    let mut st = state();
    assert!(st.doc.scene().render_config().is_none());
    execute(
        &mut st,
        "render.settings",
        json!({"width": 640, "height": 360}),
    )
    .unwrap();
    assert_eq!(st.doc.scene().render_config().unwrap().width, 640);
    assert!(execute(&mut st, "render.settings", json!({"filter": "nope"})).is_err());
    assert!(execute(&mut st, "render.settings", json!({"width": 0})).is_err());
    execute(&mut st, "render.settings", json!({"reset": true})).unwrap();
    assert!(st.doc.scene().render_config().is_none());
    assert!(st.doc.undo());
    assert!(st.doc.scene().render_config().is_some());
}

#[test]
fn the_overlay_draws_camera_light_and_floor_gizmos() {
    use forge_ui::scene_overlay::SceneOverlay;
    use std::collections::BTreeSet;
    let mut st = state();
    let none = SceneOverlay::collect(
        st.doc.scene(),
        &BTreeSet::new(),
        &forge_core::EvalCtx::at_frame(0, 30.0),
    );
    assert!(none.objects.is_empty());
    for t in ["forge.camera", "forge.light", "forge.floor", "forge.sky"] {
        create(&mut st, t);
    }
    let o = SceneOverlay::collect(
        st.doc.scene(),
        &st.selection.nodes,
        &forge_core::EvalCtx::at_frame(0, 30.0),
    );
    // frustum 10 + omni 72 + floor grid 22; the sky has no gizmo
    assert_eq!(o.objects.len(), 10 + 72 + 22);
}
