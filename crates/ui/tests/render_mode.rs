//! UI.md §6.3, RENDER.md R22.7: the viewport's Render display mode is UI state (no undo entry)
//! and its frame follows the pane's own camera. The frame test needs an adapter (skips without
//! one unless `FORGE_REQUIRE_GPU=1`); the menu tests run without a GPU.

mod common;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_render_api::PostSettings;
use forge_ui::app::Editor;
use forge_ui::commands::execute;
use forge_ui::render_mode::RenderView;
use forge_ui::state::{DisplayMode, EditorState};
use forge_viewport::camera::{Camera, Handedness, Projection};
use glam::DVec3;
use serde_json::json;

fn harness(editor: Editor) -> Harness<'static, Editor> {
    let mut h = Harness::builder()
        .with_size([1280.0, 800.0])
        .build_ui_state(|ui, e: &mut Editor| e.show(ui, None), editor);
    h.run();
    h
}

/// The "Render" button of the viewport header, which switches the display mode.
fn toggle_render(h: &mut Harness<'_, Editor>) {
    h.get_by_label("Render").click();
    h.run();
}

/// The header's Post menu, where the five effect toggles live.
fn open_post_menu(h: &mut Harness<'_, Editor>) {
    h.get_by_label("Post").click();
    h.run();
}

#[test]
fn the_header_switches_to_render_and_back_without_an_undo_entry() {
    let mut h = harness(Editor::new(common::state()));
    let (depth, revision) = {
        let doc = &h.state().state.app.doc;
        (doc.undo_depth(), doc.scene().revision())
    };
    toggle_render(&mut h);
    assert_eq!(h.state().state.display, DisplayMode::Render);
    // UI state: no undo entry and no scene change
    assert_eq!(h.state().state.app.doc.undo_depth(), depth);
    assert_eq!(h.state().state.app.doc.scene().revision(), revision);
    // the kittest window has no device, so the pane keeps the placeholder
    assert_eq!(h.query_all_by_label("Viewport unavailable").count(), 1);

    toggle_render(&mut h);
    assert_eq!(h.state().state.display, DisplayMode::Shaded);
    assert_eq!(h.state().state.app.doc.undo_depth(), depth);
    assert_eq!(h.state().state.app.doc.scene().revision(), revision);
    assert_eq!(h.query_all_by_label("Viewport unavailable").count(), 1);
}

#[test]
fn the_post_toggles_are_ui_state() {
    let mut h = harness(Editor::new(common::state()));
    let depth = h.state().state.app.doc.undo_depth();
    assert_eq!(h.state().state.post, PostSettings::default());
    toggle_render(&mut h);
    open_post_menu(&mut h);
    h.get_by_label("SSAO").click();
    h.run();
    assert!(h.state().state.post.ssao);
    open_post_menu(&mut h);
    h.get_by_label("Bloom").click();
    h.run();
    assert!(h.state().state.post.bloom);
    assert!(!h.state().state.post.ssr);
    assert_eq!(h.state().state.app.doc.undo_depth(), depth);
}

/// The pane camera becomes a render camera with the same basis: an object up and to the right
/// of the target lands in the top-right quadrant (no flip or mirror, R22.7).
#[test]
fn the_render_mode_frame_follows_the_pane_camera() {
    match forge_render_rt::request_default_device() {
        Ok(_) => {}
        Err(e) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
            panic!("FORGE_REQUIRE_GPU=1 but no GPU: {e}")
        }
        Err(e) => {
            eprintln!("skipping GPU test: {e}");
            return;
        }
    }
    let mut st: EditorState = common::state();
    let cube = common::create(&mut st, "forge.cube");
    execute(
        &mut st,
        "transform.set",
        json!({"node": cube.0, "pos": [200.0, 200.0, 0.0]}),
    )
    .unwrap();
    let scene = st.app.scheduled_draw(0).scene.clone();
    let camera = Camera {
        target: DVec3::ZERO,
        distance: 500.0,
        yaw: 0.0,
        pitch: 0.0,
        projection: Projection::Perspective {
            fov_y: 45f64.to_radians(),
        },
        handedness: Handedness::Right,
    };
    let mut view = RenderView::default();
    let ctx = egui::Context::default();
    let native = view
        .image(
            &ctx,
            &scene,
            &camera,
            0,
            [931, 517],
            PostSettings::default(),
        )
        .unwrap();
    assert_eq!(native.size(), [931, 517]);
    let (w, h) = (64usize, 48usize);
    let image = view
        .frame(
            &scene,
            &camera,
            0,
            [w as u32, h as u32],
            PostSettings::default(),
        )
        .unwrap();
    assert_eq!(image.size, [w, h]);
    let (mut sx, mut sy, mut n) = (0.0f64, 0.0f64, 0usize);
    for y in 0..h {
        for x in 0..w {
            let c = image.pixels[y * w + x];
            if c.r() as u32 + c.g() as u32 + c.b() as u32 > 30 {
                sx += x as f64;
                sy += y as f64;
                n += 1;
            }
        }
    }
    assert!(n > 20, "the cube is in the frame ({n} pixels)");
    let (cx, cy) = (sx / n as f64, sy / n as f64);
    assert!(
        cx > 0.55 * w as f64,
        "world +X is on the right (centroid x {cx} of {w})"
    );
    assert!(
        cy < 0.45 * h as f64,
        "world +Y is up (centroid y {cy} of {h})"
    );
}
