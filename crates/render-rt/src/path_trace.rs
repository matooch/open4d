//! Tier 3 final frames and per-view previews (R25).

use forge_image::Image;
use forge_render_api::{
    Control, RenderEngine, RenderError, RenderOutput, RenderSettings, Snapshot,
};
use forge_render_cpu::scene::Scene;
use forge_render_gpu::{BoundScene, GpuEngine, TransportHistory, pack::Packed};
use glam::{DVec3, DVec4};
use std::sync::Arc;
use wgpu::util::DeviceExt;

use crate::path_preview::{PrimaryGuide, denoise};
use crate::ray::{BackendKind, ComputeBackend, HardwareBackend, RtBackend};

const INSTANCE_ROWS: usize = 7;
const TRIANGLE_ROWS: usize = 10;
const ATTRIBUTES_BINDING: u32 = 4;

/// The WGSL extension directive and scene module used by the Tier 3 integrator.
/// Hardware backends share the same acceleration-structure interface.
pub fn path_scene_wgsl(backend: BackendKind) -> (&'static str, String) {
    let (directives, mut source) = crate::ray::inline_ray_scene(backend);
    source.push_str(crate::shader::INSTANCE_GEOMETRY_WGSL);
    source.push_str(include_str!("path_scene.wgsl"));
    (directives, source)
}

pub(crate) fn attributes(
    device: &wgpu::Device,
    snapshot: &Snapshot,
    geometry: &Scene,
) -> Result<wgpu::Buffer, RenderError> {
    let count = (INSTANCE_ROWS as u64 * snapshot.instances.len() as u64
        + TRIANGLE_ROWS as u64 * geometry.triangles as u64)
        .max(1);
    let limits = device.limits();
    if count * 16
        > limits
            .max_storage_buffer_binding_size
            .min(limits.max_buffer_size)
    {
        return Err(RenderError::Unsupported(
            "instanced path attributes exceed the GPU storage binding limit".into(),
        ));
    }
    let mut rows = vec![[0.0; 4]; INSTANCE_ROWS * snapshot.instances.len()];
    let mut offsets = Vec::with_capacity(snapshot.meshes.len());
    let row = |v: DVec4| v.as_vec4().to_array();
    for (mesh, blas) in snapshot.meshes.iter().zip(&geometry.blases) {
        offsets.push(rows.len() as u32);
        for (i, corners) in mesh.triangles.iter().enumerate() {
            let p = corners.map(|c| DVec3::from(mesh.positions[c as usize]));
            rows.extend(p.map(|v| row(v.extend(1.0))));
            let ng = (p[1] - p[0]).cross(p[2] - p[0]).normalize_or_zero();
            rows.push(row((if blas.flip() { -ng } else { ng }).extend(0.0)));
            let attribute = |values: &[[f64; 3]], k| {
                row(DVec3::from(values.get(i * 3 + k).copied().unwrap_or_default()).extend(0.0))
            };
            rows.extend((0..3).map(|k| attribute(&mesh.normals, k)));
            rows.extend((0..3).map(|k| attribute(&mesh.uvs, k)));
        }
    }
    let mut end = 0u32;
    for (i, inst) in geometry.insts.iter().enumerate() {
        let base = i * INSTANCE_ROWS;
        rows[base] = [
            f32::from_bits(offsets[inst.mesh]),
            inst.material as f32,
            inst.object_id as f32,
            f32::from(geometry.blases[inst.blas].closed),
        ];
        let normal = inst.to_world.inverse().transpose();
        for j in 0..3 {
            rows[base + 1 + j] = row(inst.to_world.row(j));
            rows[base + 4 + j] = row(normal.row(j));
        }
        // Raster keys are narrower than u32. Do not impose that key space
        // on Tier 3, which addresses triangles by instance and local index.
        let triangles =
            u32::try_from(snapshot.meshes[inst.mesh].triangles.len()).unwrap_or(u32::MAX);
        end = end.saturating_add(triangles);
        rows[base + 4][3] = f32::from_bits(end);
    }
    if rows.is_empty() {
        rows.push([0.0; 4]);
    }
    Ok(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("instanced path attributes"),
            contents: bytemuck::cast_slice(&rows),
            usage: wgpu::BufferUsages::STORAGE,
        }),
    )
}

enum Traversal {
    Compute(ComputeBackend),
    Hardware(BackendKind, HardwareBackend),
}

impl Traversal {
    fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        snap: &Snapshot,
    ) -> Result<Self, RenderError> {
        let kind = crate::ray::enabled_backend(device);
        if kind != BackendKind::Compute
            && snap
                .instances
                .iter()
                .any(|i| !snap.meshes[i.mesh].triangles.is_empty())
            && let Ok(mut hardware) = HardwareBackend::new(device.clone(), queue.clone())
        {
            hardware.update_scene(snap)?;
            if hardware.tlas().is_some() {
                return Ok(Self::Hardware(kind, hardware));
            }
        }
        let mut compute = ComputeBackend::new(device.clone(), queue.clone())?;
        compute.update_scene(snap)?;
        Ok(Self::Compute(compute))
    }

    fn bind(
        &self,
        device: &wgpu::Device,
        attrs: &wgpu::Buffer,
    ) -> (BackendKind, wgpu::BindGroupLayout, wgpu::BindGroup) {
        let storage = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let (backend, mut resources) = match self {
            Self::Compute(compute) => (
                BackendKind::Compute,
                compute
                    .scene_buffers()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .map(|(i, buf)| (i as u32, storage, buf.as_entire_binding()))
                    .collect::<Vec<_>>(),
            ),
            Self::Hardware(kind, hardware) => (
                *kind,
                vec![(
                    0,
                    wgpu::BindingType::AccelerationStructure {
                        vertex_return: false,
                    },
                    hardware.tlas().unwrap().as_binding(),
                )],
            ),
        };
        resources.push((ATTRIBUTES_BINDING, storage, attrs.as_entire_binding()));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("instanced path scene"),
            entries: &resources
                .iter()
                .map(|(binding, ty, _)| wgpu::BindGroupLayoutEntry {
                    binding: *binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: *ty,
                    count: None,
                })
                .collect::<Vec<_>>(),
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("instanced path scene"),
            layout: &layout,
            entries: &resources
                .into_iter()
                .map(|(binding, _, resource)| wgpu::BindGroupEntry { binding, resource })
                .collect::<Vec<_>>(),
        });
        (backend, layout, bind)
    }
}

pub struct PathTraceEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    transport: GpuEngine,
}

struct PathScene {
    kind: BackendKind,
    inputs: Packed,
    layout: wgpu::BindGroupLayout,
    bind: wgpu::BindGroup,
    directives: &'static str,
    source: String,
}

impl PathScene {
    fn bound(&self) -> BoundScene<'_> {
        BoundScene {
            directives: self.directives,
            source: &self.source,
            layout: &self.layout,
            bind: &self.bind,
        }
    }
}

impl PathTraceEngine {
    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let transport = GpuEngine::with_device(device.clone(), queue.clone());
        Self {
            device,
            queue,
            transport,
        }
    }

    fn prepare(&self, snapshot: &Snapshot, default_light: bool) -> Result<PathScene, RenderError> {
        let geometry = Scene::build(snapshot)?;
        let attrs = attributes(&self.device, snapshot, &geometry)?;
        let inputs = Packed::frame_inputs(
            snapshot,
            default_light,
            geometry.scale,
            snapshot
                .instances
                .iter()
                .map(|i| snapshot.meshes[i.mesh].triangles.len())
                .sum(),
        )?;
        let traversal = Traversal::build(&self.device, &self.queue, snapshot)?;
        let (kind, layout, bind) = traversal.bind(&self.device, &attrs);
        let (directives, source) = path_scene_wgsl(kind);
        Ok(PathScene {
            kind,
            inputs,
            directives,
            source,
            layout,
            bind,
        })
    }

    /// Renders raw linear output and reports the traversal actually used.
    pub fn render_with_backend(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<(RenderOutput, BackendKind), RenderError> {
        settings.validate()?;
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let scene = self.prepare(snapshot, settings.default_light)?;
        let output = self.transport.render_transport(
            snapshot,
            settings,
            control,
            &scene.inputs,
            Some(scene.bound()),
        )?;
        Ok((output, scene.kind))
    }

    /// Start an independent preview. Final renders do not read or modify this view.
    pub fn preview_view(&self) -> PathTraceView<'_> {
        PathTraceView {
            engine: self,
            scene: None,
            history: TransportHistory::default(),
        }
    }
}

/// A completed progressive frame. Only raw samples contribute to later frames.
pub struct PathTracePreview {
    pub raw: RenderOutput,
    pub denoised: Image,
    pub samples: u32,
    pub backend: BackendKind,
}

/// Accumulation and prepared scene owned by a single Tier 3 view.
pub struct PathTraceView<'a> {
    engine: &'a PathTraceEngine,
    scene: Option<(Snapshot, bool, PathScene)>,
    history: TransportHistory,
}

fn same_snapshot(a: &Snapshot, b: &Snapshot) -> bool {
    a.meshes == b.meshes
        && a.instances == b.instances
        && a.camera == b.camera
        && a.environment == b.environment
        && a.lights == b.lights
        && a.materials.len() == b.materials.len()
        && a.materials
            .iter()
            .zip(&b.materials)
            .all(|(a, b)| a.graph == b.graph && Arc::ptr_eq(&a.sampler, &b.sampler))
}

impl PathTraceView<'_> {
    /// Render `samples` additional samples per pixel, ignoring `settings.samples`.
    /// A cancelled or failed batch leaves the previously committed frame intact.
    pub fn render_batch(
        &mut self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        samples: u32,
        control: &Control,
    ) -> Result<PathTracePreview, RenderError> {
        let mut settings = settings.clone();
        settings.samples = samples;
        settings.validate()?;
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let cached = self.scene.as_ref().filter(|(prior, default_light, _)| {
            *default_light == settings.default_light && same_snapshot(prior, snapshot)
        });
        let mut pending = None;
        let (scene, mut history) = if let Some((_, _, scene)) = cached {
            (scene, self.history.clone())
        } else {
            let scene = pending.insert(self.engine.prepare(snapshot, settings.default_light)?);
            (&*scene, TransportHistory::default())
        };
        let backend = scene.kind;
        let preview = self.engine.transport.preview_transport(
            snapshot,
            &settings,
            control,
            &scene.inputs,
            scene.bound(),
            &mut history,
        )?;
        let guides: Vec<_> = preview
            .guides
            .into_iter()
            .map(|[x, y, z, depth]| match depth {
                0.0 => PrimaryGuide::Inactive,
                d if d > 0.0 => PrimaryGuide::Hit {
                    depth,
                    normal: [x, y, z],
                },
                _ => PrimaryGuide::Miss,
            })
            .collect();
        let denoised = denoise(&preview.raw.beauty, &guides, history.samples())?;
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        if let Some(scene) = pending {
            self.scene = Some((snapshot.clone(), settings.default_light, scene));
        }
        let samples = history.samples();
        self.history = history;
        Ok(PathTracePreview {
            raw: preview.raw,
            denoised,
            samples,
            backend,
        })
    }
}

impl RenderEngine for PathTraceEngine {
    fn id(&self) -> &str {
        "forge.render.pathtrace"
    }

    fn render(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError> {
        self.render_with_backend(snapshot, settings, control)
            .map(|(output, _)| output)
    }
}
