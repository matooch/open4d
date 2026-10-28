//! wgpu renderer: shaded/wire meshes, grid, gizmo; offscreen readback
//! (docs/specs/VIEWPORT.md §6).

use std::f64::consts::PI;

use bytemuck::{Pod, Zeroable};
use forge_geometry::PolyMesh;
use forge_geometry::normals::{Weighting, corner_normals, poly_normal};
use forge_geometry::selection::Selection;
use forge_geometry::triangulate::triangulate;
use glam::DVec3;
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::gizmo::{Axis, Gizmo, GizmoMode, Plane};
use crate::grid::{Grid, LineKind};

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
    renderer: Renderer,
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
            renderer,
        })
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Settings {
    pub shading: Shading,
    pub show_points: bool,
    /// Side of a point in pixels; 0 draws 1 px points (docs/specs/VIEWPORT.md §6).
    pub point_size: f32,
    pub palette: Palette,
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

/// Pipelines for one colour target format.
pub struct Renderer {
    layout: wgpu::BindGroupLayout,
    tri: wgpu::RenderPipeline,
    edge: wgpu::RenderPipeline,
    point: wgpu::RenderPipeline,
    point_quad: wgpu::RenderPipeline,
    grid: wgpu::RenderPipeline,
    gizmo: wgpu::RenderPipeline,
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
    pub fn new(device: &wgpu::Device, color_format: wgpu::TextureFormat) -> Renderer {
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
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("viewport layout"),
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
                multisample: Default::default(),
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
        Renderer {
            tri: make(spec(
                "tris",
                "vs_tri",
                "fs_tri",
                &tri_buf,
                TriangleList,
                true,
                Less,
            )),
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
        }
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

    /// Records the frame into `encoder`, clearing both attachments.
    #[allow(clippy::too_many_arguments)] // device, encoder, targets, size, camera and frame are independent
    pub fn render(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        color: &wgpu::TextureView,
        depth: &wgpu::TextureView,
        width: u32,
        height: u32,
        cam: &Camera,
        frame: &Frame,
    ) {
        let s = &frame.settings;
        let pal = &s.palette;
        let view_proj = cam
            .view_proj(width as f64 / height as f64)
            .as_mat4()
            .to_cols_array_2d();
        let light = f3(-cam.forward_world());
        let uniforms = |base: [f32; 3], smooth: bool, bias: f32| Uniforms {
            view_proj,
            base: rgba(base),
            selection: rgba(pal.selection),
            wire: rgba(pal.wire),
            light: [light[0], light[1], light[2], 0.0],
            params: [flag(smooth), bias, 0.0, 0.0],
            point: [
                s.point_size / 2.0,
                2.0 / width.max(1) as f32,
                2.0 / height.max(1) as f32,
                0.0,
            ],
        };
        let (fill, smooth, wire) = match s.shading {
            Shading::Smooth => (true, true, false),
            Shading::Flat => (true, false, false),
            Shading::Wire => (false, false, true),
            Shading::SmoothWire => (true, true, true),
            Shading::FlatWire => (true, false, true),
        };
        let bias = if fill { OVERLAY_BIAS } else { 0.0 };
        let mesh_groups: Vec<wgpu::BindGroup> = frame
            .meshes
            .iter()
            .map(|m| self.bind(device, uniforms(m.color, smooth, bias)))
            .collect();
        let plain = self.bind(device, uniforms(pal.object, false, 0.0));
        let grid = frame
            .grid
            .map(|g| Vertices::new(device, "grid", &grid_vertices(g, pal)));
        let gizmo = frame
            .gizmo
            .map(|g| Vertices::new(device, "gizmo", &gizmo_vertices(g, cam, height as f64, pal)));

        let [r, g, b] = pal.background.map(f64::from);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("viewport"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a: 1.0 }),
                    store: wgpu::StoreOp::Store,
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
        if let Some(grid) = &grid {
            pass.set_pipeline(&self.grid);
            grid.draw(&mut pass);
        }
        if let Some(gizmo) = &gizmo {
            pass.set_pipeline(&self.gizmo);
            gizmo.draw(&mut pass);
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

/// Renders offscreen and returns tightly packed RGBA8 rows, top row first.
pub fn render_to_rgba(gpu: &Gpu, width: u32, height: u32, cam: &Camera, frame: &Frame) -> Vec<u8> {
    let device = &gpu.device;
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = |label, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let color = texture(
        "offscreen color",
        COLOR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = texture(
        "offscreen depth",
        DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());

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
    gpu.renderer.render(
        device,
        &mut encoder,
        &color_view,
        &depth_view,
        width,
        height,
        cam,
        frame,
    );
    encoder.copy_texture_to_buffer(
        color.as_image_copy(),
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
