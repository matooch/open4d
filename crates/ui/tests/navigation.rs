//! Viewport navigation (docs/specs/UI.md §6.1, VIEWPORT.md §7, COMMANDS.md §6; brief 153):
//! the input mapping (pure), the `view.*` camera commands, and drags and keys in the editor
//! driven with egui_kittest (no GPU: navigation runs before the wgpu check).

mod common;

use egui::{Key, Modifiers, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::navigation::scene_bounds;
use forge_ui::state::EditorState;
use forge_ui::viewport_panel::{NavAction, NavInput, NavMode, apply_nav, nav_action};
use forge_viewport::camera::Projection;
use glam::DVec3;
use serde_json::{Value, json};

// ---- input mapping (pure) -----------------------------------------------------------

fn input() -> NavInput {
    NavInput {
        drag: [10.0, -4.0],
        cursor: [30.0, 40.0],
        ..NavInput::default()
    }
}

#[test]
fn every_gesture_maps_to_its_action() {
    let base = input();
    assert_eq!(nav_action(&base), NavAction::None);
    let alt = NavInput { alt: true, ..base };
    // Alt + left / middle / right drag
    assert_eq!(
        nav_action(&NavInput {
            primary: true,
            ..alt
        }),
        NavAction::Orbit(10.0, -4.0)
    );
    assert_eq!(
        nav_action(&NavInput {
            middle: true,
            ..alt
        }),
        NavAction::Pan(10.0, -4.0)
    );
    assert_eq!(
        nav_action(&NavInput {
            secondary: true,
            ..alt
        }),
        NavAction::Dolly(10.0)
    );
    // middle drag alone pans; left drag alone does not navigate
    assert_eq!(
        nav_action(&NavInput {
            middle: true,
            ..base
        }),
        NavAction::Pan(10.0, -4.0)
    );
    assert_eq!(
        nav_action(&NavInput {
            primary: true,
            ..base
        }),
        NavAction::None
    );
    // hold keys + primary drag
    for (mode, want) in [
        (NavMode::Pan, NavAction::Pan(10.0, -4.0)),
        (NavMode::Orbit, NavAction::Orbit(10.0, -4.0)),
        (NavMode::Dolly, NavAction::Dolly(10.0)),
    ] {
        let held = NavInput {
            held: Some(mode),
            primary: true,
            ..base
        };
        assert_eq!(nav_action(&held), want);
        assert!(held.navigating());
        // without the button it does nothing
        assert_eq!(
            nav_action(&NavInput {
                primary: false,
                ..held
            }),
            NavAction::None
        );
    }
    // the wheel zooms toward the cursor and wins over drags
    let NavAction::ZoomAt(f, at) = nav_action(&NavInput {
        scroll: 50.0,
        middle: true,
        ..base
    }) else {
        panic!("wheel")
    };
    assert!(f < 1.0 && at == [30.0, 40.0]);
    let NavAction::ZoomAt(f, _) = nav_action(&NavInput {
        scroll: -50.0,
        ..base
    }) else {
        panic!("wheel")
    };
    assert!(f > 1.0);
}

#[test]
fn apply_nav_runs_the_pure_camera_functions() {
    let mut st = common::state();
    let before = st.camera;
    apply_nav(&mut st.camera, NavAction::Orbit(50.0, 0.0), [800.0, 600.0]);
    assert_eq!(st.camera.distance, before.distance);
    assert_eq!(st.camera.target, before.target);
    assert!(st.camera.yaw != before.yaw);
    let c = st.camera;
    apply_nav(
        &mut st.camera,
        NavAction::ZoomAt(0.5, [100.0, 100.0]),
        [800.0, 600.0],
    );
    assert!((st.camera.distance - c.distance * 0.5).abs() < 1e-9);
    apply_nav(&mut st.camera, NavAction::None, [800.0, 600.0]);
}

// ---- commands -----------------------------------------------------------------------

fn run(st: &mut EditorState, id: &str, args: Value) -> Result<Value, String> {
    execute(st, id, args)
}

fn cube_at(st: &mut EditorState, pos: [f64; 3]) -> forge_core::NodeId {
    let id = common::create(st, "forge.cube");
    run(st, "transform.set", json!({"node": id.0, "pos": pos})).unwrap();
    id
}

fn on_screen(st: &EditorState, bounds: (DVec3, DVec3), w: f64, h: f64) {
    for i in 0..8 {
        let p = DVec3::new(
            if i & 1 == 1 { bounds.1.x } else { bounds.0.x },
            if i & 2 == 2 { bounds.1.y } else { bounds.0.y },
            if i & 4 == 4 { bounds.1.z } else { bounds.0.z },
        );
        let s = st.camera.project(p, w, h).expect("in front");
        assert!(s.x >= 0.0 && s.x <= w && s.y >= 0.0 && s.y <= h, "{s}");
    }
}

fn bounds_of(
    st: &EditorState,
    only: Option<&std::collections::BTreeSet<forge_core::NodeId>>,
) -> (DVec3, DVec3) {
    let scene = st.evaluated();
    scene_bounds(&scene, &forge_core::EvalCtx::default(), only).expect("geometry")
}

#[test]
fn frame_all_and_selected_fit_their_bounds() {
    let mut st = common::state();
    let a = cube_at(&mut st, [1000.0, 0.0, 0.0]);
    let b = cube_at(&mut st, [-600.0, 300.0, 200.0]);
    st.view_aspect = 4.0 / 3.0;

    run(&mut st, "view.frame_all", json!({})).unwrap();
    let all = bounds_of(&st, None);
    assert!((all.0.x + 700.0).abs() < 1e-9 && (all.1.x - 1100.0).abs() < 1e-9);
    on_screen(&st, all, 800.0, 600.0);
    assert!((st.camera.target_world() - (all.0 + all.1) / 2.0).length() < 1e-9);

    // frame selected: only b
    st.selection.click(b);
    run(&mut st, "view.frame_selected", json!({})).unwrap();
    let only_b = std::collections::BTreeSet::from([b]);
    let bb = bounds_of(&st, Some(&only_b));
    on_screen(&st, bb, 800.0, 600.0);
    assert!((st.camera.target_world() - (bb.0 + bb.1) / 2.0).length() < 1e-9);
    assert!(st.camera.distance < 1000.0, "closer than for both cubes");

    // nothing selected frames everything again
    st.selection.clear();
    run(&mut st, "view.frame_selected", json!({})).unwrap();
    assert!((st.camera.target_world() - (all.0 + all.1) / 2.0).length() < 1e-9);
    let _ = a;
}

#[test]
fn frame_selected_of_a_null_recentres_and_an_empty_scene_resets() {
    let mut st = common::state();
    run(&mut st, "view.frame_all", json!({})).unwrap();
    assert_eq!(st.camera, EditorState::default_camera());
    st.camera.orbit(0.3, 0.1);
    let n = common::create(&mut st, "forge.null");
    run(
        &mut st,
        "transform.set",
        json!({"node": n.0, "pos": [10.0, 20.0, 30.0]}),
    )
    .unwrap();
    let before = st.camera;
    run(&mut st, "view.frame_selected", json!({})).unwrap();
    assert!((st.camera.target_world() - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-9);
    assert_eq!(st.camera.distance, before.distance);
    assert_eq!(st.camera.yaw, before.yaw);
}

#[test]
fn camera_commands_are_not_undo_steps() {
    let mut st = common::state();
    cube_at(&mut st, [0.0, 0.0, 0.0]);
    let steps = st.doc.history().undo_labels().len();
    for (id, args) in [
        ("view.frame_all", json!({})),
        ("view.frame_selected", json!({})),
        ("view.set_view", json!({"view": "top"})),
        ("view.set_camera", json!({"distance": 50.0, "yaw": 10.0})),
    ] {
        run(&mut st, id, args).unwrap();
    }
    assert_eq!(st.doc.history().undo_labels().len(), steps);
    // undo undoes the cube creation step, never a camera move
    let cam = st.camera;
    run(&mut st, "edit.undo", json!({})).unwrap();
    assert_eq!(st.camera, cam);
    for id in [
        "view.frame_all",
        "view.frame_selected",
        "view.set_view",
        "view.set_camera",
        "view.get_camera",
    ] {
        assert!(!st.commands.get(id).expect(id).undoable(), "{id}");
    }
}

#[test]
fn set_view_switches_projection_and_side() {
    let mut st = common::state();
    run(&mut st, "view.set_view", json!({"view": "top"})).unwrap();
    assert!(matches!(
        st.camera.projection,
        Projection::Orthographic { .. }
    ));
    let cam = run(&mut st, "view.get_camera", json!({})).unwrap();
    assert_eq!(cam["projection"], "orthographic");
    assert!(cam["eye"][1].as_f64().unwrap() > 0.0, "{cam}");
    run(&mut st, "view.set_view", json!({"view": "right"})).unwrap();
    let cam = run(&mut st, "view.get_camera", json!({})).unwrap();
    assert!(cam["eye"][0].as_f64().unwrap() > 0.0);
    run(&mut st, "view.set_view", json!({"view": "perspective"})).unwrap();
    assert!(matches!(
        st.camera.projection,
        Projection::Perspective { .. }
    ));
    assert!(run(&mut st, "view.set_view", json!({"view": "sideways"})).is_err());
}

#[test]
fn set_camera_changes_only_the_given_fields_and_validates() {
    let mut st = common::state();
    run(
        &mut st,
        "view.set_camera",
        json!({"target": [1.0, 2.0, 3.0], "distance": 80.0, "yaw": 90.0, "pitch": 30.0, "fov": 60.0}),
    )
    .unwrap();
    let c = run(&mut st, "view.get_camera", json!({})).unwrap();
    assert_eq!(c["target"], json!([1.0, 2.0, 3.0]));
    assert_eq!(c["distance"], 80.0);
    assert!((c["yaw"].as_f64().unwrap() - 90.0).abs() < 1e-9);
    assert!((c["pitch"].as_f64().unwrap() - 30.0).abs() < 1e-9);
    assert!((c["fov"].as_f64().unwrap() - 60.0).abs() < 1e-9);
    run(
        &mut st,
        "view.set_camera",
        json!({"projection": "orthographic"}),
    )
    .unwrap();
    let c = run(&mut st, "view.get_camera", json!({})).unwrap();
    assert_eq!(c["projection"], "orthographic");
    assert_eq!(c["distance"], 80.0);
    assert!(c["height"].as_f64().unwrap() > 0.0);
    let before = st.camera;
    for bad in [
        json!({"distance": 0.0}),
        json!({"distance": -3.0}),
        json!({"fov": 200.0}),
        json!({"height": 0.0}),
        json!({"target": [1.0, 2.0]}),
        json!({"pitch": "up"}),
        json!({"typo": 1}),
    ] {
        assert!(
            run(&mut st, "view.set_camera", bad.clone()).is_err(),
            "{bad}"
        );
        assert_eq!(st.camera, before, "{bad}");
    }
    // pitch is clamped short of the pole
    run(&mut st, "view.set_camera", json!({"pitch": 200.0})).unwrap();
    assert!(st.camera.pitch < std::f64::consts::FRAC_PI_2);
}

// ---- kittest: gestures and keys -----------------------------------------------------

fn harness(editor: Editor) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    h
}

/// A point inside the viewport.
fn viewport_point(h: &Harness<'_, Editor>) -> Pos2 {
    h.get_by_label("Viewport unavailable").rect().center() + egui::vec2(20.0, 20.0)
}

fn drag(h: &mut Harness<'_, Editor>, button: egui::PointerButton, mods: Modifiers, by: egui::Vec2) {
    let start = viewport_point(h);
    h.event(egui::Event::ModifiersChanged(mods));
    h.hover_at(start);
    h.run();
    h.event(egui::Event::PointerButton {
        pos: start,
        button,
        pressed: true,
        modifiers: mods,
    });
    h.run();
    for k in 1..=4 {
        h.hover_at(start + by * (k as f32 / 4.0));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: start + by,
        button,
        pressed: false,
        modifiers: mods,
    });
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

#[test]
fn alt_drags_orbit_pan_and_dolly_the_camera() {
    let mut h = harness(Editor::new(common::state()));
    let c0 = h.state().state.camera;

    drag(
        &mut h,
        egui::PointerButton::Primary,
        Modifiers::ALT,
        egui::vec2(60.0, 20.0),
    );
    let c1 = h.state().state.camera;
    assert!(c1.yaw < c0.yaw && c1.pitch > c0.pitch, "{c0:?} {c1:?}");
    assert_eq!((c1.target, c1.distance), (c0.target, c0.distance));

    drag(
        &mut h,
        egui::PointerButton::Middle,
        Modifiers::ALT,
        egui::vec2(40.0, 0.0),
    );
    let c2 = h.state().state.camera;
    assert!(c2.target != c1.target);
    assert!((c2.target - c1.target).dot(c1.forward()).abs() < 1e-6);
    assert_eq!((c2.yaw, c2.distance), (c1.yaw, c1.distance));

    drag(
        &mut h,
        egui::PointerButton::Secondary,
        Modifiers::ALT,
        egui::vec2(50.0, 0.0),
    );
    let c3 = h.state().state.camera;
    assert!(c3.distance < c2.distance);
    assert_eq!(c3.target, c2.target);
    // camera moves never reach the undo history
    assert!(h.state().state.doc.history().undo_labels().is_empty());
}

#[test]
fn a_held_hold_key_turns_a_plain_drag_into_navigation() {
    let mut h = harness(Editor::new(common::state()));
    let c0 = h.state().state.camera;
    h.key_down(Key::V);
    drag(
        &mut h,
        egui::PointerButton::Primary,
        Modifiers::NONE,
        egui::vec2(40.0, 0.0),
    );
    h.key_up(Key::V);
    h.run();
    assert!(h.state().state.camera.yaw != c0.yaw);
    // without the key the same drag leaves the camera alone
    let c1 = h.state().state.camera;
    drag(
        &mut h,
        egui::PointerButton::Primary,
        Modifiers::NONE,
        egui::vec2(40.0, 0.0),
    );
    assert_eq!(h.state().state.camera, c1);
}

#[test]
fn wheel_zooms_toward_the_cursor() {
    let mut h = harness(Editor::new(common::state()));
    let before = h.state().state.camera;
    let at = viewport_point(&h);
    h.hover_at(at);
    h.run();
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    });
    h.run();
    let after = h.state().state.camera;
    assert!(after.distance < before.distance);
    // the cursor is off-centre, so the pivot moved
    assert!(after.target != before.target);
}

#[test]
fn frame_and_view_keys_run_their_commands() {
    let mut st = common::state();
    cube_at(&mut st, [800.0, 0.0, 0.0]);
    let mut h = harness(Editor::new(st));
    h.key_press(Key::H);
    h.run();
    let all = h.state().state.camera;
    assert!((all.target_world().x - 800.0).abs() < 1e-6);
    h.state_mut().state.camera.orbit(0.2, 0.0);
    h.key_press(Key::S);
    h.run();
    assert!((h.state().state.camera.target_world().x - 800.0).abs() < 1e-6);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num4);
    h.run();
    assert!(matches!(
        h.state().state.camera.projection,
        Projection::Orthographic { .. }
    ));
    h.key_press_modifiers(Modifiers::COMMAND, Key::Num1);
    h.run();
    assert!(matches!(
        h.state().state.camera.projection,
        Projection::Perspective { .. }
    ));
    // none of these were undo steps beyond the setup ones
    let labels = h.state().state.doc.history().undo_labels();
    assert!(
        labels.iter().all(|l| !l.to_lowercase().contains("view")),
        "{labels:?}"
    );
}
