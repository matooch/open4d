use forge_render_api::{MeshData, RenderError, Snapshot};
use glam::DMat4;
use wgpu::util::DeviceExt;

use super::{Ray, RayHit, RtBackend, SceneStats, compute::QueryDispatch};

/// Inline closest-hit kernel. The caller enables `wgpu_ray_query`, declares the
/// `scene` acceleration structure and supplies the shared `RAY_HIT_WGSL` definitions.
pub const HARDWARE_TRAVERSAL_WGSL: &str = include_str!("hardware.wgsl");
pub const HARDWARE_WGSL: &str = concat!(
    "enable wgpu_ray_query;\n",
    include_str!("hit.wgsl"),
    include_str!("hardware.wgsl"),
    "
@group(0) @binding(0) var scene: acceleration_structure;
@group(0) @binding(1) var<storage, read> queries: array<Query>;
@group(0) @binding(2) var<storage, read_write> hits: array<RtHit>;
@group(0) @binding(3) var<storage, read_write> blocked: array<u32>;
",
    include_str!("batch.wgsl"),
);

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

struct MeshSlot {
    geometry: MeshData,
    blas: Option<MeshBlas>,
}

pub(crate) struct HardwareBackend {
    dispatch: QueryDispatch,
    meshes: Vec<MeshSlot>,
    top: Option<wgpu::Tlas>,
    stats: SceneStats,
}

impl HardwareBackend {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Result<Self, RenderError> {
        Ok(Self {
            dispatch: QueryDispatch::new(device, queue, HARDWARE_WGSL)?,
            meshes: Vec::new(),
            top: None,
            stats: SceneStats::default(),
        })
    }

    pub(crate) fn tlas(&self) -> Option<&wgpu::Tlas> {
        self.top.as_ref()
    }
}

impl RtBackend for HardwareBackend {
    fn update_scene(&mut self, snapshot: &Snapshot) -> Result<(), RenderError> {
        snapshot.validate()?;
        let device = &self.dispatch.device;
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let result = (|| {
            // Outer None reuses a slot; Some(None) replaces it with an empty mesh.
            let replacements = snapshot
                .meshes
                .iter()
                .enumerate()
                .map(|(slot, mesh)| {
                    if self.meshes.get(slot).is_some_and(|old| {
                        old.geometry.positions == mesh.positions
                            && old.geometry.triangles == mesh.triangles
                    }) {
                        Ok(None)
                    } else {
                        MeshBlas::new(device, mesh).map(Some)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut instances = Vec::new();
            for (index, instance) in snapshot.instances.iter().enumerate() {
                let matrix = DMat4::from_cols_array(&instance.transform);
                let det = matrix.determinant();
                if !det.is_finite() || det.abs() < 1e-24 {
                    return Err(RenderError::Snapshot(
                        "an instance transform is singular".into(),
                    ));
                }
                let blas = match &replacements[instance.mesh] {
                    Some(blas) => blas,
                    None => &self.meshes[instance.mesh].blas,
                };
                if let Some(blas) = blas {
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
            let entries: Vec<_> = replacements
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
            Ok((replacements, top, count))
        })();
        let error = pollster::block_on(errors.pop());
        let (replacements, top, count) = result?;
        if let Some(err) = error {
            return Err(RenderError::Settings(format!("ray query scene: {err}")));
        }
        let mut geometry_update =
            self.stats.blas_builds == 0 || self.meshes.len() != replacements.len();
        self.meshes.truncate(replacements.len());
        for (slot, replacement) in replacements.into_iter().enumerate() {
            if let Some(blas) = replacement {
                let mesh = &snapshot.meshes[slot];
                let resident = MeshSlot {
                    geometry: MeshData::new(mesh.positions.clone(), mesh.triangles.clone()),
                    blas,
                };
                if let Some(old) = self.meshes.get_mut(slot) {
                    *old = resident;
                } else {
                    self.meshes.push(resident);
                }
                geometry_update = true;
            }
        }
        if geometry_update {
            self.stats.blases = self
                .meshes
                .iter()
                .filter(|mesh| mesh.blas.is_some())
                .count();
            self.stats.triangles = self
                .meshes
                .iter()
                .map(|mesh| mesh.geometry.triangles.len())
                .sum();
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

    fn occluded(&self, rays: &[Ray]) -> Result<Vec<bool>, RenderError> {
        if self.stats.tlas_updates == 0 {
            return Err(RenderError::Settings(
                "ray query scene has not been built".into(),
            ));
        }
        match &self.top {
            Some(top) => self.dispatch.occluded(rays, [top.as_binding()]),
            None => Ok(vec![false; rays.len()]),
        }
    }

    fn stats(&self) -> SceneStats {
        self.stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_render_api::{Instance, MaterialData};
    use glam::DVec3;

    fn resources(
        backend: &HardwareBackend,
        slot: usize,
    ) -> (wgpu::Blas, wgpu::Buffer, wgpu::Buffer) {
        let mesh = backend.meshes[slot].blas.as_ref().unwrap();
        (
            mesh.handle.clone(),
            mesh.vertices.clone(),
            mesh.indices.clone(),
        )
    }

    #[test]
    fn native_scene_replaces_only_edited_mesh_slots() {
        let _serial = crate::gpu_test_serial();
        let (device, queue) = match crate::request_rt_device() {
            Ok(pair) => pair,
            Err(err) if std::env::var("FORGE_REQUIRE_GPU").as_deref() == Ok("1") => {
                panic!("FORGE_REQUIRE_GPU=1 but no GPU: {err}");
            }
            Err(err) => {
                eprintln!("skipping native BLAS test: {err}");
                return;
            }
        };
        let kind = super::super::enabled_backend(&device);
        if kind == super::super::BackendKind::Compute {
            eprintln!("skipping native BLAS test: no native ray queries");
            return;
        }
        eprintln!("testing native BLAS residency on {kind:?}");
        let mut backend = HardwareBackend::new(device, queue).unwrap();
        let mesh = MeshData::new(
            vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
            vec![[0, 1, 2]],
        );
        let mut frame = Snapshot {
            meshes: vec![mesh.clone(), mesh.clone()],
            materials: vec![MaterialData::new("test", Default::default())],
            instances: vec![
                Instance::new(0, DMat4::IDENTITY, 0, 42),
                Instance::new(1, DMat4::from_translation(2. * DVec3::X), 0, 42),
            ],
            ..Default::default()
        };
        let rays = [0.25, 2.25, 2.75].map(|x| Ray {
            origin: DVec3::new(x, if x == 2.75 { 0.75 } else { 0.25 }, 5.),
            direction: -DVec3::Z,
            t_max: 10.,
        });
        let hits = |backend: &HardwareBackend| {
            backend
                .trace(&rays)
                .unwrap()
                .into_iter()
                .map(|hit| hit.map(|hit| (hit.instance, hit.triangle, hit.t)))
                .collect::<Vec<_>>()
        };

        backend.update_scene(&frame).unwrap();
        let stable = resources(&backend, 0);
        let old = resources(&backend, 1);
        assert_eq!(hits(&backend), [Some((0, 0, 5.)), Some((1, 0, 5.)), None]);

        frame.meshes[0].uvs = vec![[0.1, 0.2, 0.]; 3];
        for p in &mut frame.meshes[1].positions {
            p[2] = 1.;
        }
        backend.update_scene(&frame).unwrap();
        assert_eq!(resources(&backend, 0), stable);
        let changed = resources(&backend, 1);
        assert_ne!(changed.0, old.0);
        assert_ne!(changed.1, old.1);
        assert_ne!(changed.2, old.2);
        assert_eq!(hits(&backend), [Some((0, 0, 5.)), Some((1, 0, 4.)), None]);
        assert_eq!(backend.stats().blas_builds, 2);

        frame.meshes[1].triangles[0] = [3, 2, 1];
        let good_transform = frame.instances[1].transform;
        frame.instances[1].transform = DMat4::ZERO.to_cols_array();
        let stats = backend.stats();
        let top = backend.top.clone();
        assert!(backend.update_scene(&frame).is_err());
        assert_eq!(backend.stats(), stats);
        assert_eq!(backend.top, top);
        assert_eq!(resources(&backend, 0), stable);
        assert_eq!(resources(&backend, 1), changed);
        assert_eq!(hits(&backend), [Some((0, 0, 5.)), Some((1, 0, 4.)), None]);

        frame.instances[1].transform = good_transform;
        backend.update_scene(&frame).unwrap();
        assert_eq!(resources(&backend, 0), stable);
        assert_ne!(resources(&backend, 1).0, changed.0);
        assert_eq!(hits(&backend), [Some((0, 0, 5.)), None, Some((1, 0, 4.))]);

        frame.meshes[1].triangles.clear();
        frame.meshes.push(mesh);
        frame.instances.push(Instance::new(
            2,
            DMat4::from_cols_array(&good_transform),
            0,
            42,
        ));
        backend.update_scene(&frame).unwrap();
        assert_eq!(resources(&backend, 0), stable);
        assert!(backend.meshes[1].blas.is_none());
        assert_eq!(hits(&backend), [Some((0, 0, 5.)), Some((2, 0, 5.)), None]);
        assert_eq!(backend.stats().blases, 2);

        frame.instances.truncate(1);
        frame.meshes.truncate(1);
        backend.update_scene(&frame).unwrap();
        assert_eq!(resources(&backend, 0), stable);
        assert_eq!(backend.meshes.len(), 1);
        assert_eq!(hits(&backend), [Some((0, 0, 5.)), None, None]);
        assert_eq!(backend.stats().blas_builds, 5);
    }
}
