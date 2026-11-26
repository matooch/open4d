//! Host dispatch of the R24.6 raw ambient-visibility stage.

use bytemuck::{Pod, Zeroable};
use forge_render_api::{RenderError, RenderSettings};
use forge_render_gpu::{await_mapped, map_read};
use wgpu::util::DeviceExt;

use crate::ray::{BackendKind, InlineScene};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct AmbientFrame {
    seed: [u32; 2],
    history_frame: u32,
    radius: f32,
    epsilon: f32,
    _pad: u32,
}

impl AmbientFrame {
    pub fn new(settings: &RenderSettings, history_frame: u32, epsilon: f64) -> Self {
        Self {
            seed: [settings.seed as u32, (settings.seed >> 32) as u32],
            history_frame,
            radius: settings.hybrid.ao_radius as f32,
            epsilon: epsilon as f32,
            _pad: 0,
        }
    }
}

pub(crate) struct AmbientProgram {
    pipeline: wgpu::ComputePipeline,
    layouts: [wgpu::BindGroupLayout; 3],
}

impl AmbientProgram {
    pub fn new(device: &wgpu::Device, kind: BackendKind) -> Self {
        use wgpu::{BindingType as Binding, BufferBindingType as Buffer};
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty,
            count: None,
        };
        let layout = |entries: &[wgpu::BindGroupLayoutEntry]| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("raw AO"),
                entries,
            })
        };
        let buffer = |ty| Binding::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let texture = Binding::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let scene = if kind == BackendKind::Compute {
            layout(&[0, 1, 2, 3].map(|i| entry(i, buffer(Buffer::Storage { read_only: true }))))
        } else {
            layout(&[entry(
                0,
                Binding::AccelerationStructure {
                    vertex_return: false,
                },
            )])
        };
        let layouts = [
            layout(&[
                entry(0, buffer(Buffer::Uniform)),
                entry(1, texture),
                entry(2, texture),
            ]),
            layout(&[entry(0, buffer(Buffer::Storage { read_only: false }))]),
            scene,
        ];
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("raw AO"),
            bind_group_layouts: &layouts.each_ref().map(Some),
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raw AO"),
            source: wgpu::ShaderSource::Wgsl(crate::shader::ambient_visibility_wgsl(kind).into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("raw AO"),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("ambient_visibility"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layouts }
    }

    pub fn read(
        &self,
        (device, queue): (&wgpu::Device, &wgpu::Queue),
        scene: InlineScene<'_>,
        textures: [&wgpu::TextureView; 2],
        settings: &RenderSettings,
        frame: AmbientFrame,
    ) -> Result<Vec<f32>, RenderError> {
        let size = u64::from(settings.width) * u64::from(settings.height) * 4;
        let limits = device.limits();
        let groups = [settings.width.div_ceil(16), settings.height.div_ceil(16)];
        if size > limits.max_buffer_size
            || size > limits.max_storage_buffer_binding_size
            || groups
                .iter()
                .any(|&n| n > limits.max_compute_workgroups_per_dimension)
        {
            return Err(RenderError::Unsupported(
                "raw AO dispatch exceeds device limits".into(),
            ));
        }
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("raw AO frame"),
            contents: bytemuck::bytes_of(&frame),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let alloc = |usage| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("raw AO visibility"),
                size,
                usage,
                mapped_at_creation: false,
            })
        };
        let visibility = alloc(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC);
        let staging = alloc(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ);
        let bind = |layout, resources: &[wgpu::BindingResource<'_>]| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("raw AO"),
                layout,
                entries: &resources
                    .iter()
                    .enumerate()
                    .map(|(binding, resource)| wgpu::BindGroupEntry {
                        binding: binding as u32,
                        resource: resource.clone(),
                    })
                    .collect::<Vec<_>>(),
            })
        };
        let frame_bind = bind(
            &self.layouts[0],
            &[
                uniform.as_entire_binding(),
                wgpu::BindingResource::TextureView(textures[0]),
                wgpu::BindingResource::TextureView(textures[1]),
            ],
        );
        let output_bind = bind(&self.layouts[1], &[visibility.as_entire_binding()]);
        let scene_bind = match scene {
            InlineScene::Compute(buffers) => bind(
                &self.layouts[2],
                &buffers.map(wgpu::Buffer::as_entire_binding),
            ),
            InlineScene::Hardware(tlas) => bind(&self.layouts[2], &[tlas.as_binding()]),
        };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("raw AO"),
        });
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &frame_bind, &[]);
            pass.set_bind_group(1, &output_bind, &[]);
            pass.set_bind_group(2, &scene_bind, &[]);
            pass.dispatch_workgroups(groups[0], groups[1], 1);
        }
        encoder.copy_buffer_to_buffer(&visibility, 0, &staging, 0, size);
        queue.submit([encoder.finish()]);
        let slice = staging.slice(..);
        await_mapped(device, &map_read(&slice))?;
        let data = slice
            .get_mapped_range()
            .map_err(|err| RenderError::Output(err.to_string()))?;
        Ok(bytemuck::cast_slice(&data).to_vec())
    }
}
