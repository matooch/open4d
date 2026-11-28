//! RENDER.md §R17: submit real path-traced jobs and drive history comparison in egui.

use std::sync::Arc;
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use forge_core::{RenderConfig, Value};
use forge_ui::picture_viewer::{Channel, PictureViewer};
use forge_ui::state::EditorState;

#[test]
fn queued_renders_appear_in_history_and_can_be_compared() {
    if let Err(error) = forge_render_rt::request_rt_device() {
        assert_ne!(
            std::env::var("FORGE_REQUIRE_GPU").as_deref(),
            Ok("1"),
            "{error}"
        );
        return;
    }
    let mut st = EditorState::new(Arc::new(forge_app::default_registry().unwrap()));
    let cube = st
        .doc
        .transact("preview scene", |tx| {
            tx.set_render_config(Some(RenderConfig {
                width: 32,
                height: 32,
                samples: 4,
                ..RenderConfig::default()
            }))?;
            tx.create("forge.cube", None, None)
        })
        .unwrap();
    let mut h = Harness::new_ui_state(
        |ui, (viewer, st): &mut (PictureViewer, EditorState)| {
            viewer.show(ui, st.doc.scene(), st.timeline.current, Some(st.camera));
        },
        (PictureViewer::default(), st),
    );
    h.get_by_label("All passes").click();
    h.run();
    h.get_by_label("Queue frame").click();
    h.step(); // a render is now running and keeps requesting repaints; see below
    {
        let st = &mut h.state_mut().1;
        st.timeline.current = 7;
        st.doc
            .transact("edit cube", |tx| {
                tx.set_param(cube, "size_x", Value::Float(310.0))
            })
            .unwrap();
    }
    h.get_by_label("Queue frame").click();
    // While a render is in progress the viewer keeps requesting repaints (progress), so step
    // frame by frame until both queued renders are in the history; `run()` would wait for a
    // settled UI and can exceed its step limit depending on the machine's speed.
    h.step();
    let deadline = Instant::now() + Duration::from_secs(10);
    while h.state().0.history_len() < 2 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        h.step();
    }
    h.run();
    assert_eq!(h.state().0.history_len(), 2);
    h.get_by_label("Render #1 · Frame 0").click();
    h.run();
    assert_eq!(h.state().0.viewed_id(), Some(1));
    let first = h.state().0.viewed_output().unwrap().beauty.data.clone();
    let mut preview = forge_ui::interactive_render::InteractiveRender::default();
    let scene = h.state().1.doc.scene().clone();
    let camera = forge_ui::render_mode::render_camera(&h.state().1.camera, 32, 32);
    let context = egui::Context::default();
    let mut settings = forge_render_bridge::settings_from_scene(
        &scene,
        forge_render_api::RenderSettings::default(),
    );
    settings.tile = 64;
    h.get_by_label("Render #2 · Frame 7").click();
    h.run();
    let frame = h.state().0.viewed_output().unwrap();
    let expected: Vec<_> = frame
        .beauty
        .data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|raw| {
            let [r, g, b] = [raw[0], raw[1], raw[2]].map(forge_image::linear_to_srgb8);
            egui::Color32::from_rgb(r, g, b)
        })
        .collect();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        preview.update(&context, &scene, 7, settings.clone(), Some(camera));
        if let Some(image) = preview.take_image()
            && image.pixels == expected
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "refined viewport never matched the four-sample render"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    h.get_by_label("Render #1 · Frame 0").click();
    h.run();

    h.get_by_label("Set A").click();
    h.run();
    h.get_by_label("Render #2 · Frame 7").click();
    h.run();
    h.get_by_label("Set B").click();
    h.run();
    h.get_by_label("Beauty").click();
    h.run();
    h.get_by_label("normal").click();
    h.run();
    assert_eq!(
        h.state().0.channel(),
        Channel::Pass(forge_render_api::Aov::Normal)
    );
    h.get_by_label("Show A").click();
    h.run();
    assert_eq!(h.state().0.viewed_id(), Some(1));
    assert_eq!(
        h.state().0.channel(),
        Channel::Pass(forge_render_api::Aov::Normal)
    );
    h.get_by_label("Show B").click();
    h.run();
    assert_eq!(h.state().0.viewed_id(), Some(2));
    assert_eq!(h.state().0.history_len(), 2);
    let second = &h.state().0.viewed_output().unwrap().beauty.data;
    assert!(
        first.as_chunks::<4>().0.iter().any(|p| p[3] > 0.0),
        "the snapshot includes geometry"
    );
    assert_ne!(&first, second, "each job renders its submitted scene");
}
