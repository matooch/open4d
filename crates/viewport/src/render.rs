//! wgpu renderer: shaded/wire meshes, grid, gizmo; offscreen readback
//! (docs/specs/VIEWPORT.md §6).

use std::f64::consts::PI;

use bytemuck::{Pod, Zeroable};
use forge_geometry::normals::{Weighting, corner_normals, poly_normal};
use forge_geometry::selection::Selection;
use forge_geometry::triangulate::triangulate;
use forge_geometry::{CornerUvs, PolyMesh};
use std::sync::{Arc, Mutex, OnceLock};

use glam::{DMat4, DVec3};
use wgpu::util::DeviceExt;

use crate::camera::Camera;
use crate::cull::{Frustum, union_aabb};
use crate::gizmo::{Axis, Gizmo, GizmoMode, Plane};
use crate::grid::{Grid, LineKind};
use crate::instancing::{
    DrawRange, INSTANCE_CHUNK, InstanceInput, InstanceLayout, InstanceRaw, LodView, build_layout,
    visible_ranges,
};
use crate::lights::{self, AMBIENT, Light, LightKind, Lighting, MAX_LIGHTS};
use crate::lod;
use crate::material_preview::{self, PreviewMaterial};

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

    /// What the last offscreen frame rendered with `msaa` drew (draw calls, culling, LOD).
    pub fn last_stats(&self, msaa: Msaa) -> FrameStats {
        self.renderer_for(msaa).last_stats()
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
    uv: [f32; 2],
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

/// Box of f32 positions, padded one ulp-ish so conversions never shrink it.
fn vertex_bounds(it: impl Iterator<Item = [f32; 3]>) -> Option<(DVec3, DVec3)> {
    it.fold(None, |acc, p| {
        let p = DVec3::new(f64::from(p[0]), f64::from(p[1]), f64::from(p[2]));
        Some(acc.map_or((p, p), |(lo, hi)| (lo.min(p), hi.max(p))))
    })
}

fn f3(v: DVec3) -> [f32; 3] {
    v.as_vec3().to_array()
}

fn flag(b: bool) -> f32 {
    if b { 1.0 } else { 0.0 }
}

/// Largest buffer `Vertices::new` creates; bigger data is split (docs/specs/VIEWPORT.md §10.3).
const MAX_CHUNK_BYTES: usize = 64 << 20;

/// One vertex buffer of a [`Vertices`] list with its vertex count and, for mesh chunks, the box
/// of its positions.
struct VChunk {
    buf: wgpu::Buffer,
    n: u32,
    bounds: Option<(DVec3, DVec3)>,
}

/// Vertex data in one or more buffers; empty when there is nothing to draw.
struct Vertices(Vec<VChunk>);

impl Vertices {
    /// Splits by `MAX_CHUNK_BYTES` only (multiples of 6 vertices, so no primitive straddles a
    /// chunk); no bounds.
    fn new<T: Pod>(device: &wgpu::Device, label: &str, data: &[T]) -> Vertices {
        let per = (MAX_CHUNK_BYTES / size_of::<T>().max(1)) / 6 * 6;
        Vertices(
            data.chunks(per.max(6))
                .map(|c| VChunk::new(device, label, c, None))
                .collect(),
        )
    }

    fn from_chunks(chunks: Vec<VChunk>) -> Vertices {
        Vertices(chunks)
    }

    fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        for c in &self.0 {
            pass.set_vertex_buffer(0, c.buf.slice(..));
            pass.draw(0..c.n, 0..1);
        }
    }

    /// As [`Vertices::draw`], skipping chunks whose box is outside `frustum`; returns how many
    /// chunks were skipped.
    fn draw_culled(&self, pass: &mut wgpu::RenderPass<'_>, frustum: &Frustum) -> u32 {
        let mut culled = 0;
        for c in &self.0 {
            if c.bounds
                .is_some_and(|(lo, hi)| !frustum.intersects_aabb(lo, hi))
            {
                culled += 1;
                continue;
            }
            pass.set_vertex_buffer(0, c.buf.slice(..));
            pass.draw(0..c.n, 0..1);
        }
        culled
    }

    /// Draws every chunk once for the instances `range` (vertex buffer 1 must be set).
    fn draw_instanced(&self, pass: &mut wgpu::RenderPass<'_>, range: std::ops::Range<u32>) {
        for c in &self.0 {
            pass.set_vertex_buffer(0, c.buf.slice(..));
            pass.draw(0..c.n, range.clone());
        }
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl VChunk {
    fn new<T: Pod>(
        device: &wgpu::Device,
        label: &str,
        data: &[T],
        bounds: Option<(DVec3, DVec3)>,
    ) -> VChunk {
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytemuck::cast_slice(data),
            usage: wgpu::BufferUsages::VERTEX,
        });
        VChunk {
            buf,
            n: data.len() as u32,
            bounds,
        }
    }
}

/// How a mesh is split into vertex buffers (docs/specs/VIEWPORT.md §10.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadOptions {
    /// Triangles per vertex buffer (and line segments per edge buffer).
    pub chunk_tris: usize,
}

/// Default triangles per chunk: 262 144 (31 MiB of triangle vertices).
pub const DEFAULT_CHUNK_TRIS: usize = 262_144;

impl Default for UploadOptions {
    fn default() -> Self {
        UploadOptions {
            chunk_tris: DEFAULT_CHUNK_TRIS,
        }
    }
}

impl UploadOptions {
    /// `chunk_tris` clamped so that one triangle buffer stays within half of the device limit.
    fn tris_per_chunk(&self, device: &wgpu::Device) -> usize {
        let cap = (device.limits().max_buffer_size / 2) as usize / (3 * size_of::<TriVertex>());
        self.chunk_tris.clamp(1, cap.max(1))
    }
}

/// GPU copy of a polygon mesh: triangles, edges and points with selection flags.
pub struct MeshBuffers {
    preview: Option<Arc<PreviewMaterial>>,
    tris: Vertices,
    edges: Vertices,
    points: Vertices,
    /// Two triangles per point, for `Settings::point_size`.
    point_quads: Vertices,
    bounds: Option<(DVec3, DVec3)>,
    /// A cheaper stand-in drawn for small objects (`Settings::lod_px`, §10.6).
    lod: Option<Box<MeshBuffers>>,
    triangle_count: usize,
}

impl MeshBuffers {
    /// Axis-aligned box of the mesh's points; `None` for a mesh without points.
    pub fn bounds(&self) -> Option<(DVec3, DVec3)> {
        self.bounds
    }

    /// Number of triangles uploaded.
    pub fn triangle_count(&self) -> usize {
        self.triangle_count
    }

    /// Number of vertex buffers holding the triangles.
    pub fn triangle_chunks(&self) -> usize {
        self.tris.0.len()
    }

    /// Attaches the proxy drawn when the object is small on screen (§10.6).
    pub fn set_lod(&mut self, proxy: MeshBuffers) {
        self.lod = Some(Box::new(proxy));
    }

    pub fn has_lod(&self) -> bool {
        self.lod.is_some()
    }

    fn lod_or_self(&self, use_lod: bool) -> &MeshBuffers {
        match (&self.lod, use_lod) {
            (Some(l), true) => l,
            _ => self,
        }
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
        Self::build(
            d,
            mesh,
            selection,
            smoothing,
            true,
            UploadOptions::default(),
            None,
        )
    }

    /// As [`MeshBuffers::with_device_no_points`] with explicit chunking (§10.3).
    pub fn with_options(
        d: &wgpu::Device,
        mesh: &PolyMesh,
        selection: &Selection,
        smoothing: Option<f64>,
        opts: UploadOptions,
    ) -> MeshBuffers {
        Self::build(d, mesh, selection, smoothing, false, opts, None)
    }

    /// As [`MeshBuffers::with_device`] without point buffers (the points are never drawn).
    pub fn with_device_no_points(
        d: &wgpu::Device,
        mesh: &PolyMesh,
        selection: &Selection,
        smoothing: Option<f64>,
    ) -> MeshBuffers {
        Self::build(
            d,
            mesh,
            selection,
            smoothing,
            false,
            UploadOptions::default(),
            None,
        )
    }

    pub fn with_preview(
        d: &wgpu::Device,
        mesh: &PolyMesh,
        selection: &Selection,
        smoothing: Option<f64>,
        with_points: bool,
        uvs: &CornerUvs,
        material: Arc<PreviewMaterial>,
    ) -> MeshBuffers {
        Self::build(
            d,
            mesh,
            selection,
            smoothing,
            with_points,
            UploadOptions::default(),
            Some((uvs, material)),
        )
    }

    fn build(
        d: &wgpu::Device,
        mesh: &PolyMesh,
        selection: &Selection,
        smoothing: Option<f64>,
        with_points: bool,
        opts: UploadOptions,
        preview: Option<(&CornerUvs, Arc<PreviewMaterial>)>,
    ) -> MeshBuffers {
        let preview = preview.filter(|(uvs, _)| uvs.matches(mesh));
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
        // Chunked: each slice of triangles becomes vertices, is uploaded and dropped, so the CPU
        // never holds the vertex data of the whole mesh (§10.3).
        let per = opts.tris_per_chunk(d);
        let tri_list = triangulate(mesh);
        let tri_chunks: Vec<VChunk> = tri_list
            .chunks(per)
            .map(|slice| {
                let mut tris = Vec::with_capacity(slice.len() * 3);
                for (f, tri) in slice {
                    let poly = &mesh.polys[*f as usize];
                    let selected = flag(selection.polys.contains(*f));
                    for v in tri {
                        let corner = poly
                            .iter()
                            .position(|c| *c == *v)
                            .expect("triangle vertex in polygon");
                        tris.push(TriVertex {
                            pos: f3(mesh.points[*v as usize]),
                            flat_n: f3(flat[*f as usize]),
                            smooth_n: f3(smooth[*f as usize][corner]),
                            selected,
                            uv: preview.as_ref().map_or([0.0; 2], |(uvs, _)| {
                                uvs.corners[*f as usize][corner].map(|x| x as f32)
                            }),
                        });
                    }
                }
                let bounds = vertex_bounds(tris.iter().map(|t| t.pos));
                VChunk::new(d, "mesh tris", &tris, bounds)
            })
            .collect();
        let triangle_count = tri_list.len();
        drop(tri_list);
        let all_edges = mesh.edges();
        let edge_chunks: Vec<VChunk> = all_edges
            .chunks(per)
            .enumerate()
            .map(|(k, slice)| {
                let edges: Vec<EdgeVertex> = slice
                    .iter()
                    .enumerate()
                    .flat_map(|(i, e)| {
                        let selected = flag(selection.edges.contains((k * per + i) as u32));
                        [e.0, e.1].map(|v| EdgeVertex {
                            pos: f3(mesh.points[v as usize]),
                            selected,
                        })
                    })
                    .collect();
                let bounds = vertex_bounds(edges.iter().map(|e| e.pos));
                VChunk::new(d, "mesh edges", &edges, bounds)
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
            preview: preview.map(|(_, material)| material),
            tris: Vertices::from_chunks(tri_chunks),
            edges: Vertices::from_chunks(edge_chunks),
            points: Vertices::new(d, "mesh points", &points),
            point_quads: Vertices::new(d, "mesh point quads", &quads),
            bounds: point_bounds(&mesh.points),
            lod: None,
            triangle_count,
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
    /// The selection outline of an object, and the stronger one of the active object
    /// (docs/specs/VIEWPORT.md §16.1).
    pub outline: [f32; 3],
    pub outline_active: [f32; 3],
    /// How far minor grid lines fade toward the background at the grid's edge: 0 = not at all
    /// (the renderer's default), 1 = fully (§16.4).
    pub grid_fade: f32,
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
            outline: [243.0 / 255.0, 179.0 / 255.0, 107.0 / 255.0],
            outline_active: [1.0, 214.0 / 255.0, 158.0 / 255.0],
            grid_fade: 0.0,
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
    /// Objects (and instance chunks) whose projected diameter is below this many pixels draw
    /// their LOD proxy when they have one; 0 = off (§10.6).
    pub lod_px: f32,
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
            lod_px: 0.0,
        }
    }
}

#[derive(Clone, Copy)]
pub struct DrawMesh<'a> {
    pub mesh: &'a MeshBuffers,
    pub color: [f32; 3],
}

/// GPU data of one cloner: prototype meshes and an instance buffer (docs/specs/VIEWPORT.md §10.1).
pub struct InstanceSet {
    preview: Option<Arc<PreviewMaterial>>,
    protos: Vec<MeshBuffers>,
    buffer: Option<wgpu::Buffer>,
    layout: InstanceLayout,
}

impl InstanceSet {
    /// One `MeshBuffers` per prototype (smoothing as in [`MeshBuffers::with_device`]) and the
    /// instance buffer for `input`, each clone placed by `world * matrix`.
    pub fn new(
        d: &wgpu::Device,
        prototypes: &[PolyMesh],
        smoothing: Option<f64>,
        input: &InstanceInput,
        world: &DMat4,
    ) -> InstanceSet {
        Self::with_chunk(d, prototypes, smoothing, input, world, INSTANCE_CHUNK)
    }

    /// As [`InstanceSet::new`] with an explicit culling chunk size.
    pub fn with_chunk(
        d: &wgpu::Device,
        prototypes: &[PolyMesh],
        smoothing: Option<f64>,
        input: &InstanceInput,
        world: &DMat4,
        chunk: usize,
    ) -> InstanceSet {
        Self::build(d, prototypes, smoothing, input, world, chunk, None)
    }

    pub fn with_preview(
        d: &wgpu::Device,
        prototypes: &[PolyMesh],
        smoothing: Option<f64>,
        input: &InstanceInput,
        world: &DMat4,
        uvs: &[CornerUvs],
        material: Arc<PreviewMaterial>,
    ) -> InstanceSet {
        Self::build(
            d,
            prototypes,
            smoothing,
            input,
            world,
            INSTANCE_CHUNK,
            Some((uvs, material)),
        )
    }

    fn build(
        d: &wgpu::Device,
        prototypes: &[PolyMesh],
        smoothing: Option<f64>,
        input: &InstanceInput,
        world: &DMat4,
        chunk: usize,
        preview: Option<(&[CornerUvs], Arc<PreviewMaterial>)>,
    ) -> InstanceSet {
        let preview = preview.filter(|(uvs, _)| {
            uvs.len() == prototypes.len() && uvs.iter().zip(prototypes).all(|(uv, m)| uv.matches(m))
        });
        let protos: Vec<MeshBuffers> = prototypes
            .iter()
            .enumerate()
            .map(|(i, m)| match &preview {
                Some((uvs, mat)) => MeshBuffers::with_preview(
                    d,
                    m,
                    &Selection::default(),
                    smoothing,
                    false,
                    &uvs[i],
                    mat.clone(),
                ),
                None => MeshBuffers::with_device_no_points(d, m, &Selection::default(), smoothing),
            })
            .collect();
        let bounds: Vec<_> = protos.iter().map(|p| p.bounds).collect();
        let mut layout = build_layout(&bounds, input, world, chunk);
        // an instance buffer must fit the device limit (1M clones are 64 MB)
        let cap = (d.limits().max_buffer_size as usize / size_of::<InstanceRaw>()).max(1);
        layout.raw.truncate(cap);
        let buffer = (!layout.raw.is_empty()).then(|| {
            d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("clone instances"),
                contents: bytemuck::cast_slice(&layout.raw),
                usage: wgpu::BufferUsages::VERTEX,
            })
        });
        InstanceSet {
            preview: preview.map(|(_, material)| material),
            protos,
            buffer,
            layout,
        }
    }

    /// Attaches a proxy mesh to prototype `proto` (§10.6).
    pub fn set_proto_lod(
        &mut self,
        d: &wgpu::Device,
        proto: usize,
        proxy: &PolyMesh,
        smoothing: Option<f64>,
    ) {
        if let Some(p) = self.protos.get_mut(proto) {
            p.set_lod(MeshBuffers::with_device_no_points(
                d,
                proxy,
                &Selection::default(),
                smoothing,
            ));
        }
    }

    /// Draws `ranges` with the vertices `pick` selects from each prototype (its LOD proxy for
    /// `lod` ranges). Returns the draw calls issued and the instances drawn.
    fn draw_ranges(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        ranges: &[DrawRange],
        pick: impl Fn(&MeshBuffers) -> &Vertices,
    ) -> (u32, u64) {
        let Some(buffer) = &self.buffer else {
            return (0, 0);
        };
        pass.set_vertex_buffer(1, buffer.slice(..));
        let (mut calls, mut instances) = (0, 0u64);
        for r in ranges {
            let Some(proto) = self.protos.get(r.proto as usize) else {
                continue;
            };
            let verts = pick(proto.lod_or_self(r.lod));
            if verts.is_empty() || r.first as usize >= self.layout.raw.len() {
                continue;
            }
            let end = (r.first + r.count).min(self.layout.raw.len() as u32);
            verts.draw_instanced(pass, r.first..end);
            calls += verts.0.len() as u32;
            instances += u64::from(end - r.first);
        }
        (calls, instances)
    }

    /// Number of instances in the buffer.
    pub fn len(&self) -> usize {
        self.layout.raw.len()
    }

    pub fn is_empty(&self) -> bool {
        self.layout.raw.is_empty()
    }

    pub fn layout(&self) -> &InstanceLayout {
        &self.layout
    }

    pub fn prototype_count(&self) -> usize {
        self.protos.len()
    }

    /// World box of all instances.
    pub fn bounds(&self) -> Option<(DVec3, DVec3)> {
        self.layout.bounds
    }
}

/// One instance set to draw with a base colour (multiplied by each clone's colour).
#[derive(Clone, Copy)]
pub struct DrawInstanced<'a> {
    pub set: &'a InstanceSet,
    pub color: [f32; 3],
}

/// How a drawn object is marked as selected (docs/specs/VIEWPORT.md §16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Highlight {
    #[default]
    None,
    /// Selected: amber outline (amber edges in wire modes).
    Selected,
    /// The active object of the selection: a brighter, wider outline.
    Active,
}

/// The selection display of one frame: a [`Highlight`] per `Frame.meshes` and `Frame.instanced`
/// entry, and the outline widths. The default draws nothing extra.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectionDisplay {
    /// `meshes[i]` marks `Frame.meshes[i]`; entries past the end are `None`.
    pub meshes: Vec<Highlight>,
    /// `instanced[i]` marks `Frame.instanced[i]`.
    pub instanced: Vec<Highlight>,
    /// Outline width of a selected object in pixels.
    pub width_px: f32,
    /// Outline width of the active object in pixels.
    pub active_width_px: f32,
    /// Draw only the outlines of the marked objects on a transparent target, hidden by every
    /// mesh of the frame (the overlay for the render engine's image, §16.2).
    pub overlay_only: bool,
}

impl Default for SelectionDisplay {
    fn default() -> Self {
        SelectionDisplay {
            meshes: Vec::new(),
            instanced: Vec::new(),
            width_px: 2.0,
            active_width_px: 3.0,
            overlay_only: false,
        }
    }
}

impl SelectionDisplay {
    fn mesh(&self, i: usize) -> Highlight {
        self.meshes.get(i).copied().unwrap_or_default()
    }

    fn instance(&self, i: usize) -> Highlight {
        self.instanced.get(i).copied().unwrap_or_default()
    }

    fn any(&self) -> bool {
        self.meshes
            .iter()
            .chain(&self.instanced)
            .any(|h| *h != Highlight::None)
    }
}

/// What the last frame of a [`Renderer`] drew (for tests and the benchmarks).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FrameStats {
    /// `draw` calls in the colour pass.
    pub draw_calls: u32,
    /// Instances in those calls.
    pub instances_drawn: u64,
    /// Meshes, vertex chunks and instance chunks skipped by the frustum test in the colour pass.
    pub culled: u32,
    /// Objects and instance ranges drawn with their LOD proxy.
    pub lod_draws: u32,
    /// `draw` calls of the selection outline (§16.3); not part of `draw_calls`.
    pub outline_draws: u32,
}

/// Coloured line segments on the GPU: the splines of one pane (docs/specs/VIEWPORT.md section 11).
pub struct LineSet(Vertices);

impl LineSet {
    pub fn new(device: &wgpu::Device, lines: &[crate::curves::DrawLine]) -> LineSet {
        let data: Vec<ColorVertex> = lines
            .iter()
            .flat_map(|l| {
                [l.a, l.b].map(|p| ColorVertex {
                    pos: f3(p),
                    color: l.color,
                })
            })
            .collect();
        LineSet(Vertices::new(device, "curve lines", &data))
    }
}

pub struct Frame<'a> {
    pub meshes: &'a [DrawMesh<'a>],
    /// Cloners drawn with GPU instancing, after the plain meshes (§10.1).
    pub instanced: &'a [DrawInstanced<'a>],
    pub grid: Option<&'a Grid>,
    /// Line sets drawn after the meshes and the grid (splines, section 11).
    pub lines: &'a [&'a LineSet],
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
    preview: [f32; 4],
    params: [f32; 4],
    /// x: half point size in pixels, y/z: NDC units per pixel (2/width, 2/height).
    point: [f32; 4],
    eye: [f32; 4],
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
    tri_inst: wgpu::RenderPipeline,
    shadow_inst: wgpu::RenderPipeline,
    edge_inst: wgpu::RenderPipeline,
    edge: wgpu::RenderPipeline,
    point: wgpu::RenderPipeline,
    point_quad: wgpu::RenderPipeline,
    grid: wgpu::RenderPipeline,
    gizmo: wgpu::RenderPipeline,
    gizmo_tips: wgpu::RenderPipeline,
    /// The selection outline (§16.2): mask pipelines for a mesh and an instance set, the
    /// dilation pass and the mask targets.
    outline: OutlineGpu,
    /// Depth only (no colour writes): the occluders of the overlay-only outline.
    depth_only: wgpu::RenderPipeline,
    depth_only_inst: wgpu::RenderPipeline,
    shadow_sampler: wgpu::Sampler,
    preview_sampler: wgpu::Sampler,
    dummy_preview: wgpu::TextureView,
    /// 1x1 single-layer map bound when nothing casts a shadow.
    dummy_shadow: ShadowMaps,
    /// The map set of the last shadowed frame, reused while size and layer count match.
    shadow_cache: Mutex<Option<std::sync::Arc<ShadowMaps>>>,
    stats: Mutex<FrameStats>,
}

struct PipelineSpec<'a> {
    label: &'a str,
    vs: &'a str,
    fs: &'a str,
    buffers: &'a [Option<wgpu::VertexBufferLayout<'a>>],
    topology: wgpu::PrimitiveTopology,
    depth_write: bool,
    depth_compare: wgpu::CompareFunction,
    bias: wgpu::DepthBiasState,
    writes: wgpu::ColorWrites,
    /// The colour target format; `None` = the renderer's.
    format: Option<wgpu::TextureFormat>,
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
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
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
        let tri_attrs = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x3, 3 => Float32, 8 => Float32x2];
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
        let inst_attrs = wgpu::vertex_attr_array![
            4 => Float32x4, 5 => Float32x4, 6 => Float32x4, 7 => Float32x4
        ];
        let inst_layout = wgpu::VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &inst_attrs,
        };
        let tri_inst_buf = [tri_buf[0].clone(), Some(inst_layout.clone())];
        let edge_inst_buf = [edge_buf[0].clone(), Some(inst_layout)];
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
                    bias: s.bias,
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
                        format: s.format.unwrap_or(color_format),
                        blend: None,
                        write_mask: s.writes,
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
            bias: Default::default(),
            writes: wgpu::ColorWrites::ALL,
            format: None,
        };
        // the mask is tested against the depth the fills wrote, pulled slightly toward the
        // camera so equal depths pass (no depth write)
        let mask_spec = |label, vs, buffers| PipelineSpec {
            bias: wgpu::DepthBiasState {
                constant: -4,
                slope_scale: -2.0,
                clamp: 0.0,
            },
            format: Some(MASK_FORMAT),
            ..spec(
                label,
                vs,
                "fs_mask",
                buffers,
                TriangleList,
                false,
                LessEqual,
            )
        };
        let depth_spec = |label, vs, buffers| PipelineSpec {
            writes: wgpu::ColorWrites::empty(),
            ..spec(label, vs, "fs_none", buffers, TriangleList, true, Less)
        };
        let make_shadow = |label, entry, buffers: &[Option<wgpu::VertexBufferLayout>]| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&shadow_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    buffers,
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
            })
        };
        let shadow = make_shadow("shadow", "vs_shadow", &tri_buf);
        let shadow_inst = make_shadow("shadow instanced", "vs_shadow_inst", &tri_inst_buf);
        let shadow_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("shadow sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Renderer {
            samples,
            outline: OutlineGpu::new(
                device,
                color_format,
                make(mask_spec("outline mask", "vs_tri", &tri_buf)),
                make(mask_spec(
                    "outline mask instanced",
                    "vs_tri_inst",
                    &tri_inst_buf,
                )),
            ),
            depth_only: make(depth_spec("depth only", "vs_tri", &tri_buf)),
            depth_only_inst: make(depth_spec(
                "depth only instanced",
                "vs_tri_inst",
                &tri_inst_buf,
            )),
            preview_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            dummy_preview: device
                .create_texture(&material_preview::texture(1))
                .create_view(&Default::default()),
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
            shadow_inst,
            tri_inst: make(spec(
                "tris instanced",
                "vs_tri_inst",
                "fs_tri",
                &tri_inst_buf,
                TriangleList,
                true,
                Less,
            )),
            edge_inst: make(spec(
                "edges instanced",
                "vs_edge_inst",
                "fs_edge",
                &edge_inst_buf,
                LineList,
                true,
                LessEqual,
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
            gizmo_tips: make(spec(
                "gizmo tips",
                "vs_color",
                "fs_color",
                &color_buf,
                TriangleList,
                false,
                Always,
            )),
            layout,
            light_layout,
            shadow_sampler,
            dummy_shadow: ShadowMaps::new(device, 1, 1),
            shadow_cache: Mutex::new(None),
            stats: Mutex::new(FrameStats::default()),
        }
    }

    /// What the last recorded frame drew.
    pub fn last_stats(&self) -> FrameStats {
        *self.stats.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Samples per pixel of the targets this renderer draws to.
    pub fn samples(&self) -> u32 {
        self.samples
    }

    fn bind(
        &self,
        device: &wgpu::Device,
        mut u: Uniforms,
        preview: Option<&PreviewMaterial>,
    ) -> wgpu::BindGroup {
        if let Some(preview) = preview {
            u.preview = preview.params;
        }
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("viewport uniforms"),
            contents: bytemuck::bytes_of(&u),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("viewport uniforms"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(
                        preview.map_or(&self.dummy_preview, |p| &p.view),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.preview_sampler),
                },
            ],
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
        self.render_with(
            device,
            encoder,
            (color, depth, resolve),
            (width, height),
            cam,
            frame,
            &SelectionDisplay::default(),
        );
    }

    /// [`Renderer::render`] with the selection display of docs/specs/VIEWPORT.md §16: outlines
    /// of the highlighted objects, amber edges in the wire modes, or (with
    /// `sel.overlay_only`) the outlines alone on a transparent target.
    /// `(color, depth, resolve)` are the attachments, `(width, height)` their size.
    #[allow(clippy::too_many_arguments)] // device, encoder, targets, size, camera, frame, selection
    pub fn render_with(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        (color, depth, resolve): (
            &wgpu::TextureView,
            &wgpu::TextureView,
            Option<&wgpu::TextureView>,
        ),
        (width, height): (u32, u32),
        cam: &Camera,
        frame: &Frame,
        sel: &SelectionDisplay,
    ) {
        let s = &frame.settings;
        let pal = &s.palette;
        let style = s.shading.style();
        let view_proj = cam
            .view_proj(width as f64 / height as f64)
            .as_mat4()
            .to_cols_array_2d();
        let eye = f3(cam.eye_world());
        let uniforms = |base: [f32; 3], smooth: bool, bias: f32, unlit: bool| Uniforms {
            view_proj,
            base: rgba(base),
            selection: rgba(pal.selection),
            wire: rgba(pal.wire),
            preview: [0.0; 4],
            params: [flag(smooth), bias, flag(unlit), 0.0],
            point: [
                s.point_size / 2.0,
                2.0 / width.max(1) as f32,
                2.0 / height.max(1) as f32,
                0.0,
            ],
            eye: [eye[0], eye[1], eye[2], flag(base != pal.object)],
        };
        let (fill, wire) = (style.fill, style.wire);
        let bias = if fill { OVERLAY_BIAS } else { 0.0 };
        // the edges of a highlighted object take the outline colour (§16.1)
        let outline_of = |h: Highlight| match h {
            Highlight::None => None,
            Highlight::Selected => Some(pal.outline),
            Highlight::Active => Some(pal.outline_active),
        };
        let mesh_groups: Vec<wgpu::BindGroup> = frame
            .meshes
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let base = if style.hidden {
                    pal.background
                } else {
                    m.color
                };
                let mut u = uniforms(base, style.smooth, bias, style.unlit);
                if let Some(c) = outline_of(sel.mesh(i)) {
                    u.wire = rgba(c);
                }
                self.bind(
                    device,
                    u,
                    m.mesh.preview.as_deref().filter(|_| !style.hidden),
                )
            })
            .collect();
        let inst_groups: Vec<wgpu::BindGroup> = frame
            .instanced
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let base = if style.hidden {
                    pal.background
                } else {
                    m.color
                };
                let mut u = uniforms(base, style.smooth, bias, style.unlit);
                if let Some(c) = outline_of(sel.instance(i)) {
                    u.wire = rgba(c);
                }
                // clone colours tint the fill, except in Hidden Line (background colour)
                u.params[3] = flag(!style.hidden);
                self.bind(
                    device,
                    u,
                    m.set.preview.as_deref().filter(|_| !style.hidden),
                )
            })
            .collect();
        let plain = self.bind(device, uniforms(pal.object, false, 0.0, false), None);
        let overlay = sel.overlay_only;
        // mask values of the two highlight classes (R: highlighted, G: active)
        let mask_groups = (sel.any() && (fill || overlay)).then(|| {
            let group = |v: [f32; 3]| self.bind(device, uniforms(v, true, 0.0, true), None);
            (group([1.0, 0.0, 0.0]), group([1.0, 1.0, 0.0]))
        });
        let grid = frame
            .grid
            .filter(|_| !overlay)
            .map(|g| Vertices::new(device, "grid", &grid_vertices(g, pal)));
        let gizmo = frame
            .gizmo
            .map(|g| Vertices::new(device, "gizmo", &gizmo_vertices(g, cam, height as f64, pal)));
        let gizmo_tips = frame
            .gizmo
            .filter(|g| g.mode != GizmoMode::Rotate)
            .and_then(|g| {
                let vertices = gizmo_tip_vertices(g, cam, width as f64, height as f64, pal);
                (!vertices.is_empty()).then(|| Vertices::new(device, "gizmo tips", &vertices))
            });
        let boxes = (style.boxes && !overlay).then(|| {
            let mut out = Vec::new();
            for (i, m) in frame.meshes.iter().enumerate() {
                if let Some(b) = m.mesh.bounds {
                    box_vertices(&mut out, b, outline_of(sel.mesh(i)).unwrap_or(m.color));
                }
            }
            for (i, m) in frame.instanced.iter().enumerate() {
                if let Some(b) = m.set.bounds() {
                    box_vertices(&mut out, b, outline_of(sel.instance(i)).unwrap_or(m.color));
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
            .chain(frame.instanced.iter().filter_map(|m| m.set.bounds()))
            .reduce(union_aabb);
        let casting = s.shadows && fill && !style.unlit && !style.quick && !overlay;
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
                let group = self.bind(device, u, None);
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
                // casters outside the light volume cannot shadow anything in the map
                let light_frustum = Frustum::from_view_proj(matrix);
                pass.set_pipeline(&self.shadow);
                pass.set_bind_group(0, &group, &[]);
                for m in frame.meshes {
                    if m.mesh
                        .bounds
                        .is_none_or(|(lo, hi)| light_frustum.intersects_aabb(lo, hi))
                    {
                        m.mesh.tris.draw_culled(&mut pass, &light_frustum);
                    }
                }
                pass.set_pipeline(&self.shadow_inst);
                for m in frame.instanced {
                    let ranges = visible_ranges(&m.set.layout.chunks, &light_frustum, None);
                    m.set.draw_ranges(&mut pass, &ranges, |p| &p.tris);
                }
            }
        }
        let light_group = self.bind_lights(device, &packed, maps);

        let [r, g, b] = pal.background.map(f64::from);
        // the overlay-only target starts transparent
        let clear = if overlay {
            wgpu::Color::TRANSPARENT
        } else {
            wgpu::Color { r, g, b, a: 1.0 }
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("viewport"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(clear),
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
        let frustum = Frustum::from_view_proj(&cam.view_proj(width as f64 / height as f64));
        let lod_px = f64::from(s.lod_px);
        let view_h = f64::from(height);
        let mut stats = FrameStats::default();
        // meshes that survive the frustum test, with the level of detail they draw
        let visible: Vec<(usize, &MeshBuffers)> = frame
            .meshes
            .iter()
            .enumerate()
            .filter_map(|(i, m)| {
                if m.mesh
                    .bounds
                    .is_some_and(|(lo, hi)| !frustum.intersects_aabb(lo, hi))
                {
                    stats.culled += 1;
                    return None;
                }
                let small = m
                    .mesh
                    .bounds
                    .is_some_and(|(lo, hi)| lod::use_proxy(lo, hi, cam, view_h, lod_px));
                if small && m.mesh.has_lod() {
                    stats.lod_draws += 1;
                }
                Some((i, m.mesh.lod_or_self(small)))
            })
            .collect();
        let lod_view = (lod_px > 0.0).then_some(LodView {
            camera: cam,
            viewport_h: view_h,
            min_px: lod_px,
        });
        let ranges: Vec<Vec<DrawRange>> = frame
            .instanced
            .iter()
            .map(|m| {
                let all = m.set.layout.chunks.len() as u32;
                let r = visible_ranges(&m.set.layout.chunks, &frustum, lod_view);
                let kept: u32 = r
                    .iter()
                    .map(|r| {
                        m.set
                            .layout
                            .chunks
                            .iter()
                            .filter(|c| c.first >= r.first && c.first < r.first + r.count)
                            .count() as u32
                    })
                    .sum();
                stats.culled += all - kept;
                r
            })
            .collect();
        let mut draw = |calls: u32, instances: u64| {
            stats.draw_calls += calls;
            stats.instances_drawn += instances;
        };
        if overlay {
            // occluders only: every mesh writes depth, nothing writes colour
            pass.set_pipeline(&self.depth_only);
            for (i, mesh) in &visible {
                pass.set_bind_group(0, &mesh_groups[*i], &[]);
                stats.culled += mesh.tris.draw_culled(&mut pass, &frustum);
            }
            pass.set_pipeline(&self.depth_only_inst);
            for ((m, group), r) in frame.instanced.iter().zip(&inst_groups).zip(&ranges) {
                pass.set_bind_group(0, group, &[]);
                m.set.draw_ranges(&mut pass, r, |p| &p.tris);
            }
        } else if fill {
            pass.set_pipeline(&self.tri);
            for (i, mesh) in &visible {
                pass.set_bind_group(0, &mesh_groups[*i], &[]);
                stats.culled += mesh.tris.draw_culled(&mut pass, &frustum);
                draw(mesh.tris.0.len() as u32, 0);
            }
            pass.set_pipeline(&self.tri_inst);
            for ((m, group), r) in frame.instanced.iter().zip(&inst_groups).zip(&ranges) {
                pass.set_bind_group(0, group, &[]);
                let (calls, n) = m.set.draw_ranges(&mut pass, r, |p| &p.tris);
                draw(calls, n);
            }
        }
        if !overlay && (wire || s.show_points) {
            for (i, mesh) in &visible {
                pass.set_bind_group(0, &mesh_groups[*i], &[]);
                if wire {
                    pass.set_pipeline(&self.edge);
                    stats.culled += mesh.edges.draw_culled(&mut pass, &frustum);
                }
                if s.show_points && s.point_size > 0.0 {
                    pass.set_pipeline(&self.point_quad);
                    mesh.point_quads.draw(&mut pass);
                } else if s.show_points {
                    pass.set_pipeline(&self.point);
                    mesh.points.draw(&mut pass);
                }
            }
        }
        if wire && !overlay {
            pass.set_pipeline(&self.edge_inst);
            for ((m, group), r) in frame.instanced.iter().zip(&inst_groups).zip(&ranges) {
                pass.set_bind_group(0, group, &[]);
                let (calls, n) = m.set.draw_ranges(&mut pass, r, |p| &p.edges);
                draw(calls, n);
            }
        }
        if !overlay {
            pass.set_bind_group(0, &plain, &[]);
            if let Some(boxes) = &boxes {
                pass.set_pipeline(&self.grid);
                boxes.draw(&mut pass);
            }
            if let Some(grid) = &grid {
                pass.set_pipeline(&self.grid);
                grid.draw(&mut pass);
            }
            if !frame.lines.is_empty() {
                pass.set_pipeline(&self.grid);
                for set in frame.lines {
                    set.0.draw(&mut pass);
                }
            }
        }
        if gizmo.is_some() {
            pass.set_bind_group(0, &plain, &[]);
            if let Some(gizmo) = &gizmo {
                pass.set_pipeline(&self.gizmo);
                gizmo.draw(&mut pass);
            }
            if let Some(tips) = &gizmo_tips {
                pass.set_pipeline(&self.gizmo_tips);
                tips.draw(&mut pass);
            }
        }
        drop(pass);
        // the selection outline: the mask of the highlighted objects, dilated over the image
        // (§16.2)
        if let Some((selected, active)) = &mask_groups {
            self.outline_passes(
                device,
                encoder,
                (depth, resolve.unwrap_or(color)),
                (width, height),
                (cam, frame, sel),
                (&visible, &ranges),
                (selected, active, &light_group),
                &mut stats,
            );
        }
        *self.stats.lock().unwrap_or_else(|e| e.into_inner()) = stats;
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
        self.render_to_with(
            device,
            encoder,
            targets,
            cam,
            frame,
            &SelectionDisplay::default(),
        );
    }

    /// [`Renderer::render_to`] with a selection display (§16).
    pub fn render_to_with(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        targets: &RenderTargets,
        cam: &Camera,
        frame: &Frame,
        sel: &SelectionDisplay,
    ) {
        assert_eq!(
            targets.samples, self.samples,
            "render targets and renderer sample counts differ"
        );
        let resolved = targets.color.create_view(&Default::default());
        let depth = targets.depth.create_view(&Default::default());
        let size = (targets.width, targets.height);
        match &targets.msaa_color {
            Some(ms) => self.render_with(
                device,
                encoder,
                (
                    &ms.create_view(&Default::default()),
                    &depth,
                    Some(&resolved),
                ),
                size,
                cam,
                frame,
                sel,
            ),
            None => self.render_with(
                device,
                encoder,
                (&resolved, &depth, None),
                size,
                cam,
                frame,
                sel,
            ),
        }
    }
}

// ---------------------------------------------------------------------------
// Selection outline (docs/specs/VIEWPORT.md §16.2)
// ---------------------------------------------------------------------------

/// Format of the outline mask: R = highlighted coverage, G = active coverage.
const MASK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rg8Unorm;

/// What `outline.wgsl` reads (docs/specs/VIEWPORT.md §16.2).
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OutlineParams {
    selected: [f32; 4],
    stronger: [f32; 4],
    widths: [f32; 4],
}

/// The mask textures of one frame size: a multisampled one (with MSAA) resolved into a single
/// sample one the dilation reads.
struct MaskTargets {
    size: (u32, u32),
    samples: u32,
    msaa: Option<wgpu::TextureView>,
    resolved: wgpu::TextureView,
    // keep the textures alive; the views reference them
    _textures: Vec<wgpu::Texture>,
}

/// Pipelines and targets of the selection outline.
struct OutlineGpu {
    mask_tri: wgpu::RenderPipeline,
    mask_tri_inst: wgpu::RenderPipeline,
    dilate: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    targets: Mutex<Option<std::sync::Arc<MaskTargets>>>,
}

impl OutlineGpu {
    fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        mask_tri: wgpu::RenderPipeline,
        mask_tri_inst: wgpu::RenderPipeline,
    ) -> OutlineGpu {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("outline shaders"),
            source: wgpu::ShaderSource::Wgsl(include_str!("outline.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("outline"),
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("outline layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let over = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::SrcAlpha,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let dilate = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("outline dilate"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_full"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_dilate"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState {
                        color: over,
                        // alpha accumulates like premultiplied coverage, so an overlay target
                        // that starts transparent ends with the ring's coverage
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        OutlineGpu {
            mask_tri,
            mask_tri_inst,
            dilate,
            layout,
            targets: Mutex::new(None),
        }
    }

    /// The mask textures for `size` and `samples`, reusing the last ones if they fit.
    fn targets(
        &self,
        device: &wgpu::Device,
        size: (u32, u32),
        samples: u32,
    ) -> std::sync::Arc<MaskTargets> {
        let mut cache = self.targets.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(t) = cache.as_ref()
            && t.size == size
            && t.samples == samples
        {
            return t.clone();
        }
        let make = |label, count| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: size.0,
                    height: size.1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: count,
                dimension: wgpu::TextureDimension::D2,
                format: MASK_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let resolved = make("outline mask", 1);
        let msaa = (samples > 1).then(|| make("outline mask (msaa)", samples));
        let targets = std::sync::Arc::new(MaskTargets {
            size,
            samples,
            msaa: msaa.as_ref().map(|t| t.create_view(&Default::default())),
            resolved: resolved.create_view(&Default::default()),
            _textures: std::iter::once(resolved).chain(msaa).collect(),
        });
        *cache = Some(targets.clone());
        targets
    }
}

/// The pixel rectangle `[x, y, w, h]` that bounds the highlighted objects of `frame` grown by
/// `margin` pixels; `None` when it is off screen, the whole target when an object reaches
/// behind the camera.
fn outline_rect(
    frame: &Frame,
    sel: &SelectionDisplay,
    view_proj: &DMat4,
    (width, height): (u32, u32),
    margin: f64,
) -> Option<[u32; 4]> {
    let boxes = frame
        .meshes
        .iter()
        .enumerate()
        .filter(|(i, _)| sel.mesh(*i) != Highlight::None)
        .filter_map(|(_, m)| m.mesh.bounds)
        .chain(
            frame
                .instanced
                .iter()
                .enumerate()
                .filter(|(i, _)| sel.instance(*i) != Highlight::None)
                .filter_map(|(_, m)| m.set.bounds()),
        );
    let (w, h) = (f64::from(width), f64::from(height));
    let (mut lo, mut hi) = ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN));
    for (a, b) in boxes {
        for k in 0..8u32 {
            let p = DVec3::new(
                if k & 1 == 1 { b.x } else { a.x },
                if k & 2 == 2 { b.y } else { a.y },
                if k & 4 == 4 { b.z } else { a.z },
            );
            let c = *view_proj * p.extend(1.0);
            if c.w <= 1e-9 {
                return Some([0, 0, width, height]);
            }
            let (x, y) = ((c.x / c.w * 0.5 + 0.5) * w, (0.5 - c.y / c.w * 0.5) * h);
            lo = (lo.0.min(x), lo.1.min(y));
            hi = (hi.0.max(x), hi.1.max(y));
        }
    }
    if lo.0 > hi.0 {
        return None;
    }
    let (x0, y0) = (
        (lo.0 - margin).floor().max(0.0),
        (lo.1 - margin).floor().max(0.0),
    );
    let (x1, y1) = ((hi.0 + margin).ceil().min(w), (hi.1 + margin).ceil().min(h));
    (x1 > x0 && y1 > y0).then_some([x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32])
}

impl Renderer {
    /// The mask pass and the dilation pass of the selection outline (docs/specs/VIEWPORT.md
    /// §16.2), after the colour pass: the highlighted meshes are drawn into the mask with the
    /// depth the fills wrote (so nearer objects hide them), then the ring around the mask is
    /// drawn over `target`.
    #[allow(clippy::too_many_arguments)] // the pieces of one frame, grouped by what they are
    fn outline_passes(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        (depth, target): (&wgpu::TextureView, &wgpu::TextureView),
        size: (u32, u32),
        (cam, frame, sel): (&Camera, &Frame, &SelectionDisplay),
        (visible, ranges): (&[(usize, &MeshBuffers)], &[Vec<DrawRange>]),
        (selected, active, light_group): (&wgpu::BindGroup, &wgpu::BindGroup, &wgpu::BindGroup),
        stats: &mut FrameStats,
    ) {
        let pal = &frame.settings.palette;
        let (width, height) = size;
        let reach = f64::from(sel.width_px.max(sel.active_width_px).max(0.0));
        let view_proj = cam.view_proj(f64::from(width) / f64::from(height.max(1)));
        let Some([rx, ry, rw, rh]) = outline_rect(frame, sel, &view_proj, size, reach + 1.0) else {
            return;
        };
        let masks = self.outline.targets(device, size, self.samples);
        let frustum = Frustum::from_view_proj(&view_proj);
        {
            let (view, resolve) = match &masks.msaa {
                Some(ms) => (ms, Some(&masks.resolved)),
                None => (&masks.resolved, None),
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("outline mask"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: resolve,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: if resolve.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                })],
                // read-only: the fills' depth hides the objects behind others
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth,
                    depth_ops: None,
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_bind_group(1, light_group, &[]);
            let group = |h: Highlight| match h {
                Highlight::Active => Some(active),
                Highlight::Selected => Some(selected),
                Highlight::None => None,
            };
            pass.set_pipeline(&self.outline.mask_tri);
            for (i, mesh) in visible {
                let Some(g) = group(sel.mesh(*i)) else {
                    continue;
                };
                pass.set_bind_group(0, g, &[]);
                mesh.tris.draw_culled(&mut pass, &frustum);
                stats.outline_draws += mesh.tris.0.len() as u32;
            }
            pass.set_pipeline(&self.outline.mask_tri_inst);
            for (i, (m, r)) in frame.instanced.iter().zip(ranges).enumerate() {
                let Some(g) = group(sel.instance(i)) else {
                    continue;
                };
                pass.set_bind_group(0, g, &[]);
                let (calls, _) = m.set.draw_ranges(&mut pass, r, |p| &p.tris);
                stats.outline_draws += calls;
            }
        }
        let params = OutlineParams {
            selected: rgba(pal.outline),
            stronger: rgba(pal.outline_active),
            widths: [
                sel.width_px.max(0.0),
                sel.active_width_px.max(0.0),
                0.0,
                0.0,
            ],
        };
        let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("outline params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("outline"),
            layout: &self.outline.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&masks.resolved),
                },
            ],
        });
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("outline"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.outline.dilate);
        pass.set_bind_group(0, &group, &[]);
        pass.set_scissor_rect(rx, ry, rw, rh);
        pass.draw(0..3, 0..1);
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

/// Segments a minor line is cut into when the grid fades (§16.4).
const FADE_SEGMENTS: usize = 16;

/// Mix toward the background of a minor grid vertex at XZ distance `d` from the grid centre, for
/// a grid of half extent `half` and fade strength `fade` (docs/specs/VIEWPORT.md §16.4).
pub fn grid_fade_mix(d: f64, half: f64, fade: f32) -> f32 {
    let t = ((d / half.max(1e-12) - 0.35) / 0.65).clamp(0.0, 1.0) as f32;
    fade.clamp(0.0, 1.0) * t * t * (3.0 - 2.0 * t)
}

/// The `(position, colour)` of every vertex of the grid lines as the renderer draws them
/// (two per segment), for tests of the fade (docs/specs/VIEWPORT.md §16.4).
pub fn grid_vertex_list(grid: &Grid, pal: &Palette) -> Vec<([f32; 3], [f32; 3])> {
    grid_vertices(grid, pal)
        .into_iter()
        .map(|v| (v.pos, v.color))
        .collect()
}

/// The vertices of the grid lines. With `pal.grid_fade` > 0 minor lines are cut into segments
/// whose vertex colours fade toward the background with the distance from the grid centre.
fn grid_vertices(grid: &Grid, pal: &Palette) -> Vec<ColorVertex> {
    let mut out = Vec::with_capacity(grid.lines.len() * 2);
    let n = grid.lines.len().max(1) as f64;
    let centre = grid
        .lines
        .iter()
        .fold(DVec3::ZERO, |acc, l| acc + (l.a + l.b) * 0.5)
        / n;
    let half = grid
        .lines
        .iter()
        .map(|l| (l.b - l.a).length() * 0.5)
        .fold(0.0, f64::max);
    for l in &grid.lines {
        let color = match l.kind {
            LineKind::Minor => pal.grid_minor,
            LineKind::Major => pal.grid_major,
            LineKind::AxisX => pal.axis_x,
            LineKind::AxisZ => pal.axis_z,
        };
        if l.kind != LineKind::Minor || pal.grid_fade <= 0.0 {
            line(&mut out, l.a, l.b, color);
            continue;
        }
        let at = |k: usize| {
            let p = l.a + (l.b - l.a) * (k as f64 / FADE_SEGMENTS as f64);
            let d = DVec3::new(p.x - centre.x, 0.0, p.z - centre.z).length();
            let m = grid_fade_mix(d, half, pal.grid_fade);
            let c = std::array::from_fn(|i| color[i] + (pal.background[i] - color[i]) * m);
            (p, c)
        };
        for k in 0..FADE_SEGMENTS {
            let ((pa, ca), (pb, cb)) = (at(k), at(k + 1));
            out.push(ColorVertex {
                pos: f3(pa),
                color: ca,
            });
            out.push(ColorVertex {
                pos: f3(pb),
                color: cb,
            });
        }
    }
    out
}

fn gizmo_axis_color(pal: &Palette, axis: Axis) -> [f32; 3] {
    match axis {
        Axis::X => pal.gizmo_x,
        Axis::Y => pal.gizmo_y,
        Axis::Z => pal.gizmo_z,
    }
}

fn gizmo_tip_vertices(g: &Gizmo, cam: &Camera, w: f64, h: f64, pal: &Palette) -> Vec<ColorVertex> {
    let screen = |point| cam.project(point, w, h).map(|p| p.truncate());
    let Some(origin) = screen(g.origin) else {
        return Vec::new();
    };
    let l = g.world_length(cam, h);
    let forward = cam.forward_world();
    let light = (-0.35 * cam.right_world() + 0.5 * cam.up_world() - 0.75 * forward).normalize();
    let mut faces = Vec::new();
    for axis in Axis::ALL {
        if g.projected_axis_end(axis, l, origin, screen).is_none() {
            continue;
        }
        let Some(tip) = g.axis_tip(axis, l) else {
            continue;
        };
        for points @ [a, b, c] in tip.triangles {
            let center = (a + b + c) / 3.0;
            let mut normal = (b - a).cross(c - a).normalize_or_zero();
            if normal.dot(center - tip.inside) < 0.0 {
                normal = -normal;
            }
            let toward_camera = match cam.projection {
                crate::camera::Projection::Perspective { .. } => cam.eye_world() - center,
                crate::camera::Projection::Orthographic { .. } => -forward,
            };
            // Only visible faces of the closed tips. They overlay the scene, so its depth
            // cannot be used to order the gizmo's own geometry.
            if normal.dot(toward_camera) <= 0.0 {
                continue;
            }
            let shade = 0.68 + 0.32 * normal.dot(light).max(0.0) as f32;
            let color = gizmo_axis_color(pal, axis).map(|c| c * shade);
            faces.push((
                center.dot(forward),
                points.map(|p| ColorVertex { pos: f3(p), color }),
            ));
        }
    }
    faces.sort_by(|a, b| b.0.total_cmp(&a.0));
    faces
        .into_iter()
        .flat_map(|(_, vertices)| vertices)
        .collect()
}

fn gizmo_vertices(g: &Gizmo, cam: &Camera, viewport_h: f64, pal: &Palette) -> Vec<ColorVertex> {
    let l = g.world_length(cam, viewport_h);
    let axis_color = |axis| gizmo_axis_color(pal, axis);
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
        line(&mut out, g.origin, g.shaft_end(a, l), axis_color(a));
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
    render_to_rgba_with(gpu, width, height, cam, frame, &SelectionDisplay::default())
}

/// [`render_to_rgba`] with a selection display (docs/specs/VIEWPORT.md §16).
pub fn render_to_rgba_with(
    gpu: &Gpu,
    width: u32,
    height: u32,
    cam: &Camera,
    frame: &Frame,
    sel: &SelectionDisplay,
) -> Vec<u8> {
    try_render_to_rgba_with(gpu, width, height, cam, frame, sel).expect("offscreen readback")
}

/// [`render_to_rgba_with`] that reports a failed readback instead of panicking (the tests of
/// §16.5 use it, so a lost device fails with a message).
pub fn try_render_to_rgba_with(
    gpu: &Gpu,
    width: u32,
    height: u32,
    cam: &Camera,
    frame: &Frame,
    sel: &SelectionDisplay,
) -> Result<Vec<u8>, GpuError> {
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
    renderer.render_to_with(device, &mut encoder, &targets, cam, frame, sel);
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
    let (tx, rx) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| GpuError(format!("device poll: {e}")))?;
    rx.recv()
        .map_err(|e| GpuError(format!("readback channel: {e}")))?
        .map_err(|e| GpuError(format!("readback map: {e}")))?;
    let data = slice
        .get_mapped_range()
        .map_err(|e| GpuError(format!("mapped readback: {e}")))?;
    let mut out = Vec::with_capacity((unpadded * height) as usize);
    for row in data.chunks(padded as usize) {
        out.extend_from_slice(&row[..unpadded as usize]);
    }
    Ok(out)
}
