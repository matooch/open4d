//! Gizmo drags → transform params, and object picking (docs/specs/MANIPULATE.md §3, §5).

use forge_core::transform::{self, Transform, hpb_from_rotation, rotation_part};
use forge_core::{Document, EditError, EvalCtx, NodeId, Scene, Tx, Value};
use forge_generators::axis::AxisEdit;
use forge_viewport::camera::{Camera, Ray};
use forge_viewport::gizmo::DragDelta;
use forge_viewport::picking::{pick_instances, pick_poly};
use glam::DMat3;

use crate::animate::AnimEdit;
use crate::attributes::Gesture;
use crate::viewport_panel::{DrawItem, instance_input, scene_draw_items_at, scheduled_draw_items};

pub(crate) use forge_app::coords::write_transform;

/// Sets the node's transform to `start` changed by `delta` (one undo step per drag).
pub fn apply_drag(
    doc: &mut Document,
    id: NodeId,
    start: &Transform,
    delta: DragDelta,
    gesture: Gesture,
) -> Result<(), EditError> {
    apply_drag_anim(doc, id, start, delta, gesture, None)
}

/// [`apply_drag`] on an animated scene: `start` and the parent matrix come from the
/// evaluated scene and the result is keyed per autokey / rule 5.
pub fn apply_drag_anim(
    doc: &mut Document,
    id: NodeId,
    start: &Transform,
    delta: DragDelta,
    gesture: Gesture,
    anim: Option<&AnimEdit>,
) -> Result<(), EditError> {
    let parent = anim
        .map_or(doc.scene(), |a| a.eval)
        .parent_world_matrix(id)
        .ok_or(EditError::UnknownNode(id))?;
    let (label, next) = dragged_transform(start, &parent, delta);
    let edit = |tx: &mut Tx| write_transform(tx, id, &next, anim);
    match gesture {
        Gesture::Commit => doc.transact(label, edit),
        Gesture::Drag(n) => doc.transact_merged(label, &format!("gizmo:{id}:{n}"), edit),
    }
}

/// An interactive drag of the gizmo on the editor state: [`dragged_transform`], then
/// [`apply_transform_edits`] (UI.md §12, owner note N10).
pub fn apply_drag_edit(
    app: &mut forge_app::App,
    id: NodeId,
    start: &Transform,
    delta: DragDelta,
    gesture: Gesture,
) -> Result<(), EditError> {
    let parent = app
        .evaluated()
        .parent_world_matrix(id)
        .ok_or(EditError::UnknownNode(id))?;
    let (label, next) = dragged_transform(start, &parent, delta);
    let merge = match gesture {
        Gesture::Commit => None,
        Gesture::Drag(n) => Some(format!("gizmo:{id}:{n}")),
    };
    apply_transform_edits(app, &[(id, next)], label, merge.as_deref())
}

/// True when an interactive transform edit of `ids` would stage drafts: Auto-Key is off and
/// one of them has a track on Position, Rotation or Scale.
pub fn edit_drafts(app: &forge_app::App, ids: &[NodeId]) -> bool {
    !app.autokey
        && ids.iter().any(|id| {
            [transform::POS, transform::ROT, transform::SCALE]
                .iter()
                .any(|param| forge_app::drafts::has_tracks(app.doc.scene(), *id, param))
        })
}

/// Applies edited transforms the way every interactive route does (gizmo, Coordinates
/// panel; UI.md §12 "Keyframe drafts", owner note N10). With Auto-Key off, a parameter that
/// has a track becomes a draft (or stops being one when the value is the animated value
/// again) and nothing is written to the document. Every other parameter is written as
/// before: keyed under Auto-Key, a plain `set_param` when it has no track, skipped when it
/// already shows the value. The document part is one undo step, merged by `merge`.
pub fn apply_transform_edits(
    app: &mut forge_app::App,
    edits: &[(NodeId, Transform)],
    label: &str,
    merge: Option<&str>,
) -> Result<(), EditError> {
    let ctx = app.anim_ctx();
    let shown = app.evaluated();
    let animated = app.animated();
    let (shown_edit, animated_edit) = (
        AnimEdit { ctx, eval: &shown },
        AnimEdit {
            ctx,
            eval: &animated,
        },
    );
    let mut writes: Vec<(NodeId, &str, Value)> = Vec::new();
    for (id, t) in edits {
        for (param, v) in [
            (transform::POS, t.pos),
            (transform::ROT, t.rot),
            (transform::SCALE, t.scale),
        ] {
            let value = Value::Vec3(v.to_array());
            if !ctx.autokey && forge_app::drafts::has_tracks(app.doc.scene(), *id, param) {
                if forge_app::animate::unchanged(Some(&animated_edit), *id, param, &value) {
                    app.remove_draft(*id, param);
                } else {
                    app.set_draft(*id, param, value)?;
                }
            } else if !forge_app::animate::unchanged(Some(&shown_edit), *id, param, &value) {
                writes.push((*id, param, value));
            }
        }
    }
    if writes.is_empty() {
        return Ok(());
    }
    app.keep_drafts(|app| {
        let run = |tx: &mut Tx| {
            writes.iter().try_for_each(|(id, param, value)| {
                forge_app::animate::write_param(tx, *id, param, value.clone(), Some(&ctx))
            })
        };
        match merge {
            Some(key) => app.doc.transact_merged(label, key, run),
            None => app.doc.transact(label, run),
        }
    })
}

/// The transform a gizmo `delta` makes of `start` under a parent with world matrix `parent`
/// (the rules of [`apply_drag`]) and the undo label.
pub fn dragged_transform(
    start: &Transform,
    parent: &glam::DMat4,
    delta: DragDelta,
) -> (&'static str, Transform) {
    let rp = rotation_part(parent);
    match delta {
        DragDelta::Translate(w) => (
            "Move",
            Transform {
                pos: start.pos + parent.inverse().transform_vector3(w),
                ..*start
            },
        ),
        DragDelta::Rotate { axis, angle } => {
            let r = rp.inverse() * DMat3::from_axis_angle(axis, angle) * rp * start.rotation();
            (
                "Rotate",
                Transform {
                    rot: hpb_from_rotation(r),
                    ..*start
                },
            )
        }
        DragDelta::Scale(f) => (
            "Scale",
            Transform {
                scale: start.scale * f,
                ..*start
            },
        ),
    }
}

/// Sets the axis of the captured object to the start axis changed by `delta`; the points and
/// children keep their world place (MANIPULATE.md §7.4). One undo step per drag.
pub fn apply_axis_drag(
    doc: &mut Document,
    edit: &AxisEdit,
    delta: DragDelta,
    gesture: Gesture,
) -> Result<(), EditError> {
    let id = edit.node;
    let parent = doc
        .scene()
        .parent_world_matrix(id)
        .ok_or(EditError::UnknownNode(id))?;
    let (label, next) = dragged_transform(&edit.start, &parent, delta);
    let label = format!("{label} axis");
    let run = |tx: &mut Tx| edit.apply(tx, &next);
    match gesture {
        Gesture::Commit => doc.transact(&label, run),
        Gesture::Drag(n) => doc.transact_merged(&label, &format!("axis:{id}:{n}"), run),
    }
}

/// What a pick hit: the object and, for an instanced cloner, the clone under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickHit {
    pub node: NodeId,
    /// Clone index (into the cloner's clone data) when `node` is a cloner drawn by instancing.
    pub clone: Option<u32>,
}

/// The object nearest under the cursor; cloners are picked through their instances without
/// expanding the clone geometry (docs/specs/VIEWPORT.md §10.7).
pub fn pick_object(scene: &Scene, cam: &Camera, x: f64, y: f64, w: f64, h: f64) -> Option<PickHit> {
    let ray = cam.ray(x, y, w, h);
    let items = scene_draw_items_at(scene, &Default::default(), &EvalCtx::default());
    // A locked layer cannot be selected.
    let selectable = items
        .into_iter()
        .filter(|item| !forge_core::layers::locked(scene, item.id()));
    nearest_surface(selectable, &ray).map(|(_, hit)| hit)
}

/// Navigation can anchor to any visible surface, including locked objects.
pub(crate) fn navigation_anchor(
    frame: &forge_app::schedule::Frame,
    camera: &Camera,
    [x, y]: [f64; 2],
    [width, height]: [f64; 2],
) -> Option<glam::DVec3> {
    let ray = camera.ray(x, y, width, height);
    nearest_surface(scheduled_draw_items(frame, &Default::default()), &ray)
        .map(|(t, _)| ray.at(t))
        .or_else(|| forge_viewport::grid::grid_hit(camera, height, &ray))
}

fn nearest_surface(items: impl IntoIterator<Item = DrawItem>, ray: &Ray) -> Option<(f64, PickHit)> {
    items
        .into_iter()
        .filter_map(|item| match item {
            DrawItem::Mesh { id, mesh, .. } => pick_poly(&mesh, ray).map(|hit| (hit.t, id, None)),
            DrawItem::Instanced {
                id,
                instances,
                world,
                ..
            } => pick_instances(
                &instances.prototypes,
                &instance_input(&instances),
                &world,
                ray,
            )
            .map(|hit| (hit.t, id, Some(hit.clone))),
        })
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(t, node, clone)| (t, PickHit { node, clone }))
}

/// The generator node whose world-space mesh (or instance) is nearest under the cursor.
pub fn pick_node(scene: &Scene, cam: &Camera, x: f64, y: f64, w: f64, h: f64) -> Option<NodeId> {
    pick_object(scene, cam, x, y, w, h).map(|hit| hit.node)
}
