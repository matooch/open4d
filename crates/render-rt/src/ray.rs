//! Instanced ray queries shared by the real-time tiers (RENDER.md R23).

mod compute;
mod hardware;

use forge_render_api::{RenderError, Snapshot};
use glam::DVec3;

pub use compute::{COMPUTE_TRAVERSAL_WGSL, COMPUTE_WGSL, ComputeBackend};
pub(crate) use hardware::HardwareBackend;
pub use hardware::{HARDWARE_TRAVERSAL_WGSL, HARDWARE_WGSL};

/// Common hit result and miss sentinel used by both inline traversal kernels.
pub const RAY_HIT_WGSL: &str = include_str!("ray/hit.wgsl");

/// Inline R23 declarations shared by the path transport and hybrid effects.
pub(crate) fn inline_ray_scene(backend: BackendKind) -> (&'static str, String) {
    let (directives, traversal, bindings) = match backend {
        BackendKind::Compute => (
            "",
            COMPUTE_TRAVERSAL_WGSL,
            "
@group(2) @binding(0) var<storage, read> tlas: array<Node>;
@group(2) @binding(1) var<storage, read> instances: array<Instance>;
@group(2) @binding(2) var<storage, read> blas: array<Node>;
@group(2) @binding(3) var<storage, read> triangles: array<Triangle>;
",
        ),
        BackendKind::Metal | BackendKind::Vulkan => (
            "enable wgpu_ray_query;",
            HARDWARE_TRAVERSAL_WGSL,
            "\n@group(2) @binding(0) var scene: acceleration_structure;\n",
        ),
    };
    (directives, format!("{RAY_HIT_WGSL}\n{traversal}{bindings}"))
}

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
    let (kind, backend) = on_device(device, queue)?;
    Ok((kind, backend))
}

pub(crate) fn request_device(
    adapter: &wgpu::Adapter,
    allow_hardware: bool,
) -> Result<(wgpu::Device, wgpu::Queue), RenderError> {
    if allow_hardware
        && preferred_backend(adapter.get_info().backend, adapter.features()) != BackendKind::Compute
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

/// Device features are what was enabled, not everything the adapter supports.
pub(crate) fn enabled_backend(device: &wgpu::Device) -> BackendKind {
    preferred_backend(device.adapter_info().backend, device.features())
}

pub(crate) fn on_device(
    device: wgpu::Device,
    queue: wgpu::Queue,
) -> Result<(BackendKind, Box<dyn DeviceQueries>), RenderError> {
    let kind = enabled_backend(&device);
    if kind != BackendKind::Compute
        && let Ok(native) = hardware::HardwareBackend::new(device.clone(), queue.clone())
    {
        return Ok((kind, Box::new(native)));
    }
    Ok((
        BackendKind::Compute,
        Box::new(ComputeBackend::new(device, queue)?),
    ))
}

pub(crate) enum InlineScene<'a> {
    Compute([&'a wgpu::Buffer; 4]),
    Hardware(&'a wgpu::Tlas),
}

pub(crate) trait DeviceQueries: RtBackend {
    fn inline_scene(&self) -> Option<InlineScene<'_>> {
        None
    }
}

impl DeviceQueries for ComputeBackend {
    fn inline_scene(&self) -> Option<InlineScene<'_>> {
        self.scene_buffers().map(InlineScene::Compute)
    }
}

impl DeviceQueries for HardwareBackend {
    fn inline_scene(&self) -> Option<InlineScene<'_>> {
        self.tlas().map(InlineScene::Hardware)
    }
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
    /// Returns whether each ray is blocked, in input order.
    fn occluded(&self, rays: &[Ray]) -> Result<Vec<bool>, RenderError> {
        Ok(self.trace(rays)?.iter().map(Option::is_some).collect())
    }
    fn stats(&self) -> SceneStats;
}
