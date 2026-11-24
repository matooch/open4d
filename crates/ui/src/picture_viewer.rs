//! Picture history and the background still-render queue (RENDER.md §R17).

use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use egui::{ColorImage, Context, TextureHandle, Ui};
use forge_core::{RenderConfig, Scene};
use forge_geometry::triangulate::triangulate;
use forge_image::{AlphaMode, Image};
use forge_render_api::{
    Aov, Control, RenderEngine, RenderError, RenderEvent, RenderOutput, RenderSettings, Snapshot,
};
use forge_render_bridge::{
    apply_scene_objects, settings_from_config, settings_from_scene, snapshot_from_polys,
};
use forge_render_cpu::CpuEngine;
use glam::DMat4;

use crate::material_preview::{Binding, load_image};

const QUEUE_LIMIT: usize = 8;
const HISTORY_LIMIT: usize = 8;
const IMAGE_BUDGET: u64 = 128 * 1024 * 1024;
const MAX_EDGE: u32 = 4096;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Channel {
    #[default]
    Beauty,
    Pass(Aov),
}

impl Channel {
    fn name(self) -> &'static str {
        match self {
            Self::Beauty => "Beauty",
            Self::Pass(aov) => aov.name(),
        }
    }
}

struct Job {
    id: u64,
    frame: i64,
    scene: Scene,
    settings: RenderSettings,
}

#[derive(Default)]
struct Progress {
    done: AtomicUsize,
    total: AtomicUsize,
}

struct Running {
    id: u64,
    frame: i64,
    control: Control,
    progress: Arc<Progress>,
    result: Receiver<Result<Completed, RenderError>>,
}

struct Completed {
    id: u64,
    frame: i64,
    output: RenderOutput,
    warnings: Vec<String>,
}

impl Completed {
    fn bytes(&self) -> u64 {
        (self.output.beauty.bytes() + self.output.aovs.values().map(Image::bytes).sum::<usize>())
            as u64
    }
}

/// Shared snapshot adapter for stills and IPR. Call on the render worker.
pub(crate) fn prepare_render(
    scene: &Scene,
    frame: i64,
    settings: &mut RenderSettings,
    control: &Control,
) -> Result<(Snapshot, Vec<String>), RenderError> {
    let frame = forge_app::schedule::Scheduler::default().evaluate(scene, frame);
    if control.is_cancelled() {
        return Err(RenderError::Cancelled);
    }
    let mut ids = Vec::new();
    let mut polys = Vec::new();
    for &id in &frame.drawn {
        if let Some(mesh) = frame.outputs.get(&id).and_then(|out| out.mesh.as_ref())
            && !mesh.polys.is_empty()
        {
            ids.push(id);
            polys.push(mesh.clone());
        }
    }
    let mut snap = snapshot_from_polys(
        &polys,
        f64::from(settings.width) / f64::from(settings.height),
    );
    let mut materials = BTreeMap::new();
    for (i, (&id, mesh)) in ids.iter().zip(&polys).enumerate() {
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let Some(binding) = Binding::resolve(&frame.scene, id) else {
            continue;
        };
        let ix = *materials.entry(binding.material.id).or_insert_with(|| {
            let graph = forge_material::node_material::compile_hosted(binding.material)
                .unwrap_or_else(|_| forge_material::compile(&forge_material::Material::new()));
            let (shaders, _) = forge_shaders::ShaderSet::from_material(
                &frame.scene,
                binding.material.id,
                &load_image,
            );
            let ix = snap.materials.len();
            snap.materials.push(
                forge_render_api::MaterialData::new(&binding.material.name, graph)
                    .with_sampler(Arc::new(shaders)),
            );
            ix
        });
        snap.instances[i].material = ix;
        let inverse = frame
            .scene
            .world_matrix(id)
            .filter(|m| m.determinant().abs() > 1e-20)
            .map_or(DMat4::IDENTITY, |m| m.inverse());
        let coords = binding.coords(mesh, inverse, true);
        snap.meshes[i].uvs = triangulate(mesh)
            .into_iter()
            .flat_map(|(f, tri)| {
                let face = &mesh.polys[f as usize];
                let uv = &coords.corners[f as usize];
                tri.map(|v| {
                    let [u, v] = uv[face
                        .iter()
                        .position(|&p| p == v)
                        .expect("corner of this face")];
                    [u, v, 0.0]
                })
            })
            .collect();
    }
    let applied = apply_scene_objects(&mut snap, &frame.scene);
    if applied.scene_has_lights {
        settings.default_light = false;
    }
    Ok((snap, applied.warnings))
}

fn render(mut job: Job, control: &Control) -> Result<Completed, RenderError> {
    let (snap, warnings) = prepare_render(&job.scene, job.frame, &mut job.settings, control)?;
    let output = CpuEngine::new().render(&snap, &job.settings, control)?;
    Ok(Completed {
        id: job.id,
        frame: job.frame,
        output,
        warnings,
    })
}

fn start(job: Job, ctx: &Context) -> std::io::Result<Running> {
    let (tx, result) = channel();
    let progress = Arc::new(Progress::default());
    let counter = progress.clone();
    let repaint = ctx.clone();
    let control = Control::with_events(move |event| {
        if let RenderEvent::Progress { done, total } = event {
            counter.total.store(*total, Ordering::Relaxed);
            counter.done.fetch_max(*done, Ordering::Relaxed);
            repaint.request_repaint();
        }
    });
    let running = Running {
        id: job.id,
        frame: job.frame,
        control: control.clone(),
        progress,
        result,
    };
    let ctx = ctx.clone();
    std::thread::Builder::new()
        .name("picture-render".into())
        .spawn(move || {
            let _ = tx.send(render(job, &control));
            ctx.request_repaint();
        })?;
    Ok(running)
}

#[derive(Default)]
pub struct PictureViewer {
    defaults: Option<RenderConfig>,
    next_id: u64,
    waiting: VecDeque<Job>,
    running: Option<Running>,
    history: VecDeque<Completed>,
    selected: Option<u64>,
    a: Option<u64>,
    b: Option<u64>,
    channel: Channel,
    exposure: f32,
    all_passes: bool,
    notice: String,
    texture: Option<((u64, Channel, u32), TextureHandle)>,
}

impl Drop for PictureViewer {
    fn drop(&mut self) {
        if let Some(job) = &self.running {
            job.control.cancel();
        }
    }
}

impl PictureViewer {
    pub(crate) fn set_defaults(&mut self, defaults: Option<RenderConfig>) {
        self.defaults = defaults;
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn viewed_id(&self) -> Option<u64> {
        self.selected
    }

    pub fn channel(&self) -> Channel {
        self.channel
    }

    pub fn viewed_output(&self) -> Option<&RenderOutput> {
        self.entry().map(|image| &image.output)
    }

    fn queue(&mut self, scene: &Scene, frame: i64) {
        let mut settings = editor_render_settings(scene, self.defaults.as_ref());
        if self.all_passes {
            settings.aovs = Aov::ALL.to_vec();
        }
        let bytes = u64::from(settings.width)
            * u64::from(settings.height)
            * (settings.aovs.len() as u64 + 1)
            * 16;
        if bytes > IMAGE_BUDGET || settings.width.max(settings.height) > MAX_EDGE {
            self.notice =
                "Render too large for Picture Viewer; reduce the size or use the CLI.".into();
            return;
        }
        if let Err(e) = settings.validate() {
            self.notice = e.to_string();
            return;
        }
        self.next_id += 1;
        self.waiting.push_back(Job {
            id: self.next_id,
            frame,
            scene: scene.clone(),
            settings,
        });
        self.notice.clear();
    }

    pub(crate) fn tick(&mut self, ctx: &Context) {
        if let Some(job) = self.running.take() {
            match job.result.try_recv() {
                Ok(Ok(image)) if !job.control.is_cancelled() => {
                    self.notice = image.warnings.join(" · ");
                    self.select(Some(image.id), false);
                    self.history.push_front(image);
                    while self.history.len() > HISTORY_LIMIT
                        || self.history.iter().map(Completed::bytes).sum::<u64>() > IMAGE_BUDGET
                    {
                        if let Some(old) = self.history.pop_back() {
                            self.a = self.a.filter(|id| *id != old.id);
                            self.b = self.b.filter(|id| *id != old.id);
                        }
                    }
                }
                Ok(Ok(_)) | Ok(Err(RenderError::Cancelled)) => {
                    self.notice = format!("Render #{} cancelled", job.id)
                }
                Ok(Err(e)) => self.notice = format!("Render #{}: {e}", job.id),
                Err(TryRecvError::Disconnected) => {
                    self.notice = format!("Render #{} worker stopped", job.id)
                }
                Err(TryRecvError::Empty) => self.running = Some(job),
            }
        }
        while self.running.is_none() {
            let Some(job) = self.waiting.pop_front() else {
                break;
            };
            match start(job, ctx) {
                Ok(job) => self.running = Some(job),
                Err(e) => self.notice = format!("Could not start render: {e}"),
            }
        }
    }

    fn select(&mut self, id: Option<u64>, preserve_pass: bool) {
        self.selected = id;
        if !preserve_pass || self.selected_image().is_none() {
            self.channel = Channel::Beauty;
        }
    }

    fn entry(&self) -> Option<&Completed> {
        let id = self.selected?;
        self.history.iter().find(|img| img.id == id)
    }

    fn selected_image(&self) -> Option<&Image> {
        let out = &self.entry()?.output;
        match self.channel {
            Channel::Beauty => Some(&out.beauty),
            Channel::Pass(aov) => out.aovs.get(&aov),
        }
    }

    fn display_image(&self) -> Option<ColorImage> {
        let img = self.selected_image()?;
        let gain = self.exposure.exp2();
        let pixels = img
            .data
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, a]| {
                let rgb = if let Channel::Pass(aov) = self.channel
                    && aov.is_data()
                {
                    if a == 0.0 {
                        [0.0; 3]
                    } else {
                        data_rgb(aov, [r, g, b])
                    }
                } else {
                    [r, g, b].map(|v| forge_image::linear_to_srgb((v * gain).clamp(0.0, 1.0)))
                };
                let [r, g, b] = rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
                egui::Color32::from_rgb(r, g, b)
            })
            .collect();
        Some(ColorImage::new(
            [img.width as usize, img.height as usize],
            pixels,
        ))
    }

    fn save(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name("render.png")
            .save_file()
        else {
            return;
        };
        let Some(display) = self.display_image() else {
            return;
        };
        let mut image = Image::new(
            display.size[0] as u32,
            display.size[1] as u32,
            AlphaMode::Straight,
        );
        for (px, c) in image
            .data
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .zip(display.pixels)
        {
            *px = c.to_array().map(|v| f32::from(v) / 255.0);
        }
        let opts = forge_image::EncodeOptions {
            space: forge_image::ColorSpace::Raw,
            ..Default::default()
        };
        let result = forge_image::encode(&image, forge_image::Format::Png, &opts)
            .map_err(|e| e.to_string())
            .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()));
        self.notice = match result {
            Ok(()) => format!("Saved {}", path.display()),
            Err(e) => format!("Save failed: {e}"),
        };
    }

    pub fn show(&mut self, ui: &mut Ui, scene: &Scene, frame: i64) {
        self.tick(ui.ctx());
        ui.horizontal_wrapped(|ui| {
            let full = self.waiting.len() + usize::from(self.running.is_some()) >= QUEUE_LIMIT;
            if ui
                .add_enabled(!full, egui::Button::new("Queue frame"))
                .clicked()
            {
                self.queue(scene, frame);
                self.tick(ui.ctx());
            }
            ui.checkbox(&mut self.all_passes, "All passes");
            if let Some(job) = &self.running {
                let total = job.progress.total.load(Ordering::Relaxed);
                let done = job.progress.done.load(Ordering::Relaxed);
                let text = format!("Rendering #{} · Frame {}", job.id, job.frame);
                ui.add(egui::ProgressBar::new(done as f32 / total.max(1) as f32).text(text));
                if ui.button("Cancel render").clicked() {
                    job.control.cancel();
                }
            }
        });
        self.waiting.retain(|job| {
            let mut keep = true;
            ui.horizontal(|ui| {
                ui.label(format!("Waiting #{} · Frame {}", job.id, job.frame));
                keep = !ui.small_button("Remove").clicked();
            });
            keep
        });
        if !self.notice.is_empty() {
            ui.label(&self.notice);
        }
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(self.selected.is_some(), egui::Button::new("Set A"))
                .clicked()
            {
                self.a = self.selected;
            }
            if ui
                .add_enabled(self.selected.is_some(), egui::Button::new("Set B"))
                .clicked()
            {
                self.b = self.selected;
            }
            if ui
                .add_enabled(self.a.is_some(), egui::Button::new("Show A"))
                .clicked()
            {
                self.select(self.a, true);
            }
            if ui
                .add_enabled(self.b.is_some(), egui::Button::new("Show B"))
                .clicked()
            {
                self.select(self.b, true);
            }
            if self.selected.is_some() {
                ui.menu_button(self.channel.name(), |ui| {
                    if ui.button("Beauty").clicked() {
                        self.channel = Channel::Beauty;
                        ui.close();
                    }
                    let passes: Vec<_> = self
                        .entry()
                        .into_iter()
                        .flat_map(|img| img.output.aovs.keys().copied())
                        .collect();
                    for aov in passes {
                        if ui.button(aov.name()).clicked() {
                            self.channel = Channel::Pass(aov);
                            ui.close();
                        }
                    }
                });
                ui.add(egui::Slider::new(&mut self.exposure, -8.0..=8.0).text("Exposure"));
                if ui.button("Save PNG").clicked() {
                    self.save();
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            let mut clicked = None;
            for image in &self.history {
                if ui
                    .selectable_label(
                        self.selected == Some(image.id),
                        format!("Render #{} · Frame {}", image.id, image.frame),
                    )
                    .clicked()
                {
                    clicked = Some(image.id);
                }
            }
            if clicked.is_some() {
                self.select(clicked, false);
            }
        });
        if let Some(id) = self.selected {
            let key = (id, self.channel, self.exposure.to_bits());
            if self.texture.as_ref().is_none_or(|(old, _)| *old != key) {
                self.texture = self.display_image().map(|image| {
                    (
                        key,
                        ui.ctx().load_texture(
                            "picture viewer image",
                            image,
                            egui::TextureOptions::LINEAR,
                        ),
                    )
                });
            }
            if let Some((_, texture)) = &self.texture {
                let tex = egui::load::SizedTexture::from_handle(texture);
                ui.add(egui::Image::new(tex).max_size(ui.available_size()));
            }
        }
    }
}

pub(crate) fn editor_render_settings(
    scene: &Scene,
    defaults: Option<&RenderConfig>,
) -> RenderSettings {
    let base = RenderSettings::default();
    if scene.render_config().is_none()
        && let Some(config) = defaults
    {
        return settings_from_config(config, base);
    }
    settings_from_scene(scene, base)
}

fn data_rgb(aov: Aov, [r, g, b]: [f32; 3]) -> [f32; 3] {
    match aov {
        Aov::Normal => [r, g, b].map(|v| (v + 1.0) * 0.5),
        Aov::Depth => [1.0 / (1.0 + r.max(0.0) * 0.01); 3],
        Aov::Position => [r, g, b].map(|v| 0.5 + v.atan() / std::f32::consts::PI),
        Aov::ObjectId | Aov::MaterialId => {
            let id = (r as u32).wrapping_add(1);
            [0x8e9c, 0x15bf, 0x6cbd].map(|v| ((id.wrapping_mul(v) >> 4) & 255) as f32 / 255.0)
        }
        _ => [r, g, b],
    }
}
