//! forge-render-bridge: builds render snapshots from evaluated scene geometry.
//! Spec: docs/specs/RENDER.md R1, R12 (the scene has no camera or light objects yet, so the
//! adapter frames the geometry itself and the engine adds its default light).

mod scene;

pub use scene::{
    Applied, apply_scene_objects, camera_from_data, light_from_data, settings_from_config,
    settings_from_scene,
};

use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate_poly;
use forge_material::{Material, compile};
use forge_render_api::{
    Camera, Environment, Instance, MaterialData, MeshData, Projection, Snapshot,
};
use glam::{DMat4, DVec3};

/// Crease angle of the generated shading normals, in degrees.
pub const CREASE_DEG: f64 = 40.0;

/// A triangle mesh with per-corner normals: a corner averages the face normals around its point
/// that lie within [`CREASE_DEG`] of its own face (area weighted).
pub fn mesh_from_poly(poly: &PolyMesh) -> MeshData {
    let positions: Vec<[f64; 3]> = poly.points.iter().map(|p| p.to_array()).collect();
    let mut tris: Vec<[u32; 3]> = Vec::new();
    for face in &poly.polys {
        tris.extend(triangulate_poly(&poly.points, face));
    }
    let face_n: Vec<DVec3> = tris
        .iter()
        .map(|t| {
            let p = |i: u32| poly.points[i as usize];
            (p(t[1]) - p(t[0])).cross(p(t[2]) - p(t[0]))
        })
        .collect();
    let mut around: Vec<Vec<usize>> = vec![Vec::new(); poly.points.len()];
    for (i, t) in tris.iter().enumerate() {
        for &v in t {
            around[v as usize].push(i);
        }
    }
    let cos_limit = CREASE_DEG.to_radians().cos();
    let mut normals = Vec::with_capacity(tris.len() * 3);
    for (i, t) in tris.iter().enumerate() {
        let own = face_n[i].normalize_or_zero();
        for &v in t {
            let mut n = DVec3::ZERO;
            for &j in &around[v as usize] {
                if face_n[j].normalize_or_zero().dot(own) >= cos_limit {
                    n += face_n[j];
                }
            }
            normals.push(n.normalize_or_zero().to_array());
        }
    }
    let mut m = MeshData::new(positions, tris);
    m.normals = normals;
    m
}

/// A grey Lambert material with the Standard material's other defaults.
pub fn default_material() -> MaterialData {
    MaterialData::new("Default", compile(&Material::new()))
}

/// A camera at the default 36 mm / 36 mm lens that frames the box `[min, max]` seen from the
/// direction `from` (pointing from the box toward the camera), for an image `aspect` wide/high.
pub fn frame_camera(min: DVec3, max: DVec3, from: DVec3, aspect: f64) -> Camera {
    let centre = (min + max) * 0.5;
    let radius = ((max - min).length() * 0.5).max(1e-6);
    let (focal, sensor): (f64, f64) = (36.0, 36.0);
    let half_h = (sensor * 0.5 / focal).atan();
    let half_v = (half_h.tan() / aspect).atan();
    let half = half_h.min(half_v);
    let dist = radius / half.sin() * 1.05;
    let dir = from.normalize_or_zero();
    let dir = if dir == DVec3::ZERO { -DVec3::Z } else { dir };
    let pos = centre + dir * dist;
    let fwd = -dir;
    // Rh turns +Z toward -X and Rp toward +Y (render-api camera): fwd = (-sh cp, sp, ch cp).
    let pitch = fwd.y.clamp(-1.0, 1.0).asin();
    let heading = (-fwd.x).atan2(fwd.z);
    Camera::from_hpb(
        pos,
        [heading, pitch, 0.0],
        Projection::Perspective {
            focal_mm: focal,
            sensor_mm: sensor,
            film_offset: [0.0, 0.0],
        },
    )
}

/// One snapshot of evaluated world-space meshes with the default material, the default light
/// (added by the engine) and a camera framing them.
pub fn snapshot_from_polys(polys: &[PolyMesh], aspect: f64) -> Snapshot {
    let mut snap = Snapshot {
        materials: vec![default_material()],
        environment: Environment::default(),
        ..Snapshot::default()
    };
    let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for (i, p) in polys.iter().enumerate() {
        if p.polys.is_empty() {
            continue;
        }
        for q in &p.points {
            lo = lo.min(*q);
            hi = hi.max(*q);
        }
        snap.meshes.push(mesh_from_poly(p));
        let ix = snap.meshes.len() - 1;
        snap.instances
            .push(Instance::new(ix, DMat4::IDENTITY, 0, i as u32));
    }
    if lo.x.is_finite() {
        snap.camera = frame_camera(lo, hi, DVec3::new(0.45, 0.4, -1.0), aspect);
    }
    snap
}
