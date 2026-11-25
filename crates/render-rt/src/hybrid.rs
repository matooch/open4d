//! Tier 2 ray-traced effects and per-view history (RENDER.md R24).

use std::collections::BTreeMap;
use std::sync::Arc;

use forge_render_api::{Control, RenderError, RenderSettings, Snapshot};
use forge_render_cpu::sampling::{Rng, cosine_hemisphere, to_world};
use glam::DVec3;

use crate::ray::{self, Ray, RtBackend};

const AO_RAYS: usize = 8;
// At most 512 KiB each for ray input and hit output, well within supported
// storage binding and compute dispatch limits.
const PIXELS_PER_BATCH: usize = 2048;

#[derive(Default)]
pub(crate) struct HybridView {
    backend: Option<Box<dyn RtBackend>>,
    scene: Option<Snapshot>,
    settings: Option<RenderSettings>,
    // Each raster TAA jitter has different primary visibility.
    ao: BTreeMap<[u32; 2], AoHistory>,
}

#[derive(Default)]
struct AoHistory {
    frames: u32,
    mean: Vec<f32>,
}

pub(crate) struct AoInput<'a> {
    pub positions: &'a [[f32; 4]],
    pub normals: &'a [[f32; 4]],
    pub jitter: [f32; 2],
    pub epsilon: f64,
}

impl HybridView {
    pub fn reset(&mut self) {
        self.ao.clear();
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
        input: AoInput<'_>,
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

        let filtered = spatial_ao(settings.width as usize, &input, &visibility, radius);
        history.frames = history.frames.saturating_add(1);
        if history.mean.is_empty() {
            history.mean = filtered;
        } else {
            let weight = 1.0 / history.frames as f32;
            for (mean, new) in history.mean.iter_mut().zip(filtered) {
                *mean += (new - *mean) * weight;
            }
        }
        let strength = settings.post.ssao_strength as f32;
        Ok(history
            .mean
            .iter()
            .map(|open| 1.0 - (1.0 - open) * strength)
            .collect())
    }
}

fn xyz([x, y, z, _]: [f32; 4]) -> DVec3 {
    DVec3::new(f64::from(x), f64::from(y), f64::from(z))
}

fn spatial_ao(width: usize, g: &AoInput<'_>, mask: &[f32], radius: f64) -> Vec<f32> {
    let height = mask.len() / width;
    let limit = radius * 0.15;
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
            let mut sum = value * 2.0;
            let mut weight = 2.0;
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
                    sum += mask[j];
                    weight += 1.0;
                }
            }
            sum / weight
        })
        .collect()
}
