//! The GPU engine: tiles, dispatches, readback and outputs (docs/specs/RENDER.md R20).

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

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
    /// Most pixel-samples (pixels x samples per pixel) one submission may hold (R20.4).
    submission_budget: u64,
}

/// Default for [`GpuEngine::submission_budget`]: small enough that a slow software device
/// finishes one submission well inside a driver watchdog (about 2 s on Windows).
pub const SUBMISSION_BUDGET: u64 = 4096;

type MapSlot = Arc<Mutex<Option<Result<(), wgpu::BufferAsyncError>>>>;

/// Waits for every submission, bounded by `GPU_WAIT_TIMEOUT`.
fn wait_for_gpu(device: &wgpu::Device) -> Result<(), RenderError> {
    device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(forge_render_api::GPU_WAIT_TIMEOUT),
        })
        .map(|_| ())
        .map_err(|e| {
            RenderError::Output(format!(
                "GPU did not finish within {:?} (hung, lost or too slow): {e}",
                forge_render_api::GPU_WAIT_TIMEOUT
            ))
        })
}

/// Waits until the map callback has stored its result, then turns it into an error if the map
/// failed; a map callback never panics. With several threads polling one device, a poll can
/// return before this thread's callback ran, so keep polling until the deadline.
fn await_mapped(device: &wgpu::Device, slot: &MapSlot) -> Result<(), RenderError> {
    let start = std::time::Instant::now();
    loop {
        wait_for_gpu(device)?;
        if let Some(result) = slot.lock().ok().and_then(|mut s| s.take()) {
            return result.map_err(|e| {
                RenderError::Output(format!(
                    "GPU readback failed (device lost or out of memory): {e}"
                ))
            });
        }
        if start.elapsed() > forge_render_api::GPU_WAIT_TIMEOUT {
            return Err(RenderError::Output(
                "GPU readback did not complete (device lost)".into(),
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
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
        Ok(GpuEngine {
            device,
            queue,
            submission_budget: SUBMISSION_BUDGET,
        })
    }

    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue) -> GpuEngine {
        GpuEngine {
            device,
            queue,
            submission_budget: SUBMISSION_BUDGET,
        }
    }

    /// Sets the most pixel-samples one submission may hold (at least one row of a tile is
    /// always dispatched). The image does not depend on it (R20.4).
    pub fn with_submission_budget(mut self, pixel_samples: u64) -> GpuEngine {
        self.submission_budget = pixel_samples.max(1);
        self
    }

    /// Uploads each bitmap as an `Rgba32Float` texture (REALTIME.md §2.8, RENDER.md R20.1b).
    fn bitmap_views(
        &self,
        resources: &[Arc<dyn std::any::Any + Send + Sync>],
    ) -> Result<Vec<wgpu::TextureView>, RenderError> {
        resources
            .iter()
            .map(|r| {
                let img = r.downcast_ref::<Image>().ok_or_else(|| {
                    RenderError::Unsupported("a bound resource is not an image".into())
                })?;
                let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("bitmap"),
                    size: wgpu::Extent3d {
                        width: img.width,
                        height: img.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba32Float,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                    view_formats: &[],
                });
                self.queue.write_texture(
                    tex.as_image_copy(),
                    bytemuck::cast_slice(&img.data),
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(img.width * 16),
                        rows_per_image: Some(img.height),
                    },
                    wgpu::Extent3d {
                        width: img.width,
                        height: img.height,
                        depth_or_array_layers: 1,
                    },
                );
                Ok(tex.create_view(&Default::default()))
            })
            .collect()
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

        let materials: Vec<(
            &forge_material::ShadingGraph,
            &dyn forge_material::TextureSampler,
        )> = snapshot
            .materials
            .iter()
            .map(|m| (&m.graph, &*m.sampler as &dyn forge_material::TextureSampler))
            .collect();
        let assembled = crate::shader::assemble(&materials)?;
        let source = assembled.source;

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
        if assembled.resources.len()
            > device.limits().max_sampled_textures_per_shader_stage as usize
        {
            return Err(RenderError::Unsupported(format!(
                "{} bitmaps exceed the device's sampled texture limit (RENDER.md R20.1b)",
                assembled.resources.len()
            )));
        }
        let layout1 = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bitmaps"),
            entries: &(0..assembled.resources.len() as u32)
                .map(|binding| wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                })
                .collect::<Vec<_>>(),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("path tracer"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[Some(&layout), Some(&layout1)],
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

        let views = self.bitmap_views(&assembled.resources)?;
        let bind1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bitmaps"),
            layout: &layout1,
            entries: &views
                .iter()
                .enumerate()
                .map(|(i, v)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: wgpu::BindingResource::TextureView(v),
                })
                .collect::<Vec<_>>(),
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
            let bytes = tw as u64 * th as u64 * stride as u64 * 16;
            // One submission per band of rows: pixels x samples of a band stay under the
            // budget, so a slow device's watchdog never sees a long dispatch (R20.4).
            let rows_per_band = (self.submission_budget
                / (settings.samples.max(1) as u64 * tw as u64))
                .max(1) as u32;
            let mut row = 0u32;
            while row < th {
                if control.is_cancelled() {
                    return Err(RenderError::Cancelled);
                }
                let rows = rows_per_band.min(th - row);
                let mut frame = frame;
                frame[frame::TY] = (ty + row) as f32;
                frame[frame::TILE_H] = rows as f32;
                self.queue.write_buffer(&frame_buf, 0, &frame_bytes(&frame));
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
                    pass.set_bind_group(1, &bind1, &[]);
                    pass.dispatch_workgroups(tw.div_ceil(8), rows.div_ceil(8), 1);
                }
                let band_bytes = tw as u64 * rows as u64 * stride as u64 * 16;
                let band_at = tw as u64 * row as u64 * stride as u64 * 16;
                encoder.copy_buffer_to_buffer(&out, 0, &staging, band_at, band_bytes);
                self.queue.submit([encoder.finish()]);
                wait_for_gpu(device)?;
                row += rows;
            }
            let slice = staging.slice(..bytes);
            let mapped: MapSlot = Arc::new(Mutex::new(None));
            let sink = Arc::clone(&mapped);
            slice.map_async(wgpu::MapMode::Read, move |r| {
                if let Ok(mut slot) = sink.lock() {
                    *slot = Some(r);
                }
            });
            await_mapped(device, &mapped)?;
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
