//! Effectors that need generator inputs (clone-toolset spec E25).

use std::collections::BTreeSet;

use forge_clone_engine::extended::{
    INHERITANCE, PUSH_APART, SHADER, SPLINE, VOLUME, apply_sampled, blend_pose, fields,
};
use forge_clone_engine::{CloneData, CloneError, EffectorCtx, apply_effectors_with};
use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_geometry::HalfEdgeMesh;
use forge_geometry::triangulate::triangulate;
use forge_noise::Noise;
use glam::{DMat4, DVec3};

use crate::spline_sampling::{ArcLengths, Interpolation, uniform_params};
use crate::splines::{Curve, world_curve};

pub(crate) fn apply_effectors(
    scene: &Scene,
    cloner: NodeId,
    data: &mut CloneData,
    eval: &EvalCtx,
) -> Result<(), CloneError> {
    apply_effectors_with(scene, cloner, data, eval, &|id, data, ctx| {
        let Some(n) = scene.node(id) else {
            return Ok(false);
        };
        let e = Params { scene, id };
        if !matches!(
            n.type_id.as_str(),
            SHADER | SPLINE | PUSH_APART | INHERITANCE | VOLUME
        ) {
            return Ok(false);
        }
        if !e.enabled("enabled", true) {
            return Ok(true);
        }
        match n.type_id.as_str() {
            SHADER => shader(e, data, ctx)?,
            SPLINE => spline(e, data, ctx)?,
            PUSH_APART => push_apart(e, data, ctx)?,
            INHERITANCE => inherit(e, cloner, data, ctx)?,
            VOLUME => volume(e, cloner, data, ctx)?,
            _ => unreachable!(),
        }
        Ok(true)
    })
}

#[derive(Clone, Copy)]
struct Params<'a> {
    scene: &'a Scene,
    id: NodeId,
}

impl Params<'_> {
    fn get(&self, k: &str) -> Option<&Value> {
        self.scene.param(self.id, k)
    }
    fn number(&self, k: &str, default: f64) -> f64 {
        match self.get(k) {
            Some(Value::Float(v)) => *v,
            Some(Value::Int(v)) => *v as f64,
            _ => default,
        }
    }
    fn vector(&self, k: &str, default: DVec3) -> DVec3 {
        match self.get(k) {
            Some(Value::Vec3(v)) => DVec3::from_array(*v),
            _ => default,
        }
    }
    fn option(&self, k: &str) -> &str {
        match self.get(k) {
            Some(Value::Enum(v)) => v,
            _ => "",
        }
    }
    fn enabled(&self, k: &str, default: bool) -> bool {
        match self.get(k) {
            Some(Value::Bool(v)) => *v,
            _ => default,
        }
    }
    fn link(&self, k: &str) -> Option<NodeId> {
        match self.get(k) {
            Some(Value::Link(id)) => *id,
            _ => None,
        }
    }
}

fn shader(e: Params<'_>, data: &mut CloneData, ctx: &EffectorCtx<'_>) -> Result<(), CloneError> {
    let Some(id) = e.link("shader") else {
        return Ok(());
    };
    if e.scene
        .node(id)
        .is_none_or(|n| n.type_id != "forge.shader.noise")
    {
        return Err(CloneError::Unsupported(
            "shader effector graph other than Noise",
        ));
    }
    let shader = Params { scene: e.scene, id };
    let noise = Noise {
        seed: shader.number("seed", 665.0) as u32,
        octaves: (shader.number("octaves", 5.0) as u32).clamp(1, 16),
        scale: (shader.number("global_scale", 1.0) * 0.1).max(1e-9),
        time: shader.number("animation_speed", 0.0) * ctx.eval.time,
        ..Noise::default()
    };
    let rel = shader
        .vector("relative_scale", DVec3::ONE)
        .max(DVec3::splat(1e-9));
    let offset = DVec3::new(e.number("offsetu", 0.0), e.number("offsetv", 0.0), 0.0);
    let length = DVec3::new(e.number("lengthu", 1.0), e.number("lengthv", 1.0), 1.0);
    apply_sampled(e.scene, e.id, data, ctx, |point| {
        let uv = point.uvw * length + offset;
        let p = if matches!(shader.option("space"), "world" | "object") {
            point.world
        } else if e.enabled("tile", true) {
            uv - uv.floor()
        } else {
            uv.clamp(DVec3::ZERO, DVec3::ONE)
        };
        let v = (0.5 + 0.5 * noise.sample(p / rel)).clamp(0.0, 1.0);
        if e.enabled("invert", false) {
            1.0 - v
        } else {
            v
        }
    })
}

fn spline(e: Params<'_>, data: &mut CloneData, ctx: &EffectorCtx<'_>) -> Result<(), CloneError> {
    let Some(path) = e.link("spline").and_then(|id| world_curve(e.scene, id)) else {
        return Ok(());
    };
    if ctx.cloner_world.determinant() == 0.0 {
        return Ok(());
    }
    let inv = ctx.cloner_world.inverse();
    let path = path.transformed(&inv);
    let Some(table) = ArcLengths::new(&path) else {
        return Ok(());
    };
    let end_fraction = 1.0 - 1e-4 - 5e-10 * path.segments.len() as f64;
    let samples =
        (!path.closed && path.sampling.mode == Interpolation::Uniform && path.sampling.number > 0)
            .then(|| {
                uniform_params(&path, path.sampling.number)
                    .into_iter()
                    .map(|s| path.at(s))
                    .collect::<Vec<_>>()
            });
    let rail = e
        .link("rail")
        .and_then(|id| world_curve(e.scene, id))
        .map(|c| c.transformed(&inv))
        .and_then(|c| ArcLengths::new(&c).map(|table| (c, table)));
    let start = e.number("start", 0.0);
    let span = e.number("end", 1.0) - start;
    let offset = e.number("offset", 0.0);
    let fields = fields(e.scene, e.id)?;
    let n = data.len() as f64;
    let channels = [
        e.enabled("position", true),
        e.enabled("rotation", true),
        false,
    ];
    for (i, m) in data.matrices.iter_mut().enumerate() {
        let p = m.w_axis.truncate();
        let sample = fields
            .sample(ctx.cloner_world.transform_point3(p), ctx.eval)
            .value;
        let step = i as f64 / n;
        let mode = e.option("mode");
        let f = match mode {
            "falloff" => sample,
            "relative" => data.weights[i],
            _ => step,
        };
        let u = start + f * span + offset;
        let u = if path.closed && !e.enabled("clamp", false) {
            u.rem_euclid(1.0)
        } else {
            // Use the Uniform sampler's open-curve endpoint for the entire span.
            u.clamp(0.0, 1.0) * end_fraction
        };
        let s = table.parameter(u);
        let segment = (s as usize).min(path.segments.len() - 1);
        let (at, z) = if let Some(samples) = &samples {
            let n = samples.len() - 1;
            let k = ((u * n as f64 / (1.0 - 1e-7)).floor() as usize).min(n - 1);
            let lo = k as f64 / n as f64 * (1.0 - 1e-7);
            let hi = if k + 1 == n {
                end_fraction
            } else {
                (k + 1) as f64 / n as f64 * (1.0 - 1e-7)
            };
            let span = samples[k + 1] - samples[k];
            (
                samples[k].lerp(samples[k + 1], ((u - lo) / (hi - lo)).clamp(0.0, 1.0)),
                span.normalize_or_zero(),
            )
        } else {
            (
                path.at(s),
                Curve::tangent(&path.segments[segment], s - segment as f64).normalize_or_zero(),
            )
        };
        let up = rail.as_ref().map_or_else(
            || e.vector("up_vector", DVec3::ZERO),
            |(rail, table)| rail.at(table.parameter(u)) - at,
        );
        let mut x = up.cross(z).normalize_or_zero();
        if x == DVec3::ZERO {
            let up = if z.y.abs() < 0.9 { DVec3::Y } else { DVec3::Z };
            x = up.cross(z).normalize_or_zero();
        }
        let target_p = if mode == "relative" {
            p + at - path.at(0.0)
        } else {
            at
        };
        let target = if z == DVec3::ZERO {
            DMat4::from_translation(target_p)
        } else {
            DMat4::from_cols(
                x.extend(0.0),
                z.cross(x).extend(0.0),
                z.extend(0.0),
                target_p.extend(1.0),
            )
        };
        *m = blend_pose(*m, target, channels, e.number("strength", 1.0) * sample);
    }
    Ok(())
}

fn push_apart(
    e: Params<'_>,
    data: &mut CloneData,
    ctx: &EffectorCtx<'_>,
) -> Result<(), CloneError> {
    if data.len() > 4096 {
        return Err(CloneError::Unsupported("Push Apart over 4096 clones"));
    }
    let radius = e.number("radius", 100.0) * 2.0;
    let strength = e.number("strength", 1.0).clamp(0.0, 1.0);
    if radius <= 0.0 || strength == 0.0 {
        return Ok(());
    }
    let fields = fields(e.scene, e.id)?;
    let mut p: Vec<_> = data.matrices.iter().map(|m| m.w_axis.truncate()).collect();
    let weights: Vec<_> = p
        .iter()
        .map(|&p| {
            fields
                .sample(ctx.cloner_world.transform_point3(p), ctx.eval)
                .value
        })
        .collect();
    let hide = e.option("mode") == "hide";
    let iterations = if hide {
        1
    } else {
        e.number("iterations", 10.0) as usize
    };
    let mut shifts = vec![DVec3::ZERO; p.len()];
    let density = p.len().saturating_sub(1).max(1) as f64;
    for _ in 0..iterations {
        shifts.fill(DVec3::ZERO);
        for i in 0..p.len() {
            if !data.visible[i] {
                continue;
            }
            for j in i + 1..p.len() {
                let w = strength * weights[i].min(weights[j]).clamp(0.0, 1.0);
                let delta = p[j] - p[i];
                let distance = delta.length();
                if !data.visible[j] || w == 0.0 || distance >= radius {
                    continue;
                }
                if hide {
                    data.visible[j] = false;
                } else {
                    let axis = delta
                        .try_normalize()
                        .unwrap_or([DVec3::X, DVec3::Y, DVec3::Z][(i + j) % 3]);
                    let move_by = axis * ((radius - distance) * 0.5 * w);
                    shifts[i] -= move_by;
                    shifts[j] += move_by;
                }
            }
        }
        for (p, &shift) in p.iter_mut().zip(&shifts) {
            *p += shift / density;
        }
    }
    for (m, p) in data.matrices.iter_mut().zip(p) {
        m.w_axis = p.extend(1.0);
    }
    Ok(())
}

fn inherit(
    e: Params<'_>,
    cloner: NodeId,
    data: &mut CloneData,
    ctx: &EffectorCtx<'_>,
) -> Result<(), CloneError> {
    if !e.enabled("morph_motion_object", false) {
        return Ok(());
    }
    let Some(source) = e.link("object") else {
        return Ok(());
    };
    if reaches(e.scene, source, cloner) {
        return Err(CloneError::Unsupported("cyclic inherited cloner"));
    }
    let Some(src) = super::clone_data(e.scene, source, ctx.eval)
        .map_err(|_| CloneError::Unsupported("inheritance source cloner"))?
    else {
        return Ok(());
    };
    if ctx.cloner_world.determinant() == 0.0 {
        return Ok(());
    }
    let xf = ctx.cloner_world.inverse() * e.scene.world_matrix(source).unwrap_or(DMat4::IDENTITY);
    let fields = fields(e.scene, e.id)?;
    let flags = ["position", "rotation", "scale"].map(|p| e.enabled(p, true));
    for (m, other) in data.matrices.iter_mut().zip(src.matrices) {
        let p = ctx.cloner_world.transform_point3(m.w_axis.truncate());
        let w = if e.enabled("falloff_based", false) {
            fields.sample(p, ctx.eval).value
        } else {
            1.0
        };
        *m = blend_pose(*m, xf * other, flags, w * e.number("strength", 1.0));
    }
    Ok(())
}

fn volume(
    e: Params<'_>,
    cloner: NodeId,
    data: &mut CloneData,
    ctx: &EffectorCtx<'_>,
) -> Result<(), CloneError> {
    let Some(source) = e.link("volume_object") else {
        return Ok(());
    };
    if reaches(e.scene, source, cloner) {
        return Err(CloneError::Unsupported("cyclic volume source"));
    }
    let Some(mesh) = crate::evaluate(e.scene, source, ctx.eval)
        .map_err(|_| CloneError::Unsupported("volume source mesh"))?
    else {
        return Ok(());
    };
    if !HalfEdgeMesh::build(&mesh).is_ok_and(|top| top.is_closed()) {
        return Err(CloneError::Unsupported("volume requires closed mesh"));
    }
    let xf = e.scene.world_matrix(source).unwrap_or(DMat4::IDENTITY);
    let tris: Vec<_> = triangulate(&mesh)
        .into_iter()
        .map(|(_, tri)| tri.map(|i| xf.transform_point3(mesh.points[i as usize])))
        .collect();
    apply_sampled(e.scene, e.id, data, ctx, |p| {
        if contains(&tris, p.world) { 1.0 } else { 0.0 }
    })
}

fn contains(tris: &[[DVec3; 3]], p: DVec3) -> bool {
    let mut winding = 0.0;
    for &[a, b, c] in tris {
        let [a, b, c] = [a - p, b - p, c - p];
        let (la, lb, lc) = (a.length(), b.length(), c.length());
        let det = a.dot(b.cross(c));
        let den = la * lb * lc + a.dot(b) * lc + b.dot(c) * la + c.dot(a) * lb;
        if la.min(lb).min(lc) <= 1e-9 || (det.abs() <= 1e-10 * la * lb * lc && den <= 0.0) {
            return false; // triangle boundary
        }
        winding += det.atan2(den);
    }
    winding.abs() > std::f64::consts::PI + 1e-8
}

fn reaches(scene: &Scene, source: NodeId, target: NodeId) -> bool {
    let mut stack = vec![source];
    let mut seen = BTreeSet::new();
    while let Some(id) = stack.pop() {
        if id == target {
            return true;
        }
        if seen.insert(id)
            && let Some(n) = scene.node(id)
        {
            stack.extend(&n.children);
            for value in n.params.values() {
                match value {
                    Value::Link(Some(id)) => stack.push(*id),
                    Value::LinkList(ids) => stack.extend(ids),
                    _ => {}
                }
            }
        }
    }
    false
}
