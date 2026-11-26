//! Instanced ray queries shared by the real-time tiers (RENDER.md R23).

mod compute;
mod hardware;

use forge_render_api::{RenderError, Snapshot};
use glam::DVec3;

pub use compute::{COMPUTE_TRAVERSAL_WGSL, COMPUTE_WGSL, ComputeBackend};
pub use hardware::{HARDWARE_TRAVERSAL_WGSL, HARDWARE_WGSL};

/// Common hit result and miss sentinel used by both inline traversal kernels.
pub const RAY_HIT_WGSL: &str = include_str!("ray/hit.wgsl");

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

/// Opens a ray-query backend on this adapter. Set `allow_hardware` to false to
/// explicitly request the compute path (for example, for backend comparisons).
pub fn request_backend(
    adapter: &wgpu::Adapter,
    allow_hardware: bool,
) -> Result<(BackendKind, Box<dyn RtBackend>), RenderError> {
    let (device, queue) = request_device(adapter, allow_hardware)?;
    on_device(device, queue)
}

pub(crate) fn request_device(
    adapter: &wgpu::Adapter,
    allow_hardware: bool,
) -> Result<(wgpu::Device, wgpu::Queue), RenderError> {
    if allow_hardware
        && preferred_backend(adapter.get_info().backend, adapter.features()) == BackendKind::Metal
    {
        let desc = wgpu::DeviceDescriptor {
            required_features: wgpu::Features::EXPERIMENTAL_RAY_QUERY,
            required_limits: wgpu::Limits::default()
                .using_acceleration_structure_values(adapter.limits()),
            // SAFETY: opt in to wgpu's experimental API only on a supporting adapter.
            // All acceleration structures and dispatches use wgpu's validated API.
            experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
            ..Default::default()
        };
        if let Ok((device, queue)) = pollster::block_on(adapter.request_device(&desc)) {
            return Ok((device, queue));
        }
    }
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default()))
        .map_err(|err| RenderError::Settings(format!("ray query device: {err}")))?;
    Ok((device, queue))
}

pub(crate) fn on_device(
    device: wgpu::Device,
    queue: wgpu::Queue,
) -> Result<(BackendKind, Box<dyn RtBackend>), RenderError> {
    if device
        .features()
        .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
        && let Ok(native) = hardware::HardwareBackend::new(device.clone(), queue.clone())
    {
        return Ok((BackendKind::Metal, Box::new(native)));
    }
    Ok((
        BackendKind::Compute,
        Box::new(ComputeBackend::new(device, queue)?),
    ))
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

pub trait RtBackend: Send {
    fn update_scene(&mut self, snapshot: &Snapshot) -> Result<(), RenderError>;
    /// Returns one hit or miss per ray, in input order.
    fn trace(&self, rays: &[Ray]) -> Result<Vec<Option<RayHit>>, RenderError>;
    fn stats(&self) -> SceneStats;
}
