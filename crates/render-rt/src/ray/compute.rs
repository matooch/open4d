use bytemuck::{Pod, Zeroable};
use forge_render_api::{MeshData, RenderError, Snapshot};
use forge_render_cpu::bvh::{Aabb, Bvh};
use forge_render_gpu::pack::Node;
use glam::{DMat4, DVec3};
use wgpu::util::DeviceExt;

use super::{Ray, RayHit, RtBackend, SceneStats};

/// Inline closest-hit kernel. The caller declares the `tlas`, `instances`, `blas`
/// and `triangles` bindings and supplies the shared `RAY_HIT_WGSL` definitions.
pub const COMPUTE_TRAVERSAL_WGSL: &str = include_str!("compute.wgsl");
pub const COMPUTE_WGSL: &str = concat!(
    include_str!("hit.wgsl"),
    include_str!("compute.wgsl"),
    "
@group(0) @binding(0) var<storage, read> tlas: array<Node>;
@group(0) @binding(1) var<storage, read> instances: array<Instance>;
@group(0) @binding(2) var<storage, read> blas: array<Node>;
@group(0) @binding(3) var<storage, read> triangles: array<Triangle>;
@group(0) @binding(4) var<storage, read> queries: array<Query>;
@group(0) @binding(5) var<storage, read_write> hits: array<RtHit>;
@group(0) @binding(6) var<storage, read_write> blocked: array<u32>;
",
    include_str!("batch.wgsl"),
);
const END: u32 = u32::MAX;
const WORKGROUP_SIZE: u32 = 64;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Triangle {
    p0: [f32; 4],
    e1: [f32; 4],
    e2: [f32; 4],
    ids: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Instance {
    inverse: [f32; 16],
    ids: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Query {
    origin: [f32; 4],
    direction: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Hit {
    tuv: [f32; 4],
    ids: [u32; 4],
}

fn row(p: DVec3, w: f32) -> [f32; 4] {
    p.as_vec3().extend(w).to_array()
}

// R20.1 node layout with a threaded successor in meta.z. Missing a node or
// finishing a leaf jumps to its successor, avoiding fixed-size shader stacks.
fn append_nodes(tree: &Bvh, nodes: &mut Vec<Node>, leaf_base: u32) {
    let base = nodes.len() as u32;
    nodes.extend(tree.nodes().iter().map(|n| Node {
        bounds_min: row(n.bounds.min, 0.0),
        bounds_max: row(n.bounds.max, 0.0),
        meta: [
            n.first + if n.count == 0 { base } else { leaf_base },
            n.count,
            END,
            0,
        ],
    }));
    if !tree.nodes().is_empty() {
        let mut pending = vec![(base, END)];
        while let Some((index, next)) = pending.pop() {
            let [first, count, _, _] = nodes[index as usize].meta;
            nodes[index as usize].meta[2] = next;
            if count == 0 {
                pending.push((first + 1, next));
                pending.push((first, first + 1));
            }
        }
    }
}

struct Geometry {
    meshes: Vec<MeshData>,
    roots: Vec<Option<(u32, Aabb)>>,
    nodes: wgpu::Buffer,
    triangles: wgpu::Buffer,
}

impl Geometry {
    fn matches(&self, meshes: &[MeshData]) -> bool {
        self.meshes.len() == meshes.len()
            && self
                .meshes
                .iter()
                .zip(meshes)
                .all(|(a, b)| a.positions == b.positions && a.triangles == b.triangles)
    }

    fn build(device: &wgpu::Device, meshes: &[MeshData]) -> Result<Self, RenderError> {
        let mut nodes = Vec::new();
        let mut triangles = Vec::new();
        let mut roots = Vec::with_capacity(meshes.len());
        for mesh in meshes {
            let vertices =
                |tri: [u32; 3]| tri.map(|i| DVec3::from_array(mesh.positions[i as usize]));
            let bounds: Vec<_> = mesh
                .triangles
                .iter()
                .map(|&tri| {
                    let mut bounds = Aabb::EMPTY;
                    for p in vertices(tri) {
                        bounds.grow(p);
                    }
                    bounds
                })
                .collect();
            let tree = Bvh::build(&bounds);
            roots.push((!bounds.is_empty()).then(|| (nodes.len() as u32, tree.bounds())));
            append_nodes(&tree, &mut nodes, triangles.len() as u32);
            triangles.extend(tree.prims().iter().map(|&id| {
                let [a, b, c] = vertices(mesh.triangles[id as usize]);
                Triangle {
                    p0: row(a, 0.0),
                    e1: row(b - a, 0.0),
                    e2: row(c - a, 0.0),
                    ids: [id, 0, 0, 0],
                }
            }));
        }
        Ok(Self {
            meshes: meshes
                .iter()
                .map(|m| MeshData::new(m.positions.clone(), m.triangles.clone()))
                .collect(),
            roots,
            nodes: upload(device, "BLAS nodes", &nodes)?,
            triangles: upload(device, "BLAS triangles", &triangles)?,
        })
    }
}

struct TopLevel {
    nodes: wgpu::Buffer,
    instances: wgpu::Buffer,
    count: usize,
}

impl TopLevel {
    fn build(
        device: &wgpu::Device,
        snapshot: &Snapshot,
        geometry: &Geometry,
    ) -> Result<Self, RenderError> {
        let mut instances = Vec::new();
        let mut boxes = Vec::new();
        for (index, inst) in snapshot.instances.iter().enumerate() {
            let transform = DMat4::from_cols_array(&inst.transform);
            let det = transform.determinant();
            if !det.is_finite() || det.abs() < 1e-24 {
                return Err(RenderError::Snapshot(
                    "an instance transform is singular".into(),
                ));
            }
            if let Some((root, bounds)) = geometry.roots[inst.mesh] {
                boxes.push(bounds.transformed(&transform));
                instances.push(Instance {
                    inverse: transform.inverse().as_mat4().to_cols_array(),
                    ids: [root, index as u32, 0, 0],
                });
            }
        }
        let tree = Bvh::build(&boxes);
        let mut nodes = Vec::new();
        append_nodes(&tree, &mut nodes, 0);
        if nodes.is_empty() {
            // An inline caller can trace without first checking the instance count.
            // Following either link from this root terminates without visiting a BLAS.
            nodes.push(Node {
                meta: [END, 0, END, 0],
                ..Node::zeroed()
            });
        }
        let ordered: Vec<_> = tree
            .prims()
            .iter()
            .map(|&i| instances[i as usize])
            .collect();
        Ok(Self {
            nodes: upload(device, "TLAS nodes", &nodes)?,
            instances: upload(device, "TLAS instances", &ordered)?,
            count: ordered.len(),
        })
    }
}

fn check_size(device: &wgpu::Device, label: &str, size: u64) -> Result<(), RenderError> {
    let limits = device.limits();
    if size > limits.max_buffer_size || size > limits.max_storage_buffer_binding_size {
        return Err(RenderError::Unsupported(format!(
            "{label} exceeds GPU buffer limits"
        )));
    }
    Ok(())
}

fn upload<T: Pod>(
    device: &wgpu::Device,
    label: &str,
    data: &[T],
) -> Result<wgpu::Buffer, RenderError> {
    // An empty array cannot be bound as storage. Its dummy element is never traversed.
    let zero = T::zeroed();
    let data = if data.is_empty() {
        std::slice::from_ref(&zero)
    } else {
        data
    };
    let contents = bytemuck::cast_slice(data);
    check_size(device, label, contents.len() as u64)?;
    Ok(
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents,
            usage: wgpu::BufferUsages::STORAGE,
        }),
    )
}

pub struct ComputeBackend {
    dispatch: QueryDispatch,
    geometry: Option<Geometry>,
    top: Option<TopLevel>,
    stats: SceneStats,
}

impl ComputeBackend {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Result<Self, RenderError> {
        Ok(Self {
            dispatch: QueryDispatch::new(device, queue, COMPUTE_WGSL)?,
            geometry: None,
            top: None,
            stats: SceneStats::default(),
        })
    }

    pub(crate) fn scene_buffers(&self) -> Option<[&wgpu::Buffer; 4]> {
        let (geometry, top) = (self.geometry.as_ref()?, self.top.as_ref()?);
        Some([
            &top.nodes,
            &top.instances,
            &geometry.nodes,
            &geometry.triangles,
        ])
    }
}

pub(super) struct QueryDispatch {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    closest: QueryPipeline,
    visibility: QueryPipeline,
}

struct QueryPipeline {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
    // Visibility's output follows the closest-hit output slot in both shaders.
    output_slot_offset: u32,
}

impl QueryDispatch {
    pub fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        source: &str,
    ) -> Result<Self, RenderError> {
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("instanced ray queries"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let build = |entry_point| {
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("instanced ray queries"),
                layout: None,
                module: &module,
                entry_point: Some(entry_point),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let closest = build("main");
        let visibility = build("visibility");
        if let Some(error) = pollster::block_on(errors.pop()) {
            return Err(RenderError::Settings(format!("ray query shader: {error}")));
        }
        let with_layout = |pipeline: wgpu::ComputePipeline, output_slot_offset| QueryPipeline {
            layout: pipeline.get_bind_group_layout(0),
            pipeline,
            output_slot_offset,
        };
        Ok(Self {
            device,
            queue,
            closest: with_layout(closest, 0),
            visibility: with_layout(visibility, 1),
        })
    }
}

impl RtBackend for ComputeBackend {
    fn update_scene(&mut self, snapshot: &Snapshot) -> Result<(), RenderError> {
        snapshot.validate()?;
        let replacement = if self
            .geometry
            .as_ref()
            .is_some_and(|g| g.matches(&snapshot.meshes))
        {
            None
        } else {
            Some(Geometry::build(&self.dispatch.device, &snapshot.meshes)?)
        };
        let geometry = replacement.as_ref().or(self.geometry.as_ref()).unwrap();
        let top = TopLevel::build(&self.dispatch.device, snapshot, geometry)?;
        self.stats.blases = geometry.roots.iter().flatten().count();
        self.stats.triangles = geometry.meshes.iter().map(|m| m.triangles.len()).sum();
        self.stats.instances = top.count;
        self.stats.tlas_updates += 1;
        self.top = Some(top);
        if let Some(geometry) = replacement {
            self.geometry = Some(geometry);
            self.stats.blas_builds += 1;
        }
        Ok(())
    }

    fn stats(&self) -> SceneStats {
        self.stats
    }

    fn trace(&self, rays: &[Ray]) -> Result<Vec<Option<RayHit>>, RenderError> {
        let Some(buffers) = self.scene_buffers() else {
            return Err(RenderError::Settings(
                "ray query scene has not been built".into(),
            ));
        };
        self.dispatch
            .trace(rays, buffers.map(wgpu::Buffer::as_entire_binding))
    }

    fn occluded(&self, rays: &[Ray]) -> Result<Vec<bool>, RenderError> {
        let Some(buffers) = self.scene_buffers() else {
            return Err(RenderError::Settings(
                "ray query scene has not been built".into(),
            ));
        };
        self.dispatch
            .occluded(rays, buffers.map(wgpu::Buffer::as_entire_binding))
    }
}

impl QueryDispatch {
    pub fn trace<'a>(
        &self,
        rays: &[Ray],
        scene: impl IntoIterator<Item = wgpu::BindingResource<'a>>,
    ) -> Result<Vec<Option<RayHit>>, RenderError> {
        self.run(&self.closest, rays, scene, |hit: &Hit| {
            (hit.ids[0] != END).then(|| RayHit {
                t: f64::from(hit.tuv[0]),
                u: f64::from(hit.tuv[1]),
                v: f64::from(hit.tuv[2]),
                instance: hit.ids[0] as usize,
                triangle: hit.ids[1] as usize,
            })
        })
    }

    pub fn occluded<'a>(
        &self,
        rays: &[Ray],
        scene: impl IntoIterator<Item = wgpu::BindingResource<'a>>,
    ) -> Result<Vec<bool>, RenderError> {
        self.run(&self.visibility, rays, scene, |flag: &u32| *flag != 0)
    }

    fn run<'a, Raw: Pod, T>(
        &self,
        pipeline: &QueryPipeline,
        rays: &[Ray],
        scene: impl IntoIterator<Item = wgpu::BindingResource<'a>>,
        decode: impl Fn(&Raw) -> T,
    ) -> Result<Vec<T>, RenderError> {
        if rays.is_empty() {
            return Ok(Vec::new());
        }
        let device = &self.device;
        let size = (rays.len() * size_of::<Raw>()) as u64;
        check_size(device, "ray results", size)?;
        let groups = (rays.len() as u32).div_ceil(WORKGROUP_SIZE);
        if groups > device.limits().max_compute_workgroups_per_dimension {
            return Err(RenderError::Unsupported(
                "ray batch exceeds GPU dispatch limits".into(),
            ));
        }
        let queries: Vec<_> = rays
            .iter()
            .map(|r| Query {
                origin: row(r.origin, r.t_max as f32),
                direction: row(r.direction, 0.0),
            })
            .collect();
        let queries = upload(device, "ray queries", &queries)?;
        let buffer = |label, usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let out = buffer(
            "ray results",
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let readback = buffer(
            "ray readback",
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let scene_entries = scene
            .into_iter()
            .enumerate()
            .map(|(i, resource)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource,
            })
            .collect::<Vec<_>>();
        let mut entries: Vec<wgpu::BindGroupEntry<'_>> = scene_entries;
        entries.push(wgpu::BindGroupEntry {
            binding: entries.len() as u32,
            resource: queries.as_entire_binding(),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: entries.len() as u32 + pipeline.output_slot_offset,
            resource: out.as_entire_binding(),
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ray queries"),
            layout: &pipeline.layout,
            entries: &entries,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&pipeline.pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(groups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(&out, 0, &readback, 0, size);
        self.queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        let mapped = forge_render_gpu::map_read(&slice);
        forge_render_gpu::await_mapped(device, &mapped)?;
        let view = slice
            .get_mapped_range()
            .map_err(|e| RenderError::Output(format!("ray readback: {e}")))?;
        Ok(bytemuck::cast_slice::<_, Raw>(&view)
            .iter()
            .map(decode)
            .collect())
    }
}
