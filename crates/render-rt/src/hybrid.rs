//! Tier 2 ray-traced effects and per-view history (RENDER.md R24).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_render_api::{Control, RenderError, RenderSettings, Snapshot};
use forge_render_cpu::lights;
use forge_render_cpu::sampling::{Rng, cosine_hemisphere, lum, power_heuristic, to_world};
use forge_render_cpu::scene::Scene;
use forge_render_cpu::surface::Surface;
use glam::DVec3;

use crate::ray::{self, Ray, RtBackend};

const AO_RAYS: usize = 8;
const SHADOW_RAYS: usize = 8;
// At most 512 KiB each for ray input and hit output, well within supported
// storage binding and compute dispatch limits.
const PIXELS_PER_BATCH: usize = 2048;

#[derive(Default)]
pub(crate) struct HybridView {
    backend: Option<Box<dyn RtBackend>>,
    scene: Option<Snapshot>,
    geometry: Option<PrimaryGeometry>,
    settings: Option<RenderSettings>,
    // Each raster TAA jitter has different primary visibility.
    ao: BTreeMap<[u32; 2], History<f32>>,
    shadows: BTreeMap<[u32; 2], History<[f32; 8]>>,
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

impl HybridView {
    pub fn reset(&mut self) {
        self.ao.clear();
        self.shadows.clear();
    }

    pub fn prepare(
        &mut self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), RenderError> {
        if self.backend.is_none() {
            self.backend = Some(ray::on_device(device.clone(), queue.clone())?.1);
        }
        let same_geometry = self.scene.as_ref().is_some_and(|old| {
            old.meshes == snapshot.meshes && old.instances == snapshot.instances
        });
        if !same_geometry {
            self.backend
                .as_mut()
                .expect("initialized ray backend")
                .update_scene(snapshot)?;
            self.geometry = None;
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
    ) -> Result<Vec<f32>, RenderError> {
        let radius = settings.hybrid.ao_radius;
        let mut visibility = vec![1.0; input.positions.len()];
        if radius == 0.0 || settings.post.ssao_strength == 0.0 {
            return Ok(visibility);
        }
        let backend = self.backend.as_ref().expect("prepared ray backend");
        let history = self.ao.entry(input.jitter.map(f32::to_bits)).or_default();
        let mut rays = Vec::with_capacity(PIXELS_PER_BATCH * AO_RAYS);
        let mut active = Vec::with_capacity(PIXELS_PER_BATCH);
        for first in (0..visibility.len()).step_by(PIXELS_PER_BATCH) {
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            let end = (first + PIXELS_PER_BATCH).min(visibility.len());
            rays.clear();
            active.clear();
            for i in first..end {
                if input.positions[i][3] < 0.5 {
                    continue;
                }
                let p = xyz(input.positions[i]);
                let n = xyz(input.normals[i]).normalize_or_zero();
                if n == DVec3::ZERO {
                    continue;
                }
                active.push(i);
                let mut rng = Rng::new(settings.seed ^ u64::from(history.frames), i as u64);
                for _ in 0..AO_RAYS {
                    rays.push(Ray {
                        origin: p + n * input.epsilon.min(radius * 0.01),
                        direction: to_world(n, cosine_hemisphere(rng.f2())),
                        t_max: radius,
                    });
                }
            }
            let hits = backend.trace(&rays)?;
            for (&i, hits) in active.iter().zip(hits.as_chunks::<AO_RAYS>().0) {
                let open = hits.iter().filter(|hit| hit.is_none()).count();
                visibility[i] = open as f32 / AO_RAYS as f32;
            }
        }

        let filtered = spatial_filter(settings.width as usize, &input, &visibility, radius * 0.15);
        history.accumulate(filtered);
        let strength = settings.post.ssao_strength as f32;
        Ok(history
            .mean
            .iter()
            .map(|open| 1.0 - (1.0 - open) * strength)
            .collect())
    }

    pub fn direct_shadows(
        &mut self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        input: PrimaryInput<'_>,
        control: &Control,
    ) -> Result<Vec<[f32; 8]>, RenderError> {
        let lights: Vec<_> = lights::prepare(snapshot, settings.default_light)?
            .into_iter()
            .filter(lights::PLight::casts_shadow)
            .collect();
        if lights.is_empty() {
            return Ok(Vec::new());
        }
        if self.geometry.is_none() {
            self.geometry = Some(PrimaryGeometry::new(snapshot)?);
        }
        let geometry = self.geometry.as_ref().expect("prepared shading geometry");
        let backend = self.backend.as_ref().expect("prepared ray backend");
        let history = self
            .shadows
            .entry(input.jitter.map(f32::to_bits))
            .or_default();
        let mut pixels = vec![[0.0; 8]; input.positions.len()];
        let mut rays = Vec::with_capacity(PIXELS_PER_BATCH * SHADOW_RAYS);
        let mut samples = Vec::with_capacity(rays.capacity());
        let width = settings.width as usize;
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
                    let mut rng = Rng::new(settings.seed ^ u64::from(history.frames), stream);
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
                for ((i, d, s), hit) in samples.iter().zip(backend.trace(&rays)?) {
                    let factor = if hit.is_some() { occluded } else { DVec3::ONE };
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
        let limit = 3.0
            * match snapshot.camera.projection {
                forge_render_api::Projection::Orthographic { width: w } => w / width as f64,
                _ => geometry.scene.scale / width as f64,
            };
        history.accumulate(spatial_filter(width, &input, &pixels, limit));
        Ok(history.mean.clone())
    }
}

struct PrimaryGeometry {
    scene: Scene,
    /// Cumulative end indices in the raster triangle stream, in instance order.
    ends: Vec<usize>,
}

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
        let instance = &self.scene.insts[inst];
        let geom = self.scene.geometry_at(snap, position, inst, tri);
        let w = settings.width as usize;
        let (_, d) = snap.camera.ray(
            (i % w) as f64 + 0.5 - f64::from(g.jitter[0]),
            (i / w) as f64 + 0.5 - f64::from(g.jitter[1]),
            settings.width,
            settings.height,
        );
        let view = -d;
        let mat = &snap.materials[instance.material];
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
            position,
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
