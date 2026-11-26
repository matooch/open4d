//! Shadow maps of the raster tier: one atlas, one tile per 2D light, six per omni
//! (docs/specs/RENDER.md R21.4).

use forge_render_api::{LightKind, RenderError, ShadowMode, Snapshot};
use forge_render_gpu::pack::Packed;
use glam::DVec3;

/// Edge of one cube face or one 2D shadow tile.
pub const TILE_PX: u32 = 512;
/// Tiles across the atlas.
pub const ATLAS_X: u32 = 8;
/// Tiles down the atlas.
pub const ATLAS_Y: u32 = 4;
/// Tiles in the atlas.
pub const MAX_TILES: u32 = ATLAS_X * ATLAS_Y;

/// Basis of the six cube faces, `(right, up, forward)`: the shader's face test must pick the
/// same index from a direction (R21.4).
pub const CUBE_FACES: [([f64; 3], [f64; 3], [f64; 3]); 6] = [
    ([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
    ([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),
    ([-1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, -1.0, 0.0]),
    ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
    ([-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]),
];

/// World bounds of the packed triangles.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min: DVec3,
    pub max: DVec3,
}

impl Bounds {
    pub fn corners(self) -> [DVec3; 8] {
        let (a, b) = (self.min, self.max);
        [
            DVec3::new(a.x, a.y, a.z),
            DVec3::new(b.x, a.y, a.z),
            DVec3::new(a.x, b.y, a.z),
            DVec3::new(b.x, b.y, a.z),
            DVec3::new(a.x, a.y, b.z),
            DVec3::new(b.x, a.y, b.z),
            DVec3::new(a.x, b.y, b.z),
            DVec3::new(b.x, b.y, b.z),
        ]
    }
}

pub fn scene_bounds(packed: &Packed) -> Bounds {
    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for t in &packed.tris {
        let p = DVec3::new(t.p0[0] as f64, t.p0[1] as f64, t.p0[2] as f64);
        let e1 = DVec3::new(t.e1[0] as f64, t.e1[1] as f64, t.e1[2] as f64);
        let e2 = DVec3::new(t.e2[0] as f64, t.e2[1] as f64, t.e2[2] as f64);
        for p in [p, p + e1, p + e2] {
            min = min.min(p);
            max = max.max(p);
        }
    }
    if !min.is_finite() || !max.is_finite() {
        return Bounds {
            min: DVec3::ZERO,
            max: DVec3::ZERO,
        };
    }
    Bounds { min, max }
}

/// The shadow-casting lights and the matrices of their tiles.
#[derive(Debug, Clone)]
pub struct ShadowPlan {
    /// Index into the snapshot's lights (the shading light order).
    pub light: usize,
    /// One matrix per tile; six for an omni light (cube faces in [`CUBE_FACES`] order).
    pub tiles: Vec<(u32, [f32; 16])>,
    pub soft: bool,
    pub cube: bool,
}

fn basis(forward: DVec3, up_hint: DVec3) -> (DVec3, DVec3, DVec3) {
    let f = forward.normalize_or_zero();
    let r = up_hint.cross(f).normalize_or_zero();
    let u = f.cross(r).normalize_or_zero();
    (r, u, f)
}

/// `clip = M * (p, 1)` with a perspective projection, depth 0..1 and `w = Qz`.
pub fn perspective_matrix(b: Basis, focal: f64, near: f64, far: f64) -> [f32; 16] {
    let Basis {
        right,
        up,
        forward,
        eye,
    } = b;
    let a = far / (far - near);
    let b = far * near / (far - near);
    let f = |v: DVec3| [v.x as f32, v.y as f32, v.z as f32];
    let (r, u, fw) = (f(right), f(up), f(forward));
    [
        focal as f32 * r[0],
        focal as f32 * u[0],
        a as f32 * fw[0],
        fw[0],
        focal as f32 * r[1],
        focal as f32 * u[1],
        a as f32 * fw[1],
        fw[1],
        focal as f32 * r[2],
        focal as f32 * u[2],
        a as f32 * fw[2],
        fw[2],
        -(focal * right.dot(eye)) as f32,
        -(focal * up.dot(eye)) as f32,
        -(a * forward.dot(eye) + b) as f32,
        -forward.dot(eye) as f32,
    ]
}

/// One light-space camera basis.
#[derive(Debug, Clone, Copy)]
pub struct Basis {
    pub right: DVec3,
    pub up: DVec3,
    pub forward: DVec3,
    pub eye: DVec3,
}

/// `clip = M * (p, 1)` with an orthographic projection centred on `centre` in light space,
/// depth 0..1 and `w = 1`.
pub fn ortho_matrix(b: Basis, extent: f64, centre: [f64; 2], near: f64, far: f64) -> [f32; 16] {
    let Basis {
        right,
        up,
        forward,
        eye,
    } = b;
    let a = 1.0 / (far - near);
    let b = near / (far - near);
    let (r, u, fw) = (right, up, forward);
    [
        (r.x / extent) as f32,
        (u.x / extent) as f32,
        (a * fw.x) as f32,
        0.0,
        (r.y / extent) as f32,
        (u.y / extent) as f32,
        (a * fw.y) as f32,
        0.0,
        (r.z / extent) as f32,
        (u.z / extent) as f32,
        (a * fw.z) as f32,
        0.0,
        (-(r.dot(eye) / extent + centre[0] / extent)) as f32,
        (-(u.dot(eye) / extent + centre[1] / extent)) as f32,
        (-(a * fw.dot(eye) + b)) as f32,
        1.0,
    ]
}

/// The tile matrices of every shadow-casting light (R21.4).
pub fn plans(snapshot: &Snapshot, packed: &Packed) -> Result<Vec<ShadowPlan>, RenderError> {
    plans_with_bounds(snapshot, scene_bounds(packed), packed.scale)
}

pub(crate) fn plans_with_bounds(
    snapshot: &Snapshot,
    bounds: Bounds,
    scale: f64,
) -> Result<Vec<ShadowPlan>, RenderError> {
    let corners = bounds.corners();
    let scale = scale.max(1e-6);
    let near = 0.01 * scale;
    let mut plans = Vec::new();
    let mut used = 0u32;
    for (i, l) in snapshot.lights.iter().enumerate() {
        if l.shadow == ShadowMode::None {
            continue;
        }
        let m = l.matrix();
        let eye = m.w_axis.truncate();
        let axis = m.transform_vector3(DVec3::Z).normalize_or_zero();
        let up_hint = if axis.dot(DVec3::Y).abs() > 0.999_999 {
            DVec3::Z
        } else {
            DVec3::Y
        };
        let far = corners
            .iter()
            .map(|c| (*c - eye).length())
            .fold(0.0f64, f64::max)
            .max(scale)
            * 1.05;
        let far = far.max(near * 2.0);
        let (right, up, forward) = basis(axis, up_hint);
        let mut tiles: Vec<(u32, [f32; 16])> = Vec::new();
        let mut cube = false;
        match l.kind {
            LightKind::Infinite { .. } => {
                let q = |p: DVec3| {
                    let d = p - eye;
                    [right.dot(d), up.dot(d), forward.dot(d)]
                };
                let qs: Vec<[f64; 3]> = corners.iter().map(|c| q(*c)).collect();
                let (mut x0, mut x1) = (f64::INFINITY, f64::NEG_INFINITY);
                let (mut y0, mut y1) = (f64::INFINITY, f64::NEG_INFINITY);
                let (mut z0, mut z1) = (f64::INFINITY, f64::NEG_INFINITY);
                for v in qs {
                    x0 = x0.min(v[0]);
                    x1 = x1.max(v[0]);
                    y0 = y0.min(v[1]);
                    y1 = y1.max(v[1]);
                    z0 = z0.min(v[2]);
                    z1 = z1.max(v[2]);
                }
                let extent = 0.525 * (x1 - x0).max(y1 - y0).max(1e-6);
                let centre = [(x0 + x1) * 0.5, (y0 + y1) * 0.5];
                let (zn, zf) = (z0 - 0.05 * (z1 - z0 + scale), z1 + 0.05 * (z1 - z0 + scale));
                used += 1;
                tiles.push((
                    used - 1,
                    ortho_matrix(
                        Basis {
                            right,
                            up,
                            forward,
                            eye,
                        },
                        extent,
                        centre,
                        zn,
                        zf.max(zn + near),
                    ),
                ));
            }
            LightKind::Spot { outer, .. } => {
                let half = (outer * 0.5).clamp(1e-3, 85f64.to_radians());
                used += 1;
                tiles.push((
                    used - 1,
                    perspective_matrix(
                        Basis {
                            right,
                            up,
                            forward,
                            eye,
                        },
                        1.0 / half.tan(),
                        near,
                        far,
                    ),
                ));
            }
            LightKind::Area { .. } => {
                let half = 60f64.to_radians();
                used += 1;
                tiles.push((
                    used - 1,
                    perspective_matrix(
                        Basis {
                            right,
                            up,
                            forward,
                            eye,
                        },
                        1.0 / half.tan(),
                        near,
                        far,
                    ),
                ));
            }
            LightKind::Omni => {
                cube = true;
                for (_, u, f) in CUBE_FACES {
                    let (right, up, forward) = basis(DVec3::from(f), DVec3::from(u));
                    used += 1;
                    tiles.push((
                        used - 1,
                        perspective_matrix(
                            Basis {
                                right,
                                up,
                                forward,
                                eye,
                            },
                            1.0,
                            near,
                            far,
                        ),
                    ));
                }
            }
        }
        if used > MAX_TILES {
            return Err(RenderError::Unsupported(format!(
                "more than {} shadow map tiles ({} shadow-casting lights, R21.4)",
                MAX_TILES,
                plans.len() + 1
            )));
        }
        plans.push(ShadowPlan {
            light: i,
            tiles,
            soft: l.shadow == ShadowMode::Soft,
            cube,
        });
    }
    Ok(plans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(m: &[f32; 16], p: DVec3) -> [f64; 3] {
        let mut out = [0.0f64; 4];
        for (i, o) in out.iter_mut().enumerate() {
            *o = (m[i] as f64) * p.x
                + (m[4 + i] as f64) * p.y
                + (m[8 + i] as f64) * p.z
                + m[12 + i] as f64;
        }
        [out[0] / out[3], out[1] / out[3], out[2] / out[3]]
    }

    #[test]
    fn a_perspective_tile_puts_the_axis_at_its_centre() {
        let eye = DVec3::new(10.0, 20.0, 30.0);
        let (r, u, f) = basis(DVec3::new(0.0, 0.0, 1.0), DVec3::Y);
        let m = perspective_matrix(
            Basis {
                right: r,
                up: u,
                forward: f,
                eye,
            },
            1.0,
            0.1,
            100.0,
        );
        let ndc = project(&m, eye + f * 10.0);
        assert!(ndc[0].abs() < 1e-6, "x {}", ndc[0]);
        assert!(ndc[1].abs() < 1e-6, "y {}", ndc[1]);
        assert!((0.0..=1.0).contains(&ndc[2]), "z {}", ndc[2]);
    }

    #[test]
    fn an_orthographic_tile_maps_the_bounds_into_the_unit_square() {
        let (r, u, f) = basis(DVec3::Y, DVec3::Z);
        let m = ortho_matrix(
            Basis {
                right: r,
                up: u,
                forward: f,
                eye: DVec3::ZERO,
            },
            50.0,
            [0.0, 0.0],
            -100.0,
            100.0,
        );
        let a = project(&m, r * 50.0 + f * 10.0);
        let b = project(&m, -r * 50.0 + u * 50.0);
        assert!((a[0] - 1.0).abs() < 1e-5, "right edge {}", a[0]);
        assert!((b[0] + 1.0).abs() < 1e-5 && (b[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn every_cube_face_centre_projects_to_the_tile_centre() {
        let eye = DVec3::new(3.0, -4.0, 5.0);
        for (_, u, f) in CUBE_FACES {
            let (right, up, forward) = basis(DVec3::from(f), DVec3::from(u));
            let m = perspective_matrix(
                Basis {
                    right,
                    up,
                    forward,
                    eye,
                },
                1.0,
                0.1,
                100.0,
            );
            let ndc = project(&m, eye + forward * 20.0);
            assert!(ndc[0].abs() < 1e-6 && ndc[1].abs() < 1e-6);
            assert!((0.0..=1.0).contains(&ndc[2]));
        }
    }

    #[test]
    fn a_spot_tile_with_a_90_degree_cone_covers_its_edge() {
        let (r, u, f) = basis(DVec3::Z, DVec3::Y);
        let m = perspective_matrix(
            Basis {
                right: r,
                up: u,
                forward: f,
                eye: DVec3::ZERO,
            },
            1.0,
            0.1,
            100.0,
        );
        let ndc = project(&m, DVec3::new(10.0, 0.0, 10.0));
        assert!((ndc[0] - 1.0).abs() < 1e-6, "x {}", ndc[0]);
    }

    #[test]
    fn the_tile_budget_is_enforced() {
        // Six omni lights take 36 tiles, more than the 32 in the atlas.
        let mut l = forge_render_api::Light::new(LightKind::Omni, glam::DMat4::IDENTITY);
        l.shadow = ShadowMode::Hard;
        let snap = Snapshot {
            lights: vec![l; 6],
            ..Snapshot::default()
        };
        let err = plans(&snap, &Packed::default()).unwrap_err();
        assert!(matches!(err, RenderError::Unsupported(_)));
    }
}
