//! The Landscape primitive (docs/specs/PRIMITIVES3.md §6; docs/parity/primitives-3.md).
//!
//! Grid topology and x/z positions follow the observation exactly; the heights are our own
//! seeded fractal (forge-noise Perlin), compared statistically.

use forge_core::{Category, NodeType, ParamDesc, Unit};
use forge_geometry::PolyMesh;
use forge_noise::{octave_seed, perlin};
use glam::DVec3;

use crate::lathe::{orientation_param, oriented};
use crate::round::{count, distance};
use crate::{BuildError, Params};

const OCTAVES: usize = 8;
const OFFSET: f64 = 1.2;

fn percent(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, 1.0)
        .unit(Unit::Percent)
}

pub(crate) fn node_type() -> NodeType {
    NodeType::new("forge.landscape", "Landscape", Category::Generator)
        .param(distance("size_x", "Size X", 600.0))
        .param(distance("size_y", "Size Y", 100.0))
        .param(distance("size_z", "Size Z", 600.0))
        .param(count("width_segments", "Width segments", 100, 4))
        .param(count("depth_segments", "Depth segments", 100, 4))
        .param(percent("rough_furrows", "Rough furrows", 0.5))
        .param(percent("fine_furrows", "Fine furrows", 0.5))
        .param(ParamDesc::float("scale", "Scale", 1.0).range(0.0, 10.0))
        .param(percent("sea_level", "Sea level", 0.0))
        .param(percent("plateau_level", "Plateau level", 1.0))
        .param(orientation_param())
        .param(ParamDesc::bool("multifractal", "Multifractal", true))
        .param(ParamDesc::int("seed", "Seed", 0))
        .param(ParamDesc::bool(
            "borders_at_sea_level",
            "Borders at sea level",
            true,
        ))
        .param(ParamDesc::bool("spherical", "Spherical", false))
}

/// Field settings (everything that shapes the heights).
struct Field {
    seed: u32,
    /// Lattice period of the lowest octave, in world units.
    period: f64,
    rough: f64,
    fine: f64,
    multifractal: bool,
}

impl Field {
    fn weights(&self) -> [f64; OCTAVES] {
        std::array::from_fn(|i| match i {
            0 => 1.0,
            1..=3 => self.rough.powi(i as i32),
            _ => self.rough.powi(3) * self.fine.powi(i as i32 - 3),
        })
    }

    /// Raw (unnormalised) field value at a world position (x, z).
    fn raw(&self, x: f64, z: f64, w: &[f64; OCTAVES]) -> f64 {
        let (qx, qy) = (x / self.period, 0.37 + z / self.period);
        let n: [f64; OCTAVES] = std::array::from_fn(|i| {
            let f = (1u32 << i) as f64;
            let o = 17.3 * i as f64;
            perlin::<3>(
                octave_seed(self.seed, i as u32),
                [qx * f + o, qy * f + o, o],
            )
        });
        if self.multifractal {
            let mut s = OFFSET + n[0];
            for i in 1..OCTAVES {
                s += w[i] * (n[i] + OFFSET) * s;
            }
            s.max(0.0)
        } else {
            let sum: f64 = w.iter().sum();
            0.5 + 0.5 * n.iter().zip(w).map(|(a, b)| a * b).sum::<f64>() / sum
        }
    }
}

/// Normalised heights in `[0, 1]` for the grid (before sea level, plateau and the border mask).
fn normalised(field: &Field, xs: &[f64], zs: &[f64]) -> Vec<f64> {
    let w = field.weights();
    let mut v: Vec<f64> = zs
        .iter()
        .flat_map(|&z| xs.iter().map(move |&x| (x, z)))
        .map(|(x, z)| field.raw(x, z, &w))
        .collect();
    let hi = v.iter().cloned().fold(f64::MIN, f64::max);
    let lo = if field.multifractal {
        0.0
    } else {
        v.iter().cloned().fold(f64::MAX, f64::min)
    };
    let span = (hi - lo).max(1e-12);
    for s in &mut v {
        *s = (*s - lo) / span;
    }
    v
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    let (sx, sy, sz) = (p.float("size_x"), p.float("size_y"), p.float("size_z"));
    let (nw, nd) = (p.int("width_segments"), p.int("depth_segments"));
    let (cols, rows) = (nw + 1, nd + 1);
    let seed = match p.get("seed") {
        forge_core::Value::Int(i) => i.rem_euclid(1 << 32) as u32,
        _ => 0,
    };
    let field = Field {
        seed,
        period: 0.3 * sx.max(sz) * p.float("scale").max(0.001),
        rough: p.float("rough_furrows"),
        fine: p.float("fine_furrows"),
        multifractal: p.bool("multifractal"),
    };
    let xs: Vec<f64> = (0..cols)
        .map(|i| -sx / 2.0 + sx * i as f64 / nw as f64)
        .collect();
    let zs: Vec<f64> = (0..rows)
        .map(|k| -sz / 2.0 + sz * k as f64 / nd as f64)
        .collect();
    let base = normalised(&field, &xs, &zs);
    let (sea, plateau) = (p.float("sea_level"), p.float("plateau_level"));
    let borders = p.bool("borders_at_sea_level");
    let spherical = p.bool("spherical");
    let radius = sx.min(sz) / 2.0;
    let mut points = Vec::with_capacity(cols * rows);
    for k in 0..rows {
        let v = k as f64 / nd as f64;
        for i in 0..cols {
            let u = i as f64 / nw as f64;
            let mut w = base[k * cols + i].clamp(sea.min(plateau), plateau);
            w = if sea < 1.0 {
                (w - sea) / (1.0 - sea)
            } else {
                0.0
            };
            if borders {
                let t = u.min(1.0 - u).min(v).min(1.0 - v);
                w *= (std::f64::consts::PI * t).sin().powi(2);
            }
            let y = sy * w.max(0.0);
            points.push(if spherical {
                let (lon, lat) = (std::f64::consts::TAU * u, std::f64::consts::PI * (v - 0.5));
                let r = radius + y;
                DVec3::new(
                    r * lat.cos() * lon.sin(),
                    r * lat.sin(),
                    r * lat.cos() * lon.cos(),
                )
            } else {
                DVec3::new(xs[i], y, zs[k])
            });
        }
    }
    let mut polys = Vec::with_capacity(nw * nd);
    for k in 0..nd {
        for i in 0..nw {
            let a = (k * cols + i) as u32;
            let c = cols as u32;
            polys.push(vec![a, a + c, a + c + 1, a + 1]);
        }
    }
    Ok(oriented(p, PolyMesh { points, polys }))
}
