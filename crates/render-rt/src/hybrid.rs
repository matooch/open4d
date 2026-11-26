//! Tier 2 ray-traced effects and per-view history (RENDER.md R24).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_render_api::{Control, RenderError, RenderSettings, Snapshot};
use forge_render_cpu::lights;
use forge_render_cpu::sampling::{Rng, cosine_hemisphere, lum, power_heuristic, to_world};
use forge_render_cpu::scene::{Hit, HitGeom, Scene};
use forge_render_cpu::surface::Surface;
use glam::DVec3;

use crate::ambient::{AmbientFrame, AmbientProgram};
use crate::ray::{self, BackendKind, DeviceQueries, Ray, RtBackend};
use direct_shadow::{DirectShadowBindings, DirectShadowFrame, DirectShadowProgram};
use reflection::{ReflectionBindings, ReflectionInput, ReflectionProgram};

const SHADOW_RAYS: usize = 8;
const BOUNCE_RAYS: usize = 8;
// At most 512 KiB each for ray input and hit output, well within supported
// storage binding and compute dispatch limits.
const PIXELS_PER_BATCH: usize = 2048;

#[derive(Default)]
pub(crate) struct HybridView {
    backend: Option<(BackendKind, Box<dyn DeviceQueries>)>,
    ambient_program: Option<AmbientProgram>,
    shadow_program: Option<MaterialProgram<DirectShadowProgram>>,
    reflection_program: Option<MaterialProgram<ReflectionProgram>>,
    attributes: Option<wgpu::Buffer>,
    scene: Option<Snapshot>,
    geometry: Option<PrimaryGeometry>,
    settings: Option<RenderSettings>,
    // Each raster TAA jitter has different primary visibility.
    ao: BTreeMap<[u32; 2], History<f32>>,
    shadows: BTreeMap<[u32; 2], History<[f32; 8]>>,
    reflections: BTreeMap<[u32; 2], History<[f32; 4]>>,
    gi: BTreeMap<[u32; 2], History<[f32; 4]>>,
}

#[derive(Clone, Copy)]
pub(crate) enum Bounce {
    Reflection,
    Diffuse,
}

impl Bounce {
    fn sample(self, p: &PrimarySurface, rng: &mut Rng) -> Option<(DVec3, DVec3)> {
        match self {
            Self::Reflection => p.surface.sample_specular(p.normal, p.view, rng),
            Self::Diffuse if p.normal.dot(p.view) > 0.0 => {
                let weight = p.surface.albedo * (DVec3::ONE - p.surface.trans);
                (weight != DVec3::ZERO)
                    .then(|| (to_world(p.normal, cosine_hemisphere(rng.f2())), weight))
            }
            Self::Diffuse => None,
        }
    }

    fn miss(self, snapshot: &Snapshot, direction: DVec3) -> DVec3 {
        match self {
            Self::Reflection => lights::env_radiance(&snapshot.environment.lighting, direction),
            Self::Diffuse => DVec3::ZERO,
        }
    }
}

#[derive(Default)]
struct History<T> {
    frames: u32,
    mean: Vec<T>,
}

#[derive(Clone, Copy)]
pub(crate) struct PrimaryInput<'a> {
    pub positions: &'a [[f32; 4]],
    pub normals: &'a [[f32; 4]],
    pub jitter: [f32; 2],
    pub epsilon: f64,
}

#[derive(Clone, Copy)]
pub(crate) struct AmbientGpu<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub textures: [&'a wgpu::TextureView; 2],
}

#[derive(Clone, Copy)]
pub(crate) struct MaterialGpu<'a> {
    pub primary: AmbientGpu<'a>,
    pub materials: &'a str,
    pub bitmaps: &'a [wgpu::TextureView],
}

struct MaterialProgram<P> {
    kind: BackendKind,
    materials: String,
    bitmap_count: usize,
    program: P,
}

impl<P> MaterialProgram<P> {
    fn get<'a>(
        cache: &'a mut Option<Self>,
        kind: BackendKind,
        gpu: MaterialGpu<'_>,
        compile: impl FnOnce(&str) -> P,
    ) -> &'a P {
        if cache.as_ref().is_none_or(|cached| {
            cached.kind != kind
                || cached.materials != gpu.materials
                || cached.bitmap_count != gpu.bitmaps.len()
        }) {
            // Only move the declarations. Both passes bind the same resource table.
            let group = crate::shader::BITMAP_GROUP;
            let source = gpu
                .materials
                .replace(&format!("@group({group}) @binding("), "@group(1) @binding(");
            *cache = Some(Self {
                kind,
                materials: gpu.materials.to_owned(),
                bitmap_count: gpu.bitmaps.len(),
                program: compile(&source),
            });
        }
        &cache.as_ref().expect("compiled material program").program
    }
}

fn resident_attributes<'a>(
    cache: &'a mut Option<wgpu::Buffer>,
    device: &wgpu::Device,
    snapshot: &Snapshot,
    geometry: &Scene,
) -> Result<&'a wgpu::Buffer, RenderError> {
    if cache.is_none() {
        *cache = Some(crate::path_trace::attributes(device, snapshot, geometry)?);
    }
    Ok(cache.as_ref().expect("uploaded attributes"))
}

impl HybridView {
    pub fn backend_kind(&self) -> Option<BackendKind> {
        self.backend.as_ref().map(|(kind, _)| *kind)
    }

    pub fn reset(&mut self) {
        self.ao.clear();
        self.shadows.clear();
        self.reflections.clear();
        self.gi.clear();
    }

    pub fn prepare(
        &mut self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), RenderError> {
        if self.backend.is_none() {
            self.backend = Some(ray::on_device(device.clone(), queue.clone())?);
        }
        let same_geometry = self.scene.as_ref().is_some_and(|old| {
            old.meshes == snapshot.meshes && old.instances == snapshot.instances
        });
        if !same_geometry {
            self.backend
                .as_mut()
                .expect("initialized ray backend")
                .1
                .update_scene(snapshot)?;
            self.geometry = None;
            self.attributes = None;
        }
        let same_scene = same_geometry
            && self.scene.as_ref().is_some_and(|old| {
                old.camera == snapshot.camera
                    && old.lights == snapshot.lights
                    && old.environment == snapshot.environment
                    && old.materials.len() == snapshot.materials.len()
                    && old
                        .materials
                        .iter()
                        .zip(&snapshot.materials)
                        .all(|(a, b)| a.graph == b.graph && Arc::ptr_eq(&a.sampler, &b.sampler))
            });
        if !same_scene || self.settings.as_ref() != Some(settings) {
            self.reset();
            self.settings = Some(settings.clone());
        }
        if !same_scene {
            self.scene = Some(snapshot.clone());
        }
        Ok(())
    }

    pub fn ambient_visibility(
        &mut self,
        settings: &RenderSettings,
        input: PrimaryInput<'_>,
        control: &Control,
        gpu: Option<AmbientGpu<'_>>,
    ) -> Result<Vec<f32>, RenderError> {
        let radius = settings.hybrid.ao_radius;
        if radius == 0.0 || settings.post.ssao_strength == 0.0 {
            return Ok(vec![1.0; input.positions.len()]);
        }
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let (kind, backend) = self.backend.as_ref().expect("prepared ray backend");
        let history = self.ao.entry(input.jitter.map(f32::to_bits)).or_default();
        let raw = match (gpu, backend.inline_scene()) {
            (Some(gpu), Some(scene)) => {
                let result = self
                    .ambient_program
                    .get_or_insert_with(|| AmbientProgram::new(gpu.device, *kind))
                    .read(
                        (gpu.device, gpu.queue),
                        scene,
                        gpu.textures,
                        settings,
                        AmbientFrame::new(settings, history.frames, input.epsilon),
                    );
                match result {
                    Err(RenderError::Unsupported(_)) => None,
                    result => Some(result?),
                }
            }
            _ => None,
        };
        let visibility = match raw {
            Some(raw) => raw,
            None => raw_ambient_visibility(&**backend, settings, input, history.frames, control)?,
        };
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let filtered = spatial_filter(settings.width as usize, &input, &visibility, radius * 0.15);
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        history.accumulate(filtered);
        let strength = settings.post.ssao_strength as f32;
        Ok(history
            .mean
            .iter()
            .map(|open| 1.0 - (1.0 - open) * strength)
            .collect())
    }
}

fn raw_ambient_visibility(
    backend: &dyn RtBackend,
    settings: &RenderSettings,
    input: PrimaryInput<'_>,
    history_frame: u32,
    control: &Control,
) -> Result<Vec<f32>, RenderError> {
    let radius = settings.hybrid.ao_radius;
    let mut visibility = vec![1.0; input.positions.len()];
    let width = settings.width as usize;
    let height = settings.height as usize;
    let quad_width = width.div_ceil(2);
    let quads = quad_width * height.div_ceil(2);
    let quads_per_batch = PIXELS_PER_BATCH / 4;
    let phase = history_frame as usize % 4;
    let limit = radius * 0.15;
    let epsilon = input.epsilon.min(radius * 0.01);
    let mut rays = Vec::with_capacity(PIXELS_PER_BATCH);
    let mut receivers = Vec::with_capacity(PIXELS_PER_BATCH);
    for first in (0..quads).step_by(quads_per_batch) {
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        rays.clear();
        receivers.clear();
        for quad in first..(first + quads_per_batch).min(quads) {
            let x = 2 * (quad % quad_width);
            let y = 2 * (quad / quad_width);
            let mut representatives = [None; 4];
            for (order, lane) in (phase..phase + 4).map(|lane| lane % 4).enumerate() {
                let px = x + lane % 2;
                let py = y + lane / 2;
                if px >= width || py >= height {
                    continue;
                }
                let i = py * width + px;
                let material = input.positions[i][3];
                let n = xyz(input.normals[i]).normalize_or_zero();
                if material < 0.5 || n == DVec3::ZERO {
                    continue;
                }
                let p = xyz(input.positions[i]);
                let existing = representatives.iter().flatten().find_map(
                    |&(other_p, other_n, other_material, ray)| {
                        let delta: DVec3 = p - other_p;
                        (material == other_material
                            && n.dot(other_n) >= 0.95
                            && delta.length_squared() <= limit * limit
                            && delta.dot(n).abs() <= limit * 0.1
                            && delta.dot(other_n).abs() <= limit * 0.1)
                            .then_some(ray)
                    },
                );
                let ray = existing.unwrap_or_else(|| {
                    let ray = rays.len();
                    representatives[order] = Some((p, n, material, ray));
                    let mut rng = Rng::new(settings.seed ^ u64::from(history_frame), i as u64);
                    rays.push(Ray {
                        origin: p + n * epsilon,
                        direction: to_world(n, cosine_hemisphere(rng.f2())),
                        t_max: radius,
                    });
                    ray
                });
                receivers.push((i, ray));
            }
        }
        let blocked = backend.occluded(&rays)?;
        for &(i, ray) in &receivers {
            visibility[i] = if blocked[ray] { 0.0 } else { 1.0 };
        }
    }

    Ok(visibility)
}

impl HybridView {
    pub fn direct_shadows(
        &mut self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        input: PrimaryInput<'_>,
        control: &Control,
        gpu: Option<MaterialGpu<'_>>,
    ) -> Result<Vec<[f32; 8]>, RenderError> {
        let lights: Vec<_> = lights::prepare(snapshot, settings.default_light)?
            .into_iter()
            .filter(lights::PLight::casts_shadow)
            .collect();
        if lights.is_empty() {
            return Ok(Vec::new());
        }
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        if self.geometry.is_none() {
            self.geometry = Some(PrimaryGeometry::new(snapshot)?);
        }
        let geometry = self.geometry.as_ref().expect("prepared shading geometry");
        let (kind, backend) = self.backend.as_ref().expect("prepared ray backend");
        let history = self
            .shadows
            .entry(input.jitter.map(f32::to_bits))
            .or_default();
        let inline = match (gpu, backend.inline_scene()) {
            (Some(gpu), Some(scene)) => (|| {
                let MaterialGpu {
                    primary:
                        AmbientGpu {
                            device,
                            queue,
                            textures,
                        },
                    bitmaps,
                    ..
                } = gpu;
                let storage_bindings = if *kind == BackendKind::Compute { 7 } else { 3 };
                if device.limits().max_storage_buffers_per_shader_stage < storage_bindings {
                    return Err(RenderError::Unsupported(
                        "raw direct-shadow bindings exceed device limits".into(),
                    ));
                }
                let attributes =
                    resident_attributes(&mut self.attributes, device, snapshot, &geometry.scene)?;
                MaterialProgram::get(&mut self.shadow_program, *kind, gpu, |materials| {
                    let source = crate::shader::direct_shadow_source(*kind, materials);
                    DirectShadowProgram::new(device, *kind, source, bitmaps.len())
                })
                .read(
                    (device, queue),
                    DirectShadowBindings {
                        scene,
                        primary: textures,
                        attributes,
                        bitmaps,
                        lights: &lights,
                    },
                    settings,
                    DirectShadowFrame::new(
                        snapshot,
                        settings,
                        input,
                        geometry.scene.scale,
                        history.frames,
                        lights.len(),
                    ),
                )
            })(),
            _ => Err(RenderError::Unsupported(
                "no inline direct-shadow scene".into(),
            )),
        };
        let pixels = match inline {
            Err(RenderError::Unsupported(_)) => raw_direct_shadows(
                (geometry, &**backend),
                snapshot,
                settings,
                input,
                &lights,
                history.frames,
                control,
            )?,
            result => result?,
        };
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let width = settings.width as usize;
        let limit = geometry.filter_limit(snapshot, width);
        let filtered = spatial_filter(width, &input, &pixels, limit);
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        history.accumulate(filtered);
        Ok(history.mean.clone())
    }

    #[cfg(test)]
    pub fn shadow_residency(&self) -> Option<(wgpu::Buffer, wgpu::ComputePipeline)> {
        Some((
            self.attributes.as_ref()?.clone(),
            self.shadow_program.as_ref()?.program.pipeline(),
        ))
    }

    #[cfg(test)]
    pub fn reflection_residency(&self) -> Option<(wgpu::Buffer, wgpu::ComputePipeline)> {
        Some((
            self.attributes.as_ref()?.clone(),
            self.reflection_program.as_ref()?.program.pipeline(),
        ))
    }
}

fn raw_direct_shadows(
    (geometry, backend): (&PrimaryGeometry, &dyn RtBackend),
    snapshot: &Snapshot,
    settings: &RenderSettings,
    input: PrimaryInput<'_>,
    lights: &[lights::PLight],
    history_frame: u32,
    control: &Control,
) -> Result<Vec<[f32; 8]>, RenderError> {
    let mut pixels = vec![[0.0; 8]; input.positions.len()];
    let mut rays = Vec::with_capacity(PIXELS_PER_BATCH * SHADOW_RAYS);
    let mut samples = Vec::with_capacity(rays.capacity());
    for first in (0..pixels.len()).step_by(PIXELS_PER_BATCH) {
        let end = (first + PIXELS_PER_BATCH).min(pixels.len());
        let primary: Vec<_> = (first..end)
            .filter(|&i| input.positions[i][3] >= 0.5)
            .map(|i| (i, geometry.surface(snapshot, settings, &input, i)))
            .collect();
        for (l, light) in lights.iter().enumerate() {
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            rays.clear();
            samples.clear();
            let density = light.light.shadow_density;
            let occluded =
                DVec3::splat(1.0 - density) + DVec3::from(light.light.shadow_color) * density;
            for (i, p) in &primary {
                let stream = (*i as u64).wrapping_mul(lights.len() as u64) + l as u64;
                let mut rng = Rng::new(settings.seed ^ u64::from(history_frame), stream);
                for _ in 0..SHADOW_RAYS {
                    let Some(s) = light.sample(p.position, &mut rng) else {
                        continue;
                    };
                    let cos = p.normal.dot(s.wi);
                    if cos <= 0.0 || p.geometric_normal.dot(s.wi) <= 0.0 {
                        continue;
                    }
                    let (mut fd, mut fs) = p.surface.eval(p.normal, p.view, s.wi);
                    if !light.light.diffuse {
                        fd = DVec3::ZERO;
                    }
                    if !light.light.specular {
                        fs = DVec3::ZERO;
                    }
                    if fd == DVec3::ZERO && fs == DVec3::ZERO {
                        continue;
                    }
                    let mis = if s.pdf > 0.0 {
                        power_heuristic(s.pdf, p.surface.pdf(p.normal, p.view, s.wi))
                    } else {
                        1.0
                    };
                    let weight = s.weight * (cos * mis / SHADOW_RAYS as f64);
                    samples.push((*i, fd * weight, fs * weight));
                    rays.push(Ray {
                        origin: p.position + p.geometric_normal * input.epsilon,
                        direction: s.wi,
                        t_max: (s.dist * (1.0 - 1e-6) - input.epsilon).max(0.0),
                    });
                }
            }
            for ((i, d, s), blocked) in samples.iter().zip(backend.occluded(&rays)?) {
                let factor = if blocked { occluded } else { DVec3::ONE };
                let [dr, dg, db] = (*d * factor).to_array();
                let [sr, sg, sb] = (*s * factor).to_array();
                let total = *d + *s;
                pixels[*i].add_scaled(
                    [dr, dg, db, lum(total), sr, sg, sb, lum(total * factor)].map(|v| v as f32),
                    1.0,
                );
            }
        }
    }
    Ok(pixels)
}

impl HybridView {
    pub fn indirect(
        &mut self,
        bounce: Bounce,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        input: PrimaryInput<'_>,
        control: &Control,
        gpu: Option<MaterialGpu<'_>>,
    ) -> Result<Vec<[f32; 4]>, RenderError> {
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        if self.geometry.is_none() {
            self.geometry = Some(PrimaryGeometry::new(snapshot)?);
        }
        let geometry = self.geometry.as_ref().expect("prepared shading geometry");
        let (kind, backend) = self.backend.as_ref().expect("prepared ray backend");
        let lights = lights::prepare(snapshot, settings.default_light)?;
        let history = match bounce {
            Bounce::Reflection => &mut self.reflections,
            Bounce::Diffuse => &mut self.gi,
        }
        .entry(input.jitter.map(f32::to_bits))
        .or_default();
        let inline = match (bounce, gpu, backend.inline_scene()) {
            (Bounce::Reflection, Some(gpu), Some(scene)) => (|| {
                let AmbientGpu {
                    device,
                    queue,
                    textures,
                } = gpu.primary;
                let storage_bindings = if *kind == BackendKind::Compute { 7 } else { 3 };
                let limits = device.limits();
                if limits.max_storage_buffers_per_shader_stage < storage_bindings
                    || gpu.bitmaps.len() + 3 > limits.max_sampled_textures_per_shader_stage as usize
                {
                    return Err(RenderError::Unsupported(
                        "raw reflection bindings exceed device limits".into(),
                    ));
                }
                let attributes =
                    resident_attributes(&mut self.attributes, device, snapshot, &geometry.scene)?;
                MaterialProgram::get(&mut self.reflection_program, *kind, gpu, |materials| {
                    let source = crate::shader::reflection_source(*kind, materials);
                    ReflectionProgram::new(device, *kind, source, gpu.bitmaps.len())
                })
                .read(
                    (device, queue),
                    ReflectionBindings {
                        scene,
                        primary: textures,
                        attributes,
                        bitmaps: gpu.bitmaps,
                        lights: &lights,
                    },
                    ReflectionInput {
                        snapshot,
                        settings,
                        input,
                        scale: geometry.scene.scale,
                        history: history.frames,
                    },
                )
            })(),
            _ => Err(RenderError::Unsupported(
                "no inline reflection scene".into(),
            )),
        };
        let pixels = match inline {
            Err(RenderError::Unsupported(_)) => raw_indirect(
                (geometry, &**backend),
                bounce,
                ReflectionInput {
                    snapshot,
                    settings,
                    input,
                    scale: geometry.scene.scale,
                    history: history.frames,
                },
                &lights,
                control,
            )?,
            result => result?,
        };
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let width = settings.width as usize;
        let filtered = spatial_filter(
            width,
            &input,
            &pixels,
            geometry.filter_limit(snapshot, width),
        );
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        history.accumulate(filtered);
        Ok(history.mean.clone())
    }
}

fn raw_indirect(
    (geometry, backend): (&PrimaryGeometry, &dyn RtBackend),
    bounce: Bounce,
    ReflectionInput {
        snapshot,
        settings,
        input,
        history,
        ..
    }: ReflectionInput<'_>,
    lights: &[lights::PLight],
    control: &Control,
) -> Result<Vec<[f32; 4]>, RenderError> {
    let mut pixels = vec![[0.0; 4]; input.positions.len()];
    let mut rays = Vec::with_capacity(PIXELS_PER_BATCH * BOUNCE_RAYS);
    let mut samples = Vec::with_capacity(rays.capacity());
    let mut secondary = Vec::with_capacity(rays.capacity());
    let seed = settings.seed ^ u64::from(history);
    for first in (0..pixels.len()).step_by(PIXELS_PER_BATCH) {
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let end = (first + PIXELS_PER_BATCH).min(pixels.len());
        rays.clear();
        samples.clear();
        secondary.clear();
        for i in (first..end).filter(|&i| input.positions[i][3] >= 0.5) {
            let p = geometry.surface(snapshot, settings, &input, i);
            let mut rng = Rng::new(seed, i as u64);
            for sample in 0..BOUNCE_RAYS {
                let Some((wi, weight)) = bounce.sample(&p, &mut rng) else {
                    continue;
                };
                if p.geometric_normal.dot(wi) <= 0.0 {
                    continue;
                }
                rays.push(Ray {
                    origin: p.position + p.geometric_normal * input.epsilon,
                    direction: wi,
                    t_max: f64::INFINITY,
                });
                samples.push((i, sample, weight / BOUNCE_RAYS as f64));
            }
        }
        for ((ray, &(i, sample, weight)), hit) in
            rays.iter().zip(&samples).zip(backend.trace(&rays)?)
        {
            let radiance = if let Some(h) = hit {
                let geom = geometry.scene.geometry(
                    snapshot,
                    ray.origin,
                    ray.direction,
                    &Hit {
                        t: h.t,
                        inst: h.instance,
                        tri: h.triangle,
                        u: h.u,
                        v: h.v,
                    },
                );
                let p = geometry.shade(snapshot, geom, h.instance, -ray.direction);
                let emission = p.surface.emission;
                if !lights.is_empty() {
                    secondary.push(Secondary {
                        pixel: i,
                        stream: (i * BOUNCE_RAYS + sample) as u64,
                        weight,
                        point: p,
                    });
                }
                emission
            } else {
                bounce.miss(snapshot, ray.direction)
            };
            add_rgb(&mut pixels[i], radiance * weight);
        }
        for (index, light) in lights.iter().enumerate() {
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            add_secondary_direct(
                backend,
                light,
                &secondary,
                &mut pixels,
                seed ^ 0xed20e2d079fa645b ^ index as u64,
                input.epsilon,
            )?;
        }
    }
    Ok(pixels)
}

struct Secondary {
    pixel: usize,
    stream: u64,
    weight: DVec3,
    point: PrimarySurface,
}

fn add_rgb(pixel: &mut [f32; 4], value: DVec3) {
    pixel.add_scaled(value.extend(0.0).to_array().map(|v| v as f32), 1.0);
}

fn add_secondary_direct(
    backend: &dyn RtBackend,
    light: &lights::PLight,
    secondary: &[Secondary],
    pixels: &mut [[f32; 4]],
    seed: u64,
    epsilon: f64,
) -> Result<(), RenderError> {
    let mut rays = Vec::new();
    let mut contributions = Vec::new();
    for s in secondary {
        let p = &s.point;
        let Some(sample) = light.sample(p.position, &mut Rng::new(seed, s.stream)) else {
            continue;
        };
        let cos = p.normal.dot(sample.wi);
        if cos <= 0.0 || p.geometric_normal.dot(sample.wi) <= 0.0 {
            continue;
        }
        let (d, r) = p.surface.eval(p.normal, p.view, sample.wi);
        let value = ((if light.light.diffuse { d } else { DVec3::ZERO })
            + if light.light.specular { r } else { DVec3::ZERO })
            * (cos * sample.weight)
            * s.weight;
        if value == DVec3::ZERO {
            continue;
        }
        if light.casts_shadow() {
            rays.push(Ray {
                origin: p.position + p.geometric_normal * epsilon,
                direction: sample.wi,
                t_max: (sample.dist * (1.0 - 1e-6) - epsilon).max(0.0),
            });
            contributions.push((s.pixel, value));
        } else {
            add_rgb(&mut pixels[s.pixel], value);
        }
    }
    if !rays.is_empty() {
        let density = light.light.shadow_density;
        let tint = DVec3::splat(1.0 - density) + DVec3::from(light.light.shadow_color) * density;
        for ((pixel, value), blocked) in contributions.into_iter().zip(backend.occluded(&rays)?) {
            add_rgb(
                &mut pixels[pixel],
                if blocked { value * tint } else { value },
            );
        }
    }
    Ok(())
}

struct PrimaryGeometry {
    scene: Scene,
    /// Cumulative end indices in the raster triangle stream, in instance order.
    ends: Vec<usize>,
}

#[path = "direct_shadow.rs"]
mod direct_shadow;

#[cfg(test)]
#[path = "receiver_tests.rs"]
mod receiver_tests;

#[cfg(test)]
#[path = "direct_shadow_tests.rs"]
mod direct_shadow_tests;

#[path = "reflection.rs"]
mod reflection;

#[cfg(test)]
#[path = "reflection_tests.rs"]
mod reflection_tests;

struct PrimarySurface {
    position: DVec3,
    normal: DVec3,
    geometric_normal: DVec3,
    view: DVec3,
    surface: Surface,
}

impl PrimaryGeometry {
    fn new(snapshot: &Snapshot) -> Result<Self, RenderError> {
        let scene = Scene::build(snapshot)?;
        let mut end = 0;
        let ends = snapshot
            .instances
            .iter()
            .map(|i| {
                end += snapshot.meshes[i.mesh].triangles.len();
                end
            })
            .collect();
        Ok(Self { scene, ends })
    }

    fn surface(
        &self,
        snap: &Snapshot,
        settings: &RenderSettings,
        g: &PrimaryInput<'_>,
        i: usize,
    ) -> PrimarySurface {
        let position = xyz(g.positions[i]);
        let packed = g.normals[i][3] as usize - 1;
        let inst = self.ends.partition_point(|&end| end <= packed);
        let tri = packed - inst.checked_sub(1).map_or(0, |j| self.ends[j]);
        let geom = self.scene.geometry_at(snap, position, inst, tri);
        let w = settings.width as usize;
        let (_, d) = snap.camera.ray(
            (i % w) as f64 + 0.5 - f64::from(g.jitter[0]),
            (i / w) as f64 + 0.5 - f64::from(g.jitter[1]),
            settings.width,
            settings.height,
        );
        self.shade(snap, geom, inst, -d)
    }

    fn filter_limit(&self, snapshot: &Snapshot, width: usize) -> f64 {
        3.0 * match snapshot.camera.projection {
            forge_render_api::Projection::Orthographic { width: w } => w / width as f64,
            _ => self.scene.scale / width as f64,
        }
    }

    fn shade(&self, snap: &Snapshot, geom: HitGeom, inst: usize, view: DVec3) -> PrimarySurface {
        let mat = &snap.materials[self.scene.insts[inst].material];
        let surface = Surface::at(&mat.graph, &*mat.sampler, &geom, view, self.scene.scale);
        let sign = if geom.ng.dot(view) > 0.0 { 1.0 } else { -1.0 };
        let geometric_normal = geom.ng * sign;
        let n = surface.n * sign;
        let normal = if n.dot(geometric_normal) < 0.0 {
            geometric_normal
        } else {
            n
        };
        PrimarySurface {
            position: geom.p,
            normal,
            geometric_normal,
            view,
            surface,
        }
    }
}

fn xyz([x, y, z, _]: [f32; 4]) -> DVec3 {
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
}

trait Signal: Copy {
    fn add_scaled(&mut self, value: Self, weight: f32);
    fn mean_update(&mut self, value: Self, weight: f32);
    fn scale(self, factor: f32) -> Self;
}

impl Signal for f32 {
    fn add_scaled(&mut self, value: Self, weight: f32) {
        *self += value * weight;
    }
    fn mean_update(&mut self, value: Self, weight: f32) {
        *self += (value - *self) * weight;
    }
    fn scale(self, factor: f32) -> Self {
        self * factor
    }
}

impl<const N: usize> Signal for [f32; N] {
    fn add_scaled(&mut self, value: Self, weight: f32) {
        for (a, b) in self.iter_mut().zip(value) {
            a.add_scaled(b, weight);
        }
    }
    fn mean_update(&mut self, value: Self, weight: f32) {
        for (a, b) in self.iter_mut().zip(value) {
            a.mean_update(b, weight);
        }
    }
    fn scale(self, factor: f32) -> Self {
        self.map(|v| v * factor)
    }
}

impl<T: Signal> History<T> {
    fn accumulate(&mut self, filtered: Vec<T>) {
        self.frames = self.frames.saturating_add(1);
        if self.mean.is_empty() {
            self.mean = filtered;
        } else {
            let weight = 1.0 / self.frames as f32;
            for (mean, new) in self.mean.iter_mut().zip(filtered) {
                mean.mean_update(new, weight);
            }
        }
    }
}

fn spatial_filter<T: Signal>(width: usize, g: &PrimaryInput<'_>, mask: &[T], limit: f64) -> Vec<T> {
    let height = mask.len() / width;
    mask.iter()
        .enumerate()
        .map(|(i, &value)| {
            if g.positions[i][3] < 0.5 {
                return value;
            }
            let x = i % width;
            let y = i / width;
            let p = xyz(g.positions[i]);
            let n = xyz(g.normals[i]);
            let mut sum = value.scale(2.0);
            let mut weight = 2.0_f32;
            for ny in y.saturating_sub(1)..=(y + 1).min(height - 1) {
                for nx in x.saturating_sub(1)..=(x + 1).min(width - 1) {
                    let j = ny * width + nx;
                    if j == i || g.positions[j][3] != g.positions[i][3] {
                        continue;
                    }
                    let delta = xyz(g.positions[j]) - p;
                    if n.dot(xyz(g.normals[j])) < 0.95
                        || delta.length_squared() > limit * limit
                        || delta.dot(n).abs() > limit * 0.1
                    {
                        continue;
                    }
                    sum.add_scaled(mask[j], 1.0);
                    weight += 1.0;
                }
            }
            sum.scale(weight.recip())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ray::{RayHit, SceneStats};
    use std::sync::Mutex;

    #[test]
    fn gpu_raw_ao_matches_cpu_across_all_quad_phases() {
        let _serial = crate::gpu_test_serial();
        use forge_render_api::{Instance, MaterialData, MeshData};
        use glam::DMat4;

        for request in [crate::request_default_device, crate::request_rt_device] {
            let (device, queue) = match request() {
                Ok(gpu) => gpu,
                Err(err) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
                    panic!("FORGE_REQUIRE_GPU=1 but no GPU: {err}");
                }
                Err(err) => {
                    eprintln!("skipping GPU test: {err}");
                    return;
                }
            };
            let snapshot = Snapshot {
                meshes: vec![MeshData::new(
                    vec![[-10., -10., 0.], [10., -10., 0.], [0., 10., 0.]],
                    vec![[0, 1, 2]],
                )],
                instances: vec![Instance::new(0, DMat4::IDENTITY, 0, 1)],
                materials: vec![MaterialData::new(
                    "occluder",
                    forge_material::ir::ShadingGraph {
                        nodes: vec![],
                        outputs: Default::default(),
                    },
                )],
                ..Snapshot::default()
            };
            let (kind, mut backend) = ray::on_device(device.clone(), queue.clone()).unwrap();
            assert_eq!(kind, ray::enabled_backend(&device));
            backend.update_scene(&snapshot).unwrap();
            let mut settings = RenderSettings {
                width: 3,
                height: 3,
                seed: 0x1234_abcd_f00d_5678,
                ..RenderSettings::default()
            };
            settings.hybrid.ao_radius = 1.0;
            let positions = [
                [0.00, 0., -0.005, 1.],
                [0.01, 0., 0.005, 1.], // Reuse despite different own-ray visibility.
                [0.; 4],               // Clear outside the crop.
                [0.02, 0., -0.08, 1.], // Same material, incompatible depth.
                [0.03, 0., 0.005, 2.], // Incompatible material.
                [0.04, 0., -0.005, 1.],
                [0.05, 0., 0.005, 1.],
                [0.06, 0., 0.005, 1.], // Incompatible normal within the bottom quad.
                [0.07, 0., 0.005, 1.],
            ];
            let mut normals = [[0., 0., 1., 0.]; 9];
            normals[7] = [0., 0., -1., 0.];
            normals[8] = [0.; 4];
            let input = PrimaryInput {
                positions: &positions,
                normals: &normals,
                jitter: [0.; 2],
                epsilon: 1e-6,
            };
            let texture_view = |data: &[[f32; 4]; 9]| {
                let size = wgpu::Extent3d {
                    width: 3,
                    height: 3,
                    depth_or_array_layers: 1,
                };
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba32Float,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                queue.write_texture(
                    texture.as_image_copy(),
                    bytemuck::cast_slice(data),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(48),
                        rows_per_image: Some(3),
                    },
                    size,
                );
                texture.create_view(&Default::default())
            };
            let textures = [texture_view(&positions), texture_view(&normals)];
            let program = AmbientProgram::new(&device, kind);
            let control = Control::new();
            for phase in 0..4 {
                let reference =
                    raw_ambient_visibility(&*backend, &settings, input, phase, &control).unwrap();
                let raw = program
                    .read(
                        (&device, &queue),
                        backend.inline_scene().unwrap(),
                        [&textures[0], &textures[1]],
                        &settings,
                        AmbientFrame::new(&settings, phase, input.epsilon),
                    )
                    .unwrap();
                assert_eq!(raw, reference, "{kind:?} phase {phase}");
                let shared = if phase == 1 { 1.0 } else { 0.0 };
                assert_eq!(raw, [shared, shared, 1., 0., 1., 0., 1., 0., 1.]);
            }
        }
    }

    struct RecordingBackend(Arc<Mutex<Vec<Ray>>>);

    impl DeviceQueries for RecordingBackend {}

    impl RtBackend for RecordingBackend {
        fn update_scene(&mut self, _: &Snapshot) -> Result<(), RenderError> {
            Ok(())
        }

        fn trace(&self, rays: &[Ray]) -> Result<Vec<Option<RayHit>>, RenderError> {
            self.0.lock().unwrap().extend_from_slice(rays);
            Ok(rays
                .iter()
                .map(|ray| {
                    (ray.origin.x < 0.015).then_some(RayHit {
                        t: 0.5,
                        instance: 0,
                        triangle: 0,
                        u: 0.0,
                        v: 0.0,
                    })
                })
                .collect())
        }

        fn stats(&self) -> SceneStats {
            SceneStats::default()
        }
    }

    #[test]
    fn ao_reuses_only_compatible_representatives_in_absolute_partial_quads() {
        let recorded = Arc::new(Mutex::new(Vec::new()));
        let mut view = HybridView {
            backend: Some((
                BackendKind::Compute,
                Box::new(RecordingBackend(recorded.clone())),
            )),
            ..HybridView::default()
        };
        let mut settings = RenderSettings {
            width: 3,
            height: 3,
            ..RenderSettings::default()
        };
        settings.hybrid.ao_radius = 1.0;
        settings.post.ssao_strength = 1.0;
        let positions = [
            [0.00, 0.00, 0.00, 1.0],
            [0.01, 0.00, 0.00, 1.0], // Reuse within the same surface.
            [0.00; 4],               // No coverage at the partial right quad's origin.
            [0.02, 0.01, 0.03, 1.0], // Depth discontinuity at the same material.
            [0.03, 0.01, 0.00, 2.0], // Material boundary.
            [0.04, 0.01, 0.00, 1.0], // First covered pixel in partial right quad.
            [0.05, 0.02, 0.00, 1.0],
            [0.06, 0.02, 0.00, 1.0], // Normal discontinuity.
            [0.07, 0.02, 0.00, 1.0], // Invalid normal.
        ];
        let mut normals = [[0.0, 0.0, 1.0, 1.0]; 9];
        normals[7] = [1.0, 0.0, 0.0, 1.0];
        normals[8] = [0.0; 4];
        let epsilon = 0.000001;
        let input = PrimaryInput {
            positions: &positions,
            normals: &normals,
            jitter: [0.0; 2],
            epsilon,
        };
        let check_rays = |expected: &[usize]| {
            let rays = std::mem::take(&mut *recorded.lock().unwrap());
            assert_eq!(rays.len(), expected.len());
            for (ray, &i) in rays.iter().zip(expected.iter()) {
                let n = xyz(normals[i]);
                assert_eq!(ray.origin, xyz(positions[i]) + n * epsilon);
                assert_eq!(ray.t_max, 1.0);
                assert!(ray.direction.dot(n) > 0.0);
            }
        };

        let visibility = view
            .ambient_visibility(&settings, input, &Control::new(), None)
            .unwrap();
        check_rays(&[0, 3, 4, 5, 6, 7]);
        // The full-resolution spatial filter still blends pixels 1 and 5
        // across quads; neither material nor depth/normal boundaries blend.
        assert_eq!(
            visibility,
            [0.0, 0.25, 1.0, 1.0, 1.0, 2.0 / 3.0, 1.0, 1.0, 1.0]
        );

        view.ambient_visibility(&settings, input, &Control::new(), None)
            .unwrap();
        check_rays(&[1, 3, 4, 5, 7, 6]);

        let cancelled = Control::new();
        cancelled.cancel();
        assert!(matches!(
            view.ambient_visibility(&settings, input, &cancelled, None),
            Err(RenderError::Cancelled)
        ));
        check_rays(&[]);
        assert_eq!(view.ao[&[0; 2]].frames, 2);
    }
}
