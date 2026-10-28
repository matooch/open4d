//! Cylinder, cone, disc, torus, tube and capsule (docs/specs/PRIMITIVES2.md §3–8).

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use forge_core::{Category, NodeType, ParamDesc, Unit};
use forge_geometry::PolyMesh;
use glam::DVec2;

use crate::lathe::{Profile, lathe, orientation_param, oriented, slice_of, slice_params};
use crate::{BuildError, Params};

fn distance(id: &str, label: &str, default: f64) -> ParamDesc {
    ParamDesc::float(id, label, default)
        .range(0.0, f64::MAX)
        .unit(Unit::Distance)
}

fn count(id: &str, label: &str, default: i64, min: i64) -> ParamDesc {
    ParamDesc::int(id, label, default).range(min, 1000)
}

fn with_slice(mut t: NodeType) -> NodeType {
    for p in slice_params() {
        t = t.param(p);
    }
    t
}

pub(crate) fn cylinder_type() -> NodeType {
    with_slice(
        NodeType::new("forge.cylinder", "Cylinder", Category::Generator)
            .param(distance("radius", "Radius", 50.0))
            .param(distance("height", "Height", 200.0))
            .param(count("height_segments", "Height segments", 4, 1))
            .param(count("rotation_segments", "Rotation segments", 16, 3))
            .param(orientation_param())
            .param(ParamDesc::bool("caps", "Caps", true))
            .param(count("cap_segments", "Cap segments", 1, 1))
            .param(ParamDesc::bool("fillet", "Fillet", false))
            .param(count("fillet_segments", "Fillet segments", 3, 1))
            .param(distance("fillet_radius", "Fillet radius", 3.0)),
    )
}

pub(crate) fn cone_type() -> NodeType {
    with_slice(
        NodeType::new("forge.cone", "Cone", Category::Generator)
            .param(distance("top_radius", "Top radius", 0.0))
            .param(distance("bottom_radius", "Bottom radius", 100.0))
            .param(distance("height", "Height", 200.0))
            .param(count("height_segments", "Height segments", 4, 1))
            .param(count("rotation_segments", "Rotation segments", 16, 3))
            .param(orientation_param())
            .param(ParamDesc::bool("caps", "Caps", true))
            .param(count("cap_segments", "Cap segments", 1, 1))
            .param(count("fillet_segments", "Fillet segments", 3, 1))
            .param(ParamDesc::bool("fillet_top", "Top fillet", false))
            .param(distance("fillet_top_radius", "Top fillet radius", 50.0))
            .param(distance("fillet_top_height", "Top fillet height", 50.0))
            .param(ParamDesc::bool("fillet_bottom", "Bottom fillet", false))
            .param(distance(
                "fillet_bottom_radius",
                "Bottom fillet radius",
                50.0,
            ))
            .param(distance(
                "fillet_bottom_height",
                "Bottom fillet height",
                50.0,
            )),
    )
}

pub(crate) fn disc_type() -> NodeType {
    NodeType::new("forge.disc", "Disc", Category::Generator)
        .param(distance("inner_radius", "Inner radius", 0.0))
        .param(distance("outer_radius", "Outer radius", 100.0))
        .param(count("disc_segments", "Disc segments", 1, 1))
        .param(count("rotation_segments", "Rotation segments", 16, 3))
        .param(orientation_param())
        .param(ParamDesc::bool("slice", "Slice", false))
        .param(ParamDesc::float("slice_from", "Slice from", 0.0).unit(Unit::Angle))
        .param(ParamDesc::float("slice_to", "Slice to", PI).unit(Unit::Angle))
}

pub(crate) fn torus_type() -> NodeType {
    with_slice(
        NodeType::new("forge.torus", "Torus", Category::Generator)
            .param(distance("ring_radius", "Ring radius", 100.0))
            .param(count("ring_segments", "Ring segments", 32, 3))
            .param(distance("pipe_radius", "Pipe radius", 50.0))
            .param(count("pipe_segments", "Pipe segments", 16, 3))
            .param(orientation_param()),
    )
}

pub(crate) fn tube_type() -> NodeType {
    with_slice(
        NodeType::new("forge.tube", "Tube", Category::Generator)
            .param(distance("inner_radius", "Inner radius", 50.0))
            .param(distance("outer_radius", "Outer radius", 100.0))
            .param(count("rotation_segments", "Rotation segments", 16, 3))
            .param(count("cap_segments", "Cap segments", 1, 1))
            .param(distance("height", "Height", 100.0))
            .param(count("height_segments", "Height segments", 4, 1))
            .param(orientation_param())
            .param(ParamDesc::bool("fillet", "Fillet", false))
            .param(count("fillet_segments", "Fillet segments", 8, 1))
            .param(distance("fillet_radius", "Fillet radius", 20.0)),
    )
}

pub(crate) fn capsule_type() -> NodeType {
    with_slice(
        NodeType::new("forge.capsule", "Capsule", Category::Generator)
            .param(distance("radius", "Radius", 50.0))
            .param(distance("height", "Height", 200.0))
            .param(count("height_segments", "Height segments", 4, 1))
            .param(count("cap_segments", "Cap segments", 4, 1))
            .param(count("rotation_segments", "Rotation segments", 16, 3))
            .param(orientation_param()),
    )
}

/// The cone-family profile (PRIMITIVES2.md §4).
struct ConeShape {
    bottom: f64,
    top: f64,
    height: f64,
    height_segments: usize,
    caps: bool,
    cap_segments: usize,
    fillet_segments: usize,
    /// (radius, height) of each fillet, zero when off
    fillet_bottom: (f64, f64),
    fillet_top: (f64, f64),
}

fn cone_profile(c: &ConeShape) -> Profile {
    let h = c.height / 2.0;
    let slope = if c.height > 0.0 {
        (c.top - c.bottom) / c.height
    } else {
        0.0
    };
    let (fb, gb) = (c.fillet_bottom.0.min(c.bottom), c.fillet_bottom.1.min(h));
    let (ft, gt) = (c.fillet_top.0.min(c.top), c.fillet_top.1.min(h));
    let (cs, fs) = (c.cap_segments as f64, c.fillet_segments);
    let theta = |t: usize| t as f64 * FRAC_PI_2 / fs as f64;
    let mut pts = Vec::new();
    if c.caps {
        pts.extend((1..=c.cap_segments).map(|i| DVec2::new((c.bottom - fb) * i as f64 / cs, -h)));
    } else {
        pts.push(DVec2::new(c.bottom, -h));
    }
    if gb > 0.0 || fb > 0.0 {
        pts.extend((1..=fs).map(|t| {
            let (s, co) = theta(t).sin_cos();
            DVec2::new(
                c.bottom - fb + fb * s + slope * gb * (1.0 - co),
                -h + gb * (1.0 - co),
            )
        }));
    }
    let (y0, y1) = (-h + gb, h - gt);
    pts.extend((1..=c.height_segments).map(|j| {
        let y = y0 + (y1 - y0) * j as f64 / c.height_segments as f64;
        DVec2::new(c.bottom + slope * (y + h), y)
    }));
    if gt > 0.0 || ft > 0.0 {
        pts.extend((0..fs).rev().map(|t| {
            let (s, co) = theta(t).sin_cos();
            DVec2::new(
                c.top - ft + ft * s - slope * gt * (1.0 - co),
                h - gt * (1.0 - co),
            )
        }));
    }
    let north = c.caps && c.top > 0.0;
    if north {
        pts.extend(
            (1..c.cap_segments)
                .rev()
                .map(|i| DVec2::new((c.top - ft) * i as f64 / cs, h)),
        );
    }
    Profile::open(pts, c.caps.then_some(-h), north.then_some(h))
}

pub(crate) fn build_cylinder(p: &Params) -> Result<PolyMesh, BuildError> {
    let (r, height) = (p.float("radius"), p.float("height"));
    let f = if p.bool("fillet") {
        p.float("fillet_radius").clamp(0.0, r.min(height / 2.0))
    } else {
        0.0
    };
    let caps = p.bool("caps");
    let f = if caps { f } else { 0.0 };
    let profile = cone_profile(&ConeShape {
        bottom: r,
        top: r,
        height,
        height_segments: p.int("height_segments"),
        caps,
        cap_segments: p.int("cap_segments"),
        fillet_segments: p.int("fillet_segments"),
        fillet_bottom: (f, f),
        fillet_top: (f, f),
    });
    finish(p, &profile, "rotation_segments", true)
}

pub(crate) fn build_cone(p: &Params) -> Result<PolyMesh, BuildError> {
    let caps = p.bool("caps");
    let fillet = |on: &str, r: &str, h: &str| {
        if caps && p.bool(on) {
            (p.float(r), p.float(h))
        } else {
            (0.0, 0.0)
        }
    };
    let profile = cone_profile(&ConeShape {
        bottom: p.float("bottom_radius"),
        top: p.float("top_radius"),
        height: p.float("height"),
        height_segments: p.int("height_segments"),
        caps,
        cap_segments: p.int("cap_segments"),
        fillet_segments: p.int("fillet_segments"),
        fillet_bottom: fillet(
            "fillet_bottom",
            "fillet_bottom_radius",
            "fillet_bottom_height",
        ),
        fillet_top: fillet("fillet_top", "fillet_top_radius", "fillet_top_height"),
    });
    finish(p, &profile, "rotation_segments", true)
}

pub(crate) fn build_disc(p: &Params) -> Result<PolyMesh, BuildError> {
    let (ri, ro, ds) = (
        p.float("inner_radius"),
        p.float("outer_radius"),
        p.int("disc_segments"),
    );
    let pts = (0..=ds)
        .map(|i| DVec2::new(ro - (ro - ri) * i as f64 / ds as f64, 0.0))
        .collect();
    let profile = Profile::open(pts, None, (ri <= 0.0).then_some(0.0));
    let slice = p
        .bool("slice")
        .then(|| (p.float("slice_from"), p.float("slice_to")));
    let mesh = lathe(&profile, p.int("rotation_segments"), slice, false);
    Ok(oriented(p, mesh))
}

pub(crate) fn build_torus(p: &Params) -> Result<PolyMesh, BuildError> {
    let (rr, pr, ps) = (
        p.float("ring_radius"),
        p.float("pipe_radius"),
        p.int("pipe_segments"),
    );
    let points = (0..ps)
        .map(|j| {
            let (s, c) = (TAU * j as f64 / ps as f64).sin_cos();
            DVec2::new(rr + pr * c, pr * s)
        })
        .collect();
    let profile = Profile {
        points,
        closed: true,
        south: None,
        north: None,
    };
    finish(p, &profile, "ring_segments", true)
}

pub(crate) fn build_tube(p: &Params) -> Result<PolyMesh, BuildError> {
    let (ri, ro, height) = (
        p.float("inner_radius"),
        p.float("outer_radius"),
        p.float("height"),
    );
    let h = height / 2.0;
    let (f, fs) = if p.bool("fillet") {
        (
            p.float("fillet_radius")
                .clamp(0.0, ((ro - ri) / 2.0).min(h).max(0.0)),
            p.int("fillet_segments"),
        )
    } else {
        (0.0, 1)
    };
    let (cs, hs) = (p.int("cap_segments"), p.int("height_segments"));
    // Sides: start (fillet-shortened), step to the end, outward normal, steps.
    let sides = [
        (
            DVec2::new(ri + f, -h),
            DVec2::new(ro - ri - 2.0 * f, 0.0),
            -DVec2::Y,
            cs,
        ),
        (
            DVec2::new(ro, -h + f),
            DVec2::new(0.0, height - 2.0 * f),
            DVec2::X,
            hs,
        ),
        (
            DVec2::new(ro - f, h),
            DVec2::new(-(ro - ri - 2.0 * f), 0.0),
            DVec2::Y,
            cs,
        ),
        (
            DVec2::new(ri, h - f),
            DVec2::new(0.0, -(height - 2.0 * f)),
            -DVec2::X,
            hs,
        ),
    ];
    let mut points = Vec::new();
    for k in 0..4 {
        let (start, step, normal, steps) = sides[k];
        points.extend((0..steps).map(|i| start + step * (i as f64 / steps as f64)));
        if f == 0.0 {
            continue; // the corner is the next side's start
        }
        let end = start + step;
        let next = sides[(k + 1) % 4].2;
        let centre = end - normal * f;
        points.extend((0..fs).map(|t| {
            let (s, c) = (t as f64 * FRAC_PI_2 / fs as f64).sin_cos();
            centre + (normal * c + next * s) * f
        }));
    }
    let profile = Profile {
        points,
        closed: true,
        south: None,
        north: None,
    };
    finish(p, &profile, "rotation_segments", true)
}

pub(crate) fn build_capsule(p: &Params) -> Result<PolyMesh, BuildError> {
    let (r, height) = (p.float("radius"), p.float("height"));
    let (hs, cs) = (p.int("height_segments"), p.int("cap_segments"));
    let c = (height / 2.0 - r).max(0.0);
    let theta = |k: usize| k as f64 * FRAC_PI_2 / cs as f64;
    let mut pts: Vec<DVec2> = (1..=cs)
        .map(|k| DVec2::new(r * theta(k).sin(), -c - r * theta(k).cos()))
        .collect();
    pts.extend((1..hs).map(|j| DVec2::new(r, -c + 2.0 * c * j as f64 / hs as f64)));
    pts.extend(
        (1..=cs)
            .rev()
            .map(|k| DVec2::new(r * theta(k).sin(), c + r * theta(k).cos())),
    );
    let profile = Profile::open(pts, Some(-c - r), Some(c + r));
    finish(p, &profile, "rotation_segments", true)
}

fn finish(
    p: &Params,
    profile: &Profile,
    segments: &str,
    cut: bool,
) -> Result<PolyMesh, BuildError> {
    let mesh = lathe(profile, p.int(segments), slice_of(p)?, cut);
    Ok(oriented(p, mesh))
}
