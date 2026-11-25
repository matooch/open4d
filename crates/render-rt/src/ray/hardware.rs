use forge_render_api::{MeshData, RenderError, Snapshot};
use glam::DMat4;
use wgpu::util::DeviceExt;

use super::{Ray, RayHit, RtBackend, SceneStats, compute::QueryDispatch};

pub const HARDWARE_WGSL: &str = include_str!("hardware.wgsl");

struct MeshBlas {
    handle: wgpu::Blas,
    size: wgpu::BlasTriangleGeometrySizeDescriptor,
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
}

impl MeshBlas {
    fn new(device: &wgpu::Device, mesh: &MeshData) -> Result<Option<Self>, RenderError> {
        if mesh.triangles.is_empty() {
            return Ok(None);
        }
        let limit = |what| RenderError::Unsupported(format!("ray query {what} exceeds GPU limits"));
        if mesh.triangles.len() > device.limits().max_blas_primitive_count as usize {
            return Err(limit("primitive count"));
        }
        let vertex_count =
            u32::try_from(mesh.positions.len()).map_err(|_| limit("vertex count"))?;
        let index_count =
            u32::try_from(mesh.triangles.len() * 3).map_err(|_| limit("index count"))?;
        let upload = |label, contents: &[u8]| {
            if contents.len() as u64 > device.limits().max_buffer_size {
                return Err(limit(label));
            }
            Ok(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents,
                    usage: wgpu::BufferUsages::BLAS_INPUT,
                }),
            )
        };
        let positions: Vec<_> = mesh.positions.iter().map(|p| p.map(|x| x as f32)).collect();
        let vertices = upload("BLAS vertices", bytemuck::cast_slice(&positions))?;
        let indices = upload("BLAS indices", bytemuck::cast_slice(&mesh.triangles))?;
        let size = wgpu::BlasTriangleGeometrySizeDescriptor {
            vertex_format: wgpu::VertexFormat::Float32x3,
            vertex_count,
            index_format: Some(wgpu::IndexFormat::Uint32),
            index_count: Some(index_count),
            flags: wgpu::AccelerationStructureGeometryFlags::OPAQUE,
        };
        let handle = device.create_blas(
            &wgpu::CreateBlasDescriptor {
                label: Some("mesh BLAS"),
                flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                update_mode: wgpu::AccelerationStructureUpdateMode::Build,
            },
            wgpu::BlasGeometrySizeDescriptors::Triangles {
                descriptors: vec![size.clone()],
            },
        );
        Ok(Some(Self {
            handle,
            size,
            vertices,
            indices,
        }))
    }

    fn build_entry(&self) -> wgpu::BlasBuildEntry<'_> {
        wgpu::BlasBuildEntry {
            blas: &self.handle,
            geometry: wgpu::BlasGeometries::TriangleGeometries(vec![wgpu::BlasTriangleGeometry {
                size: &self.size,
                vertex_buffer: &self.vertices,
                first_vertex: 0,
                vertex_stride: 12,
                index_buffer: Some(&self.indices),
                first_index: Some(0),
                transform_buffer: None,
                transform_buffer_offset: None,
            }]),
        }
    }
}

pub(super) struct HardwareBackend {
    dispatch: QueryDispatch,
    meshes: Vec<MeshData>,
    blases: Vec<Option<MeshBlas>>,
    top: Option<wgpu::Tlas>,
    stats: SceneStats,
}

impl HardwareBackend {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Result<Self, RenderError> {
        Ok(Self {
            dispatch: QueryDispatch::new(device, queue, HARDWARE_WGSL)?,
            meshes: Vec::new(),
            blases: Vec::new(),
            top: None,
            stats: SceneStats::default(),
        })
    }
}

impl RtBackend for HardwareBackend {
    fn update_scene(&mut self, snapshot: &Snapshot) -> Result<(), RenderError> {
        snapshot.validate()?;
        let device = &self.dispatch.device;
        let new_geometry = self.stats.blas_builds == 0
            || self.meshes.len() != snapshot.meshes.len()
            || self
                .meshes
                .iter()
                .zip(&snapshot.meshes)
                .any(|(a, b)| a.positions != b.positions || a.triangles != b.triangles);
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = (|| {
            let replacement = if new_geometry {
                Some(
                    snapshot
                        .meshes
                        .iter()
                        .map(|mesh| MeshBlas::new(device, mesh))
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            };
            let blases = replacement.as_deref().unwrap_or(&self.blases);
            let mut instances = Vec::new();
            for (index, instance) in snapshot.instances.iter().enumerate() {
                let matrix = DMat4::from_cols_array(&instance.transform);
                let det = matrix.determinant();
                if !det.is_finite() || det.abs() < 1e-24 {
                    return Err(RenderError::Snapshot(
                        "an instance transform is singular".into(),
                    ));
                }
                if let Some(blas) = &blases[instance.mesh] {
                    if index >= 1 << 24
                        || instances.len() >= device.limits().max_tlas_instance_count as usize
                    {
                        return Err(RenderError::Unsupported(
                            "ray query instance count or ID exceeds GPU limits".into(),
                        ));
                    }
                    let transform =
                        std::array::from_fn(|i| instance.transform[(i % 4) * 4 + i / 4] as f32);
                    instances.push(wgpu::TlasInstance::new(
                        &blas.handle,
                        transform,
                        index as u32,
                        0xff,
                    ));
                }
            }
            let count = instances.len();
            let top = (count > 0).then(|| {
                let mut top = device.create_tlas(&wgpu::CreateTlasDescriptor {
                    label: Some("scene TLAS"),
                    max_instances: count as u32,
                    flags: wgpu::AccelerationStructureFlags::PREFER_FAST_TRACE,
                    update_mode: wgpu::AccelerationStructureUpdateMode::Build,
                });
                for (i, instance) in instances.into_iter().enumerate() {
                    top[i] = Some(instance);
                }
                top
            });
            let entries: Vec<_> = replacement
                .iter()
                .flatten()
                .flatten()
                .map(MeshBlas::build_entry)
                .collect();
            if !entries.is_empty() || top.is_some() {
                let mut encoder = device.create_command_encoder(&Default::default());
                encoder.build_acceleration_structures(&entries, top.iter());
                self.dispatch.queue.submit([encoder.finish()]);
            }
            Ok((replacement, top, count))
        })();
        let error = pollster::block_on(errors.pop());
        let (replacement, top, count) = result?;
        if let Some(err) = error {
            return Err(RenderError::Settings(format!("ray query scene: {err}")));
        }
        if let Some(blases) = replacement {
            self.meshes = snapshot
                .meshes
                .iter()
                .map(|mesh| MeshData::new(mesh.positions.clone(), mesh.triangles.clone()))
                .collect();
            self.stats.blases = blases.iter().flatten().count();
            self.stats.triangles = self.meshes.iter().map(|mesh| mesh.triangles.len()).sum();
            self.blases = blases;
            self.stats.blas_builds += 1;
        }
        self.top = top;
        self.stats.instances = count;
        self.stats.tlas_updates += 1;
        Ok(())
    }

    fn trace(&self, rays: &[Ray]) -> Result<Vec<Option<RayHit>>, RenderError> {
        if self.stats.tlas_updates == 0 {
            return Err(RenderError::Settings(
                "ray query scene has not been built".into(),
            ));
        }
        match &self.top {
            Some(top) => self.dispatch.trace(rays, [top.as_binding()]),
            None => Ok(vec![None; rays.len()]),
        }
    }

    fn stats(&self) -> SceneStats {
        self.stats
    }
}
