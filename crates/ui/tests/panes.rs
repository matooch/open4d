//! Viewport panes, the 4-view layout, lights and anti-aliasing in the editor (docs/specs/UI.md
//! §6.2, COMMANDS.md §6; brief 143). State and commands run without a GPU; the screenshot
//! composite needs an adapter (skips without one unless FORGE_REQUIRE_GPU=1); egui_kittest drives
//! the window without a render state.

mod common;

use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_command::CommandContext;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::panes::{ORTHO_SHADING, Views, display_settings, scene_lights};
use forge_ui::state::EditorState;
use forge_viewport::camera::Projection;
use forge_viewport::lights::{Light, Lighting, MAX_LIGHTS};
use forge_viewport::panes::{Layout, Pane, pane_camera};
use forge_viewport::render::{Msaa, Shading};
use glam::DVec3;
use serde_json::{Value, json};

fn run(st: &mut EditorState, id: &str, args: Value) -> Result<Value, String> {
    execute(st, id, args)
}

fn is_ortho(c: &forge_viewport::camera::Camera) -> bool {
    matches!(c.projection, Projection::Orthographic { .. })
}

#[test]
fn panes_start_as_perspective_plus_three_line_views() {
    let st = common::state();
    assert_eq!(st.views.layout, Layout::Single);
    assert_eq!(st.views.active, Pane::Perspective);
    assert_eq!(st.views.maximized, None);
    assert_eq!(st.camera, EditorState::default_camera());
    assert_eq!(st.pane_camera(Pane::Perspective), st.camera);
    for p in [Pane::Top, Pane::Front, Pane::Right] {
        let c = st.pane_camera(p);
        assert!(is_ortho(&c), "{p:?}");
        assert_eq!(c, pane_camera(p, &EditorState::default_camera()));
        assert_eq!(st.pane_shading(p), ORTHO_SHADING);
    }
    assert_eq!(st.pane_shading(Pane::Perspective), Shading::SmoothWire);
    assert_eq!(st.views.visible(), vec![Pane::Perspective]);
    assert_eq!(st.views, Views::new(&EditorState::default_camera()));
}

#[test]
fn view_layout_sets_and_toggles() {
    let mut st = common::state();
    run(&mut st, "view.layout", json!({"layout": "four"})).unwrap();
    assert_eq!(st.views.layout, Layout::Four);
    assert_eq!(
        st.views.visible(),
        vec![Pane::Perspective, Pane::Top, Pane::Right, Pane::Front]
    );
    // setting the same layout again changes nothing
    run(&mut st, "view.layout", json!({"layout": "four"})).unwrap();
    assert_eq!(st.views.layout, Layout::Four);
    run(&mut st, "view.layout", json!({})).unwrap();
    assert_eq!(st.views.layout, Layout::Single);
    run(&mut st, "view.layout", json!({})).unwrap();
    assert_eq!(st.views.layout, Layout::Four);
    run(&mut st, "view.layout", json!({"layout": "single"})).unwrap();
    assert_eq!(st.views.layout, Layout::Single);
    // bad arguments are refused and change nothing
    for bad in [json!({"layout": "six"}), json!({"pane": "top"})] {
        assert!(run(&mut st, "view.layout", bad).is_err());
    }
    assert_eq!(st.views.layout, Layout::Single);
    // none of it is an undo step
    assert!(st.doc.history().undo_labels().is_empty());
}

#[test]
fn maximize_toggles_a_pane_in_the_four_view_layout_only() {
    let mut st = common::state();
    // single layout: nothing to maximise
    run(&mut st, "view.maximize", json!({"pane": "top"})).unwrap();
    assert_eq!(st.views.maximized, None);
    assert_eq!(st.views.active, Pane::Perspective);

    run(&mut st, "view.layout", json!({"layout": "four"})).unwrap();
    run(&mut st, "view.maximize", json!({"pane": "front"})).unwrap();
    assert_eq!(st.views.maximized, Some(Pane::Front));
    assert_eq!(st.views.active, Pane::Front, "the maximised pane is active");
    assert_eq!(st.views.visible(), vec![Pane::Front]);
    // the working camera is the front pane's
    assert_eq!(
        st.camera,
        pane_camera(Pane::Front, &EditorState::default_camera())
    );
    run(&mut st, "view.maximize", json!({"pane": "front"})).unwrap();
    assert_eq!(st.views.maximized, None);
    assert_eq!(st.views.visible().len(), 4);
    // without a pane: the active one
    run(&mut st, "view.maximize", json!({})).unwrap();
    assert_eq!(st.views.maximized, Some(Pane::Front));
    // maximising another pane replaces the first
    run(&mut st, "view.maximize", json!({"pane": "top"})).unwrap();
    assert_eq!(st.views.maximized, Some(Pane::Top));
    // going back to one view forgets it
    run(&mut st, "view.layout", json!({"layout": "single"})).unwrap();
    assert_eq!(st.views.maximized, None);
    assert_eq!(
        st.views.visible(),
        vec![Pane::Top],
        "single shows the active pane"
    );
}

#[test]
fn shading_and_views_are_per_pane() {
    let mut st = common::state();
    let persp = st.camera;
    run(
        &mut st,
        "view.shading",
        json!({"mode": "constant", "pane": "front"}),
    )
    .unwrap();
    assert_eq!(st.pane_shading(Pane::Front), Shading::Constant);
    assert_eq!(
        st.shading,
        Shading::SmoothWire,
        "the active pane is untouched"
    );
    assert_eq!(st.views.active, Pane::Perspective, "and stays active");
    assert_eq!(st.pane_shading(Pane::Top), ORTHO_SHADING);
    // no pane: the active one
    run(&mut st, "view.shading", json!({"mode": "hidden_line"})).unwrap();
    assert_eq!(st.shading, Shading::HiddenLine);
    assert_eq!(st.pane_shading(Pane::Front), Shading::Constant);
    // every new mode is reachable
    for (mode, want) in [
        ("quick", Shading::Quick),
        ("bounding_box", Shading::BoundingBox),
        ("flat_wire", Shading::FlatWire),
    ] {
        run(
            &mut st,
            "view.shading",
            json!({"mode": mode, "pane": "right"}),
        )
        .unwrap();
        assert_eq!(st.pane_shading(Pane::Right), want);
    }

    // set_view with a pane moves that pane's camera only
    run(
        &mut st,
        "view.set_view",
        json!({"view": "back", "pane": "top"}),
    )
    .unwrap();
    let top = st.pane_camera(Pane::Top);
    assert!(is_ortho(&top));
    assert!(
        (top.forward_world() - DVec3::NEG_Z).length() < 1e-6,
        "{:?}",
        top.forward_world()
    );
    assert_ne!(top, pane_camera(Pane::Top, &EditorState::default_camera()));
    assert_eq!(st.camera, persp, "the perspective pane did not move");
    run(
        &mut st,
        "view.set_view",
        json!({"view": "perspective", "pane": "front"}),
    )
    .unwrap();
    assert!(!is_ortho(&st.pane_camera(Pane::Front)));
    assert_eq!(st.views.active, Pane::Perspective);
    // without a pane, the active pane's camera
    run(&mut st, "view.set_view", json!({"view": "top"})).unwrap();
    assert!(is_ortho(&st.camera));
    // unknown pane
    assert!(
        run(
            &mut st,
            "view.shading",
            json!({"mode": "smooth", "pane": "back"})
        )
        .is_err()
    );
}

#[test]
fn activating_a_pane_keeps_every_camera() {
    let mut st = common::state();
    let persp = st.camera;
    st.activate(Pane::Top);
    assert_eq!(st.views.active, Pane::Top);
    assert!(is_ortho(&st.camera));
    st.camera.orbit(0.3, 0.1);
    let top = st.camera;
    st.shading = Shading::Quick;
    st.activate(Pane::Perspective);
    assert_eq!(st.camera, persp);
    assert_eq!(st.shading, Shading::SmoothWire);
    assert_eq!(st.pane_camera(Pane::Top), top);
    assert_eq!(st.pane_shading(Pane::Top), Shading::Quick);
    // activating the active pane is a no-op
    st.activate(Pane::Perspective);
    assert_eq!(st.camera, persp);
    // commands act on the active pane: frame / set_camera
    st.activate(Pane::Right);
    run(&mut st, "view.set_camera", json!({"distance": 77.0})).unwrap();
    assert_eq!(st.camera.distance, 77.0);
    assert_eq!(st.pane_camera(Pane::Perspective), persp);
    assert_eq!(
        run(&mut st, "view.get_camera", json!({})).unwrap()["distance"],
        77.0
    );
}

#[test]
fn opening_four_views_lets_untouched_panes_follow_the_perspective_pane() {
    let mut st = common::state();
    run(
        &mut st,
        "view.set_camera",
        json!({"target": [10.0, 20.0, 30.0], "distance": 400.0}),
    )
    .unwrap();
    // the front pane was set by hand before: it keeps its own camera
    run(
        &mut st,
        "view.set_view",
        json!({"view": "back", "pane": "front"}),
    )
    .unwrap();
    let front = st.pane_camera(Pane::Front);
    run(&mut st, "view.layout", json!({"layout": "four"})).unwrap();
    let top = st.pane_camera(Pane::Top);
    assert!(
        (top.target_world() - DVec3::new(10.0, 20.0, 30.0)).length() < 1e-6,
        "{top:?}"
    );
    assert!(is_ortho(&top));
    assert_eq!(st.pane_camera(Pane::Front), front);
    // a pane the user navigated stays when the layout is reopened
    st.activate(Pane::Right);
    st.touch_view();
    st.camera.pan(40.0, 0.0, 600.0);
    let right = st.camera;
    run(&mut st, "view.layout", json!({})).unwrap(); // single
    st.activate(Pane::Perspective);
    run(
        &mut st,
        "view.set_camera",
        json!({"target": [0.0, 0.0, 0.0]}),
    )
    .unwrap();
    run(&mut st, "view.layout", json!({})).unwrap(); // four again
    assert_eq!(st.pane_camera(Pane::Right), right);
}

#[test]
fn file_new_resets_the_panes() {
    let mut st = common::state();
    run(&mut st, "view.layout", json!({"layout": "four"})).unwrap();
    run(&mut st, "view.maximize", json!({"pane": "top"})).unwrap();
    run(
        &mut st,
        "view.shading",
        json!({"mode": "wire", "pane": "right"}),
    )
    .unwrap();
    run(&mut st, "view.lighting", json!({"mode": "rig"})).unwrap();
    run(&mut st, "view.antialias", json!({"enabled": false})).unwrap();
    st.camera.orbit(0.5, 0.2);
    run(&mut st, "file.new", json!({})).unwrap();
    assert_eq!(st.views, Views::new(&EditorState::default_camera()));
    assert_eq!(st.camera, EditorState::default_camera());
    assert_eq!(st.shading, Shading::SmoothWire);
}

#[test]
fn lighting_shadows_and_antialias_commands_feed_the_renderer_settings() {
    let mut st = common::state();
    let s = display_settings(&st.views, Shading::Smooth, &[]);
    assert_eq!(s.lighting, Lighting::Headlight);
    assert!(s.shadows);
    assert_eq!(s.msaa, Msaa::X4);
    assert_eq!(s.shading, Shading::Smooth);

    run(&mut st, "view.lighting", json!({"mode": "rig"})).unwrap();
    run(&mut st, "view.shadows", json!({"enabled": false})).unwrap();
    run(&mut st, "view.antialias", json!({"enabled": false})).unwrap();
    let s = display_settings(&st.views, Shading::Flat, &[]);
    assert_eq!(
        (s.lighting, s.shadows, s.msaa),
        (Lighting::Rig, false, Msaa::Off)
    );
    assert_eq!(s.shading, Shading::Flat);
    assert!(run(&mut st, "view.lighting", json!({"mode": "disco"})).is_err());
    assert!(run(&mut st, "view.shadows", json!({})).is_err());

    // scene lights override the lighting mode, capped at eight
    let many: Vec<Light> = (0..11)
        .map(|i| Light::point(DVec3::new(i as f64, 1.0, 0.0), 1.0))
        .collect();
    let Lighting::Scene(sl) = display_settings(&st.views, Shading::Smooth, &many).lighting else {
        panic!("scene lights expected");
    };
    assert_eq!(sl.count, MAX_LIGHTS);
    // no light node type exists yet: the hook finds none in a normal scene
    let scene = st.evaluated();
    assert!(scene_lights(&scene).is_empty());
}

// ---- screenshot composite --------------------------------------------------------------

fn skip_without_adapter(message: &str) -> bool {
    if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") {
        panic!("FORGE_REQUIRE_GPU=1 but the screenshot failed: {message}");
    }
    eprintln!("skipping GPU test: {message}");
    true
}

fn decode(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png_bytes))
        .read_info()
        .expect("valid PNG");
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    (info.width, info.height, pixels)
}

#[test]
fn a_four_view_screenshot_shows_four_panes_and_the_maximised_pane_alone() {
    let mut st = common::state();
    common::create(&mut st, "forge.cube");
    run(&mut st, "view.frame_all", json!({})).unwrap();
    run(&mut st, "view.layout", json!({"layout": "four"})).unwrap();
    let (w, h) = (202u32, 122u32);
    let shot = match forge_ui::screenshot::render_png(&mut st, w, h) {
        Ok(b) => b,
        Err(e) if e.contains("no GPU") => {
            skip_without_adapter(&e);
            return;
        }
        Err(e) => panic!("{e}"),
    };
    let (iw, ih, px) = decode(&shot);
    assert_eq!((iw, ih), (w, h));
    let at = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    // 100 + 2 + 100 wide, 60 + 2 + 60 tall: the gap column and row are the gap colour
    let gap = [18, 18, 20];
    for y in [0, 30, 59, 121] {
        assert_eq!(at(100, y), gap);
        assert_eq!(at(101, y), gap);
    }
    for x in [0, 50, 199] {
        assert_eq!(at(x, 60), gap);
    }
    // panes differ: the perspective view and the top view of the same cube
    let quad = |qx: u32, qy: u32| -> Vec<[u8; 3]> {
        let (x0, y0) = (qx * 102, qy * 62);
        (0..60)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .map(|(x, y)| at(x0 + x, y0 + y))
            .collect()
    };
    let (persp, top, right, front) = (quad(0, 0), quad(1, 0), quad(0, 1), quad(1, 1));
    assert!(persp != top && top != right && right != front);
    // every pane shows the cube: more than one colour
    for q in [&persp, &top, &right, &front] {
        let first = q[0];
        assert!(q.iter().any(|p| *p != first));
    }

    // maximised: one pane over the whole image, no gap colour column
    run(&mut st, "view.maximize", json!({"pane": "top"})).unwrap();
    let shot = forge_ui::screenshot::render_png(&mut st, w, h).unwrap();
    let (_, _, px) = decode(&shot);
    let pixel = |x: u32, y: u32| {
        let i = ((y * w + x) * 4) as usize;
        [px[i], px[i + 1], px[i + 2]]
    };
    assert_ne!(pixel(100, 30), gap);
    // and it is what a single-layout screenshot of that pane gives
    run(&mut st, "view.layout", json!({"layout": "single"})).unwrap();
    let single = forge_ui::screenshot::render_png(&mut st, w, h).unwrap();
    assert_eq!(decode(&single).2, px);
}

// ---- kittest ---------------------------------------------------------------------------

fn harness(editor: Editor) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    h
}

fn panes_shown(h: &Harness<'_, Editor>) -> usize {
    h.query_all_by_label("Viewport unavailable").count()
}

fn press(h: &mut Harness<'_, Editor>, mods: Modifiers, key: Key) {
    h.key_press_modifiers(mods, key);
    h.run();
}

#[test]
fn f4_toggles_the_four_view_layout() {
    let mut h = harness(Editor::new(common::state()));
    assert_eq!(panes_shown(&h), 1);
    press(&mut h, Modifiers::NONE, Key::F4);
    assert_eq!(h.state().state.views.layout, Layout::Four);
    assert_eq!(panes_shown(&h), 4);
    press(&mut h, Modifiers::NONE, Key::F4);
    assert_eq!(h.state().state.views.layout, Layout::Single);
    assert_eq!(panes_shown(&h), 1);
}

#[test]
fn mod_m_maximises_the_active_pane_and_restores_it() {
    let mut h = harness(Editor::new(common::state()));
    press(&mut h, Modifiers::COMMAND, Key::M);
    assert_eq!(
        h.state().state.views.maximized,
        None,
        "nothing to maximise in one view"
    );
    press(&mut h, Modifiers::NONE, Key::F4);
    assert_eq!(panes_shown(&h), 4);
    press(&mut h, Modifiers::COMMAND, Key::M);
    assert_eq!(h.state().state.views.maximized, Some(Pane::Perspective));
    assert_eq!(panes_shown(&h), 1);
    press(&mut h, Modifiers::COMMAND, Key::M);
    assert_eq!(h.state().state.views.maximized, None);
    assert_eq!(panes_shown(&h), 4);
}

#[test]
fn the_pane_under_the_pointer_becomes_active_and_navigates_alone() {
    let mut h = harness(Editor::new(common::state()));
    press(&mut h, Modifiers::NONE, Key::F4);
    // pane centres, top-left, top-right, bottom-left, bottom-right
    let mut centres: Vec<egui::Pos2> = h
        .query_all_by_label("Viewport unavailable")
        .map(|n| n.rect().center())
        .collect();
    assert_eq!(centres.len(), 4);
    centres.sort_by(|a, b| (a.y.round(), a.x).partial_cmp(&(b.y.round(), b.x)).unwrap());
    let persp_before = h.state().state.pane_camera(Pane::Perspective);
    let front_before = h.state().state.pane_camera(Pane::Front);

    h.hover_at(centres[3]);
    h.run();
    assert_eq!(h.state().state.views.active, Pane::Front);
    h.hover_at(centres[1]);
    h.run();
    assert_eq!(h.state().state.views.active, Pane::Top);

    // an Alt + drag in the front pane orbits that pane's camera only
    h.hover_at(centres[3]);
    h.run();
    h.event(egui::Event::ModifiersChanged(Modifiers::ALT));
    h.event(egui::Event::PointerButton {
        pos: centres[3],
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::ALT,
    });
    h.run();
    for k in 1..=4 {
        h.hover_at(centres[3] + egui::vec2(10.0 * k as f32, 0.0));
        h.run();
    }
    h.event(egui::Event::PointerButton {
        pos: centres[3] + egui::vec2(40.0, 0.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::ALT,
    });
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run();
    let st = &h.state().state;
    assert_eq!(st.views.active, Pane::Front);
    assert_ne!(st.pane_camera(Pane::Front).yaw, front_before.yaw);
    assert_eq!(st.pane_camera(Pane::Perspective), persp_before);
}

#[test]
fn the_view_menu_has_the_layout_and_lighting_entries() {
    let mut h = harness(Editor::new(common::state()));
    h.get_by_label("View").click();
    h.run();
    for label in [
        "Quick",
        "Constant",
        "Hidden line",
        "Bounding box",
        "Shadows",
        "Anti-aliasing",
    ] {
        h.get_by_label(label);
    }
    h.get_by_label("Layout: four views").click();
    h.run();
    assert_eq!(h.state().state.views.layout, Layout::Four);
    assert_eq!(panes_shown(&h), 4);
    h.get_by_label("View").click();
    h.run();
    h.get_by_label("Lighting: rig").click();
    h.run();
    assert_eq!(
        h.state().state.views.lighting,
        forge_command::LightingArg::Rig
    );
    h.get_by_label("View").click();
    h.run();
    h.get_by_label("Hidden line").click();
    h.run();
    assert_eq!(h.state().state.shading, Shading::HiddenLine);
    // the other panes keep their own shading
    assert_eq!(h.state().state.pane_shading(Pane::Top), ORTHO_SHADING);
}

#[test]
fn the_command_context_toggles_the_layout_without_an_argument() {
    let mut st = common::state();
    CommandContext::set_layout(&mut st, None);
    assert_eq!(st.views.layout, Layout::Four);
}
