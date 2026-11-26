//! Host dispatch of the raw R24.7 shadow signal, exercised by the standalone probe.

use bytemuck::{Pod, Zeroable};
use forge_render_api::{RenderError, RenderSettings, Snapshot};
use forge_render_cpu::lights::PLight;
use forge_render_gpu::{
    await_mapped, map_read,
    pack::{GpuLight, pack_light},
    shader::Assembled,
};
use wgpu::util::DeviceExt;

use super::PrimaryInput;
use crate::ray::{BackendKind, InlineScene};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct DirectShadowFrame {
    camera: [[f32; 4]; 3],
    film_scale: [f32; 4],
    jitter: [f32; 2],
    seed: [u32; 2],
    history: u32,
    instance_count: u32,
    light_count: u32,
    epsilon: f32,
}

impl DirectShadowFrame {
    pub fn new(
        snapshot: &Snapshot,
        settings: &RenderSettings,
        input: PrimaryInput<'_>,
        scale: f64,
        history: u32,
        light_count: usize,
    ) -> Self {
        let (f, [x, y]) = snapshot
            .camera
            .pixel_frame(settings.width, settings.height)
            .unwrap_or_default();
        let [u, v, w, _] = snapshot.camera.matrix().as_mat4().to_cols_array_2d();
        Self {
            camera: [u, v, w],
            film_scale: [f as f32, x as f32, y as f32, scale as f32],
            jitter: input.jitter,
            seed: [settings.seed as u32, (settings.seed >> 32) as u32],
            history,
            instance_count: snapshot.instances.len() as u32,
            light_count: light_count as u32,
            epsilon: input.epsilon as f32,
        }
    }
}

pub(super) struct DirectShadowBindings<'a> {
    pub scene: InlineScene<'a>,
    pub primary: [&'a wgpu::TextureView; 2],
    pub attributes: &'a wgpu::Buffer,
    pub bitmaps: &'a [wgpu::TextureView],
    pub lights: &'a [PLight],
}

pub(super) struct DirectShadowProgram {
    pipeline: wgpu::ComputePipeline,
    layouts: [wgpu::BindGroupLayout; 3],
}

impl DirectShadowProgram {
    pub fn new(device: &wgpu::Device, kind: BackendKind, assembled: Assembled) -> Self {
        use wgpu::{BindingType as Binding, BufferBindingType as Buffer};
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let layout = |entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("raw direct shadows"),
                entries,
            })
        };
        let buffer = |ty| Binding::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let storage = buffer(Buffer::Storage { read_only: true });
        let texture = Binding::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let mut scene = if kind == BackendKind::Compute {
            (0..4).map(|i| entry(i, storage)).collect()
        } else {
            vec![entry(
                0,
                Binding::AccelerationStructure {
                    vertex_return: false,
                },
            )]
        };
        scene.push(entry(4, storage));
        let layouts = [
            layout(&[
                entry(0, buffer(Buffer::Uniform)),
                entry(1, texture),
                entry(2, texture),
                entry(3, storage),
                entry(4, buffer(Buffer::Storage { read_only: false })),
            ]),
            layout(
                &(0..assembled.resources.len() as u32)
                    .map(|i| entry(i, texture))
                    .collect::<Vec<_>>(),
            ),
            layout(&scene),
        ];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("raw direct shadows"),
            bind_group_layouts: &layouts.each_ref().map(Some),
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raw direct shadows"),
            source: wgpu::ShaderSource::Wgsl(assembled.source.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("raw direct shadows"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("direct_shadow"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layouts }
    }

    pub fn read(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        bindings: DirectShadowBindings<'_>,
        settings: &RenderSettings,
        frame: DirectShadowFrame,
    ) -> Result<Vec<[f32; 8]>, RenderError> {
        let size = u64::from(settings.width) * u64::from(settings.height);
        let groups = [settings.width.div_ceil(8), settings.height.div_ceil(8)];
        let limits = device.limits();
        let storage_limit = limits
            .max_buffer_size
            .min(limits.max_storage_buffer_binding_size);
        let size = size
            .checked_mul(32)
            .filter(|&bytes| bytes > 0 && bytes <= storage_limit);
        let Some(size) = size.filter(|_| {
            groups
                .iter()
                .all(|&n| n <= limits.max_compute_workgroups_per_dimension)
        }) else {
            return Err(RenderError::Unsupported(
                "raw direct-shadow dispatch exceeds device limits".into(),
            ));
        };
        let upload = |contents, usage| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("raw direct shadows"),
                contents,
                usage,
            })
        };
        let uniform = upload(bytemuck::bytes_of(&frame), wgpu::BufferUsages::UNIFORM);
        let lights: Vec<_> = bindings.lights.iter().map(pack_light).collect();
        let dummy = GpuLight::zeroed();
        let light_bytes = if lights.is_empty() {
            bytemuck::bytes_of(&dummy)
        } else {
            bytemuck::cast_slice(&lights)
        };
        let lights = upload(light_bytes, wgpu::BufferUsages::STORAGE);
        let alloc = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("raw direct shadows"),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let signal = alloc(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let staging = alloc(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
        let bind = |group: usize, entries: &[wgpu::BindGroupEntry<'_>]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raw direct shadows"),
                layout: &self.layouts[group],
                entries,
            })
        };
        let texture = wgpu::BindingResource::TextureView;
        let sequential = |resources: Vec<wgpu::BindingResource<'_>>, group| {
            bind(
                group,
                &resources
                    .into_iter()
                    .enumerate()
                    .map(|(i, resource)| wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource,
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let frame_bind = sequential(
            vec![
                uniform.as_entire_binding(),
                texture(bindings.primary[0]),
                texture(bindings.primary[1]),
                lights.as_entire_binding(),
                signal.as_entire_binding(),
            ],
            0,
        );
        let bitmap_bind = sequential(bindings.bitmaps.iter().map(texture).collect(), 1);
        let mut scene: Vec<_> = match bindings.scene {
            InlineScene::Compute(buffers) => buffers
                .into_iter()
                .enumerate()
                .map(|(i, buffer)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: buffer.as_entire_binding(),
                })
                .collect(),
            InlineScene::Hardware(tlas) => vec![wgpu::BindGroupEntry {
                binding: 0,
                resource: tlas.as_binding(),
            }],
        };
        scene.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: bindings.attributes.as_entire_binding(),
        });
        let scene_bind = bind(2, &scene);
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &frame_bind, &[]);
            pass.set_bind_group(1, &bitmap_bind, &[]);
            pass.set_bind_group(2, &scene_bind, &[]);
            pass.dispatch_workgroups(groups[0], groups[1], 1);
        }
        encoder.copy_buffer_to_buffer(&signal, 0, &staging, 0, size);
        queue.submit([encoder.finish()]);
        let slice = staging.slice(..);
        await_mapped(device, &map_read(&slice))?;
        let mapped = slice
            .get_mapped_range()
            .map_err(|err| RenderError::Output(err.to_string()))?;
        Ok(bytemuck::cast_slice(&mapped).to_vec())
    }
}
