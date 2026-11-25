//! Instanced ray queries shared by the real-time tiers (RENDER.md R23).

mod compute;

use forge_render_api::{RenderError, Snapshot};
use glam::DVec3;

pub use compute::{COMPUTE_WGSL, ComputeBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind {
    Compute,
    Metal,
    Vulkan,
}

/// Preferred path given the adapter's advertised API and features. The compute
/// implementation works without the experimental ray-query feature enabled.
pub fn preferred_backend(api: wgpu::Backend, features: wgpu::Features) -> BackendKind {
    if features.contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY) {
        match api {
            wgpu::Backend::Metal => return BackendKind::Metal,
            wgpu::Backend::Vulkan => return BackendKind::Vulkan,
            _ => (),
        }
    }
    BackendKind::Compute
}

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: DVec3,
    pub direction: DVec3,
    pub t_max: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    pub t: f64,
    /// Index into the snapshot's instance array, not its object ID.
    pub instance: usize,
    /// Index into the hit mesh's triangle array.
    pub triangle: usize,
    pub u: f64,
    pub v: f64,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SceneStats {
    pub blases: usize,
    pub triangles: usize,
    pub instances: usize,
    /// Number of geometry uploads, not the number of meshes uploaded.
    pub blas_builds: usize,
    pub tlas_updates: usize,
}

pub trait RtBackend {
    fn update_scene(&mut self, snapshot: &Snapshot) -> Result<(), RenderError>;
    /// Returns one hit or miss per ray, in input order.
    fn trace(&self, rays: &[Ray]) -> Result<Vec<Option<RayHit>>, RenderError>;
    fn stats(&self) -> SceneStats;
}
