//! Figure: a procedural T-pose mannequin (docs/specs/PRIMITIVES3.md §4).
//!
//! Our own design from public figure-drawing proportions (the eight-head canon). Nothing
//! of the reference's mesh is read or tabulated: only its overall size is a test target.

use std::f64::consts::FRAC_PI_2;

use forge_core::{Category, NodeType};
use forge_geometry::PolyMesh;
use glam::{DQuat, DVec2, DVec3};

use crate::lathe::{Profile, lathe};
use crate::round::{count, distance};
use crate::{BuildError, Params};

pub(crate) fn node_type() -> NodeType {
    NodeType::new("forge.figure", "Figure", Category::Generator)
        .param(distance("height", "Height", 180.0))
        .param(count("segments", "Segments", 14, 4))
}

/// Appends `part` to `mesh`.
fn merge(mesh: &mut PolyMesh, part: PolyMesh) {
    let offset = mesh.points.len() as u32;
    mesh.points.extend(part.points);
    mesh.polys.extend(
        part.polys
            .into_iter()
            .map(|f| f.into_iter().map(|v| v + offset).collect()),
    );
}

/// Round cap rings and columns shared by the round parts.
struct Detail {
    columns: usize,
    rings: usize,
}

/// A capsule from `a` to `b` whose end spheres have radii `ra` and `rb`.
fn limb(a: DVec3, b: DVec3, ra: f64, rb: f64, d: &Detail) -> PolyMesh {
    let length = (b - a).length();
    let theta = |k: usize| k as f64 * FRAC_PI_2 / d.rings as f64;
    let mut pts: Vec<DVec2> = (1..=d.rings)
        .map(|k| DVec2::new(ra * theta(k).sin(), -ra * theta(k).cos()))
        .collect();
    pts.extend(
        (1..=d.rings)
            .rev()
            .map(|k| DVec2::new(rb * theta(k).sin(), length + rb * theta(k).cos())),
    );
    let profile = Profile::open(pts, Some(-ra), Some(length + rb));
    let mut mesh = lathe(&profile, d.columns, None, false);
    let turn = DQuat::from_rotation_arc(DVec3::Y, (b - a) / length);
    for q in &mut mesh.points {
        *q = a + turn * *q;
    }
    mesh
}

/// An ellipsoid with the given centre and radii.
fn ellipsoid(centre: DVec3, radii: DVec3, d: &Detail) -> PolyMesh {
    let steps = 2 * d.rings;
    let pts = (1..steps)
        .map(|k| {
            let t = k as f64 * FRAC_PI_2 / d.rings as f64;
            DVec2::new(t.sin(), -t.cos())
        })
        .collect();
    let profile = Profile::open(pts, Some(-1.0), Some(1.0));
    let mut mesh = lathe(&profile, d.columns, None, false);
    for q in &mut mesh.points {
        *q = centre + *q * radii;
    }
    mesh
}

/// An axis-aligned box, faces wound outwards.
fn block(min: DVec3, max: DVec3) -> PolyMesh {
    let c = |x: bool, y: bool, z: bool| {
        DVec3::new(
            if x { max.x } else { min.x },
            if y { max.y } else { min.y },
            if z { max.z } else { min.z },
        )
    };
    PolyMesh {
        points: vec![
            c(false, false, false),
            c(true, false, false),
            c(true, true, false),
            c(false, true, false),
            c(false, false, true),
            c(true, false, true),
            c(true, true, true),
            c(false, true, true),
        ],
        polys: vec![
            vec![0, 3, 2, 1],
            vec![4, 5, 6, 7],
            vec![0, 1, 5, 4],
            vec![3, 7, 6, 2],
            vec![0, 4, 7, 3],
            vec![1, 2, 6, 5],
        ],
    }
}

pub(crate) fn build(p: &Params) -> Result<PolyMesh, BuildError> {
    let h = p.float("height");
    let columns = p.int("segments");
    let d = Detail {
        columns,
        rings: (columns / 4).max(2),
    };
    let at = |x: f64, y: f64| DVec3::new(x * h, y * h, 0.0);
    let mut mesh = PolyMesh::default();

    merge(
        &mut mesh,
        ellipsoid(at(0.0, 0.9375), DVec3::new(0.055, 0.0625, 0.06) * h, &d),
    );
    merge(
        &mut mesh,
        limb(at(0.0, 0.84), at(0.0, 0.89), 0.032 * h, 0.028 * h, &d),
    );
    let mut torso = limb(at(0.0, 0.52), at(0.0, 0.78), 0.075 * h, 0.1 * h, &d);
    for q in &mut torso.points {
        q.z *= 0.6;
    }
    merge(&mut mesh, torso);

    for side in [-1.0, 1.0] {
        let x = |v: f64| side * v;
        // arms along the X axis at shoulder height
        merge(
            &mut mesh,
            limb(at(x(0.1), 0.8), at(x(0.3), 0.8), 0.04 * h, 0.032 * h, &d),
        );
        merge(
            &mut mesh,
            limb(at(x(0.3), 0.8), at(x(0.46), 0.8), 0.032 * h, 0.022 * h, &d),
        );
        let (x0, x1) = (x(0.46) * h, x(0.5085) * h);
        merge(
            &mut mesh,
            block(
                DVec3::new(x0.min(x1), 0.778 * h, -0.025 * h),
                DVec3::new(x0.max(x1), 0.822 * h, 0.025 * h),
            ),
        );
        // legs
        merge(
            &mut mesh,
            limb(
                at(x(0.055), 0.5),
                at(x(0.055), 0.28),
                0.055 * h,
                0.04 * h,
                &d,
            ),
        );
        merge(
            &mut mesh,
            limb(
                at(x(0.055), 0.28),
                at(x(0.05), 0.06),
                0.04 * h,
                0.03 * h,
                &d,
            ),
        );
        let fx = x(0.05) * h;
        merge(
            &mut mesh,
            block(
                DVec3::new(fx - 0.035 * h, 0.0, -0.065 * h),
                DVec3::new(fx + 0.035 * h, 0.05 * h, 0.107 * h),
            ),
        );
    }
    Ok(mesh)
}
