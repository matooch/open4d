//! Tier 3 raw rendering: R23 traversal with the shared R20 integrator (R25.1).

use forge_render_api::{
    Control, RenderEngine, RenderError, RenderOutput, RenderSettings, Snapshot,
};
use forge_render_cpu::scene::Scene;
use forge_render_gpu::{BoundScene, GpuEngine, pack::Packed};
use glam::{DVec3, DVec4};
use wgpu::util::DeviceExt;

use crate::ray::{
    BackendKind, COMPUTE_TRAVERSAL_WGSL, ComputeBackend, HARDWARE_TRAVERSAL_WGSL, HardwareBackend,
    RAY_HIT_WGSL, RtBackend,
};

const INSTANCE_ROWS: usize = 7;
const TRIANGLE_ROWS: usize = 10;
const ATTRIBUTES_BINDING: u32 = 4;

/// The WGSL extension directive and scene module used by the Tier 3 integrator.
/// Hardware backends share the same acceleration-structure interface.
pub fn path_scene_wgsl(backend: BackendKind) -> (&'static str, String) {
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
    (
        directives,
        format!(
            "{RAY_HIT_WGSL}\n{traversal}{bindings}{}",
            include_str!("path_scene.wgsl")
        ),
    )
}

fn attributes(
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
    Hardware(HardwareBackend),
}

impl Traversal {
    fn build(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        snap: &Snapshot,
    ) -> Result<Self, RenderError> {
        if device
            .features()
            .contains(wgpu::Features::EXPERIMENTAL_RAY_QUERY)
            && snap
                .instances
                .iter()
                .any(|i| !snap.meshes[i.mesh].triangles.is_empty())
            && let Ok(mut hardware) = HardwareBackend::new(device.clone(), queue.clone())
        {
            hardware.update_scene(snap)?;
            if hardware.tlas().is_some() {
                return Ok(Self::Hardware(hardware));
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
            Self::Hardware(hardware) => (
                BackendKind::Metal,
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

impl PathTraceEngine {
    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let transport = GpuEngine::with_device(device.clone(), queue.clone());
        Self {
            device,
            queue,
            transport,
        }
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
        let geometry = Scene::build(snapshot)?;
        let attrs = attributes(&self.device, snapshot, &geometry)?;
        let inputs = Packed::frame_inputs(
            snapshot,
            settings.default_light,
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
        let bound = BoundScene {
            directives,
            source: &source,
            layout: &layout,
            bind: &bind,
        };
        let output =
            self.transport
                .render_transport(snapshot, settings, control, &inputs, Some(bound))?;
        Ok((output, kind))
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
