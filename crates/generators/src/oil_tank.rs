//! Oil Tank (docs/specs/PRIMITIVES3.md §2; docs/parity/primitives-3.md).

use std::f64::consts::PI;

use forge_core::{Category, NodeType};
use forge_geometry::PolyMesh;
use glam::DVec2;

use crate::lathe::{Profile, lathe, orientation_param, oriented, slice_of, slice_params};
use crate::round::{count, distance};
use crate::{BuildError, Params};

/// Arc angle of the elliptical cap, fitted on `oil-tank/defaults` (PRIMITIVES3.md §2).
const CAP_ARC: f64 = PI / 8.0;

pub(crate) fn node_type() -> NodeType {
    let mut t = NodeType::new("forge.oil_tank", "Oil Tank", Category::Generator)
        .param(distance("radius", "Radius", 100.0))
        .param(distance("height", "Height", 200.0))
        .param(count("height_segments", "Height segments", 4, 1))
        .param(distance("cap_height", "Cap height", 25.0))
        .param(count("cap_segments", "Cap segments", 4, 1))
        .param(count("rotation_segments", "Rotation segments", 16, 3))
        .param(orientation_param());
    for p in slice_params() {
        t = t.param(p);
    }
    t
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    let (r, height) = (p.float("radius"), p.float("height"));
    let (hs, cs) = (p.int("height_segments"), p.int("cap_segments"));
    let cap = p.float("cap_height").min(height / 2.0);
    let c = (height / 2.0 - cap).max(0.0);
    // Point k of the cap, measured from the pole: (radius, offset along the axis).
    let arc = |k: usize| {
        let t = CAP_ARC * k as f64 / cs as f64;
        DVec2::new(
            r * t.sin() / CAP_ARC.sin(),
            cap * (1.0 - t.cos()) / (1.0 - CAP_ARC.cos()),
        )
    };
    let mut pts: Vec<DVec2> = (1..=cs)
        .map(|k| {
            let q = arc(k);
            DVec2::new(q.x, -c - cap + q.y)
        })
        .collect();
    pts.extend((1..hs).map(|j| DVec2::new(r, -c + 2.0 * c * j as f64 / hs as f64)));
    pts.extend((1..=cs).rev().map(|k| {
        let q = arc(k);
        DVec2::new(q.x, c + cap - q.y)
    }));
    let profile = Profile::open(pts, Some(-c - cap), Some(c + cap));
    let mesh = lathe(&profile, p.int("rotation_segments"), slice_of(p)?, true);
    Ok(oriented(p, mesh))
}
