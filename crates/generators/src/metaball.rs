//! Metaball (docs/specs/GENERATORS3.md §3; docs/parity/generators-3.md, `metaball/*`).
//!
//! The implicit field is the sum of every source's inverse-square falloff
//! `strength * (radius / distance)^2`; the hull value is the iso level. Sphere children
//! contribute a point source at their placed origin, spline children the distance to their
//! sampled polyline. The surface is extracted with the shared cell mesher and each vertex is
//! projected onto the true iso surface (the mesher's cell mean sits inside on curves).

use std::collections::BTreeMap;

use forge_core::{Category, NodeId, NodeType, ParamDesc, Scene, Unit, Value};
use forge_geometry::PolyMesh;
use forge_geometry::isosurface::{Polarity, SparseGrid, volume_to_mesh};
use glam::{DMat4, DVec3};

use crate::{BuildError, Params, splines};

pub const TYPE: &str = "forge.metaball";

/// A child source: a sphere centre or a sampled spline polyline.
enum Source {
    Point {
        centre: DVec3,
        radius: f64,
        strength: f64,
    },
    Line {
        points: Vec<DVec3>,
        radius: f64,
        strength: f64,
    },
}

impl Source {
    fn radius(&self) -> f64 {
        match self {
            Self::Point { radius, .. } | Self::Line { radius, .. } => *radius,
        }
    }

    fn strength(&self) -> f64 {
        match self {
            Self::Point { strength, .. } | Self::Line { strength, .. } => *strength,
        }
    }

    /// The field and its gradient at `p`.
    fn sample(&self, p: DVec3) -> (f64, DVec3) {
        match self {
            Self::Point {
                centre,
                radius,
                strength,
            } => point_field(p, *centre, *radius, *strength),
            Self::Line { points, .. } => {
                let (dist, closest) = distance_to_polyline(p, points);
                let (r2, s) = (self.radius().powi(2), self.strength());
                if dist <= 1e-9 {
                    return (1e18, DVec3::ZERO);
                }
                let v = p - closest;
                let f = s * r2 / (dist * dist);
                let g = -2.0 * s * r2 * v / dist.powi(4);
                (f, g)
            }
        }
    }
}

fn point_field(p: DVec3, centre: DVec3, radius: f64, strength: f64) -> (f64, DVec3) {
    let v = p - centre;
    let d = v.length();
    if d <= 1e-9 {
        return (1e18, DVec3::ZERO);
    }
    let r2 = radius * radius;
    (
        strength * r2 / (d * d),
        -2.0 * strength * r2 * v / d.powi(4),
    )
}

fn distance_to_polyline(p: DVec3, points: &[DVec3]) -> (f64, DVec3) {
    let mut best = (f64::INFINITY, points.first().copied().unwrap_or(p));
    if points.len() == 1 {
        return ((p - points[0]).length(), points[0]);
    }
    for w in points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let ab = b - a;
        let len2 = ab.length_squared();
        let t = if len2 > 0.0 {
            ((p - a).dot(ab) / len2).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let q = a + ab * t;
        let d = (p - q).length();
        if d < best.0 {
            best = (d, q);
        }
    }
    best
}

pub(crate) fn node_type() -> NodeType {
    NodeType::new(TYPE, "Metaball", Category::Generator)
        .menu_path("Generator", 0)
        .param(
            ParamDesc::float("hull_value", "Hull Value", 1.0)
                .range(0.0, 10.0)
                .unit(Unit::Percent),
        )
        .param(ParamDesc::float("editor_subdiv", "Editor Subdivision", 40.0).range(0.001, 1e9))
        .param(ParamDesc::float("render_subdiv", "Render Subdivision", 5.0).range(0.001, 1e9))
        .param(ParamDesc::bool(
            "exponential_falloff",
            "Exponential Falloff",
            false,
        ))
        .param(ParamDesc::bool(
            "accurate_normals",
            "Accurate Normals",
            false,
        ))
}

/// The source children of `id`, placed in its local space.
fn sources(scene: &Scene, id: NodeId) -> Vec<Source> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for child in &node.children {
        let Some(c) = scene.node(*child) else {
            continue;
        };
        let m = scene
            .local_transform(*child)
            .map(|t| t.matrix())
            .unwrap_or(DMat4::IDENTITY);
        if c.type_id == "forge.sphere" {
            let radius = match c.params.get("radius") {
                Some(Value::Float(r)) => *r,
                _ => 100.0,
            };
            let centre = m.transform_point3(DVec3::ZERO);
            out.push(Source::Point {
                centre,
                radius,
                strength: 1.0,
            });
        } else if let Some(line) = splines::polyline_mesh(scene, *child) {
            let points: Vec<DVec3> = line.points.iter().map(|p| m.transform_point3(*p)).collect();
            let radius = match c.params.get("radius") {
                Some(Value::Float(r)) => *r,
                _ => 100.0,
            };
            out.push(Source::Line {
                points,
                radius,
                strength: 1.0,
            });
            continue;
        }
        // not a source: ignored
    }
    out
}

/// True when `child` is a source of the metaball `parent` (it is consumed, not drawn).
pub fn is_source(scene: &Scene, parent: NodeId, child: NodeId) -> bool {
    scene
        .node(parent)
        .is_some_and(|n| n.children.contains(&child))
        && scene
            .node(child)
            .is_some_and(|c| c.type_id == "forge.sphere" || splines::is_spline(&c.type_id))
}

fn field(sources: &[Source], p: DVec3) -> (f64, DVec3) {
    let mut f = 0.0;
    let mut g = DVec3::ZERO;
    for s in sources {
        let (a, b) = s.sample(p);
        f += a;
        g += b;
    }
    (f, g)
}

/// Meshes the field of the source children: the hull value is the iso level and the editor
/// subdivision the lattice spacing.
pub fn evaluate(
    scene: &Scene,
    id: NodeId,
    values: &BTreeMap<String, Value>,
) -> Result<Option<PolyMesh>, BuildError> {
    let ty = node_type();
    let p = Params { ty: &ty, values };
    let iso = p.float("hull_value");
    let h = p.float("editor_subdiv");
    if !(h.is_finite() && h > 0.0) {
        return Err(BuildError::Failed(
            "the metaball subdivision must be positive".into(),
        ));
    }
    if !(iso.is_finite() && iso > 0.0) {
        return Ok(None);
    }
    let sources = sources(scene, id);
    if sources.is_empty() {
        return Ok(None);
    }
    // Outside every source's own reach every contribution is below iso / n, so the surface
    // lies within the union of those spheres.
    let n = sources.len() as f64;
    let reach: Vec<f64> = sources
        .iter()
        .map(|s| s.radius() * (s.strength() * n / iso).max(0.0).sqrt())
        .collect();
    if reach.iter().any(|r| !r.is_finite()) {
        return Ok(None);
    }
    let (mut lo, mut hi) = (DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY));
    for (s, r) in sources.iter().zip(&reach) {
        match s {
            Source::Point { centre, .. } => {
                lo = lo.min(*centre - DVec3::splat(*r));
                hi = hi.max(*centre + DVec3::splat(*r));
            }
            Source::Line { points, .. } => {
                for q in points {
                    lo = lo.min(*q - DVec3::splat(*r));
                    hi = hi.max(*q + DVec3::splat(*r));
                }
            }
        }
    }
    let i0 = (lo / h).floor() - DVec3::splat(1.0);
    let i1 = (hi / h).ceil() + DVec3::splat(1.0);
    let dims = (i1 - i0 + DVec3::ONE).as_ivec3();
    let cells = f64::from(dims.x) * f64::from(dims.y) * f64::from(dims.z);
    if cells > 8_000_000.0 {
        return Err(BuildError::Failed(
            "the metaball grid is too large: raise Editor Subdivision".into(),
        ));
    }
    let mut grid = SparseGrid::new(0.0f32, h, DVec3::ZERO);
    for x in i0.x as i32..=i1.x as i32 {
        for y in i0.y as i32..=i1.y as i32 {
            for z in i0.z as i32..=i1.z as i32 {
                let q = DVec3::new(x as f64, y as f64, z as f64) * h;
                let (f, _) = field(&sources, q);
                grid.set(glam::IVec3::new(x, y, z), f as f32);
            }
        }
    }
    let mut mesh = volume_to_mesh(&grid, iso, Polarity::HighInside);
    // The cell mean lies inside the true surface on curved parts; project it back with two
    // Newton steps on the analytic field.
    for point in &mut mesh.points {
        for _ in 0..2 {
            let (f, g) = field(&sources, *point);
            let gg = g.length_squared();
            if gg > 1e-24 {
                *point += (iso - f) * g / gg;
            }
        }
    }
    Ok(Some(mesh))
}
