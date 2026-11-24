//! Displacer deformer (docs/specs/MODIFIERS.md §22.2): shader height along a direction.

use forge_geometry::PolyMesh;
use forge_geometry::normals::{Weighting, vertex_normals};
use forge_material::ShadeCtx;
use forge_shaders::shader::Shader;
use glam::DVec3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Intensity,
    Centered,
    Red,
    Green,
    ObjectRgb,
    WorldRgb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Normal,
    Spherical,
    Planar,
}

pub struct DisplacerParams {
    pub strength: f64,
    pub height: f64,
    pub kind: Kind,
    pub direction: Direction,
    pub offset_u: f64,
    pub offset_v: f64,
    pub length_u: f64,
    pub length_v: f64,
    pub tile: bool,
    pub invert: bool,
}

impl Default for DisplacerParams {
    fn default() -> Self {
        Self {
            strength: 1.0,
            height: 10.0,
            kind: Kind::Centered,
            direction: Direction::Normal,
            offset_u: 0.0,
            offset_v: 0.0,
            length_u: 1.0,
            length_v: 1.0,
            tile: true,
            invert: false,
        }
    }
}

fn luminance(c: [f64; 3]) -> f64 {
    0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]
}

fn scalar(kind: Kind, c: [f64; 3]) -> f64 {
    match kind {
        Kind::Intensity => luminance(c),
        Kind::Centered => 2.0 * luminance(c) - 1.0,
        Kind::Red => c[0],
        Kind::Green => c[1],
        Kind::ObjectRgb | Kind::WorldRgb => 1.0,
    }
}

/// UVW of a point: the object box mapped to 0..1 (no UVW tag is read yet).
fn uvw(point: DVec3, lo: DVec3, size: DVec3, p: &DisplacerParams) -> [f64; 3] {
    let t = (point - lo) / size;
    let mut u = p.offset_u + t.x * p.length_u;
    let mut v = p.offset_v + t.y * p.length_v;
    if !p.tile {
        u = u.clamp(0.0, 1.0);
        v = v.clamp(0.0, 1.0);
    }
    [u, v, t.z]
}

/// Moves the points by the linked shader's height (nothing without a shader).
pub fn displace(mesh: &mut PolyMesh, shader: Option<&Shader>, p: &DisplacerParams, time: f64) {
    let Some(shader) = shader else {
        return;
    };
    if p.strength == 0.0 || p.height == 0.0 {
        return;
    }
    let normals = vertex_normals(mesh, Weighting::Area);
    let lo = mesh
        .points
        .iter()
        .fold(DVec3::splat(f64::INFINITY), |a, b| a.min(*b));
    let hi = mesh
        .points
        .iter()
        .fold(DVec3::splat(f64::NEG_INFINITY), |a, b| a.max(*b));
    let size = (hi - lo).max(DVec3::splat(1e-9));
    let mut out = mesh.points.clone();
    for i in 0..mesh.points.len() {
        let ctx = ShadeCtx {
            uvw: uvw(mesh.points[i], lo, size, p),
            position: mesh.points[i].to_array(),
            normal: normals[i].to_array(),
            view: [0.0; 3],
            time,
            vertex_weight: None,
            ao: None,
        };
        let c = shader.sample(&ctx);
        let along = match p.kind {
            Kind::ObjectRgb | Kind::WorldRgb => {
                // Per-axis: object and world axes coincide in the deformer's local space.
                let mut d = DVec3::from_array(c) * p.height;
                if p.invert {
                    d = -d;
                }
                out[i] += d * p.strength;
                continue;
            }
            _ => scalar(p.kind, c),
        };
        let along = if p.invert { -along } else { along } * p.height * p.strength;
        let dir = match p.direction {
            Direction::Normal => {
                if normals[i].length_squared() > 0.0 {
                    normals[i]
                } else {
                    DVec3::Y
                }
            }
            Direction::Spherical => mesh.points[i].normalize_or(DVec3::Y),
            Direction::Planar => DVec3::Y,
        };
        out[i] += dir * along;
    }
    mesh.points = out;
}

#[cfg(test)]
mod tests {
    use super::*;
    use forge_shaders::build::{Inputs, Values};

    fn flat_grid() -> PolyMesh {
        PolyMesh {
            points: vec![
                DVec3::new(-1.0, -1.0, 0.0),
                DVec3::new(1.0, -1.0, 0.0),
                DVec3::new(1.0, 1.0, 0.0),
                DVec3::new(-1.0, 1.0, 0.0),
            ],
            polys: vec![vec![0, 1, 2, 3]],
        }
    }

    #[test]
    fn without_a_shader_nothing_moves() {
        let mut mesh = flat_grid();
        let before = mesh.points.clone();
        displace(
            &mut mesh,
            None,
            &DisplacerParams {
                height: 20.0,
                ..Default::default()
            },
            0.0,
        );
        assert_eq!(mesh.points, before);
    }

    #[test]
    fn a_constant_shader_moves_along_the_normal() {
        let mut mesh = flat_grid();
        let shader =
            forge_shaders::build("forge.shader.color", &Values::new(), &Inputs::default()).unwrap();
        displace(
            &mut mesh,
            Some(&shader),
            &DisplacerParams {
                height: 20.0,
                ..Default::default()
            },
            0.0,
        );
        // The Color shader's 0.8 sRGB default is linear 0.603827; centered gives
        // (2 * 0.603827 - 1) * 20 = 4.153093 along +Z.
        for p in &mesh.points {
            assert!((p.z - 4.153093554213507).abs() < 1e-9, "{p:?}");
        }
    }
}
