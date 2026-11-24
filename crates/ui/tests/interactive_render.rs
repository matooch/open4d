//! RENDER.md §R18: timed restart with a deterministic engine instead of a ray-trace benchmark.

use std::collections::BTreeMap;
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
        settings: &RenderSettings,
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
        self.signal.send(Signal::Started(x)).unwrap();
        if first {
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
            self.signal.send(Signal::Cancelled).unwrap();
            return Err(RenderError::Cancelled);
        }
        let mut beauty = forge_image::Image::new(
            settings.width,
            settings.height,
            forge_image::AlphaMode::Straight,
        );
        beauty.data[..4].copy_from_slice(&[0.0, 1.0, 0.0, 1.0]);
        Ok(RenderOutput {
            width: settings.width,
            height: settings.height,
            beauty,
            aovs: BTreeMap::new(),
            stats: Default::default(),
        })
    }
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
    let Signal::Started(original) = rx.recv_timeout(Duration::from_secs(2)).unwrap() else {
        panic!("first render did not start");
    };
    assert_eq!(preview.take_image().unwrap().pixels[0], egui::Color32::RED);

    doc.transact("wider cube", |tx| {
        tx.set_param(cube, "size_x", Value::Float(360.0))
    })
    .unwrap();
    let start = Instant::now();
    let deadline = start + Duration::from_millis(100);
    preview.update(&ctx, doc.scene(), 0);
    assert!(
        preview.take_image().is_none(),
        "old accumulation was cleared"
    );
    assert!(matches!(
        rx.recv_timeout(deadline.saturating_duration_since(Instant::now())),
        Ok(Signal::Cancelled)
    ));
    let changed = loop {
        preview.update(&ctx, doc.scene(), 0);
        if let Ok(Signal::Started(x)) = rx.try_recv() {
            break x;
        }
        assert!(Instant::now() < deadline, "restart exceeded 100 ms");
        std::thread::sleep(Duration::from_millis(1));
    };
    let latency = start.elapsed();
    assert!(
        latency < Duration::from_millis(100),
        "restart took {latency:?}"
    );
    assert!(changed > original);
    assert_eq!(
        preview.take_image().unwrap().pixels[0],
        egui::Color32::GREEN
    );
}
