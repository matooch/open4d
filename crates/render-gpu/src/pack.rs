//! Packing the snapshot into the shader's buffers (docs/specs/RENDER.md R20.1).

use bytemuck::{Pod, Zeroable};
use forge_render_api::{
    AreaShape, Background, Falloff, Instance, LightKind, Lighting, RenderError, ShadowMode,
    Snapshot,
};
use forge_render_cpu::bvh::{Aabb, Bvh};
use forge_render_cpu::lights::{PLight, prepare};
use forge_render_cpu::scene::Scene;
use glam::{DMat3, DMat4, DVec3};

/// One world-space triangle, 176 bytes: 11 `vec4` rows.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Tri {
    pub p0: [f32; 4],
    pub e1: [f32; 4],
    pub e2: [f32; 4],
    pub ng: [f32; 4],
    pub n0: [f32; 4],
    pub n1: [f32; 4],
    pub n2: [f32; 4],
    pub u0: [f32; 4],
    pub u1: [f32; 4],
    pub u2: [f32; 4],
    /// `x` material, `y` object id, `w` closed (1) or open (0).
    pub ids: [f32; 4],
}

/// One BVH node, 48 bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct Node {
    pub bounds_min: [f32; 4],
    pub bounds_max: [f32; 4],
    /// `x` first, `y` count (0 = interior), `z`/`w` unused.
    pub meta: [u32; 4],
}

/// One prepared light, 160 bytes: 10 `vec4` rows.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GpuLight {
    /// Position, soft-shadow radius.
    pub pos_radius: [f32; 4],
    /// Direction it shines along, kind (0 omni, 1 spot, 2 infinite, 3 area).
    pub axis_kind: [f32; 4],
    /// Colour times intensity.
    pub color: [f32; 4],
    /// Tangent, area shape (0 rectangle, 1 disc, 2 sphere).
    pub tangent_shape: [f32; 4],
    /// Bitangent, two-sided.
    pub bitangent: [f32; 4],
    /// Falloff kind, falloff radius, shadow cast, shadow density.
    pub params: [f32; 4],
    /// Soft shadows, camera-visible, diffuse, specular.
    pub flags: [f32; 4],
    /// Transmission colour of a shadow.
    pub shadow_color: [f32; 4],
    /// Spot inner/outer half angle; the infinite light's angular diameter in `x`.
    pub spot: [f32; 4],
    /// Rectangle width/height, sphere or disc radius, area scale.
    pub shape: [f32; 4],
}

/// The scene as the shader sees it.
#[derive(Debug, Default)]
pub struct Packed {
    pub tris: Vec<Tri>,
    pub nodes: Vec<Node>,
    pub prims: Vec<u32>,
    pub lights: Vec<GpuLight>,
    pub scale: f64,
    pub triangles: usize,
    pub instances: usize,
}

fn v4(x: f64, y: f64, z: f64, w: f64) -> [f32; 4] {
    [x as f32, y as f32, z as f32, w as f32]
}

fn v3(c: DVec3, w: f64) -> [f32; 4] {
    v4(c.x, c.y, c.z, w)
}

fn falloff_kind(f: Falloff) -> f32 {
    match f {
        Falloff::None => 0.0,
        Falloff::Linear => 1.0,
        Falloff::InverseLinear => 2.0,
        Falloff::InverseSquare => 3.0,
        // `prepare` rejects Steps before packing.
        Falloff::Steps => 0.0,
    }
}

fn pack_light(l: &PLight) -> GpuLight {
    let g = &l.light;
    let (mut kind, mut shape_tag, mut shape, mut spot) = (0.0, 0.0, [0.0f32; 4], [0.0f32; 4]);
    let mut two_sided = 0.0;
    match g.kind {
        LightKind::Omni => {}
        LightKind::Spot { inner, outer } => {
            kind = 1.0;
            spot[0] = (inner * 0.5) as f32;
            spot[1] = (outer * 0.5) as f32;
        }
        LightKind::Infinite { angle } => {
            kind = 2.0;
            spot[0] = angle as f32;
        }
        LightKind::Area {
            shape: s,
            two_sided: ts,
        } => {
            kind = 3.0;
            two_sided = if ts { 1.0 } else { 0.0 };
            match s {
                AreaShape::Rectangle { width, height } => {
                    shape[0] = width as f32;
                    shape[1] = height as f32;
                }
                AreaShape::Disc { radius } => {
                    shape_tag = 1.0;
                    shape[2] = radius as f32;
                }
                AreaShape::Sphere { radius } => {
                    shape_tag = 2.0;
                    shape[2] = radius as f32;
                }
                // `prepare` rejects object-shaped area lights.
                AreaShape::Object => {}
            }
            shape[3] = (l.tangent.length() * l.bitangent.length()) as f32;
        }
    }
    GpuLight {
        pos_radius: v3(l.pos, g.radius),
        axis_kind: v3(l.axis, kind),
        color: v3(l.color, 0.0),
        tangent_shape: v3(l.tangent, shape_tag),
        bitangent: v3(l.bitangent, two_sided),
        params: [
            falloff_kind(g.falloff),
            g.falloff_radius as f32,
            f32::from(g.shadow != ShadowMode::None),
            g.shadow_density as f32,
        ],
        flags: [
            f32::from(g.shadow == ShadowMode::Soft),
            f32::from(g.visible),
            f32::from(g.diffuse),
            f32::from(g.specular),
        ],
        shadow_color: v4(g.shadow_color[0], g.shadow_color[1], g.shadow_color[2], 0.0),
        spot,
        shape,
    }
}

fn empty_light() -> GpuLight {
    GpuLight {
        pos_radius: [0.0; 4],
        axis_kind: [0.0, 0.0, 0.0, 0.0],
        color: [0.0; 4],
        tangent_shape: [0.0; 4],
        bitangent: [0.0; 4],
        params: [0.0, 1.0, 0.0, 1.0],
        flags: [0.0; 4],
        shadow_color: [0.0; 4],
        spot: [0.0; 4],
        shape: [0.0; 4],
    }
}

/// A node whose box no ray enters, for an empty scene.
fn empty_node() -> Node {
    Node {
        bounds_min: [1e30; 4],
        bounds_max: [-1e30; 4],
        meta: [0, 0, 0, 0],
    }
}

fn empty_tri() -> Tri {
    Tri {
        p0: [0.0; 4],
        e1: [0.0; 4],
        e2: [0.0; 4],
        ng: [0.0; 4],
        n0: [0.0; 4],
        n1: [0.0; 4],
        n2: [0.0; 4],
        u0: [0.0; 4],
        u1: [0.0; 4],
        u2: [0.0; 4],
        ids: [0.0; 4],
    }
}

impl Packed {
    /// World-space triangles, the BVH over them and the prepared lights (R20.1).
    pub fn build(snap: &Snapshot, default_light: bool) -> Result<Packed, RenderError> {
        let scene = Scene::build(snap)?;
        if snap.instances.is_empty() {
            return Ok(Packed {
                tris: vec![empty_tri()],
                nodes: vec![empty_node()],
                prims: vec![0],
                lights: vec![empty_light()],
                scale: scene.scale,
                triangles: 0,
                instances: 0,
            });
        }
        let mut tris = Vec::with_capacity(scene.triangles);
        let mut bounds = Vec::with_capacity(scene.triangles);
        for (i, inst) in snap.instances.iter().enumerate() {
            let Instance {
                mesh,
                transform,
                material,
                object_id,
            } = inst;
            let m = DMat4::from_cols_array(transform);
            let nm = DMat3::from_mat4(m.inverse().transpose());
            let blas = &scene.blases[scene.insts[i].blas];
            let data = &snap.meshes[*mesh];
            let p = |k: u32| m.transform_point3(DVec3::from(data.positions[k as usize]));
            for (ti, t) in data.triangles.iter().enumerate() {
                let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
                let (e1, e2) = (b - a, c - a);
                let mut ng = nm * (b - a).cross(c - a);
                if blas.flip() {
                    ng = -ng;
                }
                let ng = ng.normalize_or_zero();
                let n = |k: usize| {
                    if data.normals.is_empty() {
                        DVec3::ZERO
                    } else {
                        (nm * DVec3::from(data.normals[k])).normalize_or_zero()
                    }
                };
                let uv = |k: usize| {
                    if data.uvs.is_empty() {
                        DVec3::ZERO
                    } else {
                        DVec3::from(data.uvs[k])
                    }
                };
                let flat = if data.normals.is_empty() { 0.0 } else { 1.0 };
                tris.push(Tri {
                    p0: v3(a, 1.0),
                    e1: v3(e1, 0.0),
                    e2: v3(e2, 0.0),
                    ng: v3(ng, 0.0),
                    n0: v3(n(ti * 3), flat),
                    n1: v3(n(ti * 3 + 1), 0.0),
                    n2: v3(n(ti * 3 + 2), 0.0),
                    u0: v3(uv(ti * 3), 0.0),
                    u1: v3(uv(ti * 3 + 1), 0.0),
                    u2: v3(uv(ti * 3 + 2), 0.0),
                    ids: [
                        *material as f32,
                        *object_id as f32,
                        0.0,
                        f32::from(blas.closed),
                    ],
                });
                let mut bb = Aabb::EMPTY;
                bb.grow(a);
                bb.grow(b);
                bb.grow(c);
                bounds.push(bb);
            }
        }
        let triangles = tris.len();
        if tris.is_empty() {
            tris.push(empty_tri());
        }
        let tree = Bvh::build(&bounds);
        let nodes: Vec<Node> = if tree.nodes().is_empty() {
            vec![empty_node()]
        } else {
            tree.nodes()
                .iter()
                .map(|n| Node {
                    bounds_min: v3(n.bounds.min, 0.0),
                    bounds_max: v3(n.bounds.max, 0.0),
                    meta: [n.first, n.count, 0, 0],
                })
                .collect()
        };
        let mut prims = tree.prims().to_vec();
        if prims.is_empty() {
            prims.push(0);
        }
        let lights: Vec<GpuLight> = prepare(snap, default_light)?
            .iter()
            .map(pack_light)
            .collect();
        let lights = if lights.is_empty() {
            vec![empty_light()]
        } else {
            lights
        };
        Ok(Packed {
            tris,
            nodes,
            prims,
            lights,
            scale: scene.scale,
            triangles,
            instances: snap.instances.len(),
        })
    }
}

/// The camera, environment and filter part of the frame uniform (everything but the tile).
pub fn frame_base(
    snap: &Snapshot,
    settings: &forge_render_api::RenderSettings,
    packed: &Packed,
    aovs: &[forge_render_api::Aov],
    stride: usize,
) -> [f32; crate::frame::SLOTS] {
    use crate::frame as f;
    use forge_render_api::Camera;
    use forge_render_api::Projection;
    let mut frame = [0.0f32; f::SLOTS];
    frame[f::W] = settings.width as f32;
    frame[f::H] = settings.height as f32;
    let [x0, y0, x1, y1] = settings
        .region
        .unwrap_or([0, 0, settings.width, settings.height]);
    frame[f::RX0] = x0 as f32;
    frame[f::RY0] = y0 as f32;
    frame[f::RX1] = x1 as f32;
    frame[f::RY1] = y1 as f32;
    frame[f::SAMPLES] = settings.samples as f32;
    frame[f::MAX_DEPTH] = settings.max_depth as f32;
    frame[f::SEED_LO] = f32::from_bits(settings.seed as u32);
    frame[f::SEED_HI] = f32::from_bits((settings.seed >> 32) as u32);
    frame[f::FILTER] = match settings.filter {
        forge_render_api::PixelFilter::Box => 0.0,
        forge_render_api::PixelFilter::Triangle => 1.0,
        forge_render_api::PixelFilter::Gaussian => 2.0,
        forge_render_api::PixelFilter::Cubic => 3.0,
    };
    frame[f::FILTER_WIDTH] = settings.filter_width as f32;
    frame[f::CLAMP] = settings.clamp as f32;
    frame[f::EPS] = (1e-6 * packed.scale) as f32;
    frame[f::SCALE] = packed.scale as f32;
    match snap.environment.background {
        Background::Color(c) => {
            frame[f::BG_KIND] = 0.0;
            frame[f::BG_X] = c[0] as f32;
            frame[f::BG_Y] = c[1] as f32;
            frame[f::BG_Z] = c[2] as f32;
        }
        Background::Environment => frame[f::BG_KIND] = 1.0,
    }
    match &snap.environment.lighting {
        Lighting::None => {}
        Lighting::Constant { color, intensity } => {
            frame[f::LIGHTING_KIND] = 1.0;
            frame[f::LIGHTING_INTENSITY] = *intensity as f32;
            for (k, c) in color.iter().enumerate() {
                frame[f::CONST_X + k] = *c as f32;
            }
        }
        Lighting::Sky {
            zenith,
            horizon,
            ground,
            intensity,
        } => {
            frame[f::LIGHTING_KIND] = 2.0;
            frame[f::LIGHTING_INTENSITY] = *intensity as f32;
            for (k, c) in zenith.iter().enumerate() {
                frame[f::SKY_ZEN_X + k] = *c as f32;
            }
            for (k, c) in horizon.iter().enumerate() {
                frame[f::SKY_HOR_X + k] = *c as f32;
            }
            for (k, c) in ground.iter().enumerate() {
                frame[f::SKY_GND_X + k] = *c as f32;
            }
        }
        // R20.4: image environments keep the CPU path only.
        Lighting::Image { .. } => {}
    }
    let cam: &Camera = &snap.camera;
    for (k, v) in cam.transform.iter().enumerate() {
        frame[f::CAMERA + k] = *v as f32;
    }
    match cam.projection {
        Projection::Perspective {
            focal_mm,
            sensor_mm,
            film_offset,
        } => {
            frame[f::PROJ_KIND] = 0.0;
            frame[f::FOCAL] = focal_mm as f32;
            frame[f::SENSOR] = sensor_mm as f32;
            frame[f::FILM_OFF_X] = film_offset[0] as f32;
            frame[f::FILM_OFF_Y] = film_offset[1] as f32;
        }
        Projection::Orthographic { width } => {
            frame[f::PROJ_KIND] = 1.0;
            frame[f::ORTHO_WIDTH] = width as f32;
        }
    }
    if let Some(ap) = cam.aperture {
        frame[f::APERTURE_PRESENT] = 1.0;
        frame[f::APERTURE_RADIUS] = ap.radius as f32;
        frame[f::APERTURE_FOCUS] = ap.focus_distance as f32;
    }
    frame[f::AOV_COUNT] = aovs.len() as f32;
    frame[f::STRIDE] = stride as f32;
    for (k, a) in aovs.iter().enumerate() {
        frame[f::AOV_TAG + k] = f32::from_bits(crate::aov_tag(*a));
    }
    frame
}
