//! wgpu renderer: shaded/wire meshes, grid, gizmo; offscreen readback
//! (docs/specs/VIEWPORT.md §6).

use std::f64::consts::PI;

use bytemuck::{Pod, Zeroable};
use forge_geometry::PolyMesh;
use forge_geometry::normals::{Weighting, corner_normals, poly_normal};
use forge_geometry::selection::Selection;
use forge_geometry::triangulate::triangulate;
use std::sync::{Mutex, OnceLock};

use glam::{DMat4, DVec3};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::gizmo::{Axis, Gizmo, GizmoMode, Plane};
use crate::grid::{Grid, LineKind};
use crate::lights::{self, AMBIENT, Light, LightKind, Lighting, MAX_LIGHTS};

const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
/// NDC depth offset applied to edges drawn over shaded faces.
const OVERLAY_BIAS: f32 = 1e-5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuError(pub String);

impl std::fmt::Display for GpuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for GpuError {}

/// Device, queue and an offscreen renderer.
pub struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub adapter_info: wgpu::AdapterInfo,
    renderer: Renderer,
    /// Built on first use: only needed when a frame asks for MSAA.
    renderer_msaa: OnceLock<Renderer>,
}

impl Gpu {
    /// Any available adapter (software fallbacks included), no surface.
    pub fn new_headless() -> Result<Gpu, GpuError> {
        let instance =
            wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle_from_env());
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .map_err(|e| GpuError(format!("no GPU adapter: {e}")))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|e| GpuError(format!("no GPU device: {e}")))?;
        let renderer = Renderer::new(&device, COLOR_FORMAT);
        Ok(Gpu {
            device,
            queue,
            adapter_info: adapter.get_info(),
            renderer,
            renderer_msaa: OnceLock::new(),
        })
    }

    /// Whether the adapter is a CPU rasteriser (lavapipe, WARP): baseline images use their
    /// tight tolerance only there (docs/specs/VIEWPORT.md §8.5).
    pub fn is_software(&self) -> bool {
        self.adapter_info.device_type == wgpu::DeviceType::Cpu
    }

    fn renderer_for(&self, msaa: Msaa) -> &Renderer {
        match msaa {
            Msaa::Off => &self.renderer,
            Msaa::X4 => self
                .renderer_msaa
                .get_or_init(|| Renderer::with_samples(&self.device, COLOR_FORMAT, msaa.samples())),
        }
    }
}

// ---------------------------------------------------------------------------
// Mesh buffers
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct TriVertex {
    pos: [f32; 3],
    flat_n: [f32; 3],
    smooth_n: [f32; 3],
    selected: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct EdgeVertex {
    pos: [f32; 3],
    selected: f32,
}

/// One corner of a screen-space point quad: `corner` is (+-1, +-1), scaled to pixels in
/// the vertex shader.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct PointVertex {
    pos: [f32; 3],
    selected: f32,
    corner: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ColorVertex {
    pos: [f32; 3],
    color: [f32; 3],
}

/// Box of the points at f32 precision (what the GPU draws).
fn point_bounds(points: &[DVec3]) -> Option<(DVec3, DVec3)> {
    let first = points.first()?.as_vec3().as_dvec3();
    Some(points.iter().fold((first, first), |(lo, hi), p| {
        let p = p.as_vec3().as_dvec3();
        (lo.min(p), hi.max(p))
    }))
}

fn f3(v: DVec3) -> [f32; 3] {
    v.as_vec3().to_array()
}

fn flag(b: bool) -> f32 {
    if b { 1.0 } else { 0.0 }
}

/// A vertex buffer and its vertex count; `None` when empty.
struct Vertices(Option<(wgpu::Buffer, u32)>);

impl Vertices {
    fn new<T: Pod>(device: &wgpu::Device, label: &str, data: &[T]) -> Vertices {
        if data.is_empty() {
            return Vertices(None);
        }
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Vertices(Some((buf, data.len() as u32)))
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        if let Some((buf, n)) = &self.0 {
            pass.set_vertex_buffer(0, buf.slice(..));
            pass.draw(0..*n, 0..1);
        }
    }
}

/// GPU copy of a polygon mesh: triangles, edges and points with selection flags.
pub struct MeshBuffers {
    tris: Vertices,
    edges: Vertices,
    points: Vertices,
    /// Two triangles per point, for `Settings::point_size`.
    point_quads: Vertices,
    bounds: Option<(DVec3, DVec3)>,
}

impl MeshBuffers {
    /// Axis-aligned box of the mesh's points; `None` for a mesh without points.
    pub fn bounds(&self) -> Option<(DVec3, DVec3)> {
        self.bounds
    }
}

impl MeshBuffers {
    /// Fully smooth normals; see [`MeshBuffers::with_device`] for a smoothing angle.
    pub fn new(gpu: &Gpu, mesh: &PolyMesh, selection: &Selection) -> MeshBuffers {
        Self::with_device(&gpu.device, mesh, selection, Some(PI))
    }

    /// For any wgpu device (e.g. an app's own). `smoothing`: the smoothing angle of the
    /// "smooth" normals, `None` for faceted (docs/specs/SMOOTHING.md §3).
    pub fn with_device(
        d: &wgpu::Device,
        mesh: &PolyMesh,
        selection: &Selection,
        smoothing: Option<f64>,
    ) -> MeshBuffers {
        Self::build(d, mesh, selection, smoothing, true)
    }

    /// As [`MeshBuffers::with_device`] without point buffers (the points are never drawn).
    pub fn with_device_no_points(
        d: &wgpu::Device,
        mesh: &PolyMesh,
        selection: &Selection,
        smoothing: Option<f64>,
    ) -> MeshBuffers {
        Self::build(d, mesh, selection, smoothing, false)
    }

    fn build(
        d: &wgpu::Device,
        mesh: &PolyMesh,
        selection: &Selection,
        smoothing: Option<f64>,
        with_points: bool,
    ) -> MeshBuffers {
        let smooth = match smoothing {
            Some(angle) => corner_normals(mesh, angle, Weighting::Angle),
            None => mesh
                .polys
                .iter()
                .map(|p| vec![poly_normal(&mesh.points, p); p.len()])
                .collect(),
        };
        let flat: Vec<DVec3> = mesh
            .polys
            .iter()
            .map(|p| poly_normal(&mesh.points, p))
            .collect();
        let mut tris = Vec::new();
        for (f, tri) in triangulate(mesh) {
            let poly = &mesh.polys[f as usize];
            let selected = flag(selection.polys.contains(f));
            for v in tri {
                let corner = poly
                    .iter()
                    .position(|c| *c == v)
                    .expect("triangle vertex in polygon");
                tris.push(TriVertex {
                    pos: f3(mesh.points[v as usize]),
                    flat_n: f3(flat[f as usize]),
                    smooth_n: f3(smooth[f as usize][corner]),
                    selected,
                });
            }
        }
        let edges: Vec<EdgeVertex> = mesh
            .edges()
            .iter()
            .enumerate()
            .flat_map(|(id, e)| {
                let selected = flag(selection.edges.contains(id as u32));
                [e.0, e.1].map(|v| EdgeVertex {
                    pos: f3(mesh.points[v as usize]),
                    selected,
                })
            })
            .collect();
        let (mut points, mut quads) = (Vec::new(), Vec::new());
        if with_points {
            const CORNERS: [[f32; 2]; 6] = [
                [-1.0, -1.0],
                [1.0, -1.0],
                [1.0, 1.0],
                [-1.0, -1.0],
                [1.0, 1.0],
                [-1.0, 1.0],
            ];
            for (i, p) in mesh.points.iter().enumerate() {
                let (pos, selected) = (f3(*p), flag(selection.points.contains(i as u32)));
                points.push(EdgeVertex { pos, selected });
                quads.extend(CORNERS.map(|corner| PointVertex {
                    pos,
                    selected,
                    corner,
                }));
            }
        }
        MeshBuffers {
            tris: Vertices::new(d, "mesh tris", &tris),
            edges: Vertices::new(d, "mesh edges", &edges),
            points: Vertices::new(d, "mesh points", &points),
            point_quads: Vertices::new(d, "mesh point quads", &quads),
            bounds: point_bounds(&mesh.points),
        }
    }
}

// ---------------------------------------------------------------------------
// Frame description
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shading {
    #[default]
    Smooth,
    Flat,
    Wire,
    SmoothWire,
    FlatWire,
    /// Smooth, headlight only, no shadows (docs/specs/VIEWPORT.md §8.3; parity-pending G8).
    Quick,
    /// Plain base colour, no lighting.
    Constant,
    /// Faces in the background colour plus edges: hidden edges stay hidden.
    HiddenLine,
    /// The bounding box of each mesh.
    BoundingBox,
}

/// What a [`Shading`] draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Style {
    fill: bool,
    smooth: bool,
    wire: bool,
    /// Faces drawn without lighting.
    unlit: bool,
    /// Faces drawn in the background colour.
    hidden: bool,
    /// Headlight only, no shadows.
    quick: bool,
    boxes: bool,
}

impl Shading {
    /// Every mode, in the order of the number keys.
    pub const ALL: [Shading; 9] = [
        Shading::Smooth,
        Shading::Flat,
        Shading::Wire,
        Shading::SmoothWire,
        Shading::FlatWire,
        Shading::Quick,
        Shading::Constant,
        Shading::HiddenLine,
        Shading::BoundingBox,
    ];

    fn style(self) -> Style {
        let base = Style {
            fill: false,
            smooth: false,
            wire: false,
            unlit: false,
            hidden: false,
            quick: false,
            boxes: false,
        };
        match self {
            Shading::Smooth => Style {
                fill: true,
                smooth: true,
                ..base
            },
            Shading::Flat => Style { fill: true, ..base },
            Shading::Wire => Style { wire: true, ..base },
            Shading::SmoothWire => Style {
                fill: true,
                smooth: true,
                wire: true,
                ..base
            },
            Shading::FlatWire => Style {
                fill: true,
                wire: true,
                ..base
            },
            Shading::Quick => Style {
                fill: true,
                smooth: true,
                quick: true,
                ..base
            },
            Shading::Constant => Style {
                fill: true,
                unlit: true,
                ..base
            },
            Shading::HiddenLine => Style {
                fill: true,
                unlit: true,
                hidden: true,
                wire: true,
                ..base
            },
            Shading::BoundingBox => Style {
                boxes: true,
                ..base
            },
        }
    }
}

/// Multisample anti-aliasing (docs/specs/VIEWPORT.md §8.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Msaa {
    #[default]
    Off,
    X4,
}

impl Msaa {
    pub fn samples(self) -> u32 {
        match self {
            Msaa::Off => 1,
            Msaa::X4 => 4,
        }
    }
}

/// Linear RGB colours (our own palette; parity-pending).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub background: [f32; 3],
    pub object: [f32; 3],
    pub wire: [f32; 3],
    pub selection: [f32; 3],
    pub grid_minor: [f32; 3],
    pub grid_major: [f32; 3],
    pub axis_x: [f32; 3],
    pub axis_z: [f32; 3],
    pub gizmo_x: [f32; 3],
    pub gizmo_y: [f32; 3],
    pub gizmo_z: [f32; 3],
    pub gizmo_center: [f32; 3],
}

impl Default for Palette {
    fn default() -> Self {
        Palette {
            background: [0.16, 0.17, 0.19],
            object: [0.70, 0.70, 0.70],
            wire: [0.05, 0.05, 0.06],
            selection: [1.00, 0.55, 0.10],
            grid_minor: [0.22, 0.23, 0.25],
            grid_major: [0.30, 0.31, 0.34],
            axis_x: [0.55, 0.20, 0.20],
            axis_z: [0.20, 0.25, 0.55],
            gizmo_x: [0.90, 0.20, 0.20],
            gizmo_y: [0.25, 0.85, 0.25],
            gizmo_z: [0.25, 0.40, 0.95],
            gizmo_center: [0.95, 0.95, 0.80],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub shading: Shading,
    pub show_points: bool,
    /// Side of a point in pixels; 0 draws 1 px points (docs/specs/VIEWPORT.md §6).
    pub point_size: f32,
    pub palette: Palette,
    /// The lights (docs/specs/VIEWPORT.md §8.1); the headlight by default.
    pub lighting: Lighting,
    /// Shadow maps for shadow-casting lights (§8.2).
    pub shadows: bool,
    /// Shadow map side in texels, clamped to 64..=4096.
    pub shadow_size: u32,
    /// Used by [`render_to_rgba`]; direct [`Renderer::render`] callers pick the sample count
    /// with their renderer and targets (§8.4).
    pub msaa: Msaa,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            shading: Shading::default(),
            show_points: false,
            point_size: 0.0,
            palette: Palette::default(),
            lighting: Lighting::default(),
            shadows: false,
            shadow_size: 1024,
            msaa: Msaa::Off,
        }
    }
}

#[derive(Clone, Copy)]
pub struct DrawMesh<'a> {
    pub mesh: &'a MeshBuffers,
    pub color: [f32; 3],
}

pub struct Frame<'a> {
    pub meshes: &'a [DrawMesh<'a>],
    pub grid: Option<&'a Grid>,
    pub gizmo: Option<&'a Gizmo>,
    pub settings: Settings,
}

// ---------------------------------------------------------------------------
// Pipelines
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    base: [f32; 4],
    selection: [f32; 4],
    wire: [f32; 4],
    light: [f32; 4],
    params: [f32; 4],
    /// x: half point size in pixels, y/z: NDC units per pixel (2/width, 2/height).
    point: [f32; 4],
}

fn rgba(c: [f32; 3]) -> [f32; 4] {
    [c[0], c[1], c[2], 1.0]
}

/// One light as the shader reads it (render.wgsl `Light`).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLight {
    pos_kind: [f32; 4],
    color: [f32; 4],
    axis_cos: [f32; 4],
    params: [f32; 4],
    shadow_vp: [[f32; 4]; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GpuLights {
    lights: [GpuLight; MAX_LIGHTS],
    /// x: count, y: ambient, z: 1 / shadow size.
    info: [f32; 4],
}

/// Depth bias of the shading pass, in NDC depth (on top of the pass's slope-scaled bias).
const SHADOW_BIAS: f32 = 1.5e-3;

/// Packs the lights; `layers[i]` is light `i`'s shadow layer and matrix.
fn pack_lights(
    lights: &[Light],
    layers: &[Option<(u32, DMat4)>],
    shadow_size: u32,
    ambient: f32,
) -> GpuLights {
    let mut out = GpuLights::zeroed();
    for (i, (l, layer)) in lights.iter().zip(layers).enumerate().take(MAX_LIGHTS) {
        let d = l.travel();
        let f = |v: DVec3, w: f64| [v.x as f32, v.y as f32, v.z as f32, w as f32];
        let (pos_kind, axis_cos) = match l.kind {
            LightKind::Directional => (f(-d, 0.0), [0.0; 4]),
            LightKind::Point => (f(l.position, 1.0), [0.0; 4]),
            LightKind::Spot => (f(l.position, 2.0), f(d, l.outer.cos())),
        };
        let cos_inner = l.inner.cos().max(l.outer.cos() + 1e-4);
        out.lights[i] = GpuLight {
            pos_kind,
            color: [
                l.color[0] * l.intensity,
                l.color[1] * l.intensity,
                l.color[2] * l.intensity,
                layer.map_or(-1.0, |(n, _)| n as f32),
            ],
            axis_cos,
            params: [cos_inner as f32, l.range as f32, SHADOW_BIAS, 0.0],
            shadow_vp: layer.map_or([[0.0; 4]; 4], |(_, m)| m.as_mat4().to_cols_array_2d()),
        };
    }
    out.info = [
        lights.len().min(MAX_LIGHTS) as f32,
        ambient,
        1.0 / shadow_size.max(1) as f32,
        0.0,
    ];
    out
}

/// Shadow map storage: a depth array texture with one layer per shadow-casting light.
struct ShadowMaps {
    size: u32,
    layers: u32,
    array_view: wgpu::TextureView,
    layer_views: Vec<wgpu::TextureView>,
    // keeps the texture alive; the views reference it
    _texture: wgpu::Texture,
}

impl ShadowMaps {
    fn new(device: &wgpu::Device, size: u32, layers: u32) -> ShadowMaps {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("shadow maps"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: layers,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let array_view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });
        let layer_views = (0..layers)
            .map(|i| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: i,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        ShadowMaps {
            size,
            layers,
            array_view,
            layer_views,
            _texture: texture,
        }
    }
}

/// Pipelines for one colour target format and sample count.
pub struct Renderer {
    samples: u32,
    layout: wgpu::BindGroupLayout,
    light_layout: wgpu::BindGroupLayout,
    tri: wgpu::RenderPipeline,
    shadow: wgpu::RenderPipeline,
    edge: wgpu::RenderPipeline,
    point: wgpu::RenderPipeline,
    point_quad: wgpu::RenderPipeline,
    grid: wgpu::RenderPipeline,
    gizmo: wgpu::RenderPipeline,
    shadow_sampler: wgpu::Sampler,
    /// 1x1 single-layer map bound when nothing casts a shadow.
    dummy_shadow: ShadowMaps,
    /// The map set of the last shadowed frame, reused while size and layer count match.
    shadow_cache: Mutex<Option<std::sync::Arc<ShadowMaps>>>,
}

struct PipelineSpec<'a> {
    label: &'a str,
    vs: &'a str,
    fs: &'a str,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    topology: wgpu::PrimitiveTopology,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
}

impl Renderer {
    /// Single-sample pipelines.
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Renderer {
        Self::with_samples(device, color_format, 1)
    }

    /// Pipelines for `samples` (1 or 4) samples per pixel (docs/specs/VIEWPORT.md §8.4).
    pub fn with_samples(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        samples: u32,
    ) -> Renderer {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("viewport shaders"),
            source: wgpu::ShaderSource::Wgsl(include_str!("render.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let light_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("viewport lights"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport layout"),
            bind_group_layouts: &[Some(&layout), Some(&light_layout)],
            immediate_size: 0,
        });
        // the shadow pass must not bind the shadow maps it renders into
        let shadow_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("shadow layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let tri_attrs =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32];
        let edge_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32];
        let quad_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x2];
        let color_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];
        let tri_buf = [Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<TriVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &tri_attrs,
        })];
        let edge_buf = [Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<EdgeVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &edge_attrs,
        })];
        let quad_buf = [Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<PointVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &quad_attrs,
        })];
        let color_buf = [Some(wgpu::VertexBufferLayout {
            array_stride: size_of::<ColorVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &color_attrs,
        })];
        let make = |s: PipelineSpec| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(s.label),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(s.vs),
                    compilation_options: Default::default(),
                    buffers: s.buffers,
                },
                primitive: wgpu::PrimitiveState {
                    topology: s.topology,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: Some(s.depth_write),
                    depth_compare: Some(s.depth_compare),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: samples,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(s.fs),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: color_format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        use wgpu::CompareFunction::{Always, Less, LessEqual};
        use wgpu::PrimitiveTopology::{LineList, PointList, TriangleList};
        let spec = |label, vs, fs, buffers, topology, depth_write, depth_compare| PipelineSpec {
            label,
            vs,
            fs,
            buffers,
            topology,
            depth_write,
            depth_compare,
        };
        let shadow = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("shadow"),
            layout: Some(&shadow_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_shadow"),
                compilation_options: Default::default(),
                buffers: &tri_buf,
            },
            primitive: wgpu::PrimitiveState {
                topology: TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(Less),
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
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Renderer {
            samples,
            tri: make(spec(
                "tris",
                "vs_tri",
                "fs_tri",
                &tri_buf,
                TriangleList,
                true,
                Less,
            )),
            shadow,
            edge: make(spec(
                "edges", "vs_edge", "fs_edge", &edge_buf, LineList, true, LessEqual,
            )),
            point: make(spec(
                "points", "vs_edge", "fs_edge", &edge_buf, PointList, true, LessEqual,
            )),
            point_quad: make(spec(
                "point quads",
                "vs_point",
                "fs_edge",
                &quad_buf,
                TriangleList,
                false,
                LessEqual,
            )),
            grid: make(spec(
                "grid", "vs_color", "fs_color", &color_buf, LineList, false, LessEqual,
            )),
            gizmo: make(spec(
                "gizmo", "vs_color", "fs_color", &color_buf, LineList, false, Always,
            )),
            layout,
            light_layout,
            shadow_sampler,
            dummy_shadow: ShadowMaps::new(device, 1, 1),
            shadow_cache: Mutex::new(None),
        }
    }

    /// Samples per pixel of the targets this renderer draws to.
    pub fn samples(&self) -> u32 {
        self.samples
    }

    fn bind(&self, device: &wgpu::Device, u: Uniforms) -> wgpu::BindGroup {
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport uniforms"),
            contents: bytemuck::bytes_of(&u),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport uniforms"),
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        })
    }

    fn bind_lights(
        &self,
        device: &wgpu::Device,
        l: &GpuLights,
        maps: &ShadowMaps,
    ) -> wgpu::BindGroup {
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport lights"),
            contents: bytemuck::bytes_of(l),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport lights"),
            layout: &self.light_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&maps.array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.shadow_sampler),
                },
            ],
        })
    }

    /// The shadow maps for `layers` layers of `size` texels, reusing the last ones if they fit.
    fn shadow_maps(
        &self,
        device: &wgpu::Device,
        size: u32,
        layers: u32,
    ) -> std::sync::Arc<ShadowMaps> {
        let mut cache = self.shadow_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(maps) = cache.as_ref()
            && maps.size == size
            && maps.layers == layers
        {
            return maps.clone();
        }
        let maps = std::sync::Arc::new(ShadowMaps::new(device, size, layers));
        *cache = Some(maps.clone());
        maps
    }

    /// Records the frame into `encoder`, clearing both attachments. With a multisampled
    /// `color` (see [`Renderer::samples`]) pass the single-sample `resolve` view.
    #[allow(clippy::too_many_arguments)] // device, encoder, targets, size, camera and frame are independent
    pub fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        resolve: Option<&wgpu::TextureView>,
        width: u32,
        height: u32,
        cam: &Camera,
        frame: &Frame,
    ) {
        let s = &frame.settings;
        let pal = &s.palette;
        let style = s.shading.style();
        let view_proj = cam
            .view_proj(width as f64 / height as f64)
            .as_mat4()
            .to_cols_array_2d();
        let uniforms = |base: [f32; 3], smooth: bool, bias: f32, unlit: bool| Uniforms {
            view_proj,
            base: rgba(base),
            selection: rgba(pal.selection),
            wire: rgba(pal.wire),
            light: [0.0; 4],
            params: [flag(smooth), bias, flag(unlit), 0.0],
            point: [
                s.point_size / 2.0,
                2.0 / width.max(1) as f32,
                2.0 / height.max(1) as f32,
                0.0,
            ],
        };
        let (fill, wire) = (style.fill, style.wire);
        let bias = if fill { OVERLAY_BIAS } else { 0.0 };
        let mesh_groups: Vec<wgpu::BindGroup> = frame
            .meshes
            .iter()
            .map(|m| {
                let base = if style.hidden {
                    pal.background
                } else {
                    m.color
                };
                self.bind(device, uniforms(base, style.smooth, bias, style.unlit))
            })
            .collect();
        let plain = self.bind(device, uniforms(pal.object, false, 0.0, false));
        let grid = frame
            .grid
            .map(|g| Vertices::new(device, "grid", &grid_vertices(g, pal)));
        let gizmo = frame
            .gizmo
            .map(|g| Vertices::new(device, "gizmo", &gizmo_vertices(g, cam, height as f64, pal)));
        let boxes = style.boxes.then(|| {
            let mut out = Vec::new();
            for m in frame.meshes {
                if let Some(b) = m.mesh.bounds {
                    box_vertices(&mut out, b, m.color);
                }
            }
            Vertices::new(device, "bounding boxes", &out)
        });

        // lights and shadow maps (docs/specs/VIEWPORT.md §8.1-§8.2)
        let light_list: Vec<Light> = if style.quick {
            vec![lights::headlight(cam)]
        } else {
            lights::resolve(&s.lighting, cam)
        }
        .into_iter()
        .take(MAX_LIGHTS)
        .collect();
        let bounds = frame
            .meshes
            .iter()
            .filter_map(|m| m.mesh.bounds)
            .reduce(|(a, b), (c, d)| (a.min(c), b.max(d)));
        let casting = s.shadows && fill && !style.unlit && !style.quick;
        let mut next_layer = 0u32;
        let layers: Vec<Option<(u32, DMat4)>> = light_list
            .iter()
            .map(|l| {
                let m = bounds
                    .filter(|_| casting)
                    .and_then(|b| lights::shadow_matrix(l, b))?;
                next_layer += 1;
                Some((next_layer - 1, m))
            })
            .collect();
        let shadow_size = s.shadow_size.clamp(64, 4096);
        let packed = pack_lights(&light_list, &layers, shadow_size, AMBIENT);
        let real = (next_layer > 0).then(|| self.shadow_maps(device, shadow_size, next_layer));
        let maps: &ShadowMaps = real.as_deref().unwrap_or(&self.dummy_shadow);
        if real.is_some() {
            for (layer, matrix) in layers.iter().flatten() {
                let u = Uniforms {
                    view_proj: matrix.as_mat4().to_cols_array_2d(),
                    ..uniforms(pal.object, false, 0.0, false)
                };
                let group = self.bind(device, u);
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("shadow map"),
                    color_attachments: &[],
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &maps.layer_views[*layer as usize],
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
                pass.set_pipeline(&self.shadow);
                pass.set_bind_group(0, &group, &[]);
                for m in frame.meshes {
                    m.mesh.tris.draw(&mut pass);
                }
            }
        }
        let light_group = self.bind_lights(device, &packed, maps);

        let [r, g, b] = pal.background.map(f64::from);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("viewport"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                    store: if resolve.is_some() {
                        wgpu::StoreOp::Discard
                    } else {
                        wgpu::StoreOp::Store
                    },
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
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
        pass.set_bind_group(1, &light_group, &[]);
        if fill {
            pass.set_pipeline(&self.tri);
            for (m, group) in frame.meshes.iter().zip(&mesh_groups) {
                pass.set_bind_group(0, group, &[]);
                m.mesh.tris.draw(&mut pass);
            }
        }
        if wire || s.show_points {
            for (m, group) in frame.meshes.iter().zip(&mesh_groups) {
                pass.set_bind_group(0, group, &[]);
                if wire {
                    pass.set_pipeline(&self.edge);
                    m.mesh.edges.draw(&mut pass);
                }
                if s.show_points && s.point_size > 0.0 {
                    pass.set_pipeline(&self.point_quad);
                    m.mesh.point_quads.draw(&mut pass);
                } else if s.show_points {
                    pass.set_pipeline(&self.point);
                    m.mesh.points.draw(&mut pass);
                }
            }
        }
        pass.set_bind_group(0, &plain, &[]);
        if let Some(boxes) = &boxes {
            pass.set_pipeline(&self.grid);
            boxes.draw(&mut pass);
        }
        if let Some(grid) = &grid {
            pass.set_pipeline(&self.grid);
            grid.draw(&mut pass);
        }
        if let Some(gizmo) = &gizmo {
            pass.set_pipeline(&self.gizmo);
            gizmo.draw(&mut pass);
        }
    }

    /// Records the frame into `targets`, resolving MSAA if it has samples. `targets` must
    /// have been made for this renderer's sample count.
    pub fn render_to(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        targets: &RenderTargets,
        cam: &Camera,
        frame: &Frame,
    ) {
        assert_eq!(
            targets.samples, self.samples,
            "render targets and renderer sample counts differ"
        );
        let resolved = targets.color.create_view(&Default::default());
        let depth = targets.depth.create_view(&Default::default());
        match &targets.msaa_color {
            Some(ms) => self.render(
                device,
                encoder,
                &ms.create_view(&Default::default()),
                &depth,
                Some(&resolved),
                targets.width,
                targets.height,
                cam,
                frame,
            ),
            None => self.render(
                device,
                encoder,
                &resolved,
                &depth,
                None,
                targets.width,
                targets.height,
                cam,
                frame,
            ),
        }
    }
}

/// Colour (resolved), depth and, with MSAA, the multisampled colour texture of one frame.
pub struct RenderTargets {
    pub width: u32,
    pub height: u32,
    pub samples: u32,
    /// The finished image (single sample).
    pub color: wgpu::Texture,
    pub depth: wgpu::Texture,
    msaa_color: Option<wgpu::Texture>,
}

impl RenderTargets {
    /// `extra` usages are added to the colour texture (`COPY_SRC`, `TEXTURE_BINDING`, ...).
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        samples: u32,
        extra: wgpu::TextureUsages,
    ) -> RenderTargets {
        let (width, height) = (width.max(1), height.max(1));
        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let make = |label, format, usage, sample_count| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: extent,
                mip_level_count: 1,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
        };
        let attach = wgpu::TextureUsages::RENDER_ATTACHMENT;
        RenderTargets {
            width,
            height,
            samples,
            color: make("viewport color", COLOR_FORMAT, attach | extra, 1),
            depth: make("viewport depth", DEPTH_FORMAT, attach, samples),
            msaa_color: (samples > 1)
                .then(|| make("viewport color (msaa)", COLOR_FORMAT, attach, samples)),
        }
    }
}

/// Edges of an axis-aligned box as a line list.
fn box_vertices(out: &mut Vec<ColorVertex>, (lo, hi): (DVec3, DVec3), color: [f32; 3]) {
    let corner = |i: usize| {
        DVec3::new(
            if i & 1 == 1 { hi.x } else { lo.x },
            if i & 2 == 2 { hi.y } else { lo.y },
            if i & 4 == 4 { hi.z } else { lo.z },
        )
    };
    for i in 0..8usize {
        for bit in [1usize, 2, 4] {
            if i & bit == 0 {
                line(out, corner(i), corner(i | bit), color);
            }
        }
    }
}

fn line(out: &mut Vec<ColorVertex>, a: DVec3, b: DVec3, color: [f32; 3]) {
    out.push(ColorVertex { pos: f3(a), color });
    out.push(ColorVertex { pos: f3(b), color });
}

fn grid_vertices(grid: &Grid, pal: &Palette) -> Vec<ColorVertex> {
    let mut out = Vec::with_capacity(grid.lines.len() * 2);
    for l in &grid.lines {
        let color = match l.kind {
            LineKind::Minor => pal.grid_minor,
            LineKind::Major => pal.grid_major,
            LineKind::AxisX => pal.axis_x,
            LineKind::AxisZ => pal.axis_z,
        };
        line(&mut out, l.a, l.b, color);
    }
    out
}

fn gizmo_vertices(g: &Gizmo, cam: &Camera, viewport_h: f64, pal: &Palette) -> Vec<ColorVertex> {
    let l = g.world_length(cam, viewport_h);
    let axis_color = |a: Axis| match a {
        Axis::X => pal.gizmo_x,
        Axis::Y => pal.gizmo_y,
        Axis::Z => pal.gizmo_z,
    };
    let mut out = Vec::new();
    if g.mode == GizmoMode::Rotate {
        for a in Axis::ALL {
            let ring = g.ring_points(a, l);
            for k in 0..ring.len() {
                line(&mut out, ring[k], ring[(k + 1) % ring.len()], axis_color(a));
            }
        }
        return out;
    }
    for a in Axis::ALL {
        line(&mut out, g.origin, g.origin + g.axis(a) * l, axis_color(a));
    }
    for p in Plane::ALL {
        let c = g.plane_corners(p, l);
        let (a, b) = p.axes();
        // each side takes the colour of the axis it runs along
        let colors = [axis_color(a), axis_color(b), axis_color(a), axis_color(b)];
        for i in 0..4 {
            line(&mut out, c[i], c[(i + 1) % 4], colors[i]);
        }
    }
    // centre handle: a small square in the view plane
    let r = l * 0.06;
    let (rt, up) = (cam.right_world() * r, cam.up_world() * r);
    let sq = [
        g.origin - rt - up,
        g.origin + rt - up,
        g.origin + rt + up,
        g.origin - rt + up,
    ];
    for i in 0..4 {
        line(&mut out, sq[i], sq[(i + 1) % 4], pal.gizmo_center);
    }
    out
}

// ---------------------------------------------------------------------------
// Offscreen readback
// ---------------------------------------------------------------------------

/// Renders offscreen and returns tightly packed RGBA8 rows, top row first. Uses
/// `frame.settings.msaa`.
pub fn render_to_rgba(gpu: &Gpu, width: u32, height: u32, cam: &Camera, frame: &Frame) -> Vec<u8> {
    let device = &gpu.device;
    let renderer = gpu.renderer_for(frame.settings.msaa);
    let targets = RenderTargets::new(
        device,
        width,
        height,
        renderer.samples(),
        wgpu::TextureUsages::COPY_SRC,
    );
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let unpadded = width * 4;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded = unpadded.div_ceil(align) * align;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("offscreen readback"),
        size: padded as u64 * height as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&Default::default());
    renderer.render_to(device, &mut encoder, &targets, cam, frame);
    encoder.copy_texture_to_buffer(
        targets.color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        extent,
    );
    gpu.queue.submit([encoder.finish()]);

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |r| r.expect("readback map"));
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("device poll");
    let data = slice.get_mapped_range().expect("mapped readback");
    let mut out = Vec::with_capacity((unpadded * height) as usize);
    for row in data.chunks(padded as usize) {
        out.extend_from_slice(&row[..unpadded as usize]);
    }
    out
}
