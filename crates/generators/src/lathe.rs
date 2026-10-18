//! Orientation and the lathe shared by the round primitives
//! (docs/specs/PRIMITIVES2.md §1–2).

use forge_core::ParamDesc;
use forge_geometry::PolyMesh;
use forge_geometry::triangulate::triangulate_poly;
use glam::{DVec2, DVec3};

use crate::Params;

/// The `orientation` enum param (PRIMITIVES2.md §1).
pub(crate) fn orientation_param() -> ParamDesc {
    ParamDesc::enumeration(
        "orientation",
        "Orientation",
        "pos_y",
        &[
            ("pos_x", "+X"),
            ("neg_x", "-X"),
            ("pos_y", "+Y"),
            ("neg_y", "-Y"),
            ("pos_z", "+Z"),
            ("neg_z", "-Z"),
        ],
    )
}

/// Maps a point from the `pos_y` frame to the given orientation.
pub(crate) fn orient(key: &str, p: DVec3) -> DVec3 {
    match key {
        "neg_y" => DVec3::new(-p.x, -p.y, p.z),
        "pos_x" => DVec3::new(p.y, -p.x, p.z),
        "neg_x" => DVec3::new(-p.y, p.x, p.z),
        "pos_z" => DVec3::new(p.x, -p.z, p.y),
        "neg_z" => DVec3::new(p.x, p.z, -p.y),
        _ => p,
    }
}

/// Applies the node's `orientation` to every point.
pub(crate) fn oriented(p: &Params, mut mesh: PolyMesh) -> PolyMesh {
    let key = p.enumeration("orientation");
    for q in &mut mesh.points {
        *q = orient(&key, *q);
    }
    mesh
}

/// The slice params shared by the round primitives.
pub(crate) fn slice_params() -> [ParamDesc; 5] {
    [
        ParamDesc::bool("slice", "Slice", false),
        ParamDesc::float("slice_from", "Slice from", 0.0).unit(forge_core::Unit::Angle),
        ParamDesc::float("slice_to", "Slice to", std::f64::consts::TAU)
            .unit(forge_core::Unit::Angle),
        ParamDesc::bool("regular_grid", "Regular grid", false),
        ParamDesc::float("regular_grid_width", "Grid width", 10.0)
            .range(0.0, f64::MAX)
            .unit(forge_core::Unit::Distance),
    ]
}

/// A profile in the (r, y) half-plane (PRIMITIVES2.md §2).
pub(crate) struct Profile {
    pub points: Vec<DVec2>,
    pub closed: bool,
    pub south: Option<f64>,
    pub north: Option<f64>,
}

impl Profile {
    pub(crate) fn open(points: Vec<DVec2>, south: Option<f64>, north: Option<f64>) -> Self {
        let on_pole = |q: &DVec2, pole: Option<f64>| {
            pole.is_some_and(|y| q.x.abs() < 1e-12 && (q.y - y).abs() < 1e-12)
        };
        let points = points
            .into_iter()
            .filter(|q| !on_pole(q, south) && !on_pole(q, north))
            .collect();
        Profile {
            points,
            closed: false,
            south,
            north,
        }
    }
}

/// Sweeps `profile` around Y; `slice` is `(from, to)` in radians; `cut` adds the
/// slice end faces.
pub(crate) fn lathe(
    profile: &Profile,
    segments: usize,
    slice: Option<(f64, f64)>,
    cut: bool,
) -> PolyMesh {
    let columns = if slice.is_some() {
        segments + 1
    } else {
        segments
    };
    let angle = |k: usize| match slice {
        Some((from, to)) => from + (to - from) * k as f64 / segments as f64,
        None => std::f64::consts::TAU * k as f64 / segments as f64,
    };
    let mut points = Vec::new();
    let south = profile.south.map(|y| {
        points.push(DVec3::new(0.0, y, 0.0));
        (points.len() - 1) as u32
    });
    let north = profile.north.map(|y| {
        points.push(DVec3::new(0.0, y, 0.0));
        (points.len() - 1) as u32
    });
    let on_axis: Vec<bool> = profile.points.iter().map(|q| q.x.abs() < 1e-12).collect();
    let m = profile.points.len();
    let mut index = vec![vec![0u32; m]; columns];
    for k in 0..columns {
        let (s, c) = angle(k).sin_cos();
        for (j, q) in profile.points.iter().enumerate() {
            if k > 0 && on_axis[j] {
                index[k][j] = index[0][j];
                continue;
            }
            index[k][j] = points.len() as u32;
            points.push(DVec3::new(q.x * c, q.y, q.x * s));
        }
    }
    let c = |k: usize, j: usize| index[k % columns][j % m];

    let mut polys = Vec::new();
    let rows = if profile.closed { m } else { m - 1 };
    for k in 0..segments {
        if let Some(s) = south {
            polys.push(vec![s, c(k, 0), c(k + 1, 0)]);
        }
        for j in 0..rows {
            polys.push(vec![c(k, j), c(k, j + 1), c(k + 1, j + 1), c(k + 1, j)]);
        }
        if let Some(n) = north {
            polys.push(vec![c(k, m - 1), n, c(k + 1, m - 1)]);
        }
    }
    if slice.is_some() && cut {
        for (k, start) in [(0, true), (segments, false)] {
            let mut q: Vec<u32> = south.into_iter().collect();
            q.extend((0..m).map(|j| c(k, j)));
            q.extend(north);
            q.dedup();
            if q.len() == 4 {
                if start {
                    polys.push(vec![q[0], q[3], q[1]]);
                    polys.push(vec![q[1], q[3], q[2]]);
                } else {
                    polys.push(vec![q[0], q[1], q[3]]);
                    polys.push(vec![q[1], q[2], q[3]]);
                }
            } else {
                if start {
                    q.reverse();
                }
                polys.extend(
                    triangulate_poly(&points, &q)
                        .into_iter()
                        .map(|t| t.to_vec()),
                );
            }
        }
    }
    PolyMesh { points, polys }
}

/// The node's slice range, or `Unsupported` for the unobserved regular grid.
pub(crate) fn slice_of(p: &Params) -> Result<Option<(f64, f64)>, crate::BuildError> {
    if !p.bool("slice") {
        return Ok(None);
    }
    if p.bool("regular_grid") {
        return Err(crate::BuildError::Unsupported("slice regular grid"));
    }
    Ok(Some((p.float("slice_from"), p.float("slice_to"))))
}
