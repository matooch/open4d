//! Camera commands of the editor: framing, named views, `view.set_camera` / `view.get_camera`
//! (docs/specs/UI.md §6.1, COMMANDS.md §6). The maths is `forge_viewport::nav`.

use std::collections::BTreeSet;

use forge_command::{CameraSet, ProjectionArg, ViewArg};
use forge_core::{EvalCtx, NodeId, Scene};
use forge_viewport::camera::{Camera, Projection};
use forge_viewport::nav::{StandardView, frame_box, view_camera};
use glam::DVec3;
use serde_json::{Value, json};

use crate::state::EditorState;
use crate::viewport_panel::scene_meshes_at;

/// World bounding box of the visible geometry. With `only`, restricted to those nodes and
/// their descendants; when none of them has geometry, the box of their world positions.
/// `None` when there is nothing to frame.
pub fn scene_bounds(
    scene: &Scene,
    ctx: &EvalCtx,
    only: Option<&BTreeSet<NodeId>>,
) -> Option<(DVec3, DVec3)> {
    let included: Option<BTreeSet<NodeId>> = only.map(|sel| {
        sel.iter()
            .flat_map(|id| std::iter::once(*id).chain(scene.descendants(*id)))
            .collect()
    });
    let mut points: Vec<DVec3> = Vec::new();
    for (id, mesh, _, _) in scene_meshes_at(scene, &BTreeSet::new(), ctx) {
        if included.as_ref().is_some_and(|set| !set.contains(&id)) {
            continue;
        }
        points.extend(mesh.points.iter().copied());
    }
    if points.is_empty()
        && let Some(sel) = only
    {
        points.extend(
            sel.iter()
                .filter_map(|id| scene.world_matrix(*id))
                .map(|m| m.transform_point3(DVec3::ZERO)),
        );
    }
    let first = *points.first()?;
    Some(
        points
            .iter()
            .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p))),
    )
}

fn standard_view(v: ViewArg) -> StandardView {
    match v {
        ViewArg::Perspective => StandardView::Perspective,
        ViewArg::Front => StandardView::Front,
        ViewArg::Back => StandardView::Back,
        ViewArg::Left => StandardView::Left,
        ViewArg::Right => StandardView::Right,
        ViewArg::Top => StandardView::Top,
        ViewArg::Bottom => StandardView::Bottom,
    }
}

impl EditorState {
    fn frame_nodes(&mut self, only: Option<BTreeSet<NodeId>>) {
        self.touch_view();
        let scene = self.evaluated();
        let ctx = EvalCtx::at_frame(self.timeline.current, f64::from(scene.settings().fps));
        match scene_bounds(&scene, &ctx, only.as_ref()) {
            Some((min, max)) => {
                self.camera = frame_box(&self.camera, min, max, self.view_aspect);
            }
            // no geometry: back to the default view
            None if only.is_none() => self.camera = EditorState::default_camera(),
            None => {}
        }
    }

    /// `view.frame_all`.
    pub fn frame_all_geometry(&mut self) {
        self.frame_nodes(None);
    }

    /// `view.frame_selected`: nothing selected frames everything.
    pub fn frame_selected_nodes(&mut self) {
        let sel = self.selection.nodes.clone();
        self.frame_nodes((!sel.is_empty()).then_some(sel));
    }

    pub fn show_view(&mut self, view: ViewArg) {
        self.touch_view();
        self.camera = view_camera(&self.camera, standard_view(view));
    }

    pub fn apply_camera_set(&mut self, set: CameraSet) {
        self.touch_view();
        let c = &mut self.camera;
        if let Some(t) = set.target {
            c.target = DVec3::from(t) * c.mirror();
        }
        if let Some(d) = set.distance {
            c.distance = d;
        }
        if let Some(y) = set.yaw {
            c.yaw = y.to_radians();
        }
        if let Some(p) = set.pitch {
            c.orbit(0.0, p.to_radians() - c.pitch);
        }
        let fov_y = match c.projection {
            Projection::Perspective { fov_y } => fov_y,
            Projection::Orthographic { .. } => std::f64::consts::FRAC_PI_4,
        };
        let height = match c.projection {
            Projection::Orthographic { height } => height,
            Projection::Perspective { fov_y } => 2.0 * c.distance * (fov_y / 2.0).tan(),
        };
        c.projection = match set.projection {
            Some(ProjectionArg::Perspective) => Projection::Perspective {
                fov_y: set.fov.map_or(fov_y, f64::to_radians),
            },
            Some(ProjectionArg::Orthographic) => Projection::Orthographic {
                height: set.height.unwrap_or(height),
            },
            None => match c.projection {
                Projection::Perspective { .. } => Projection::Perspective {
                    fov_y: set.fov.map_or(fov_y, f64::to_radians),
                },
                Projection::Orthographic { .. } => Projection::Orthographic {
                    height: set.height.unwrap_or(height),
                },
            },
        };
    }

    pub fn camera_json(&self) -> Value {
        camera_json(&self.camera)
    }
}

/// `view.get_camera` result.
pub fn camera_json(c: &Camera) -> Value {
    let v = |p: DVec3| json!([p.x, p.y, p.z]);
    let mut out = json!({
        "target": v(c.target_world()),
        "eye": v(c.eye_world()),
        "distance": c.distance,
        "yaw": c.yaw.to_degrees(),
        "pitch": c.pitch.to_degrees(),
    });
    match c.projection {
        Projection::Perspective { fov_y } => {
            out["projection"] = json!("perspective");
            out["fov"] = json!(fov_y.to_degrees());
        }
        Projection::Orthographic { height } => {
            out["projection"] = json!("orthographic");
            out["height"] = json!(height);
        }
    }
    out
}
