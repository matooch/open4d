//! Mesh source distributions (clone-toolset spec C2).

use std::collections::{BTreeMap, BTreeSet};

use forge_clone_engine::{CloneItem, MAX_CLONES, SplitMix64};
use forge_core::tags::{SelectionKind, selection_indices};
use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_geometry::mesh::Edge;
use forge_geometry::normals::{Weighting, poly_normal, vertex_normals};
use forge_geometry::triangulate::triangulate;
use forge_geometry::{HalfEdgeMesh, PolyMesh};
use glam::{DMat4, DVec3};

use crate::{BuildError, evaluate};

pub(super) fn placements(
    scene: &Scene,
    id: NodeId,
    ctx: &EvalCtx,
) -> Result<Vec<CloneItem>, BuildError> {
    let Some(Value::Link(Some(source))) = scene.param(id, "object") else {
        return Ok(Vec::new());
    };
    let get = |param| scene.param(id, param);
    let enumeration = |param, default| match get(param) {
        Some(Value::Enum(s)) => s.as_str(),
        _ => default,
    };
    let dist = enumeration("distribution", "surface");
    let selected = match get("selection") {
        Some(Value::String(s)) => s.as_str(),
        _ => "",
    };
    let up = match enumeration("up_vector", "none") {
        "x" => Some(DVec3::X),
        "y" => Some(DVec3::Y),
        "z" => Some(DVec3::Z),
        _ => None,
    };
    let align = !matches!(get("align_clone"), Some(Value::Bool(false)));
    let (Some(cloner), Some(world)) = (scene.world_matrix(id), scene.world_matrix(*source)) else {
        return Ok(Vec::new());
    };
    if cloner.determinant() == 0.0 {
        return Ok(Vec::new());
    }
    let m = cloner.inverse() * world;
    if dist == "axis" {
        return Ok(vec![CloneItem::at(
            if align {
                DMat4::from_rotation_translation(
                    m.to_scale_rotation_translation().1,
                    m.w_axis.truncate(),
                )
            } else {
                DMat4::from_translation(m.w_axis.truncate())
            },
            DVec3::ZERO,
        )]);
    }
    if reaches(scene, *source, id) {
        return Err(BuildError::Unsupported("cyclic cloner object source"));
    }
    if let Some(mut points) = forge_particles::EmitterPoints::new(scene, *source) {
        return particle_items(&mut points, cloner, ctx, align, up);
    }
    // A spline has no mesh of its own: its sampled polyline is the clone source (its points
    // are the Vertex distribution, its chords the Edge and Polygon Center ones).
    let spline = crate::splines::polyline_mesh(scene, *source);
    let Some(mut mesh) = (match spline {
        Some(polyline) => Some(polyline),
        None => evaluate(scene, *source, ctx)?,
    }) else {
        return Ok(Vec::new());
    };
    for p in &mut mesh.points {
        *p = m.transform_point3(*p);
    }
    let kind = match dist {
        "vertex" => SelectionKind::Points,
        "edge" => SelectionKind::Edges,
        _ => SelectionKind::Polys,
    };
    let selection: Option<BTreeSet<u32>> = (!selected.is_empty()).then(|| {
        scene
            .node(*source)
            .into_iter()
            .flat_map(|n| &n.tags)
            .find_map(|&tag| {
                let node = scene.node(tag)?;
                let (k, indices) = selection_indices(scene, tag)?;
                (node.name == selected && k == kind).then_some(indices)
            })
            .unwrap_or_default()
            .iter()
            .copied()
            .collect()
    });
    if selection.as_ref().is_some_and(BTreeSet::is_empty) {
        return Ok(Vec::new());
    }
    let allowed = |i: usize| selection.as_ref().is_none_or(|s| s.contains(&(i as u32)));
    let item = |p, z, y| {
        CloneItem::at(
            if align {
                frame(p, z, up.unwrap_or(y))
            } else {
                DMat4::from_translation(p)
            },
            DVec3::ZERO,
        )
    };
    let count = match get("object_count") {
        Some(Value::Int(n)) => (*n as usize).clamp(1, MAX_CLONES),
        _ => 20,
    };
    let seed = match get("object_seed") {
        Some(Value::Int(n)) => *n as u64,
        _ => 1_234_567,
    };
    let mut rng = SplitMix64::new(seed);
    let items: Vec<_> = match dist {
        "vertex" => mesh
            .points
            .iter()
            .zip(vertex_normals(&mesh, Weighting::Uniform))
            .enumerate()
            .filter(|(i, _)| allowed(*i))
            .map(|(_, (&p, z))| item(p, z, DVec3::Y))
            .collect(),
        "edge" => edge_frames(&mesh)
            .into_iter()
            .enumerate()
            .filter(|(i, _)| allowed(*i))
            .map(|(_, (p, z, y))| item(p, z, y))
            .collect(),
        "polygon_center" => mesh
            .polys
            .iter()
            .enumerate()
            .filter(|(i, _)| allowed(*i))
            .map(|(_, poly)| {
                let pos = |i| mesh.points[poly[i] as usize];
                let factor = if poly.len() == 3 {
                    f64::from(0.333_f32)
                } else {
                    1.0 / poly.len() as f64
                };
                let p = poly.iter().map(|&i| mesh.points[i as usize]).sum::<DVec3>() * factor;
                let z = poly_normal(&mesh.points, poly);
                let y = if poly.len() == 4 {
                    z.cross(pos(0) - pos(3))
                } else {
                    pos(1) - pos(0)
                };
                item(p, z, y)
            })
            .collect(),
        "surface" => surface(&mesh, allowed, count, &mut rng)
            .into_iter()
            .map(|(p, z)| item(p, z, DVec3::Y))
            .collect(),
        "volume" if selection.is_none() => volume(&mesh, count, &mut rng)?
            .into_iter()
            .map(|p| item(p, DVec3::ZERO, DVec3::Y))
            .collect(),
        "volume" => return Err(BuildError::Unsupported("volume from a polygon selection")),
        _ => {
            return Err(BuildError::Unsupported(
                "unknown cloner object distribution",
            ));
        }
    };
    if items.len() > MAX_CLONES {
        return Err(BuildError::Unsupported("more than 1000000 clones"));
    }
    Ok(items)
}

/// One clone per live particle at the context's frame (DYNAMICS.md section 11): the
/// particle's position in the cloner's space, +Z along its velocity when aligned (the frame of
/// `frame(p, z, up)`), uniform scale the particle's scale.
fn particle_items(
    points: &mut dyn forge_particles::PointSource,
    cloner: DMat4,
    ctx: &EvalCtx,
    align: bool,
    up: Option<DVec3>,
) -> Result<Vec<CloneItem>, BuildError> {
    let at = (ctx.time * ctx.fps).round() as i64;
    let inverse = cloner.inverse();
    let live = points.points_at(at);
    if live.len() > MAX_CLONES {
        return Err(BuildError::Unsupported("more than 1000000 clones"));
    }
    Ok(live
        .iter()
        .map(|pt| {
            let p = inverse.transform_point3(pt.position);
            let z = inverse.transform_vector3(pt.velocity);
            let m = if align && z.length_squared() > 0.0 {
                frame(p, z, up.unwrap_or(DVec3::Y))
            } else {
                DMat4::from_translation(p)
            };
            CloneItem::at(m * DMat4::from_scale(DVec3::splat(pt.scale)), DVec3::ZERO)
        })
        .collect())
}

/// Evaluate linked sources only if their hierarchy and object links do not reach us.
fn reaches(scene: &Scene, from: NodeId, target: NodeId) -> bool {
    let mut seen = BTreeSet::new();
    let mut pending = vec![from];
    while let Some(id) = pending.pop() {
        if id == target {
            return true;
        }
        if seen.insert(id)
            && let Some(n) = scene.node(id)
        {
            pending.extend(&n.children);
            if let Some(Value::Link(Some(id))) = n.params.get("object") {
                pending.push(*id);
            }
        }
    }
    false
}

fn frame(p: DVec3, z: DVec3, up: DVec3) -> DMat4 {
    let z = z.normalize_or_zero();
    if z == DVec3::ZERO {
        return DMat4::from_translation(p);
    }
    let mut x = up.cross(z);
    if x.length_squared() < 1e-20 {
        let fallback = if z.z.abs() < 0.9 { DVec3::Z } else { DVec3::X };
        x = fallback.cross(z);
    }
    x = x.normalize();
    let y = z.cross(x);
    DMat4::from_cols(x.extend(0.0), y.extend(0.0), z.extend(0.0), p.extend(1.0))
}

fn edge_frames(mesh: &PolyMesh) -> Vec<(DVec3, DVec3, DVec3)> {
    let mut slots = BTreeMap::new();
    let mut out: Vec<(DVec3, DVec3, DVec3)> = Vec::new();
    for poly in &mesh.polys {
        let z = poly_normal(&mesh.points, poly);
        for (i, &a) in poly.iter().enumerate() {
            let b = poly[(i + 1) % poly.len()];
            let entry = slots.entry(Edge::new(a, b)).or_insert_with(|| {
                let a = mesh.points[a as usize];
                let b = mesh.points[b as usize];
                out.push(((a + b) * 0.5, DVec3::ZERO, a - b));
                out.len() - 1
            });
            out[*entry].1 += z;
        }
    }
    out
}

fn surface(
    mesh: &PolyMesh,
    allowed: impl Fn(usize) -> bool,
    count: usize,
    rng: &mut SplitMix64,
) -> Vec<(DVec3, DVec3)> {
    let mut total = 0.0;
    let tris: Vec<_> = triangulate(mesh)
        .into_iter()
        .filter(|(f, _)| allowed(*f as usize))
        .filter_map(|(f, indices)| {
            let [a, b, c] = indices.map(|i| mesh.points[i as usize]);
            let area = (b - a).cross(c - a).length();
            if area <= 0.0 {
                return None;
            }
            total += area;
            Some((
                total,
                [a, b, c],
                poly_normal(&mesh.points, &mesh.polys[f as usize]),
            ))
        })
        .collect();
    if tris.is_empty() {
        return Vec::new();
    }
    (0..count)
        .map(|_| {
            let draw = total * rng.next_f64();
            let (_, [a, b, c], normal) =
                tris[tris.partition_point(|t| t.0 <= draw).min(tris.len() - 1)];
            let r = rng.next_f64().sqrt();
            let s = rng.next_f64();
            (a * (1.0 - r) + (b * (1.0 - s) + c * s) * r, normal)
        })
        .collect()
}

fn volume(mesh: &PolyMesh, count: usize, rng: &mut SplitMix64) -> Result<Vec<DVec3>, BuildError> {
    if mesh.points.is_empty() || !HalfEdgeMesh::build(mesh).is_ok_and(|top| top.is_closed()) {
        return Err(BuildError::Unsupported(
            "cloner volume requires a closed mesh",
        ));
    }
    let lo = mesh.points.iter().copied().reduce(DVec3::min).unwrap();
    let hi = mesh.points.iter().copied().reduce(DVec3::max).unwrap();
    let tris: Vec<_> = triangulate(mesh)
        .into_iter()
        .map(|(_, tri)| tri.map(|i| mesh.points[i as usize]))
        .collect();
    let dir = DVec3::new(1.0, 0.377, 0.173);
    (0..count)
        .map(|_| {
            (0..128)
                .find_map(|_| {
                    let p =
                        lo + (hi - lo) * DVec3::new(rng.next_f64(), rng.next_f64(), rng.next_f64());
                    let n = tris.iter().filter(|&&tri| ray_hit(p, dir, tri)).count();
                    (!n.is_multiple_of(2)).then_some(p)
                })
                .ok_or(BuildError::Unsupported("volume sample search exhausted"))
        })
        .collect()
}

fn ray_hit(p: DVec3, dir: DVec3, [a, b, c]: [DVec3; 3]) -> bool {
    let e1 = b - a;
    let e2 = c - a;
    let h = dir.cross(e2);
    let det = e1.dot(h);
    if det.abs() < 1e-12 {
        return false;
    }
    let s = p - a;
    let u = s.dot(h) / det;
    let q = s.cross(e1);
    let v = dir.dot(q) / det;
    u >= 0.0 && v >= 0.0 && u + v <= 1.0 && e2.dot(q) / det > 0.0
}
