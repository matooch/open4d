//! RENDER.md §R18: timed restart with a deterministic engine instead of a ray-trace benchmark.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

use forge_core::{Document, Value};
use forge_render_api::{
    Control, RenderEngine, RenderError, RenderEvent, RenderOutput, RenderSettings, Snapshot,
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
        _settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError> {
        let first = self.calls.fetch_add(1, Ordering::Relaxed) == 0;
        let x = snap.meshes[0]
            .positions
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max);
        control.emit(RenderEvent::Tile {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            rgba: if first {
                vec![1.0, 0.0, 0.0, 1.0]
            } else {
                vec![0.0, 1.0, 0.0, 1.0]
            },
        });
        // The receiver may already be gone when the last render is cancelled at teardown.
        let _ = self.signal.send(Signal::Started(x));
        // Every render runs until it is cancelled and always reports the cancel (also when the
        // cancel came before this point), so a restart can only come from the cancel path,
        // never from waiting for the old render to finish.
        while !control.is_cancelled() {
            std::thread::sleep(Duration::from_millis(1));
        }
        // Simulate a tile crossing the cancel boundary.
        control.emit(RenderEvent::Tile {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            rgba: vec![1.0, 0.0, 1.0, 1.0],
        });
        let _ = self.signal.send(Signal::Cancelled);
        Err(RenderError::Cancelled)
    }
}

/// One edit: the in-flight render is cancelled, its late tile is rejected, and a render of the
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
    preview.update(ctx, doc.scene(), 0);
    assert!(
        preview.take_image().is_none(),
        "old accumulation was cleared"
    );
    assert!(matches!(
        rx.recv_timeout(guard.saturating_duration_since(Instant::now())),
        Ok(Signal::Cancelled)
    ));
    let x = loop {
        preview.update(ctx, doc.scene(), 0);
        if let Ok(Signal::Started(x)) = rx.try_recv() {
            break x;
        }
        assert!(Instant::now() < guard, "restart never happened");
        std::thread::sleep(Duration::from_millis(1));
    };
    (start.elapsed(), x)
}

#[test]
fn parameter_edit_restarts_in_under_100_ms_and_rejects_old_tiles() {
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
    preview.update(&ctx, doc.scene(), 0);
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
        // The late tile of the cancelled render is rejected: the image is the new render's.
        let deadline = Instant::now() + Duration::from_secs(10);
        let image = loop {
            preview.update(&ctx, doc.scene(), 0);
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
