//! Encoder-side R24.12 history and lighting-layer copy.

use forge_render_api::RenderError;
use wgpu::util::DeviceExt;

const LABEL: &str = "Tier 2 diffuse history";
const PIXEL_BYTES: u32 = 16;

pub(super) struct DiffuseHistoryProgram(wgpu::ComputePipeline);

#[derive(Clone)]
pub(crate) struct DiffuseHistory {
    mean: wgpu::Buffer,
    film: [u32; 2],
    pitch: u32,
    frames: u32,
}

impl DiffuseHistoryProgram {
    pub fn new(device: &wgpu::Device) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(LABEL),
            source: wgpu::ShaderSource::Wgsl(crate::shader::DIFFUSE_HISTORY_WGSL.into()),
        });
        Self(
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(LABEL),
                layout: None,
                module: &module,
                entry_point: Some("diffuse_history"),
                compilation_options: Default::default(),
                cache: None,
            }),
        )
    }

    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        film: [u32; 2],
        filtered: &wgpu::Buffer,
        previous: Option<&DiffuseHistory>,
    ) -> Result<DiffuseHistory, RenderError> {
        let [width, height] = film;
        let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let limits = device.limits();
        let unsupported =
            || RenderError::Unsupported("diffuse history exceeds device limits".into());
        let pitch = width
            .div_ceil(alignment / PIXEL_BYTES)
            .checked_mul(alignment)
            .ok_or_else(unsupported)?;
        let stride = pitch / PIXEL_BYTES;
        let size = u64::from(pitch) * u64::from(height);
        let groups = [stride.div_ceil(8), height.div_ceil(8)];
        if size == 0
            || size
                > limits
                    .max_buffer_size
                    .min(limits.max_storage_buffer_binding_size)
            || u64::from(stride) * u64::from(height) > u64::from(u32::MAX)
            || groups
                .iter()
                .any(|&n| n > limits.max_compute_workgroups_per_dimension)
        {
            return Err(unsupported());
        }
        assert_eq!(
            filtered.size(),
            u64::from(width) * u64::from(height) * u64::from(PIXEL_BYTES),
            "history input must be compact RGBA32Float"
        );
        assert!(previous.is_none_or(|p| p.film == film));
        let frames = previous.map_or(0, |p| p.frames);
        let next = frames.saturating_add(1);
        let params = [
            width,
            height,
            stride,
            frames,
            (1. / next as f32).to_bits(),
            0,
            0,
            0,
        ];
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(LABEL),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let mean = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(LABEL),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        // On the first frame the shader does not read binding 2.
        let previous = previous.map_or(filtered, |p| &p.mean);
        let buffers = [&uniform, filtered, previous, &mean];
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(LABEL),
            layout: &self.0.get_bind_group_layout(0),
            entries: &std::array::from_fn::<_, 4, _>(|i| wgpu::BindGroupEntry {
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
        Ok(DiffuseHistory {
            mean,
            film,
            pitch,
            frames: next,
        })
    }
}

impl DiffuseHistory {
    pub fn frames(&self) -> u32 {
        self.frames
    }

    pub fn read_mean(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<[f32; 4]>, RenderError> {
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("diffuse history fallback"),
            size: self.mean.size(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.mean, 0, &staging, 0, self.mean.size());
        queue.submit([encoder.finish()]);
        let slice = staging.slice(..);
        forge_render_gpu::await_mapped(device, &forge_render_gpu::map_read(&slice))?;
        let bytes = slice
            .get_mapped_range()
            .map_err(|err| RenderError::Output(err.to_string()))?;
        let row = (self.film[0] * PIXEL_BYTES) as usize;
        Ok(bytes
            .chunks_exact(self.pitch as usize)
            .flat_map(|padded| bytemuck::cast_slice::<u8, [f32; 4]>(&padded[..row]))
            .copied()
            .collect())
    }

    pub fn copy_to_layer(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        texture: &wgpu::Texture,
        layer: u32,
    ) {
        let [width, height] = self.film;
        let mut dst = texture.as_image_copy();
        dst.origin.z = layer;
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer: &self.mean,
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
