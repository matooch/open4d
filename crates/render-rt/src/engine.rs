//! The raster engine: resources, passes, readback and outputs (docs/specs/RENDER.md R21).

use std::collections::BTreeMap;

use bytemuck::{Pod, Zeroable};
use forge_image::{AlphaMode, Image};
use forge_material::TextureSampler;
use forge_material::ir::ShadingGraph;
use forge_render_api::{
    Aov, Background, Control, Lighting, Projection, RenderEngine, RenderError, RenderEvent,
    RenderOutput, RenderSettings, RenderStats, Snapshot,
};
use forge_render_gpu::pack::Packed;
use glam::{DVec3, Mat4};
use wgpu::util::DeviceExt;

use crate::frame;
use crate::framegraph::FrameGraph;
use crate::shadow::{self, ATLAS_X, ATLAS_Y, ShadowPlan, TILE_PX};

/// One shadow map descriptor, 416 bytes (R21.4).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ShadowGpu {
    /// Six matrices of four columns each: tile 0 (or the cube's six faces).
    m: [[f32; 4]; 24],
    /// Light index, cube, tile, soft.
    params: [u32; 4],
    /// Depth bias in `x`.
    bias: [f32; 4],
}

impl ShadowGpu {
    fn new(plan: &ShadowPlan) -> ShadowGpu {
        let mut m = [[0.0f32; 4]; 24];
        for (t, (_, mat)) in plan.tiles.iter().enumerate() {
            for c in 0..4 {
                m[t * 4 + c].copy_from_slice(&mat[c * 4..c * 4 + 4]);
            }
        }
        ShadowGpu {
            m,
            params: [
                plan.light as u32,
                u32::from(plan.cube),
                plan.tiles.first().map(|t| t.0).unwrap_or(0),
                u32::from(plan.soft),
            ],
            bias: [2e-3, 0.0, 0.0, 0.0],
        }
    }
}

/// A wgpu real-time raster engine. Create with [`RtEngine::new`], or share a device with
/// [`RtEngine::with_device`].
#[derive(Clone)]
pub struct RtEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
}

/// One frame's targets (R21.3). Two G-buffer targets keep the pass inside Metal's 32-byte
/// per-sample colour-attachment limit; the light split gets its own pass when a pass is asked
/// for.
struct Targets {
    g: Vec<wgpu::Texture>,
    g_views: Vec<wgpu::TextureView>,
    _depth: wgpu::Texture,
    depth_view: wgpu::TextureView,
    beauty: wgpu::Texture,
    beauty_view: wgpu::TextureView,
    /// The screen-space reflection pass's output (R22.3), read back in place of `beauty`.
    beauty_ssr: wgpu::Texture,
    beauty_ssr_view: wgpu::TextureView,
    shadow: wgpu::Texture,
    shadow_view: wgpu::TextureView,
    split: Vec<wgpu::Texture>,
    split_views: Vec<wgpu::TextureView>,
    albedo: wgpu::Texture,
    albedo_view: wgpu::TextureView,
}

impl RtEngine {
    /// The default adapter; `Err` carries the reason there is none.
    pub fn new() -> Result<RtEngine, RenderError> {
        let (device, queue) = crate::request_default_device()
            .map_err(|e| RenderError::Unsupported(format!("no GPU adapter: {e}")))?;
        Ok(RtEngine { device, queue })
    }

    pub fn with_device(device: wgpu::Device, queue: wgpu::Queue) -> RtEngine {
        RtEngine { device, queue }
    }

    /// Uploads each bitmap as an `Rgba32Float` texture, the way the path tracer does
    /// (REALTIME.md §2.8, RENDER.md R20.1b, R21.3).
    fn bitmap_views(
        &self,
        resources: &[std::sync::Arc<dyn std::any::Any + Send + Sync>],
    ) -> Result<Vec<wgpu::TextureView>, RenderError> {
        resources
            .iter()
            .map(|r| {
                let img = r.downcast_ref::<Image>().ok_or_else(|| {
                    RenderError::Unsupported("a bound resource is not an image".into())
                })?;
                let size = wgpu::Extent3d {
                    width: img.width,
                    height: img.height,
                    depth_or_array_layers: 1,
                };
                let tex = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("bitmap"),
                    size,
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
                    size,
                );
                Ok(tex.create_view(&Default::default()))
            })
            .collect()
    }

    fn targets(&self, w: u32, h: u32) -> Targets {
        let make = |label: &str, format: wgpu::TextureFormat| {
            self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            })
        };
        let mut g = Vec::new();
        let mut g_views = Vec::new();
        for i in 0..2 {
            let t = make(&format!("g{i}"), wgpu::TextureFormat::Rgba32Float);
            g_views.push(t.create_view(&Default::default()));
            g.push(t);
        }
        let depth = make("depth", wgpu::TextureFormat::Depth32Float);
        let depth_view = depth.create_view(&Default::default());
        let beauty = make("beauty", wgpu::TextureFormat::Rgba32Float);
        let beauty_view = beauty.create_view(&Default::default());
        let beauty_ssr = make("beauty_ssr", wgpu::TextureFormat::Rgba32Float);
        let beauty_ssr_view = beauty_ssr.create_view(&Default::default());
        let shadow = make("shadow", wgpu::TextureFormat::Rgba32Float);
        let shadow_view = shadow.create_view(&Default::default());
        let mut split = Vec::new();
        let mut split_views = Vec::new();
        for name in ["diffuse", "specular"] {
            let t = make(name, wgpu::TextureFormat::Rgba32Float);
            split_views.push(t.create_view(&Default::default()));
            split.push(t);
        }
        let albedo = make("albedo", wgpu::TextureFormat::Rgba32Float);
        let albedo_view = albedo.create_view(&Default::default());
        Targets {
            g,
            g_views,
            _depth: depth,
            depth_view,
            beauty,
            beauty_view,
            beauty_ssr,
            beauty_ssr_view,
            shadow,
            shadow_view,
            split,
            split_views,
            albedo,
            albedo_view,
        }
    }
}

/// The frame uniform: one scalar in each row's `x` (R21.5).
fn frame_bytes(values: &[f32; frame::SLOTS]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(frame::SLOTS * 16);
    for v in values {
        bytes.extend_from_slice(&v.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 12]);
    }
    bytes
}

fn m4_slots(values: &mut [f32; frame::SLOTS], base: usize, m: &Mat4) {
    let c = m.to_cols_array();
    values[base..base + 16].copy_from_slice(&c);
}

/// The frame uniform base for this render (camera, background, environment, counts).
fn frame_base(
    snapshot: &Snapshot,
    settings: &RenderSettings,
    packed: &Packed,
    shadow_count: usize,
) -> [f32; frame::SLOTS] {
    use frame as f;
    let mut v = [0.0f32; f::SLOTS];
    let (w, h) = (settings.width, settings.height);
    let [rx0, ry0, rx1, ry1] = settings.region.unwrap_or([0, 0, w, h]);
    v[f::W] = w as f32;
    v[f::H] = h as f32;
    v[f::RX0] = rx0 as f32;
    v[f::RY0] = ry0 as f32;
    v[f::RX1] = rx1 as f32;
    v[f::RY1] = ry1 as f32;
    let scale = packed.scale.max(1e-6);
    v[f::SCALE] = scale as f32;
    v[f::LIGHT_COUNT] = packed.light_count as f32;
    v[f::SHADOW_COUNT] = shadow_count as f32;
    v[f::TILE_PX] = TILE_PX as f32;
    v[f::ATLAS_X] = ATLAS_X as f32;
    v[f::ATLAS_Y] = ATLAS_Y as f32;
    let (kind, focal, sensor, off, ortho) = match snapshot.camera.projection {
        Projection::Perspective {
            focal_mm,
            sensor_mm,
            film_offset,
        } => (0.0, focal_mm, sensor_mm, film_offset, 0.0),
        Projection::Orthographic { width } => (1.0, 0.0, 0.0, [0.0; 2], width),
    };
    v[f::PROJ_KIND] = kind;
    v[f::FOCAL] = focal as f32;
    v[f::SENSOR] = sensor as f32;
    v[f::FILM_OFF_X] = off[0] as f32;
    v[f::FILM_OFF_Y] = off[1] as f32;
    v[f::ORTHO_W] = ortho as f32;
    v[f::NEAR] = (1e-4 * scale) as f32;
    v[f::FAR] = (100.0 * scale) as f32;
    let cam32 = Mat4::from_cols_array(&snapshot.camera.transform.map(|x| x as f32));
    m4_slots(&mut v, f::CAM, &cam32);
    m4_slots(&mut v, f::VIEW, &cam32.inverse());
    let eye = snapshot.camera.position();
    v[f::CAM_POS] = eye.x as f32;
    v[f::CAM_POS + 1] = eye.y as f32;
    v[f::CAM_POS + 2] = eye.z as f32;
    match snapshot.environment.background {
        Background::Color(c) => {
            v[f::BG_KIND] = 0.0;
            v[f::BG_X] = c[0] as f32;
            v[f::BG_Y] = c[1] as f32;
            v[f::BG_Z] = c[2] as f32;
        }
        Background::Environment => v[f::BG_KIND] = 1.0,
    }
    match &snapshot.environment.lighting {
        Lighting::None => {}
        Lighting::Constant { color, intensity } => {
            v[f::LIGHTING_KIND] = 1.0;
            v[f::LIGHTING_INTENSITY] = *intensity as f32;
            v[f::CONST] = color[0] as f32;
            v[f::CONST + 1] = color[1] as f32;
            v[f::CONST + 2] = color[2] as f32;
        }
        Lighting::Sky {
            zenith,
            horizon,
            ground,
            intensity,
        } => {
            v[f::LIGHTING_KIND] = 2.0;
            v[f::LIGHTING_INTENSITY] = *intensity as f32;
            for (k, c) in [zenith, horizon, ground].iter().enumerate() {
                let base = f::SKY_ZEN + k * 3;
                v[base] = c[0] as f32;
                v[base + 1] = c[1] as f32;
                v[base + 2] = c[2] as f32;
            }
        }
        Lighting::Image {
            image,
            rotation,
            intensity,
        } => {
            v[f::LIGHTING_KIND] = 3.0;
            v[f::LIGHTING_INTENSITY] = *intensity as f32;
            v[f::ENV_PRESENT] = 1.0;
            v[f::ENV_ROT] = *rotation as f32;
            v[f::ENV_W] = image.width as f32;
            v[f::ENV_H] = image.height as f32;
        }
    }
    v[f::EPS] = (scale * 1e-6) as f32;
    // R22: the post toggles the lighting and reflection passes read (the jitter is added per
    // TAA subframe by `render_with`).
    let post = settings.post;
    v[f::SSAO_ON] = if post.ssao { 1.0 } else { 0.0 };
    v[f::SSAO_RADIUS] = post.ssao_radius as f32;
    v[f::SSAO_STRENGTH] = post.ssao_strength as f32;
    v[f::SSR_ON] = if post.ssr { 1.0 } else { 0.0 };
    v[f::SSR_STEPS] = post.ssr_steps as f32;
    v
}

/// The passes this engine registers, in order (R21.1).
fn frame_pass_graph(shadow_count: usize, split: bool, albedo: bool, ssr: bool) -> FrameGraph {
    let mut g = FrameGraph::new();
    g.input("tris");
    let names: Vec<String> = (0..shadow_count).map(|i| format!("shadow:{i}")).collect();
    for name in &names {
        g.add(name, &["tris"], &[name.as_str()]);
    }
    g.add("gbuffer", &["tris"], &["g0", "g1", "depth"]);
    let mut reads = vec!["g0", "g1", "depth"];
    reads.extend(names.iter().map(|s| s.as_str()));
    g.add("lighting", &reads, &["beauty", "shadow"]);
    if ssr {
        // R22.3: the reflections read the lit beauty and write its own target
        g.add("ssr", &["beauty", "g0", "g1"], &["beauty_ssr"]);
    }
    if split {
        g.add("split", &["g0", "g1"], &["diffuse", "specular"]);
    }
    if albedo {
        g.add("albedo", &["g0", "g1"], &["albedo"]);
    }
    g
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn texture_entry(binding: u32, depth: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: if depth {
                wgpu::TextureSampleType::Depth
            } else {
                // every colour target is Rgba32Float, which is not filterable
                wgpu::TextureSampleType::Float { filterable: false }
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn sampler_entry(binding: u32, comparison: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(if comparison {
            wgpu::SamplerBindingType::Comparison
        } else {
            wgpu::SamplerBindingType::NonFiltering
        }),
        count: None,
    }
}

impl RenderEngine for RtEngine {
    fn id(&self) -> &str {
        "forge.render.rt"
    }

    fn render(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
    ) -> Result<RenderOutput, RenderError> {
        settings.validate()?;
        // R22.5: TAA averages `taa_samples` jittered subframes in linear radiance and takes
        // the display effects once, on the mean; a subframe keeps its own events.
        if settings.post.taa && settings.post.taa_samples > 1 {
            let mut sub = settings.clone();
            sub.post.taa = false;
            sub.post.bloom = false;
            sub.post.tonemap = false;
            let n = settings.post.taa_samples as usize;
            let mut acc: Option<Vec<[f32; 4]>> = None;
            let mut last: Option<RenderOutput> = None;
            for j in 0..n {
                if control.is_cancelled() {
                    return Err(RenderError::Cancelled);
                }
                let jitter = crate::post::halton_jitter(j as u32 + 1);
                let out = self.render_with(snapshot, &sub, control, jitter)?;
                let pixels: Vec<[f32; 4]> = out.beauty.data.as_chunks::<4>().0.to_vec();
                crate::post::accumulate(&mut acc, &pixels, n);
                last = Some(out);
            }
            let mut out = last.expect("at least one subframe");
            if let Some(mut mean) = acc {
                if settings.post.bloom {
                    crate::post::bloom(&mut mean, settings.width, settings.height, &settings.post);
                }
                if settings.post.tonemap {
                    crate::post::tonemap_image(&mut mean, &settings.post);
                }
                out.beauty.data = mean.iter().flat_map(|p| p.iter().copied()).collect();
            }
            return Ok(out);
        }
        self.render_with(snapshot, settings, control, [0.0, 0.0])
    }
}

impl RtEngine {
    fn render_with(
        &self,
        snapshot: &Snapshot,
        settings: &RenderSettings,
        control: &Control,
        jitter: [f32; 2],
    ) -> Result<RenderOutput, RenderError> {
        settings.validate()?;
        snapshot.validate()?;
        if settings.depth_of_field {
            return Err(RenderError::Unsupported(
                "depth of field in the raster tier (RENDER.md R21.6)".into(),
            ));
        }
        let packed = Packed::build(snapshot, settings.default_light)?;
        let materials: Vec<(&ShadingGraph, &dyn TextureSampler)> = snapshot
            .materials
            .iter()
            .map(|m| (&m.graph, &*m.sampler as &dyn TextureSampler))
            .collect();
        let assembled = crate::shader::assemble(&materials)?;
        // R21.3: the lighting pass samples five textures of its own besides the bitmaps
        if assembled.resources.len() + 5
            > self.device.limits().max_sampled_textures_per_shader_stage as usize
        {
            return Err(RenderError::Unsupported(format!(
                "{} bitmaps exceed the device's sampled texture limit (RENDER.md R21.3)",
                assembled.resources.len()
            )));
        }
        let bitmap_views = self.bitmap_views(&assembled.resources)?;
        let source = assembled.source;
        let plans = shadow::plans(snapshot, &packed)?;
        let split = settings
            .aovs
            .iter()
            .any(|a| matches!(a, Aov::Diffuse | Aov::Specular | Aov::Emission));
        let albedo = settings.aovs.contains(&Aov::Albedo);
        let graph = frame_pass_graph(plans.len(), split, albedo, settings.post.ssr);
        graph
            .validate()
            .map_err(|e| RenderError::Settings(format!("frame graph: {e}")))?;

        let device = &self.device;
        let queue = &self.queue;
        let (w, h) = (settings.width, settings.height);
        let [rx0, ry0, rx1, ry1] = settings.region.unwrap_or([0, 0, w, h]);
        let targets = self.targets(w, h);

        // shadow atlas and the per-light descriptors
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_X * TILE_PX,
                height: ATLAS_Y * TILE_PX,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let atlas_view = atlas.create_view(&Default::default());
        let mut shadow_gpu: Vec<ShadowGpu> = plans.iter().map(ShadowGpu::new).collect();
        if shadow_gpu.is_empty() {
            shadow_gpu.push(ShadowGpu::zeroed());
        }
        let shadow_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shadows"),
            contents: bytemuck::cast_slice(&shadow_gpu),
            usage: wgpu::BufferUsages::STORAGE,
        });

        // environment image (nearest lat-long sampling, R16)
        let env_image = match &snapshot.environment.lighting {
            Lighting::Image { image, .. } => Some(image.clone()),
            _ => None,
        };
        let (env_w, env_h) = env_image
            .as_ref()
            .map(|i| (i.width.max(1), i.height.max(1)))
            .unwrap_or((1, 1));
        let env_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("environment"),
            size: wgpu::Extent3d {
                width: env_w,
                height: env_h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let env_data: Vec<f32> = match &env_image {
            Some(img) => {
                let mut d = img.data.clone();
                d.resize((env_w * env_h * 4) as usize, 1.0);
                d
            }
            None => vec![1.0; 4],
        };
        let env_bytes: Vec<u8> = env_data.iter().flat_map(|v| v.to_le_bytes()).collect();
        queue.write_texture(
            env_tex.as_image_copy(),
            &env_bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(env_w * 16),
                rows_per_image: Some(env_h),
            },
            wgpu::Extent3d {
                width: env_w,
                height: env_h,
                depth_or_array_layers: 1,
            },
        );
        let env_view = env_tex.create_view(&Default::default());
        let env_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("environment"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });

        let errors = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("raster"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let geometry_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("geometry"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
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
            ],
        });
        let lighting_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("lighting"),
            entries: &[
                texture_entry(0, false),
                texture_entry(1, false),
                texture_entry(2, true),
                sampler_entry(3, true),
                texture_entry(4, false),
                sampler_entry(5, false),
                texture_entry(6, false),
            ],
        });
        // R21.3: the shader-node bitmaps, one sampled texture per binding, in group 2
        let bitmap_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bitmaps"),
            entries: &(0..bitmap_views.len() as u32)
                .map(|binding| texture_entry(binding, false))
                .collect::<Vec<_>>(),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("geometry"),
            bind_group_layouts: &[Some(&geometry_layout)],
            immediate_size: 0,
        });
        let lighting_pipeline = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("lighting"),
            bind_group_layouts: &[
                Some(&geometry_layout),
                Some(&lighting_layout),
                Some(&bitmap_layout),
            ],
            immediate_size: 0,
        });
        let color_target = |format: wgpu::TextureFormat| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        };
        let f = wgpu::TextureFormat::Rgba32Float;
        let gbuffer = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gbuffer"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_gbuffer"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_gbuffer"),
                compilation_options: Default::default(),
                targets: &[color_target(f), color_target(f)],
            }),
            multiview_mask: None,
            cache: None,
        });
        let shadow_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: Default::default(),
                bias: wgpu::DepthBiasState {
                    constant: 2,
                    slope_scale: 2.0,
                    clamp: 0.0,
                },
            }),
            multisample: Default::default(),
            fragment: None,
            multiview_mask: None,
            cache: None,
        });
        let fullscreen = |label: &str, entry: &str| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&lighting_pipeline),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs_full"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[color_target(f), color_target(f)],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let lighting = fullscreen("lighting", "fs_light");
        let split_pass = fullscreen("split", "fs_split");
        // R22.3: one colour target, so its own pipeline descriptor
        let ssr_pass = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ssr"),
            layout: Some(&lighting_pipeline),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_full"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_ssr"),
                compilation_options: Default::default(),
                targets: &[color_target(f)],
            }),
            multiview_mask: None,
            cache: None,
        });
        let albedo_pass = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("albedo"),
            layout: Some(&lighting_pipeline),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_full"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_albedo"),
                compilation_options: Default::default(),
                targets: &[color_target(f)],
            }),
            multiview_mask: None,
            cache: None,
        });
        if let Some(e) = pollster::block_on(errors.pop()) {
            return Err(RenderError::Settings(format!("GPU shader: {e}")));
        }

        let frame_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("frame"),
            size: (frame::SLOTS * 16) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut values = frame_base(snapshot, settings, &packed, plans.len());
        // R22.5: this subframe's subpixel jitter (0 when TAA is off)
        values[frame::JITTER_X] = jitter[0];
        values[frame::JITTER_Y] = jitter[1];
        queue.write_buffer(&frame_buf, 0, &frame_bytes(&values));
        let tris = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("tris"),
            contents: bytemuck::cast_slice(&packed.tris),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let lights = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights"),
            contents: bytemuck::cast_slice(&packed.lights),
            usage: wgpu::BufferUsages::STORAGE,
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("geometry"),
            layout: &geometry_layout,
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
                    resource: lights.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: shadow_buf.as_entire_binding(),
                },
            ],
        });
        let bitmap_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bitmaps"),
            layout: &bitmap_layout,
            entries: &bitmap_views
                .iter()
                .enumerate()
                .map(|(i, v)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: wgpu::BindingResource::TextureView(v),
                })
                .collect::<Vec<_>>(),
        });
        let lit_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("lighting"),
            layout: &lighting_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&targets.g_views[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.g_views[1]),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&env_view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::Sampler(&env_sampler),
                },
                // the lighting pass does not read the beauty source; the SSR pass binds the
                // lit beauty here (R22.3), so both share one bind group layout
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&targets.g_views[0]),
                },
            ],
        });

        control.emit(RenderEvent::Started {
            width: w,
            height: h,
            tiles: 1,
        });

        // shadow tiles: one submit per tile, since the matrix lives in the frame uniform
        let vertex_count = (packed.triangles * 3) as u32;
        for (k, plan) in plans.iter().enumerate() {
            for (t, (tile, matrix)) in plan.tiles.iter().enumerate() {
                if control.is_cancelled() {
                    return Err(RenderError::Cancelled);
                }
                values[frame::SHADOW_MAT..frame::SHADOW_MAT + 16].copy_from_slice(matrix);
                queue.write_buffer(&frame_buf, 0, &frame_bytes(&values));
                let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("shadow"),
                });
                {
                    let first = k == 0 && t == 0;
                    let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("shadow"),
                        color_attachments: &[],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &atlas_view,
                            depth_ops: Some(wgpu::Operations {
                                load: if first {
                                    wgpu::LoadOp::Clear(1.0)
                                } else {
                                    wgpu::LoadOp::Load
                                },
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        timestamp_writes: None,
                        occlusion_query_set: None,
                        multiview_mask: None,
                    });
                    let x = (tile % ATLAS_X) * TILE_PX;
                    let y = (tile / ATLAS_X) * TILE_PX;
                    pass.set_viewport(x as f32, y as f32, TILE_PX as f32, TILE_PX as f32, 0.0, 1.0);
                    pass.set_scissor_rect(x, y, TILE_PX, TILE_PX);
                    pass.set_pipeline(&shadow_pipeline);
                    pass.set_bind_group(0, &bind, &[]);
                    pass.draw(0..vertex_count, 0..1);
                }
                queue.submit([enc.finish()]);
            }
        }

        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        {
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gbuffer"),
            });
            let clear = wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            };

            let attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = targets
                .g_views
                .iter()
                .map(|v| {
                    Some(wgpu::RenderPassColorAttachment {
                        view: v,
                        depth_slice: None,
                        resolve_target: None,
                        ops: clear,
                    })
                })
                .collect();
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gbuffer"),
                color_attachments: &attachments,
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &targets.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_viewport(0.0, 0.0, w as f32, h as f32, 0.0, 1.0);
            pass.set_scissor_rect(rx0, ry0, rx1 - rx0, ry1 - ry0);
            pass.set_pipeline(&gbuffer);
            pass.set_bind_group(0, &bind, &[]);
            pass.draw(0..vertex_count, 0..1);
            drop(pass);
            queue.submit([enc.finish()]);
        }
        if control.is_cancelled() {
            return Err(RenderError::Cancelled);
        }
        let run_full =
            |label: &str, pipeline: &wgpu::RenderPipeline, views: &[&wgpu::TextureView]| {
                let mut enc = device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
                let attachments: Vec<Option<wgpu::RenderPassColorAttachment>> = views
                    .iter()
                    .map(|v| {
                        Some(wgpu::RenderPassColorAttachment {
                            view: v,
                            depth_slice: None,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                                store: wgpu::StoreOp::Store,
                            },
                        })
                    })
                    .collect();
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some(label),
                    color_attachments: &attachments,
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_viewport(0.0, 0.0, w as f32, h as f32, 0.0, 1.0);
                pass.set_scissor_rect(rx0, ry0, rx1 - rx0, ry1 - ry0);
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &bind, &[]);
                pass.set_bind_group(1, &lit_bind, &[]);
                pass.set_bind_group(2, &bitmap_bind, &[]);
                pass.draw(0..3, 0..1);
                drop(pass);
                queue.submit([enc.finish()]);
            };
        run_full(
            "lighting",
            &lighting,
            &[&targets.beauty_view, &targets.shadow_view],
        );
        // R22.3: screen-space reflections, reading the lit beauty into their own target
        let beauty_target = if settings.post.ssr {
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            let ssr_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ssr"),
                layout: &lighting_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&targets.g_views[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&targets.g_views[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&atlas_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&shadow_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&env_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::Sampler(&env_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&targets.beauty_view),
                    },
                ],
            });
            let mut enc = device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("ssr") });
            {
                let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ssr"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &targets.beauty_ssr_view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_viewport(0.0, 0.0, w as f32, h as f32, 0.0, 1.0);
                pass.set_scissor_rect(rx0, ry0, rx1 - rx0, ry1 - ry0);
                pass.set_pipeline(&ssr_pass);
                pass.set_bind_group(0, &bind, &[]);
                pass.set_bind_group(1, &ssr_bind, &[]);
                pass.set_bind_group(2, &bitmap_bind, &[]);
                pass.draw(0..3, 0..1);
            }
            queue.submit([enc.finish()]);
            &targets.beauty_ssr
        } else {
            &targets.beauty
        };
        if split {
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            run_full(
                "split",
                &split_pass,
                &[&targets.split_views[0], &targets.split_views[1]],
            );
        }
        if albedo {
            if control.is_cancelled() {
                return Err(RenderError::Cancelled);
            }
            run_full("albedo", &albedo_pass, &[&targets.albedo_view]);
        }

        // read back the beauty, the G-buffer and (when asked for) the light split
        let mut order: Vec<&wgpu::Texture> =
            vec![beauty_target, &targets.shadow, &targets.g[0], &targets.g[1]];
        if split {
            order.push(&targets.split[0]);
            order.push(&targets.split[1]);
        }
        if albedo {
            order.push(&targets.albedo);
        }
        let row_bytes = w * 16;
        let stride = row_bytes.div_ceil(256) * 256;
        let bytes = stride as u64 * h as u64;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("readback"),
        });
        let mut staging = Vec::new();
        for t in &order {
            let buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("staging"),
                size: bytes,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            enc.copy_texture_to_buffer(
                t.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buf,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(h),
                    },
                },
                wgpu::Extent3d {
                    width: w,
                    height: h,
                    depth_or_array_layers: 1,
                },
            );
            staging.push(buf);
        }
        queue.submit([enc.finish()]);
        let mut pixels: Vec<Vec<[f32; 4]>> = Vec::new();
        for buf in &staging {
            let slice = buf.slice(..);
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback"));
        }
        device
            .poll(wgpu::PollType::Wait {
                submission_index: None,
                timeout: Some(forge_render_api::GPU_WAIT_TIMEOUT),
            })
            .map_err(|e| {
                RenderError::Output(format!(
                    "GPU did not finish within {:?} (hung or too slow): {e}",
                    forge_render_api::GPU_WAIT_TIMEOUT
                ))
            })?;
        for buf in &staging {
            let slice = buf.slice(..);
            let data = slice
                .get_mapped_range()
                .map_err(|e| RenderError::Output(format!("GPU readback: {e}")))?;
            let mut out = Vec::with_capacity((w * h) as usize);
            for y in 0..h as usize {
                let row = &data[y * stride as usize..y * stride as usize + row_bytes as usize];
                for b in row.as_chunks::<16>().0 {
                    let f = |k: usize| {
                        f32::from_le_bytes([b[k * 4], b[k * 4 + 1], b[k * 4 + 2], b[k * 4 + 3]])
                    };
                    out.push([f(0), f(1), f(2), f(3)]);
                }
            }
            pixels.push(out);
            drop(data);
            buf.unmap();
        }
        let beauty_px = &pixels[0];
        let shadow_px = &pixels[1];
        let g0_px = &pixels[2];
        let g1_px = &pixels[3];
        let diffuse_px = if split { Some(&pixels[4]) } else { None };
        let specular_px = if split { Some(&pixels[5]) } else { None };
        let albedo_px = if albedo {
            Some(&pixels[4 + usize::from(split) * 2])
        } else {
            None
        };
        // R22.4/R22.6: bloom and tone mapping on this subframe's linear beauty. The AOVs keep
        // the linear values (they are data, not display colour).
        let linear_px = beauty_px;
        let mut beauty_px = beauty_px.clone();
        if settings.post.bloom {
            crate::post::bloom(&mut beauty_px, w, h, &settings.post);
        }
        if settings.post.tonemap {
            crate::post::tonemap_image(&mut beauty_px, &settings.post);
        }
        let beauty_px = &beauty_px;
        let mut beauty = Image::new(w, h, AlphaMode::Premultiplied);
        let mut aov_imgs: BTreeMap<Aov, Image> = settings
            .aovs
            .iter()
            .map(|a| (*a, Image::new(w, h, AlphaMode::Straight)))
            .collect();
        let eye = snapshot.camera.position();
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) as usize;
                beauty.set_pixel(x, y, beauty_px[i]);
                if aov_imgs.is_empty() {
                    continue;
                }
                let g0 = g0_px[i];
                let g1 = g1_px[i];
                let hit = g0[3] > 0.5;
                let alpha = if hit { 1.0 } else { 0.0 };
                let p = [g0[0], g0[1], g0[2]];
                for (a, img) in aov_imgs.iter_mut() {
                    let v = match a {
                        Aov::Depth => {
                            let d = if hit {
                                (DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64) - eye).length()
                            } else {
                                0.0
                            };
                            [d as f32, d as f32, d as f32, alpha]
                        }
                        Aov::Normal => [g1[0], g1[1], g1[2], alpha],
                        Aov::Albedo => albedo_px.map(|d| d[i]).unwrap_or([0.0; 4]),
                        Aov::Position => [p[0], p[1], p[2], alpha],
                        Aov::ObjectId => {
                            // g1.w is the triangle index + 1 (R21.3)
                            let id = if hit {
                                packed.tris[g1[3] as usize - 1].ids[1] + 1.0
                            } else {
                                0.0
                            };
                            [id, id, id, alpha]
                        }
                        Aov::MaterialId => {
                            let id = g0[3];
                            [id, id, id, alpha]
                        }
                        Aov::Shadow => [shadow_px[i][0], 1.0, 1.0, alpha],
                        Aov::Diffuse => diffuse_px.map(|d| d[i]).unwrap_or([0.0; 4]),
                        Aov::Specular => specular_px.map(|d| d[i]).unwrap_or([0.0; 4]),
                        Aov::Emission => {
                            let d = diffuse_px.map(|d| d[i]).unwrap_or([0.0; 4]);
                            let s = specular_px.map(|d| d[i]).unwrap_or([0.0; 4]);
                            let b = linear_px[i];
                            [
                                (b[0] - d[0] - s[0]).max(0.0),
                                (b[1] - d[1] - s[1]).max(0.0),
                                (b[2] - d[2] - s[2]).max(0.0),
                                alpha,
                            ]
                        }
                    };
                    img.set_pixel(x, y, v);
                }
            }
        }
        control.emit(RenderEvent::Tile {
            x: rx0,
            y: ry0,
            width: rx1 - rx0,
            height: ry1 - ry0,
            rgba: beauty_px
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    let (x, y) = ((*i as u32) % w, (*i as u32) / w);
                    x >= rx0 && x < rx1 && y >= ry0 && y < ry1
                })
                .flat_map(|(_, p)| *p)
                .collect(),
        });
        control.emit(RenderEvent::Progress { done: 1, total: 1 });
        control.emit(RenderEvent::Finished);
        Ok(RenderOutput {
            width: w,
            height: h,
            beauty,
            aovs: aov_imgs,
            stats: RenderStats {
                triangles: packed.triangles,
                instances: packed.instances,
                tiles: 1,
                samples: (rx1 - rx0) as u64 * (ry1 - ry0) as u64,
                ignored_channels: forge_render_cpu::ignored_channels(snapshot),
            },
        })
    }
}
