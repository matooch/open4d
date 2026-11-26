//! GPU spatial reconstruction of Tier 2 bounce and direct-shadow signals (R24.11).

use wgpu::util::DeviceExt;

const LABEL: &str = "Tier 2 signal spatial";

pub(super) struct BounceSpatialProgram {
    pipeline: wgpu::ComputePipeline,
    layout: wgpu::BindGroupLayout,
}

pub(super) struct BounceSpatialPass<'a> {
    program: &'a BounceSpatialProgram,
    limits: SpatialLimits,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SpatialLimits {
    distance_squared: f32,
    depth: f32,
    lanes: u32,
    _padding: u32,
}

impl BounceSpatialProgram {
    pub fn new(device: &wgpu::Device) -> Self {
        use wgpu::{BindingType, BufferBindingType};
        let buffer = |ty| BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        let texture = BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        };
        let entries = [
            buffer(BufferBindingType::Uniform),
            texture,
            texture,
            buffer(BufferBindingType::Storage { read_only: true }),
            buffer(BufferBindingType::Storage { read_only: false }),
        ];
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(LABEL),
            entries: &std::array::from_fn::<_, 5, _>(|i| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: entries[i],
                count: None,
            }),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(LABEL),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(LABEL),
            source: wgpu::ShaderSource::Wgsl(crate::shader::BOUNCE_SPATIAL_WGSL.into()),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(LABEL),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("bounce_spatial"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { pipeline, layout }
    }

    pub fn pass(&self, limit: f64) -> BounceSpatialPass<'_> {
        self.pass_with_lanes(limit, 1)
    }

    pub fn shadow_pass(&self, limit: f64) -> BounceSpatialPass<'_> {
        self.pass_with_lanes(limit, 2)
    }

    fn pass_with_lanes(&self, limit: f64, lanes: u32) -> BounceSpatialPass<'_> {
        BounceSpatialPass {
            program: self,
            limits: SpatialLimits {
                distance_squared: (limit * limit) as f32,
                depth: (limit * 0.1) as f32,
                lanes,
                _padding: 0,
            },
        }
    }
}

impl BounceSpatialPass<'_> {
    pub fn encode(
        self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        primary: [&wgpu::TextureView; 2],
        raw: &wgpu::Buffer,
        film: [u32; 2],
    ) -> wgpu::Buffer {
        assert_eq!(
            raw.size(),
            u64::from(film[0]) * u64::from(film[1]) * u64::from(self.limits.lanes) * 16,
            "spatial signal stride mismatch"
        );
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(LABEL),
            contents: bytemuck::bytes_of(&self.limits),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let filtered = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(LABEL),
            size: raw.size(),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let texture = wgpu::BindingResource::TextureView;
        let resources = [
            uniform.as_entire_binding(),
            texture(primary[0]),
            texture(primary[1]),
            raw.as_entire_binding(),
            filtered.as_entire_binding(),
        ];
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &self.program.layout,
            entries: &std::array::from_fn::<_, 5, _>(|i| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: resources[i].clone(),
            }),
        });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(LABEL),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.program.pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(film[0].div_ceil(8), film[1].div_ceil(8), 1);
        }
        filtered
    }
}
