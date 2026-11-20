//! Field display: wireframe gizmos of the selected fields and an optional value preview on
//! the points of the selected meshes (docs/specs/UI.md §16.6). Pure: the editor paints the
//! result with the egui painter.

use std::collections::BTreeSet;

use forge_core::{Category, EvalCtx, NodeId, Scene, Value};
use forge_fields::{FieldList, GROUP};
use forge_viewport::camera::Camera;
use glam::DVec3;

/// Most preview dots one frame draws (the first points of the first meshes).
pub const MAX_DOTS: usize = 5000;

/// Segments of a gizmo circle.
const CIRCLE: usize = 48;

/// One gizmo line strip in world space.
pub type Strip = Vec<DVec3>;

/// A preview dot: screen position (pixels from the viewport's top left) and the field value
/// at the point, 0..1 for the usual fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dot {
    pub at: [f32; 2],
    pub value: f32,
}

/// The unit vectors of the signed axis named by a direction option id (`x_pos`, `z_neg`).
fn axis_of(id: &str) -> (usize, f64) {
    let axis = match id.as_bytes().first() {
        Some(b'x') => 0,
        Some(b'y') => 1,
        _ => 2,
    };
    (axis, if id.ends_with("neg") { -1.0 } else { 1.0 })
}

fn float(scene: &Scene, id: NodeId, param: &str, default: f64) -> f64 {
    match scene.param(id, param) {
        Some(Value::Float(f)) => *f,
        _ => default,
    }
}

fn direction(scene: &Scene, id: NodeId, default: &str) -> (usize, f64) {
    match scene.param(id, "direction") {
        Some(Value::Enum(e)) => axis_of(e),
        _ => axis_of(default),
    }
}

/// A point of a local frame: `u` and `v` the two axes other than `axis`.
fn frame(axis: usize) -> (usize, usize) {
    ((axis + 1) % 3, (axis + 2) % 3)
}

fn circle(axis: usize, at: f64, radius: f64) -> Strip {
    let (u, v) = frame(axis);
    (0..=CIRCLE)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / CIRCLE as f64;
            let mut p = DVec3::ZERO;
            p[axis] = at;
            p[u] = radius * a.cos();
            p[v] = radius * a.sin();
            p
        })
        .collect()
}

fn quad(axis: usize, at: f64, half: f64) -> Strip {
    let (u, v) = frame(axis);
    [
        (-1.0, -1.0),
        (1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (-1.0, -1.0),
    ]
    .iter()
    .map(|(a, b)| {
        let mut p = DVec3::ZERO;
        p[axis] = at;
        p[u] = a * half;
        p[v] = b * half;
        p
    })
    .collect()
}

fn line(a: DVec3, b: DVec3) -> Strip {
    vec![a, b]
}

/// The gizmo of the field node `id` in its own space: Spherical (three circles), Box (the
/// wire box), Linear (the planes at both ends of the ramp and its axis), Cylinder and Cone
/// (end circles and side lines). Other field types have no gizmo.
pub fn local_gizmo(scene: &Scene, id: NodeId) -> Vec<Strip> {
    let Some(node) = scene.node(id) else {
        return Vec::new();
    };
    match node.type_id.as_str() {
        forge_fields::SPHERICAL => {
            let r = float(scene, id, "radius", 100.0);
            (0..3).map(|a| circle(a, 0.0, r)).collect()
        }
        forge_fields::BOX => {
            let s = match scene.param(id, "size") {
                Some(Value::Vec3(v)) => DVec3::from_array(*v),
                _ => DVec3::splat(100.0),
            };
            let corner = |i: usize| {
                DVec3::new(
                    if i & 1 == 0 { -s.x } else { s.x },
                    if i & 2 == 0 { -s.y } else { s.y },
                    if i & 4 == 0 { -s.z } else { s.z },
                )
            };
            (0..8)
                .flat_map(|i| {
                    [1usize, 2, 4]
                        .into_iter()
                        .filter(move |bit| i & bit == 0)
                        .map(move |bit| (i, i | bit))
                })
                .map(|(a, b)| line(corner(a), corner(b)))
                .collect()
        }
        forge_fields::LINEAR => {
            let len = float(scene, id, "length", 100.0);
            let (axis, sign) = direction(scene, id, "x_pos");
            let mut tip = DVec3::ZERO;
            tip[axis] = sign * len;
            let mut tail = DVec3::ZERO;
            tail[axis] = -sign * len;
            vec![quad(axis, -len, len), quad(axis, len, len), line(tail, tip)]
        }
        forge_fields::CYLINDER | forge_fields::CONE => {
            let h = float(scene, id, "height", 100.0);
            let (axis, sign) = direction(scene, id, "y_pos");
            let (u, _) = frame(axis);
            let cone = node.type_id == forge_fields::CONE;
            let r = float(scene, id, "radius", if cone { 100.0 } else { 75.0 });
            let at = |a: f64, radius: f64| {
                let mut p = DVec3::ZERO;
                p[axis] = sign * a;
                p[u] = radius;
                p
            };
            if cone {
                // Base at -h/2 with the full radius, apex at +h/2 (FIELDS.md F6).
                vec![
                    circle(axis, -sign * h / 2.0, r),
                    line(at(-h / 2.0, r), at(h / 2.0, 0.0)),
                    line(at(-h / 2.0, -r), at(h / 2.0, 0.0)),
                ]
            } else {
                // The surface is at |axial| = height (d = max(r / radius, |a| / height)).
                vec![
                    circle(axis, -sign * h, r),
                    circle(axis, sign * h, r),
                    line(at(-h, r), at(h, r)),
                    line(at(-h, -r), at(h, -r)),
                ]
            }
        }
        _ => Vec::new(),
    }
}

/// The gizmo of the field node `id` in world space.
pub fn field_gizmo(scene: &Scene, id: NodeId) -> Vec<Strip> {
    let Some(world) = scene.world_matrix(id) else {
        return Vec::new();
    };
    local_gizmo(scene, id)
        .into_iter()
        .map(|s| s.into_iter().map(|p| world.transform_point3(p)).collect())
        .collect()
}

/// The field nodes to draw for a selection: the selected fields themselves and the fields in
/// the lists of selected deformers and effectors (groups opened), each once, in order.
pub fn shown_fields(scene: &Scene, selected: &BTreeSet<NodeId>) -> Vec<NodeId> {
    fn push_rows(scene: &Scene, owner: NodeId, param: &str, out: &mut Vec<NodeId>, depth: usize) {
        let Some(Value::LinkRows(rows)) = scene.param(owner, param) else {
            return;
        };
        for r in rows.iter().filter(|r| r.enabled) {
            push_node(scene, r.target, out, depth + 1);
        }
    }
    fn push_node(scene: &Scene, id: NodeId, out: &mut Vec<NodeId>, depth: usize) {
        if depth > 8 || out.contains(&id) {
            return;
        }
        let Some(node) = scene.node(id) else { return };
        if node.type_id == GROUP {
            push_rows(scene, id, forge_fields::FIELD_LIST, out, depth);
        } else if scene
            .node_type(id)
            .is_some_and(|t| t.category == Category::Field)
        {
            out.push(id);
        }
    }
    let mut out = Vec::new();
    for id in selected {
        push_node(scene, *id, &mut out, 0);
        push_rows(scene, *id, "fields", &mut out, 0);
    }
    out
}

/// The owners whose field list the preview shows: selected deformers and effectors (a
/// `fields` list) and selected fields (a list of that one field).
fn owners(scene: &Scene, selected: &BTreeSet<NodeId>) -> Vec<NodeId> {
    selected
        .iter()
        .copied()
        .filter(|id| {
            scene
                .node_type(*id)
                .is_some_and(|t| t.category == Category::Field)
                || matches!(scene.param(*id, "fields"), Some(Value::LinkRows(_)))
        })
        .collect()
}

/// The list the preview of `owner` samples.
fn list_of(scene: &Scene, owner: NodeId) -> Option<FieldList> {
    if scene
        .node_type(owner)
        .is_some_and(|t| t.category == Category::Field)
    {
        FieldList::from_nodes(scene, &[owner]).ok()
    } else {
        FieldList::from_param(scene, owner, "fields").ok()
    }
}

/// Preview dots for `points` (world space) under the list of `owner`: the value at each point
/// seen by `camera` in a viewport of `size` pixels. Points behind the camera or outside the
/// viewport are skipped; at most `MAX_DOTS`.
pub fn preview_dots(
    scene: &Scene,
    owner: NodeId,
    points: &[DVec3],
    ctx: &EvalCtx,
    camera: &Camera,
    size: [f64; 2],
) -> Vec<Dot> {
    let Some(list) = list_of(scene, owner) else {
        return Vec::new();
    };
    if size[0] <= 0.0 || size[1] <= 0.0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    for p in points {
        if out.len() >= MAX_DOTS {
            break;
        }
        let Some(s) = camera.project(*p, size[0], size[1]) else {
            continue;
        };
        if s.x < 0.0 || s.y < 0.0 || s.x > size[0] || s.y > size[1] {
            continue;
        }
        out.push(Dot {
            at: [s.x as f32, s.y as f32],
            value: list.sample(*p, ctx).value as f32,
        });
    }
    out
}

/// The preview dots of every selected owner over `points` (see [`owners`]).
pub fn selected_preview(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
    points: &[DVec3],
    ctx: &EvalCtx,
    camera: &Camera,
    size: [f64; 2],
) -> Vec<Dot> {
    let Some(owner) = owners(scene, selected).first().copied() else {
        return Vec::new();
    };
    preview_dots(scene, owner, points, ctx, camera, size)
}

/// Projects a strip to the screen; a strip with a point behind the camera is cut there.
pub fn project_strip(strip: &[DVec3], camera: &Camera, size: [f64; 2]) -> Vec<Vec<[f32; 2]>> {
    let mut out = Vec::new();
    let mut run = Vec::new();
    for p in strip {
        match camera.project(*p, size[0], size[1]) {
            Some(s) => run.push([s.x as f32, s.y as f32]),
            None => {
                if run.len() > 1 {
                    out.push(std::mem::take(&mut run));
                }
                run.clear();
            }
        }
    }
    if run.len() > 1 {
        out.push(run);
    }
    out
}

/// The colour of a preview value: blue at 0 through white to red at 1 (own design,
/// **parity-pending G4**). Values outside 0..1 are clamped.
pub fn value_colour(value: f32) -> [u8; 3] {
    let v = value.clamp(0.0, 1.0);
    let lerp = |a: f32, b: f32, t: f32| (a + (b - a) * t).round() as u8;
    if v < 0.5 {
        let t = v * 2.0;
        [
            lerp(40.0, 235.0, t),
            lerp(90.0, 235.0, t),
            lerp(230.0, 235.0, t),
        ]
    } else {
        let t = (v - 0.5) * 2.0;
        [
            lerp(235.0, 230.0, t),
            lerp(235.0, 60.0, t),
            lerp(235.0, 50.0, t),
        ]
    }
}
