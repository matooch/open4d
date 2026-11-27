//! Standalone encoder-side scalar history and lighting copy (R24.13).

use forge_render_api::RenderError;
use wgpu::util::DeviceExt;

const LABEL: &str = "Tier 2 ambient history";
const SCALAR_BYTES: u32 = std::mem::size_of::<f32>() as u32;

pub(super) struct AmbientHistoryProgram(wgpu::ComputePipeline);

#[derive(Clone, Copy)]
pub(super) struct AmbientHistoryFrame {
    pub film: [u32; 2],
    pub strength: f32,
    pub format: wgpu::TextureFormat,
}

pub(super) struct AmbientHistory {
    mean: wgpu::Buffer,
    pub(super) output: wgpu::Buffer,
    film: [u32; 2],
    format: wgpu::TextureFormat,
    pitch: u32,
    pub(super) frames: u32,
}

impl AmbientHistoryProgram {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(LABEL),
            source: wgpu::ShaderSource::Wgsl(crate::shader::AMBIENT_HISTORY_WGSL.into()),
        });
        Self(
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(LABEL),
                layout: None,
                module: &module,
                entry_point: Some("ambient_history"),
                compilation_options: Default::default(),
                cache: None,
            }),
        )
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        filtered: &wgpu::Buffer,
        previous: Option<&AmbientHistory>,
        frame: AmbientHistoryFrame,
    ) -> Result<AmbientHistory, RenderError> {
        let AmbientHistoryFrame {
            film: [width, height],
            strength,
            format,
        } = frame;
        let channels = match format {
            wgpu::TextureFormat::R32Float => 1,
            wgpu::TextureFormat::Rgba32Float => 4,
            _ => return Err(RenderError::Unsupported("ambient lighting format".into())),
        };
        let pixel_bytes = channels * SCALAR_BYTES;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let unsupported =
            || RenderError::Unsupported("ambient history exceeds device limits".into());
        let pitch = width
            .div_ceil(alignment / pixel_bytes)
            .checked_mul(alignment)
            .ok_or_else(unsupported)?;
        let stride = pitch / pixel_bytes;
        let size = u64::from(pitch) * u64::from(height);
        let groups = [stride.div_ceil(8), height.div_ceil(8)];
        let limits = device.limits();
        if size == 0
            || width.max(height) > limits.max_texture_dimension_2d
            || size
                > limits
                    .max_buffer_size
                    .min(limits.max_storage_buffer_binding_size)
            || size / u64::from(SCALAR_BYTES) > u64::from(u32::MAX)
            || groups
                .iter()
                .any(|&n| n > limits.max_compute_workgroups_per_dimension)
        {
            return Err(unsupported());
        }
        let mean_size = u64::from(width) * u64::from(height) * u64::from(SCALAR_BYTES);
        assert_eq!(
            filtered.size(),
            mean_size,
            "ambient visibility must be compact"
        );
        assert!(previous.is_none_or(|p| p.film == frame.film));
        let frames = previous.map_or(0, |p| p.frames);
        let next = frames.saturating_add(1);
        let params = [
            width,
            height,
            stride,
            channels,
            frames,
            (1.0 / next as f32).to_bits(),
            strength.to_bits(),
            0,
        ];
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(LABEL),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let storage = |size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(LABEL),
                size,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            })
        };
        let mean = storage(mean_size);
        let output = storage(size);
        // Binding 2 is not read for the first sample.
        let previous = previous.map_or(filtered, |p| &p.mean);
        let buffers = [&uniform, filtered, previous, &mean, &output];
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &self.0.get_bind_group_layout(0),
            entries: &std::array::from_fn::<_, 5, _>(|i| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buffers[i].as_entire_binding(),
            }),
        });
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.0);
            pass.set_bind_group(0, &group, &[]);
            pass.dispatch_workgroups(groups[0], groups[1], 1);
        }
        Ok(AmbientHistory {
            mean,
            output,
            film: frame.film,
            format,
            pitch,
            frames: next,
        })
    }
}

impl AmbientHistory {
    pub fn read_mean(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<f32>, RenderError> {
        super::read_encoded(
            (device, queue),
            device.create_command_encoder(&Default::default()),
            &self.mean,
        )
    }

    pub fn copy_to_layer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        layer: u32,
    ) {
        assert_eq!(self.format, texture.format());
        let [width, height] = self.film;
        let mut dst = texture.as_image_copy();
        dst.origin.z = layer;
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &self.output,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(self.pitch),
                    rows_per_image: Some(height),
                },
            },
            dst,
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }
}
