//! The GPU engine: tiles, dispatches, readback and outputs (docs/specs/RENDER.md R20).

use std::collections::BTreeMap;

use forge_image::{AlphaMode, Image};
use forge_render_api::{
    Aov, Control, RenderEngine, RenderError, RenderEvent, RenderOutput, RenderSettings,
    RenderStats, Snapshot,
};
use wgpu::util::DeviceExt;

use crate::frame;
use crate::pack::{Packed, frame_base};

/// A wgpu compute path tracer. Create it with [`GpuEngine::new`], or share a device with
/// [`GpuEngine::with_device`].
#[derive(Clone)]
pub struct GpuEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// One tile's finished pixels.
struct TileOut {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    beauty: Vec<[f32; 4]>,
    aovs: Vec<Vec<[f32; 4]>>,
}

/// `(x, y, w, h)` of every tile of the render region (R2).
fn tiles_of(settings: &RenderSettings) -> Vec<(u32, u32, u32, u32)> {
    let (w, h) = (settings.width, settings.height);
    let [rx0, ry0, rx1, ry1] = settings.region.unwrap_or([0, 0, w, h]);
    let ts = settings.tile;
    let mut tiles = Vec::new();
    let mut y = ry0;
    while y < ry1 {
        let mut x = rx0;
        while x < rx1 {
            tiles.push((x, y, ts.min(rx1 - x), ts.min(ry1 - y)));
            x += ts;
        }
        y += ts;
    }
    tiles
}

impl GpuEngine {
    /// The default adapter; `Err` carries the reason there is none.
    pub fn new() -> Result<GpuEngine, RenderError> {
        let (device, queue) = crate::request_default_device()
            .map_err(|e| RenderError::Unsupported(format!("no GPU adapter: {e}")))?;
        Ok(GpuEngine { device, queue })
    }

    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue) -> GpuEngine {
        GpuEngine { device, queue }
    }

    fn shader_buffers(
        &self,
        packed: &Packed,
    ) -> (wgpu::Buffer, wgpu::Buffer, wgpu::Buffer, wgpu::Buffer) {
        let make = |label: &str, bytes: &[u8]| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: bytes,
                    usage: wgpu::BufferUsages::STORAGE,
                })
        };
        (
            make("tris", bytemuck::cast_slice(&packed.tris)),
            make("nodes", bytemuck::cast_slice(&packed.nodes)),
            make("prims", bytemuck::cast_slice(&packed.prims)),
            make("lights", bytemuck::cast_slice(&packed.lights)),
        )
    }
}

impl RenderEngine for GpuEngine {
    fn id(&self) -> &str {
        "forge.render.gpu"
    }

    fn render(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError> {
        settings.validate()?;
        if matches!(
            snapshot.environment.lighting,
            forge_render_api::Lighting::Image { .. }
        ) {
            return Err(RenderError::Unsupported(
                "image environments on the GPU (RENDER.md R16, R20.4)".into(),
            ));
        }
        let packed = Packed::build(snapshot, settings.default_light)?;
        let mut wanted: Vec<Aov> = settings.aovs.clone();
        wanted.sort();
        wanted.dedup();
        let stride = 1 + wanted.len();

        let graphs: Vec<&forge_material::ShadingGraph> =
            snapshot.materials.iter().map(|m| &m.graph).collect();
        let source = crate::shader::assemble(&graphs)?;

        let device = &self.device;
        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("path tracer"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("path tracer"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                storage_entry(1, true),
                storage_entry(2, true),
                storage_entry(3, true),
                storage_entry(4, true),
                storage_entry(5, false),
            ],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("path tracer"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout)],
                    immediate_size: 0,
                }),
            ),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        if let Some(e) = pollster::block_on(errors.pop()) {
            return Err(RenderError::Settings(format!("GPU shader: {e}")));
        }

        let (tris, nodes, prims, lights) = self.shader_buffers(&packed);
        let frame_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: (frame::SLOTS * 16) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let base = frame_base(snapshot, settings, &packed, &wanted, stride);

        let tiles = tiles_of(settings);
        control.emit(RenderEvent::Started {
            width: settings.width,
            height: settings.height,
            tiles: tiles.len(),
        });
        let max_pixels = settings.tile as u64 * settings.tile as u64;
        let max_bytes = (max_pixels * stride as u64 * 16).max(16);
        let out = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("tile"),
            size: max_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: max_bytes,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: tris.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: nodes.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: prims.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: lights.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: out.as_entire_binding(),
                },
            ],
        });

        let mut outs: Vec<TileOut> = Vec::with_capacity(tiles.len());
        for (i, &(tx, ty, tw, th)) in tiles.iter().enumerate() {
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            let mut frame = base;
            frame[frame::TX] = tx as f32;
            frame[frame::TY] = ty as f32;
            frame[frame::TILE_W] = tw as f32;
            frame[frame::TILE_H] = th as f32;
            self.queue.write_buffer(&frame_buf, 0, &frame_bytes(&frame));
            let bytes = tw as u64 * th as u64 * stride as u64 * 16;
            let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("tile"),
            });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("tile"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.dispatch_workgroups(tw.div_ceil(8), th.div_ceil(8), 1);
            }
            encoder.copy_buffer_to_buffer(&out, 0, &staging, 0, bytes);
            self.queue.submit([encoder.finish()]);
            let slice = staging.slice(..bytes);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("tile readback"));
            device
                .poll(wgpu::PollType::wait_indefinitely())
                .map_err(|e| RenderError::Output(format!("GPU poll: {e}")))?;
            let data = slice
                .get_mapped_range()
                .map_err(|e| RenderError::Output(format!("GPU readback: {e}")))?;
            let values: Vec<[f32; 4]> = data
                .as_chunks::<16>()
                .0
                .iter()
                .map(|b| {
                    let f = |k: usize| {
                        f32::from_le_bytes([b[k * 4], b[k * 4 + 1], b[k * 4 + 2], b[k * 4 + 3]])
                    };
                    [f(0), f(1), f(2), f(3)]
                })
                .collect();
            drop(data);
            staging.unmap();
            let mut beauty = Vec::with_capacity((tw * th) as usize);
            let mut aovs = vec![Vec::with_capacity((tw * th) as usize); wanted.len()];
            for p in 0..(tw * th) as usize {
                beauty.push(values[p * stride]);
                for (k, a) in aovs.iter_mut().enumerate() {
                    a.push(values[p * stride + 1 + k]);
                }
            }
            control.emit(RenderEvent::Tile {
                x: tx,
                y: ty,
                width: tw,
                height: th,
                rgba: beauty.iter().flatten().copied().collect(),
            });
            control.emit(RenderEvent::Progress {
                done: i + 1,
                total: tiles.len(),
            });
            outs.push(TileOut {
                x: tx,
                y: ty,
                w: tw,
                h: th,
                beauty,
                aovs,
            });
        }
        control.emit(RenderEvent::Finished);

        let mut beauty = Image::new(settings.width, settings.height, AlphaMode::Premultiplied);
        let mut aov_imgs: BTreeMap<Aov, Image> = wanted
            .iter()
            .map(|a| {
                (
                    *a,
                    Image::new(settings.width, settings.height, AlphaMode::Straight),
                )
            })
            .collect();
        for t in &outs {
            for j in 0..t.h {
                for i in 0..t.w {
                    let k = (j * t.w + i) as usize;
                    beauty.set_pixel(t.x + i, t.y + j, t.beauty[k]);
                    for (a, img) in wanted.iter().zip(t.aovs.iter()) {
                        if let Some(im) = aov_imgs.get_mut(a) {
                            im.set_pixel(t.x + i, t.y + j, img[k]);
                        }
                    }
                }
            }
        }
        let [rx0, ry0, rx1, ry1] =
            settings
                .region
                .unwrap_or([0, 0, settings.width, settings.height]);
        Ok(RenderOutput {
            width: settings.width,
            height: settings.height,
            beauty,
            aovs: aov_imgs,
            stats: RenderStats {
                triangles: packed.triangles,
                instances: packed.instances,
                tiles: tiles.len(),
                samples: (rx1 - rx0) as u64 * (ry1 - ry0) as u64 * settings.samples as u64,
                ignored_channels: forge_render_cpu::ignored_channels(snapshot),
            },
        })
    }
}

/// The frame uniform is `array<vec4<f32>, 96>`: one value in each row's `x`, rows 16 bytes apart.
fn frame_bytes(frame: &[f32; frame::SLOTS]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(frame::SLOTS * 16);
    for v in frame {
        bytes.extend_from_slice(&v.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 12]);
    }
    bytes
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
