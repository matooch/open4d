//! UI.md §6.3, RENDER.md R22.7: the viewport's Render display mode is UI state (no undo entry)
//! and its camera projection follows the pane dimensions.

mod common;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_ui::app::Editor;
use forge_ui::state::DisplayMode;
use forge_viewport::camera::{Camera, Projection};

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
fn render_camera_matches_viewport_framing_at_wide_and_tall_aspects() {
    use forge_ui::render_mode::render_camera;

    for projection in [
        Projection::Perspective { fov_y: 0.7 },
        Projection::Orthographic { height: 280.0 },
    ] {
        let camera = Camera {
            projection,
            ..Camera::default()
        };
        for (width, height) in [(1400, 540), (510, 920)] {
            let pixel = (f64::from(width) * 0.84, f64::from(height) * 0.28);
            let expected = camera.ray(pixel.0, pixel.1, width.into(), height.into());
            let render = render_camera(&camera, width, height);
            let (origin, direction) = render.ray(pixel.0, pixel.1, width, height);
            assert!(direction.distance(expected.dir) < 1e-8);
            assert!((origin - expected.origin).cross(direction).length() < 1e-8);
        }
    }
}

#[test]
fn render_settings_stay_open_while_editing() {
    use forge_ui::dock::{Area, Panel};
    let mut h = harness(Editor::new(common::state()));
    h.get_by_label("Render Settings").click();
    h.run();
    assert_eq!(
        h.state().state.dock.active(Area::RightBottom),
        Some(Panel::RenderSettings)
    );
    h.get_by_label("Preview samples").click();
    h.run();
    assert_eq!(
        h.state().state.dock.active(Area::RightBottom),
        Some(Panel::RenderSettings)
    );
    h.get_by_label("Final render");
}

#[test]
fn full_render_opens_the_picture_viewer() {
    use forge_ui::dock::{Area, Panel};
    let mut state = common::state();
    state
        .doc
        .transact("Small render", |tx| {
            tx.set_render_config(Some(forge_core::RenderConfig {
                width: 16,
                height: 16,
                samples: 1,
                ..Default::default()
            }))
        })
        .unwrap();
    let mut h = harness(Editor::new(state));
    h.get_by_label("Full Render").click();
    h.step();
    assert_eq!(
        h.state().state.dock.active(Area::Center),
        Some(Panel::PictureViewer)
    );
}

#[test]
fn shading_menu_beside_render_switches_back_to_flat_shading() {
    use forge_viewport::render::Shading;
    let mut h = harness(Editor::new(common::state()));
    toggle_render(&mut h);
    h.get_by_label("Viewport shading").click();
    h.run();
    h.get_by_label("Flat").click();
    h.run();
    assert_eq!(h.state().state.display, DisplayMode::Shaded);
    assert_eq!(h.state().state.shading, Shading::Flat);
}
