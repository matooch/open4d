//! Splines in the editor: the draw data of every spline object, Points-mode selection and
//! dragging of control points and Bezier handles through `spline.edit_points`
//! (docs/specs/VIEWPORT.md section 11). UI-free: the egui code in `app` only turns pointer input
//! into these calls.

use std::collections::BTreeSet;

use forge_app::App;
use forge_core::{NodeId, Scene};
use forge_generators::editable_spline::{SplineType, TYPE, settings_of};
use forge_generators::splines::sampled_loops;
use forge_modeling::snap::SnapSource;
use forge_viewport::curves::{
    ControlHit, Controls, DrawLine, Loop, PICK_RADIUS, SplineDraw, draw_list, pick,
};
use forge_viewport::render::{Palette, Shading};
use forge_viewport::snap::{ray_plane, snap_cursor, snap_scalar};
use glam::{DMat4, DVec3};
use serde_json::json;

use crate::components::{ClickModifiers, EditMode};
use crate::snapping::collect_sources;
use crate::state::EditorState;

/// The selected control points of the edited spline (not saved, not undoable).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SplineSelection {
    pub node: Option<NodeId>,
    pub points: BTreeSet<usize>,
}

impl SplineSelection {
    /// The selected indices of `node`; another node's selection is dropped first.
    fn of(&mut self, node: NodeId) -> &mut BTreeSet<usize> {
        if self.node != Some(node) {
            self.node = Some(node);
            self.points.clear();
        }
        &mut self.points
    }

    /// The selected indices when `node` is the selection's node, else empty.
    pub fn get(&self, node: NodeId) -> BTreeSet<usize> {
        if self.node == Some(node) {
            self.points.clone()
        } else {
            BTreeSet::new()
        }
    }
}

/// The spline being edited: the active node when Points mode is on and it is a `forge.spline`
/// on an unlocked layer.
pub fn edit_target(app: &App) -> Option<NodeId> {
    if app.mode != EditMode::Points {
        return None;
    }
    let id = app.selection.active?;
    let scene = app.doc.scene();
    (scene.node(id)?.type_id == TYPE && !forge_core::layers::locked(scene, id)).then_some(id)
}

/// The control points of `id` in world space (section 11.1); `None` if it is no editable spline.
fn controls_of(scene: &Scene, id: NodeId, selected: &BTreeSet<usize>) -> Option<Controls> {
    let (data, ty, _) = settings_of(scene, id).ok()?;
    let world = scene.world_matrix(id)?;
    let w = |p: DVec3| world.transform_point3(p);
    Some(Controls {
        points: data.points.iter().map(|p| w(*p)).collect(),
        left: data
            .points
            .iter()
            .zip(&data.left)
            .map(|(p, h)| w(*p + *h))
            .collect(),
        right: data
            .points
            .iter()
            .zip(&data.right)
            .map(|(p, h)| w(*p + *h))
            .collect(),
        selected: (0..data.points.len())
            .map(|i| selected.contains(&i))
            .collect(),
        handles: ty == SplineType::Bezier,
    })
}

/// One spline object in world space; `controls` only for the edited spline.
fn draw_of(
    scene: &Scene,
    id: NodeId,
    selected: bool,
    edit: Option<(NodeId, &BTreeSet<usize>)>,
) -> Option<SplineDraw> {
    let loops = sampled_loops(scene, id)?;
    let world = scene.world_matrix(id)?;
    let controls = edit
        .filter(|(node, _)| *node == id)
        .and_then(|(_, sel)| controls_of(scene, id, sel));
    Some(SplineDraw {
        loops: loops
            .into_iter()
            .map(|(points, closed)| Loop {
                points: points.iter().map(|p| world.transform_point3(*p)).collect(),
                closed,
            })
            .collect(),
        selected,
        controls,
    })
}

/// Every spline object the viewport draws, in scene order. `edit` is the edited spline with
/// its selected points.
pub fn spline_draws(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
    edit: Option<(NodeId, &BTreeSet<usize>)>,
) -> Vec<SplineDraw> {
    forge_app::schedule::drawn_nodes(scene)
        .into_iter()
        .filter_map(|id| draw_of(scene, id, selected.contains(&id), edit))
        .collect()
}

/// [`spline_draws`] for the editor's current selection and Points-mode state.
pub fn editor_draws(scene: &Scene, s: &EditorState) -> Vec<SplineDraw> {
    let target = edit_target(&s.app);
    let points = target.map(|id| s.spline_sel.get(id)).unwrap_or_default();
    spline_draws(scene, &s.selection.nodes, target.map(|id| (id, &points)))
}

/// The lines of all `draws` for one pane.
pub fn pane_lines(
    draws: &[SplineDraw],
    shading: Shading,
    pal: &Palette,
    cam: &forge_viewport::camera::Camera,
    size: [f64; 2],
) -> Vec<DrawLine> {
    draws
        .iter()
        .flat_map(|d| draw_list(d, shading, pal, cam, size))
        .collect()
}

/// The edited spline in the document's scene with its controls, for picking and dragging.
fn edit_draw(s: &EditorState, id: NodeId) -> Option<SplineDraw> {
    draw_of(s.doc.scene(), id, true, Some((id, &s.spline_sel.get(id))))
}

/// What is under the cursor of the edited spline.
fn hit_under(s: &EditorState, id: NodeId, cursor: [f64; 2], size: [f64; 2]) -> Option<ControlHit> {
    let draw = edit_draw(s, id)?;
    pick(
        &draw,
        &s.camera,
        (cursor[0], cursor[1]),
        (size[0], size[1]),
        PICK_RADIUS,
    )
}

/// A left click at `cursor` (pixels) in a viewport of `size`. Returns false when no spline is
/// being edited (the caller picks objects instead). A plain click on a point selects only it,
/// Shift adds it, Ctrl/Cmd toggles it; a click on nothing clears the selection unless Shift or
/// Ctrl/Cmd is held; handles are not selectable (section 11.2).
pub fn click(s: &mut EditorState, cursor: [f64; 2], size: [f64; 2], mods: ClickModifiers) -> bool {
    let Some(id) = edit_target(&s.app) else {
        return false;
    };
    let hit = hit_under(s, id, cursor, size);
    let set = s.spline_sel.of(id);
    match hit {
        Some(ControlHit::Point(i)) if mods.command => {
            if !set.remove(&i) {
                set.insert(i);
            }
        }
        Some(ControlHit::Point(i)) => {
            if !mods.shift {
                set.clear();
            }
            set.insert(i);
        }
        Some(_) => {}
        None if !mods.shift && !mods.command => set.clear(),
        None => {}
    }
    true
}

/// What a drag moves.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Grab {
    /// The selected points (their handles travel along: handles are offsets).
    Points(Vec<usize>),
    Handle {
        index: usize,
        right: bool,
        /// Shift at the press: the opposite handle does not follow.
        broken: bool,
    },
}

/// A drag of control points or one handle: one undo step for the whole gesture.
pub struct SplineDrag {
    node: NodeId,
    gesture: u64,
    grab: Grab,
    /// World position of the grabbed element at the press.
    grabbed: DVec3,
    /// The camera-facing plane through `grabbed` and where the press ray met it.
    start_hit: DVec3,
    inverse: DMat4,
    start: forge_generators::editable_spline::SplineData,
    sources: Vec<SnapSource>,
    last: DVec3,
}

impl SplineDrag {
    /// A press at `cursor`: starts a drag if a control point or handle is under it (selecting
    /// an unselected point first). `gesture` must be new for every drag.
    pub fn begin(
        s: &mut EditorState,
        cursor: [f64; 2],
        size: [f64; 2],
        mods: ClickModifiers,
        gesture: u64,
    ) -> Option<SplineDrag> {
        let node = edit_target(&s.app)?;
        let hit = hit_under(s, node, cursor, size)?;
        let draw = edit_draw(s, node)?;
        let controls = draw.controls.as_ref()?;
        let (data, _, _) = settings_of(s.doc.scene(), node).ok()?;
        let world = s.doc.scene().world_matrix(node)?;
        let (grab, grabbed) = match hit {
            ControlHit::Point(i) => {
                let set = s.spline_sel.of(node);
                if !set.contains(&i) {
                    if !mods.shift && !mods.command {
                        set.clear();
                    }
                    set.insert(i);
                }
                (
                    Grab::Points(set.iter().copied().collect()),
                    controls.points[i],
                )
            }
            ControlHit::Left(i) => (
                Grab::Handle {
                    index: i,
                    right: false,
                    broken: mods.shift,
                },
                controls.left[i],
            ),
            ControlHit::Right(i) => (
                Grab::Handle {
                    index: i,
                    right: true,
                    broken: mods.shift,
                },
                controls.right[i],
            ),
        };
        let ray = s.camera.ray(cursor[0], cursor[1], size[0], size[1]);
        let t = ray_plane(&ray, grabbed, s.camera.forward_world())?;
        let snapping = s.app.transform_settings.snap.enabled && matches!(grab, Grab::Points(_));
        let sources = if snapping {
            collect_sources(s, &[node], None).sources
        } else {
            Vec::new()
        };
        Some(SplineDrag {
            node,
            gesture,
            grab,
            grabbed,
            start_hit: ray.at(t),
            inverse: world.inverse(),
            start: data,
            sources,
            last: DVec3::ZERO,
        })
    }

    /// The world delta of the pointer at `cursor`, snapped for point drags.
    fn world_delta(
        &self,
        s: &EditorState,
        cursor: [f64; 2],
        size: [f64; 2],
        ctrl: bool,
    ) -> Option<DVec3> {
        let cam = &s.camera;
        let settings = &s.app.transform_settings.snap;
        let points = matches!(self.grab, Grab::Points(_));
        if points
            && settings.enabled
            && let Some(hit) = snap_cursor(
                &self.sources,
                cam,
                (cursor[0], cursor[1]),
                (size[0], size[1]),
                settings,
            )
        {
            return Some(hit.position - self.grabbed);
        }
        let ray = cam.ray(cursor[0], cursor[1], size[0], size[1]);
        let t = ray_plane(&ray, self.grabbed, cam.forward_world())?;
        let delta = ray.at(t) - self.start_hit;
        Some(match settings.quantize_steps(ctrl).filter(|_| points) {
            Some(step) => DVec3::new(
                snap_scalar(delta.x, step.translate),
                snap_scalar(delta.y, step.translate),
                snap_scalar(delta.z, step.translate),
            ),
            None => delta,
        })
    }

    /// One pointer step: sets the grabbed elements to their start values plus the delta with
    /// `spline.edit_points` under this drag's gesture id. No command while nothing moved.
    pub fn update(
        &mut self,
        s: &mut EditorState,
        cursor: [f64; 2],
        size: [f64; 2],
        ctrl: bool,
    ) -> Result<(), String> {
        let Some(delta) = self.world_delta(s, cursor, size, ctrl) else {
            return Ok(());
        };
        let local = self.inverse.transform_vector3(delta);
        if local == self.last {
            return Ok(());
        }
        let edits = match &self.grab {
            Grab::Points(indices) => indices
                .iter()
                .map(
                    |i| json!({"index": i, "position": (self.start.points[*i] + local).to_array()}),
                )
                .collect::<Vec<_>>(),
            Grab::Handle {
                index,
                right,
                broken,
            } => {
                let i = *index;
                let start = if *right {
                    self.start.right[i]
                } else {
                    self.start.left[i]
                };
                let moved = (start + local).to_array();
                let (key, other) = if *right {
                    ("right", "left")
                } else {
                    ("left", "right")
                };
                let mut edit = json!({"index": i, key: moved});
                if !broken {
                    // mirrored tangents (parity-pending G3 item 9)
                    edit[other] = json!((-(start + local)).to_array());
                }
                vec![edit]
            }
        };
        s.app
            .run(
                "spline.edit_points",
                json!({"node": self.node.0, "edits": edits, "gesture": self.gesture}),
            )
            .map_err(|e| e.to_string())?;
        self.last = local;
        Ok(())
    }
}
