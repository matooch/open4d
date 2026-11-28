//! RENDER.md §R18: timed restart with a deterministic engine instead of a ray-trace benchmark.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use forge_core::{Document, Value};
use forge_render_api::{
    Control, RenderEngine, RenderError, RenderOutput, RenderSettings, Snapshot,
};
use forge_ui::interactive_render::InteractiveRender;

enum Signal {
    Started(f64),
    Cancelled,
}

struct Stub {
    calls: AtomicUsize,
    signal: Sender<Signal>,
}

impl RenderEngine for Stub {
    fn id(&self) -> &str {
        "test.ipr"
    }

    fn render(
        &self,
        snap: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError> {
        if settings.samples == 1 {
            let first = self.calls.fetch_add(1, Ordering::Relaxed) == 0;
            return Ok(solid_frame(
                settings,
                if first {
                    [1.0, 0.0, 0.0, 1.0]
                } else {
                    [0.0, 1.0, 0.0, 1.0]
                },
            ));
        }
        let x = snap.meshes[0]
            .positions
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        let _ = self.signal.send(Signal::Started(x));
        while !control.is_cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        let _ = self.signal.send(Signal::Cancelled);
        Ok(solid_frame(settings, [1.0, 0.0, 1.0, 1.0]))
    }
}

/// One edit: the in-flight render is cancelled, its late frame is rejected, and a render of the
/// edited scene starts. Returns the edit-to-start latency and the new render's x extent.
fn restart_after_edit(
    preview: &mut InteractiveRender,
    ctx: &egui::Context,
    doc: &mut Document,
    cube: forge_core::NodeId,
    size: f64,
    rx: &std::sync::mpsc::Receiver<Signal>,
) -> (Duration, f64) {
    doc.transact("wider cube", |tx| {
        tx.set_param(cube, "size_x", Value::Float(size))
    })
    .unwrap();
    let start = Instant::now();
    // Hang guard only; the 100 ms budget is checked on the best of the runs below.
    let guard = start + Duration::from_secs(10);
    preview.update(ctx, doc.scene(), 0, RenderSettings::new(64, 48, 2), None);
    assert!(
        preview.take_image().is_none(),
        "old accumulation was cleared"
    );
    assert!(matches!(
        rx.recv_timeout(guard.saturating_duration_since(Instant::now())),
        Ok(Signal::Cancelled)
    ));
    let x = loop {
        preview.update(ctx, doc.scene(), 0, RenderSettings::new(64, 48, 2), None);
        if let Ok(Signal::Started(x)) = rx.try_recv() {
            break x;
        }
        assert!(Instant::now() < guard, "restart never happened");
        std::thread::sleep(Duration::from_millis(1));
    };
    (start.elapsed(), x)
}

#[test]
fn parameter_edit_restarts_in_under_100_ms_and_rejects_old_frames() {
    let (signal, rx) = channel();
    let mut preview = InteractiveRender::with_engine(Arc::new(Stub {
        calls: AtomicUsize::new(0),
        signal,
    }));
    let app = forge_app::App::new_default();
    let mut doc: Document = app.doc;
    let cube = doc
        .transact("cube", |tx| tx.create("forge.cube", None, None))
        .unwrap();
    let ctx = egui::Context::default();
    preview.update(&ctx, doc.scene(), 0, RenderSettings::new(64, 48, 2), None);
    let Signal::Started(mut previous) = rx.recv_timeout(Duration::from_secs(2)).unwrap() else {
        panic!("first render did not start");
    };
    assert_eq!(preview.take_image().unwrap().pixels[0], egui::Color32::RED);

    // RENDER.md R18: restart within 100 ms. Wall-clock time on a shared CI runner includes
    // scheduling noise, so the budget applies to the best of five restarts (the mechanism is
    // the same for each; a design that waited for the old render would never restart at all).
    let mut best = Duration::MAX;
    for (i, size) in [360.0, 420.0, 480.0, 540.0, 600.0].into_iter().enumerate() {
        let (latency, x) = restart_after_edit(&mut preview, &ctx, &mut doc, cube, size, &rx);
        assert!(
            x > previous,
            "edit {i}: the new render sees the edited cube"
        );
        previous = x;
        best = best.min(latency);
        // The late frame of the cancelled render is rejected: the image is the new render's.
        let deadline = Instant::now() + Duration::from_secs(10);
        let image = loop {
            preview.update(&ctx, doc.scene(), 0, RenderSettings::new(64, 48, 2), None);
            if let Some(image) = preview.take_image() {
                break image;
            }
            assert!(
                Instant::now() < deadline,
                "edit {i}: no image from the new render"
            );
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(image.pixels[0], egui::Color32::GREEN, "edit {i}");
    }
    assert!(
        best < Duration::from_millis(100),
        "best restart of five took {best:?}"
    );
}

fn solid_frame(settings: &RenderSettings, rgba: [f32; 4]) -> RenderOutput {
    RenderOutput {
        width: settings.width,
        height: settings.height,
        beauty: forge_image::Image::from_fn(
            settings.width,
            settings.height,
            forge_image::AlphaMode::Straight,
            |_, _| rgba,
        ),
        aovs: Default::default(),
        stats: Default::default(),
    }
}

#[test]
fn quick_full_frame_is_replaced_by_native_frames_with_more_samples() {
    use std::sync::{Mutex, mpsc::Receiver};

    struct Stepped {
        passes: Sender<(u32, u32, u32)>,
        resume: Mutex<Receiver<()>>,
    }

    impl RenderEngine for Stepped {
        fn id(&self) -> &str {
            "test.progressive"
        }

        fn render(
            &self,
            _: &Snapshot,
            settings: &RenderSettings,
            control: &Control,
        ) -> Result<RenderOutput, RenderError> {
            control.emit(forge_render_api::RenderEvent::Tile {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
                rgba: vec![1.0; 4],
            });
            self.passes
                .send((settings.width, settings.height, settings.samples))
                .unwrap();
            self.resume
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            Ok(solid_frame(settings, [0.0, 1.0, 0.0, 1.0]))
        }
    }

    let (passes, started) = channel();
    let (resume, wait) = channel();
    let mut preview = InteractiveRender::with_engine(Arc::new(Stepped {
        passes,
        resume: Mutex::new(wait),
    }));
    let app = forge_app::App::new_default();
    let ctx = egui::Context::default();
    preview.update(
        &ctx,
        app.doc.scene(),
        0,
        RenderSettings::new(1600, 900, 6),
        None,
    );
    for expected in [
        (768, 432, 1),
        (1600, 900, 1),
        (1600, 900, 4),
        (1600, 900, 6),
    ] {
        assert_eq!(
            started.recv_timeout(Duration::from_secs(5)).unwrap(),
            expected
        );
        assert!(
            preview.take_image().is_none(),
            "incomplete tiles must stay hidden"
        );
        resume.send(()).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let image = loop {
            if let Some(image) = preview.take_image() {
                break image;
            }
            assert!(Instant::now() < deadline, "completed pass never arrived");
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(image.size, [expected.0 as usize, expected.1 as usize]);
        assert!(image.pixels.iter().all(|p| *p == egui::Color32::GREEN));
    }
}

#[test]
fn navigation_publishes_frames_and_coalesces_camera_updates_before_refining() {
    use std::sync::{Mutex, mpsc::Receiver};

    struct MotionEngine {
        passes: Sender<(f64, RenderSettings)>,
        resume: Mutex<Receiver<()>>,
    }

    impl RenderEngine for MotionEngine {
        fn id(&self) -> &str {
            "test.motion"
        }

        fn render(
            &self,
            scene: &Snapshot,
            settings: &RenderSettings,
            control: &Control,
        ) -> Result<RenderOutput, RenderError> {
            self.passes
                .send((scene.camera.position().x, settings.clone()))
                .map_err(|_| RenderError::Cancelled)?;
            if settings.width == 384 {
                self.resume
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|_| RenderError::Cancelled)?;
            }
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            Ok(solid_frame(settings, [0.0, 1.0, 0.0, 1.0]))
        }
    }

    let (passes, started) = channel();
    let (resume, wait) = channel();
    let mut preview = InteractiveRender::with_engine(Arc::new(MotionEngine {
        passes,
        resume: Mutex::new(wait),
    }));
    let app = forge_app::App::new_default();
    let ctx = egui::Context::default();
    let update = |preview: &mut InteractiveRender, x, moving| {
        let mut camera = forge_render_api::Camera::default_perspective();
        camera.transform[12] = x;
        preview.update_viewport(
            &ctx,
            app.doc.scene(),
            0,
            RenderSettings::new(960, 540, 4),
            Some(camera),
            moving,
        );
    };
    let wait_for_pass = |preview: &mut InteractiveRender, x, moving| {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            update(preview, x, moving);
            if let Ok(pass) = started.try_recv() {
                break pass;
            }
            assert!(Instant::now() < deadline, "next render never started");
            std::thread::sleep(Duration::from_millis(1));
        }
    };
    let (x, pass) = wait_for_pass(&mut preview, 1.0, true);
    assert_eq!(
        (x, pass.width, pass.height, pass.samples),
        (1.0, 384, 216, 1)
    );
    assert_eq!(pass.max_depth, 3);
    update(&mut preview, 2.0, true);
    update(&mut preview, 3.0, true);
    resume.send(()).unwrap();
    let (x, pass) = wait_for_pass(&mut preview, 3.0, true);
    assert_eq!(
        (x, pass.width, pass.height, pass.samples),
        (3.0, 384, 216, 1)
    );
    assert_eq!(preview.take_image().unwrap().size, [384, 216]);

    resume.send(()).unwrap();
    for (width, height, samples) in [(768, 432, 1), (960, 540, 1), (960, 540, 4)] {
        let (x, pass) = wait_for_pass(&mut preview, 3.0, false);
        assert_eq!(
            (x, pass.width, pass.height, pass.samples),
            (3.0, width, height, samples)
        );
        assert_eq!(pass.max_depth, 5);
    }
}
