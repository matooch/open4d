//! Progressive viewport rendering with latest-snapshot cancellation (RENDER.md §R18).

use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::sync::{Arc, Mutex};

use egui::{Color32, ColorImage, Context, TextureHandle, Ui};
use forge_core::Scene;
use forge_render_api::{Control, RenderEngine, RenderError, RenderEvent, RenderSettings};
use forge_render_bridge::settings_from_scene;
use forge_render_cpu::CpuEngine;

use crate::picture_viewer::prepare_render;

struct Capture {
    ticket: u64,
    frame: i64,
    scene: Arc<Scene>,
}

struct Worker {
    ticket: u64,
    control: Control,
    result: Receiver<Result<Vec<String>, RenderError>>,
}

struct Surface {
    image: ColorImage,
    dirty: bool,
    tiles: usize,
}

impl Surface {
    fn tile(&mut self, x: u32, y: u32, w: u32, h: u32, rgba: &[f32]) -> bool {
        let [sx, sy] = self.image.size;
        let (x, y, w, h) = (x as usize, y as usize, w as usize, h as usize);
        if x >= sx
            || y >= sy
            || w == 0
            || h == 0
            || w > sx - x
            || h > sy - y
            || rgba.len() != w * h * 4
        {
            return false;
        }
        for (row, values) in rgba.chunks(w * 4).enumerate() {
            let start = (y + row) * sx + x;
            for (dst, rgba) in self.image.pixels[start..start + w]
                .iter_mut()
                .zip(values.as_chunks::<4>().0)
            {
                *dst = color(rgba);
            }
        }
        let wake = !self.dirty;
        self.dirty = true;
        self.tiles += 1;
        wake
    }
}

fn color(&[r, g, b, _]: &[f32; 4]) -> Color32 {
    egui::Rgba::from_rgb(r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)).into()
}

fn preview_settings(scene: &Scene) -> RenderSettings {
    let mut settings = settings_from_scene(scene, RenderSettings::default());
    let extent = u64::from(settings.width.max(settings.height));
    if extent > 512 {
        let scale = |v| ((u64::from(v) * 512 + extent / 2) / extent).max(1) as u32;
        settings.width = scale(settings.width);
        settings.height = scale(settings.height);
    }
    settings.samples = settings.samples.min(16);
    settings.tile = 16;
    settings
}

type SharedSurface = Arc<Mutex<Surface>>;

fn start(
    engine: Arc<dyn RenderEngine>,
    capture: Capture,
    ctx: &Context,
) -> std::io::Result<(Worker, SharedSurface)> {
    let mut settings = preview_settings(&capture.scene);
    let surface = Arc::new(Mutex::new(Surface {
        image: ColorImage::filled(
            [settings.width as usize, settings.height as usize],
            Color32::BLACK,
        ),
        dirty: false,
        tiles: 0,
    }));
    let redraw = ctx.clone();
    let sink = surface.clone();
    let control = Control::with_events(move |event| {
        if let RenderEvent::Tile {
            x,
            y,
            width,
            height,
            rgba,
        } = event
            && let Ok(mut surface) = sink.lock()
            && surface.tile(*x, *y, *width, *height, rgba)
        {
            redraw.request_repaint();
        }
    });
    let (done, result) = channel();
    let worker = Worker {
        ticket: capture.ticket,
        control: control.clone(),
        result,
    };
    let ctx = ctx.clone();
    let final_image = surface.clone();
    std::thread::Builder::new()
        .name("interactive-render".into())
        .spawn(move || {
            let result = (|| {
                let (snapshot, warnings) =
                    prepare_render(&capture.scene, capture.frame, &mut settings, &control)?;
                if control.is_cancelled() {
                    return Err(RenderError::Cancelled);
                }
                let output = engine.render(&snapshot, &settings, &control)?;
                if !control.is_cancelled()
                    && let Ok(mut surface) = final_image.lock()
                    && surface.image.size == [output.width as usize, output.height as usize]
                {
                    for (dst, src) in surface
                        .image
                        .pixels
                        .iter_mut()
                        .zip(output.beauty.data.as_chunks::<4>().0)
                    {
                        *dst = color(src);
                    }
                    surface.dirty = true;
                }
                Ok(warnings)
            })();
            let _ = done.send(result);
            ctx.request_repaint();
        })?;
    Ok((worker, surface))
}

pub struct InteractiveRender {
    engine: Arc<dyn RenderEngine>,
    submitted: Option<(Arc<Scene>, i64)>,
    ticket: u64,
    pending: Option<Capture>,
    worker: Option<Worker>,
    surface: Option<SharedSurface>,
    texture: Option<TextureHandle>,
    status: String,
    paused: bool,
}

impl Default for InteractiveRender {
    fn default() -> Self {
        Self::with_engine(Arc::new(CpuEngine::new()))
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
            paused: false,
        }
    }

    /// Hide the current generation immediately. The worker exits cooperatively.
    pub(crate) fn suspend(&mut self) {
        self.submitted = None;
        self.pending = None;
        self.surface = None;
        self.texture = None;
        if let Some(worker) = &self.worker {
            worker.control.cancel();
        }
    }

    pub fn update(&mut self, ctx: &Context, scene: &Scene, frame: i64) {
        if self.paused {
            return;
        }
        if self
            .submitted
            .as_ref()
            .is_none_or(|(old, old_frame)| *old_frame != frame || !same_scene(old, scene))
        {
            self.suspend();
            let scene = Arc::new(scene.clone());
            self.submitted = Some((scene.clone(), frame));
            self.ticket = self.ticket.wrapping_add(1);
            self.pending = Some(Capture {
                ticket: self.ticket,
                frame,
                scene,
            });
            self.status = "Preparing preview…".into();
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
            match start(self.engine.clone(), capture, ctx) {
                Ok((worker, surface)) => {
                    self.worker = Some(worker);
                    self.surface = Some(surface);
                }
                Err(e) => self.status = format!("Could not start preview: {e}"),
            }
        }
    }

    /// A copy of the latest progressive display, only when its pixels changed.
    pub fn take_image(&mut self) -> Option<ColorImage> {
        let mut surface = self.surface.as_ref()?.lock().ok()?;
        std::mem::take(&mut surface.dirty).then(|| surface.image.clone())
    }

    pub fn show(&mut self, ui: &mut Ui, scene: &Scene, frame: i64) {
        ui.horizontal(|ui| {
            if ui
                .button(if self.paused {
                    "Start preview"
                } else {
                    "Stop preview"
                })
                .clicked()
            {
                self.paused = !self.paused;
                if self.paused {
                    self.suspend();
                    self.status = "Preview stopped".into();
                }
            }
            ui.label(&self.status);
            if self.worker.is_some()
                && let Some(surface) = &self.surface
                && let Ok(surface) = surface.lock()
                && surface.tiles > 0
            {
                ui.label(format!("{} tiles", surface.tiles));
            }
        });
        self.update(ui.ctx(), scene, frame);
        if let Some(image) = self.take_image() {
            if let Some(texture) = &mut self.texture {
                texture.set(image, egui::TextureOptions::LINEAR);
            } else {
                self.texture = Some(ui.ctx().load_texture(
                    "IPR",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        if let Some(texture) = &self.texture {
            ui.add(
                egui::Image::new(egui::load::SizedTexture::from_handle(texture))
                    .max_size(ui.available_size()),
            );
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
