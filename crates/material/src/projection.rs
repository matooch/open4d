//! Texture-tag projections and Generate UVW (docs/specs/UV.md section U4).

use std::f64::consts::{PI, TAU};

use forge_geometry::{CornerUvs, PolyMesh};
use glam::{DMat3, DMat4, DVec3, EulerRot};

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProjectionError {
    #[error("unsupported projection: {0}")]
    Unsupported(String),
}

/// A scene camera for the Frontal projection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    pub position: DVec3,
    /// Heading, pitch, bank in radians.
    pub hpb: [f64; 3],
    /// Film height over width (v scale), e.g. 16 / 9 for a 1280 x 720 film.
    pub aspect: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    Spherical,
    Cylindrical,
    Flat,
    Cubic,
    Frontal(Camera),
}

impl Projection {
    /// The projection of a tag param id; `Unsupported` for the ones without a derived rule.
    pub fn from_id(id: &str, camera: Option<Camera>) -> Result<Projection, ProjectionError> {
        Ok(match id {
            "spherical" => Projection::Spherical,
            "cylindrical" => Projection::Cylindrical,
            "flat" => Projection::Flat,
            "cubic" => Projection::Cubic,
            "frontal" => Projection::Frontal(camera.ok_or_else(|| {
                ProjectionError::Unsupported("frontal needs the scene camera".into())
            })?),
            other => return Err(ProjectionError::Unsupported(other.to_string())),
        })
    }
}

/// The tag's own transform (the projection space) and Size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TagSpace {
    pub position: DVec3,
    /// Heading, pitch, bank in radians.
    pub rotation: [f64; 3],
    pub size: DVec3,
}

impl Default for TagSpace {
    fn default() -> Self {
        TagSpace {
            position: DVec3::ZERO,
            rotation: [0.0; 3],
            size: DVec3::splat(100.0),
        }
    }
}

impl TagSpace {
    fn to_tag(&self, p: DVec3) -> DVec3 {
        let [h, pi, b] = self.rotation;
        let r = DMat3::from_euler(EulerRot::YXZ, h, pi, b);
        r.transpose() * (p - self.position)
    }
}

fn angle_u(p: DVec3) -> f64 {
    let a = p.z.atan2(p.x) / TAU;
    a - a.floor()
}

fn normal(pts: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        n += DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    n
}

/// Bakes `projection` into corner UVs. `object_to_world` places the object (only Frontal looks at
/// world space); the tag space is defined in object space.
pub fn generate_uvw(
    mesh: &PolyMesh,
    tag: &TagSpace,
    projection: Projection,
    object_to_world: DMat4,
) -> Result<CornerUvs, ProjectionError> {
    let s = tag.size;
    let mut corners = Vec::with_capacity(mesh.polys.len());
    for poly in &mesh.polys {
        let obj: Vec<DVec3> = poly.iter().map(|&i| mesh.points[i as usize]).collect();
        let pts: Vec<DVec3> = obj.iter().map(|p| tag.to_tag(*p)).collect();
        let uv: Vec<[f64; 2]> = match projection {
            Projection::Flat => pts
                .iter()
                .map(|p| [0.5 + p.x / (2.0 * s.x), 0.5 + p.y / (2.0 * s.y)])
                .collect(),
            Projection::Cubic => {
                let n = normal(&pts);
                let (ax, ay, az) = (n.x.abs(), n.y.abs(), n.z.abs());
                let axis = if ax >= ay && ax >= az {
                    0
                } else if ay >= az {
                    1
                } else {
                    2
                };
                let neg = n[axis] < 0.0;
                pts.iter()
                    .map(|p| {
                        let (u, v) = match (axis, neg) {
                            (0, true) => (-p.z / s.z, p.y / s.y),
                            (0, false) => (p.z / s.z, p.y / s.y),
                            (1, true) => (p.x / s.x, -p.z / s.z),
                            (1, false) => (p.x / s.x, p.z / s.z),
                            (_, true) => (p.x / s.x, p.y / s.y),
                            (_, false) => (-p.x / s.x, p.y / s.y),
                        };
                        [0.5 + u / 2.0, 0.5 + v / 2.0]
                    })
                    .collect()
            }
            Projection::Cylindrical | Projection::Spherical => {
                let v_of = |p: DVec3| match projection {
                    Projection::Spherical => p.x.hypot(p.z).atan2(-p.y) / PI,
                    _ => 0.5 + p.y / (2.0 * s.y),
                };
                angular(&pts, v_of)
            }
            Projection::Frontal(cam) => {
                let [h, p, b] = cam.hpb;
                let f = DVec3::new(-h.sin() * p.cos(), p.sin(), h.cos() * p.cos());
                let right = DVec3::Y.cross(f).normalize();
                let up = f.cross(right);
                let (right, up) = (
                    right * b.cos() + up * b.sin(),
                    -right * b.sin() + up * b.cos(),
                );
                obj.iter()
                    .map(|o| {
                        let d = object_to_world.transform_point3(*o) - cam.position;
                        let z = d.dot(f);
                        [0.5 + d.dot(right) / z, 0.5 + cam.aspect * d.dot(up) / z]
                    })
                    .collect()
            }
        };
        corners.push(uv);
    }
    Ok(CornerUvs { corners })
}

/// Angular `u` with the per-polygon seam fix and the pole rule; `v` from `v_of`.
fn angular(pts: &[DVec3], v_of: impl Fn(DVec3) -> f64) -> Vec<[f64; 2]> {
    const EPS: f64 = 1e-9;
    let on_axis = |p: &DVec3| p.x.hypot(p.z) < EPS;
    let mut u: Vec<f64> = pts
        .iter()
        .map(|p| if on_axis(p) { f64::NAN } else { angle_u(*p) })
        .collect();
    let (lo, hi) = u
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .fold((f64::MAX, f64::MIN), |a, x| (a.0.min(x), a.1.max(x)));
    if hi - lo > 0.5 {
        for x in u.iter_mut().filter(|x| x.is_finite() && **x < 0.5) {
            *x += 1.0;
        }
    }
    // a corner on the axis takes the u of the previous corner of the polygon
    let n = u.len();
    let fixed: Vec<f64> = (0..n)
        .map(|i| {
            let prev = u[(i + n - 1) % n];
            if u[i].is_finite() {
                u[i]
            } else if prev.is_finite() {
                prev
            } else {
                0.0
            }
        })
        .collect();
    pts.iter()
        .zip(&fixed)
        .map(|(p, &x)| [x, v_of(*p)])
        .collect()
}
