//! Progressive viewport rendering that keeps publishing frames during navigation (RENDER.md §R18).

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};

use crate::render_mode::{StudioPathTracer, display_rgb, progressive_samples};
use egui::{ColorImage, Context, TextureHandle};
use forge_core::Scene;
use forge_render_api::{Camera, Control, RenderEngine, RenderError, RenderSettings};

use crate::picture_viewer::prepare_render;

#[derive(Clone)]
struct Capture {
    ticket: u64,
    frame: i64,
    scene: Arc<Scene>,
    settings: RenderSettings,
    camera: Option<Camera>,
    moving: bool,
}

struct Worker {
    ticket: u64,
    control: Control,
    result: Receiver<Result<Vec<String>, RenderError>>,
}

struct PreviewFrame {
    image: ColorImage,
    samples: u32,
    quick: bool,
}

type SharedSurface = Arc<Mutex<Option<PreviewFrame>>>;

const QUICK_PREVIEW_EDGE: u32 = 768;
const MOTION_PREVIEW_EDGE: u32 = 384;
const MOTION_PREVIEW_DEPTH: u32 = 3;

fn quick_settings(settings: &RenderSettings, edge: u32) -> Option<RenderSettings> {
    let longest = settings.width.max(settings.height);
    if longest <= edge || settings.region.is_some() {
        return None;
    }
    let scale = f64::from(edge) / f64::from(longest);
    let scaled = |size| (f64::from(size) * scale).round().max(1.0) as u32;
    Some(RenderSettings {
        width: scaled(settings.width),
        height: scaled(settings.height),
        samples: 1,
        ..settings.clone()
    })
}

fn start(
    engine: Arc<dyn RenderEngine>,
    capture: Capture,
    ctx: &Context,
    surface: SharedSurface,
) -> std::io::Result<Worker> {
    let mut settings = capture.settings;
    let control = Control::new();
    let (done, result) = channel();
    let worker = Worker {
        ticket: capture.ticket,
        control: control.clone(),
        result,
    };
    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("interactive-render".into())
        .spawn(move || {
            let result = (|| {
                settings.validate()?;
                let (mut snapshot, warnings) =
                    prepare_render(&capture.scene, capture.frame, &mut settings, &control)?;
                if control.is_cancelled() {
                    return Err(RenderError::Cancelled);
                }
                if let Some(camera) = capture.camera {
                    snapshot.camera = camera;
                }
                let publish_pass = |pass: &RenderSettings, quick| {
                    let output = engine.render(&snapshot, pass, &control)?;
                    if control.is_cancelled() {
                        return Err(RenderError::Cancelled);
                    }
                    let pixels = output
                        .beauty
                        .data
                        .as_chunks::<4>()
                        .0
                        .iter()
                        .map(|&[r, g, b, _]| display_rgb([r, g, b], 1.0))
                        .collect();
                    let image =
                        ColorImage::new([output.width as usize, output.height as usize], pixels);
                    if control.is_cancelled() {
                        return Err(RenderError::Cancelled);
                    }
                    if let Ok(mut frame) = surface.lock() {
                        *frame = Some(PreviewFrame {
                            image,
                            samples: pass.samples,
                            quick,
                        });
                    }
                    ctx.request_repaint();
                    Ok(())
                };
                if capture.moving {
                    let mut pass =
                        quick_settings(&settings, MOTION_PREVIEW_EDGE).unwrap_or(settings);
                    pass.samples = 1;
                    pass.max_depth = pass.max_depth.min(MOTION_PREVIEW_DEPTH);
                    publish_pass(&pass, true)?;
                    return Ok(warnings);
                }
                if let Some(quick) = quick_settings(&settings, QUICK_PREVIEW_EDGE) {
                    publish_pass(&quick, true)?;
                }
                // Deterministic sample prefixes keep the native-size result identical to
                // a still render and avoid holding path state for every viewport pixel.
                let target = settings.samples;
                let mut pass = settings;
                for samples in progressive_samples(target) {
                    pass.samples = samples;
                    publish_pass(&pass, false)?;
                }
                Ok(warnings)
            })();
            let _ = done.send(result);
            ctx.request_repaint();
        })?;
    Ok(worker)
}

pub struct InteractiveRender {
    engine: Arc<dyn RenderEngine>,
    submitted: Option<Capture>,
    ticket: u64,
    pending: Option<Capture>,
    worker: Option<Worker>,
    surface: Option<SharedSurface>,
    texture: Option<TextureHandle>,
    status: String,
    displayed_samples: Option<(u32, bool)>,
}

impl Default for InteractiveRender {
    fn default() -> Self {
        Self::with_engine(Arc::new(StudioPathTracer::default()))
    }
}

impl Drop for InteractiveRender {
    fn drop(&mut self) {
        self.suspend();
    }
}

impl InteractiveRender {
    pub fn with_engine(engine: Arc<dyn RenderEngine>) -> Self {
        Self {
            engine,
            submitted: None,
            ticket: 0,
            pending: None,
            worker: None,
            surface: None,
            texture: None,
            status: String::new(),
            displayed_samples: None,
        }
    }

    pub(crate) fn set_engine(&mut self, engine: Arc<dyn RenderEngine>) {
        self.suspend();
        self.engine = engine;
    }

    /// Hide the current generation immediately. The worker exits cooperatively.
    pub(crate) fn suspend(&mut self) {
        self.submitted = None;
        self.pending = None;
        self.texture = None;
        self.cancel_pass();
    }

    fn cancel_pass(&mut self) {
        self.surface = None;
        self.displayed_samples = None;
        if let Some(worker) = &self.worker {
            worker.control.cancel();
        }
    }

    pub fn update(
        &mut self,
        ctx: &Context,
        scene: &Scene,
        frame: i64,
        settings: RenderSettings,
        camera: Option<Camera>,
    ) {
        self.update_viewport(ctx, scene, frame, settings, camera, false);
    }

    pub fn update_viewport(
        &mut self,
        ctx: &Context,
        scene: &Scene,
        frame: i64,
        settings: RenderSettings,
        camera: Option<Camera>,
        moving: bool,
    ) {
        let same_content = self.submitted.as_ref().filter(|old| {
            old.frame == frame && old.settings == settings && same_scene(&old.scene, scene)
        });
        if same_content.is_none_or(|old| old.camera != camera || old.moving != moving) {
            let captured_scene =
                same_content.map_or_else(|| Arc::new(scene.clone()), |old| old.scene.clone());
            if same_content.is_none() {
                self.suspend();
            } else if !moving || self.submitted.as_ref().is_some_and(|old| !old.moving) {
                // Keep the visible image until the next pass is ready.
                self.cancel_pass();
            }
            // Motion leaves the running pass and mailbox intact so continuous input can
            // never starve the preview. Only the most recent camera waits in the queue.
            self.ticket = self.ticket.wrapping_add(1);
            let capture = Capture {
                ticket: self.ticket,
                frame,
                scene: captured_scene,
                settings,
                camera,
                moving,
            };
            self.submitted = Some(capture.clone());
            self.pending = Some(capture);
            self.status = "Preparing path trace…".into();
        }
        if let Some(worker) = self.worker.take() {
            match worker.result.try_recv() {
                Err(TryRecvError::Empty) => self.worker = Some(worker),
                result if worker.ticket == self.ticket && !worker.control.is_cancelled() => {
                    self.status = match result {
                        Ok(Ok(warnings)) if warnings.is_empty() => "Preview complete".into(),
                        Ok(Ok(warnings)) => warnings.join(" · "),
                        Ok(Err(e)) => e.to_string(),
                        Err(_) => "Preview worker stopped".into(),
                    }
                }
                _ => {}
            }
        }
        if self.worker.is_none()
            && let Some(capture) = self.pending.take()
        {
            let surface = self
                .surface
                .get_or_insert_with(|| Arc::new(Mutex::new(None)))
                .clone();
            match start(self.engine.clone(), capture, ctx, surface) {
                Ok(worker) => self.worker = Some(worker),
                Err(e) => self.status = format!("Could not start preview: {e}"),
            }
        }
    }

    /// The latest completed full-frame pass, if it has not been displayed yet.
    pub fn take_image(&mut self) -> Option<ColorImage> {
        let frame = self.surface.as_ref()?.lock().ok()?.take()?;
        self.displayed_samples = Some((frame.samples, frame.quick));
        Some(frame.image)
    }

    pub(crate) fn texture(&mut self, ctx: &Context) -> Option<&TextureHandle> {
        if let Some(image) = self.take_image() {
            if let Some(texture) = &mut self.texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.texture = Some(ctx.load_texture("IPR", image, egui::TextureOptions::LINEAR));
            }
        }
        self.texture.as_ref()
    }

    pub(crate) fn status(&self) -> String {
        if self
            .submitted
            .as_ref()
            .is_some_and(|capture| capture.moving)
            && self.displayed_samples.is_some()
        {
            return "Navigation preview · 1 spp".into();
        }
        match self.displayed_samples {
            Some((_, true)) if self.worker.is_some() => "Quick preview · refining…".into(),
            Some((samples, false)) if self.worker.is_some() => format!("{samples} spp · refining…"),
            Some((samples, false)) if self.status == "Preview complete" => format!("{samples} spp"),
            _ => self.status.clone(),
        }
    }
}

fn same_scene(a: &Scene, b: &Scene) -> bool {
    if a.revision() != b.revision() || a.render_config() != b.render_config() {
        return false;
    }
    // Revisions are document-local. The captured scene retains its root Arc, so another
    // document with a reused revision cannot accidentally compare equal after a file open.
    match (
        a.roots().first().and_then(|id| a.node(*id)),
        b.roots().first().and_then(|id| b.node(*id)),
    ) {
        (Some(a), Some(b)) => std::ptr::eq(a, b),
        (None, None) => true,
        _ => false,
    }
}
