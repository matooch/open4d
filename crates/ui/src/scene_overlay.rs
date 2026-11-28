//! Rig and volume overlays of the editor viewport: joints and bones, the weight markers of the
//! selected joints and the boxes of volume meshers, as line lists for the pane's line pass
//! (docs/specs/VIEWPORT.md sections 13.4 and 14).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use forge_core::{EvalCtx, NodeId, Scene, Value};
use forge_rig::joint::{bone_vector, first_child_joint, is_joint, joint_length};
use forge_rig::weights::{find_weight_tag, read_weights, tag_joints};
use forge_viewport::camera::Camera;
use forge_viewport::curves::DrawLine;
use forge_viewport::render::Palette;
use forge_viewport::rig::{BoneStyle, JointDraw, JointStyle, joint_lines, weight_overlay};
use forge_viewport::scene_view::OBJECT_COLOR;
use forge_viewport::volume::{BOX_COLOR, box_lines};
use forge_volume::{MESHER, Preview, PreviewWorker};
use glam::DVec3;

use forge_generators::null_display::{
    NullColorMode, NullData, NullShape, ViewFrame, follows_view, null_display_lines,
    null_display_points, null_objects,
};

/// Cells along the longest axis of the box preview the editor asks for.
pub const EDITOR_PREVIEW_CELLS: u32 = forge_volume::DEFAULT_PREVIEW_CELLS;

/// The weights of one joint on one object: world positions and the joint's weight per point.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightMarkers {
    pub points: Vec<DVec3>,
    pub weights: Vec<f64>,
}

/// Everything the scene overlay draws, before the camera is known.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneOverlay {
    pub joints: Vec<(JointDraw, BoneStyle, JointStyle)>,
    /// One entry per (selected joint, skinned object) pair.
    pub weights: Vec<WeightMarkers>,
    /// World-space lines of the volume boxes.
    pub volume: Vec<DrawLine>,
    /// World-space gizmos of camera, light and floor objects (SCENE-OBJECTS.md S7).
    pub objects: Vec<DrawLine>,
    /// The Nulls the editor shows, with whether each is selected (SCENE-OBJECTS.md S9). Their
    /// lines depend on the viewport camera, so they are made in [`SceneOverlay::lines`].
    pub nulls: Vec<(NullData, bool)>,
}

/// Half length of the Dot's cross, in pixels.
const DOT_HALF_PIXELS: f64 = 2.0;

/// The lines of one Null (S9): its shape in the right frame, placed by the Null's world
/// matrix, or by the viewport camera at the Null's position for a camera-facing flat shape.
fn null_lines(
    n: &NullData,
    selected: bool,
    pal: &Palette,
    cam: &Camera,
    size: [f64; 2],
) -> Vec<DrawLine> {
    let color = if selected {
        pal.selection
    } else {
        match n.color_mode {
            NullColorMode::Custom => n.color.map(|c| c as f32),
            NullColorMode::Layer => n.layer_color.map_or(OBJECT_COLOR, |c| c.map(|c| c as f32)),
            NullColorMode::Material | NullColorMode::Automatic => OBJECT_COLOR,
        }
    };
    let view = ViewFrame {
        right: cam.right_world(),
        up: cam.up_world(),
        toward: -cam.forward_world(),
    };
    let origin = n.world.w_axis.truncate();
    let place = |p: DVec3| {
        if follows_view(n.shape, n.orientation) {
            origin + p
        } else {
            n.world.transform_point3(p)
        }
    };
    let mut out: Vec<DrawLine> =
        null_display_lines(n.shape, n.radius, n.aspect, n.orientation, &view)
            .into_iter()
            .map(|s| DrawLine {
                a: place(s[0]),
                b: place(s[1]),
                color,
            })
            .collect();
    if n.shape == NullShape::Dot {
        let h = DOT_HALF_PIXELS * cam.world_per_pixel(size[1].max(1.0));
        for p in null_display_points(n.shape) {
            let c = place(p);
            for axis in [view.right, view.up] {
                out.push(DrawLine {
                    a: c - axis * h,
                    b: c + axis * h,
                    color,
                });
            }
        }
    }
    out
}

fn enum_param<'a>(scene: &'a Scene, id: NodeId, key: &str) -> &'a str {
    match scene.param(id, key) {
        Some(Value::Enum(s)) => s,
        _ => "",
    }
}

fn bone_style(name: &str) -> BoneStyle {
    match name {
        "none" => BoneStyle::None,
        "box" => BoneStyle::Box,
        "line" => BoneStyle::Line,
        "stick" => BoneStyle::Stick,
        "polygon" => BoneStyle::Polygon,
        _ => BoneStyle::Standard,
    }
}

fn joint_style(name: &str) -> JointStyle {
    match name {
        "none" => JointStyle::None,
        "sphere_wire" => JointStyle::SphereWire,
        "circle" => JointStyle::Circle,
        "sphere" => JointStyle::Sphere,
        _ => JointStyle::Axis,
    }
}

/// Every node of the scene, parents before children.
fn all_nodes(scene: &Scene) -> Vec<NodeId> {
    let mut out = Vec::new();
    for root in scene.roots() {
        out.push(*root);
        out.extend(scene.descendants(*root));
    }
    out
}

impl SceneOverlay {
    /// The joints, the weight markers of the selected joints and no volume lines (those come
    /// from [`VolumeViews`]).
    pub fn collect(scene: &Scene, selected: &BTreeSet<NodeId>, ctx: &EvalCtx) -> SceneOverlay {
        let nodes = all_nodes(scene);
        let mut out = SceneOverlay::default();
        for id in &nodes {
            if !is_joint(scene, *id) || !forge_core::visibility::enabled(scene, *id) {
                continue;
            }
            let Some(world) = scene.world_matrix(*id) else {
                continue;
            };
            let end = match first_child_joint(scene, *id).and_then(|c| scene.world_matrix(c)) {
                Some(c) => c.w_axis.truncate(),
                None => world.transform_point3(bone_vector(scene, *id)),
            };
            out.joints.push((
                JointDraw {
                    world,
                    end: Some(end),
                    length: joint_length(scene, *id),
                    selected: selected.contains(id),
                },
                bone_style(enum_param(scene, *id, "display_object")),
                joint_style(enum_param(scene, *id, "display_joint")),
            ));
        }
        out.objects = object_lines(scene, selected, false);
        out.nulls = null_objects(scene)
            .into_iter()
            .filter(|n| forge_core::visibility::editor_visible(scene, n.node))
            .map(|n| {
                let sel = selected.contains(&n.node);
                (n, sel)
            })
            .collect();
        for joint in selected.iter().filter(|j| is_joint(scene, **j)) {
            out.weights
                .extend(weight_markers(scene, &nodes, *joint, ctx));
        }
        out
    }

    /// The camera-dependent lines of one pane, `size` in pixels.
    pub fn lines(&self, pal: &Palette, cam: &Camera, size: [f64; 2]) -> Vec<DrawLine> {
        let mut out = Vec::new();
        for (d, bone, joint) in &self.joints {
            out.extend(joint_lines(d, *bone, *joint, pal, cam));
        }
        for w in &self.weights {
            out.extend(weight_overlay(&w.points, &w.weights, true, cam, size));
        }
        out.extend(self.volume.iter().copied());
        out.extend(self.objects.iter().copied());
        for (n, selected) in &self.nulls {
            out.extend(null_lines(n, *selected, pal, cam, size));
        }
        out
    }
}

/// Size of the omni and infinite light gizmos, in scene units.
const LIGHT_GIZMO_RADIUS: f64 = 15.0;
/// Depth of the camera frustum and length of the spot and infinite light gizmos.
const GIZMO_DEPTH: f64 = 100.0;
/// Half size and cell count of the floor gizmo grid.
const FLOOR_HALF: f64 = 1000.0;
const FLOOR_CELLS: usize = 10;

/// The gizmo lines of the scene's cameras, lights and floors that the editor shows.
pub(crate) fn object_lines(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
    selected_only: bool,
) -> Vec<DrawLine> {
    use forge_generators::scene_objects::{AreaKind, Lens as ObjLens, LightShape, scene_objects};
    use forge_viewport::scene_view::{Lens, LightGizmo, camera_lines, floor_lines, light_lines};
    let objs = scene_objects(scene);
    let aspect = scene
        .render_config()
        .map_or(16.0 / 9.0, |c| f64::from(c.width) / f64::from(c.height));
    let shown = |id: NodeId| {
        (!selected_only || selected.contains(&id))
            && forge_core::visibility::editor_visible(scene, id)
    };
    let mut out = Vec::new();
    for c in objs.cameras.iter().filter(|c| shown(c.node)) {
        let lens = match c.lens {
            ObjLens::Perspective {
                focal_mm,
                sensor_mm,
                ..
            } => Lens::Perspective {
                focal_mm,
                sensor_mm,
            },
            ObjLens::Orthographic { width } => Lens::Orthographic { width },
        };
        if c.show_cone {
            out.extend(camera_lines(
                c.world,
                lens,
                aspect,
                GIZMO_DEPTH,
                selected.contains(&c.node),
            ));
        }
    }
    for l in objs.lights.iter().filter(|l| shown(l.node)) {
        let shape = match l.shape {
            LightShape::Omni => LightGizmo::Omni {
                radius: LIGHT_GIZMO_RADIUS,
            },
            LightShape::Spot => LightGizmo::Spot {
                outer: l.outer.max(1e-3),
                length: GIZMO_DEPTH,
            },
            LightShape::Infinite => LightGizmo::Infinite {
                radius: LIGHT_GIZMO_RADIUS,
                length: GIZMO_DEPTH,
            },
            LightShape::Area => match l.area {
                AreaKind::Disc { radius } => LightGizmo::AreaDisc { radius },
                AreaKind::Sphere { radius } => LightGizmo::AreaSphere { radius },
                AreaKind::Rectangle { width, height } => LightGizmo::AreaRect { width, height },
                AreaKind::Unsupported(_) => LightGizmo::AreaRect {
                    width: LIGHT_GIZMO_RADIUS * 2.0,
                    height: LIGHT_GIZMO_RADIUS * 2.0,
                },
            },
        };
        out.extend(light_lines(l.world, shape, selected.contains(&l.node)));
    }
    for f in objs.floors.iter().filter(|f| shown(f.node)) {
        out.extend(floor_lines(
            f.world,
            f.extent.min(FLOOR_HALF),
            FLOOR_CELLS,
            OBJECT_COLOR,
        ));
    }
    out
}

/// The weights of `joint` on every object whose weight tag lists it.
fn weight_markers(
    scene: &Scene,
    nodes: &[NodeId],
    joint: NodeId,
    ctx: &EvalCtx,
) -> Vec<WeightMarkers> {
    let mut out = Vec::new();
    for obj in nodes {
        let Some(tag) = find_weight_tag(scene, *obj) else {
            continue;
        };
        let Some(slot) = tag_joints(scene, tag).iter().position(|j| *j == joint) else {
            continue;
        };
        let Ok(Some(mesh)) = forge_generators::evaluate(scene, *obj, ctx) else {
            continue;
        };
        let Some(world) = scene.world_matrix(*obj) else {
            continue;
        };
        let map = read_weights(scene, tag);
        let weights: Vec<f64> = (0..mesh.points.len() as u32)
            .map(|p| {
                map.get(p)
                    .iter()
                    .find(|(s, _)| *s as usize == slot)
                    .map_or(0.0, |(_, w)| *w)
            })
            .collect();
        out.push(WeightMarkers {
            points: mesh
                .points
                .iter()
                .map(|p| world.transform_point3(*p))
                .collect(),
            weights,
        });
    }
    out
}

/// The preview workers of the scene's volume meshers (VIEWPORT.md section 14): one per mesher,
/// asked again when the scene revision changes, never waited for.
#[derive(Default)]
pub struct VolumeViews {
    workers: BTreeMap<NodeId, (PreviewWorker, u64)>,
    /// Compute inline instead of on a worker thread (tests, screenshots).
    pub synchronous: bool,
}

impl VolumeViews {
    pub fn synchronous() -> VolumeViews {
        VolumeViews {
            synchronous: true,
            ..VolumeViews::default()
        }
    }

    /// The box lines (world space) of every mesher in `scene`, and whether a preview is still
    /// being computed (the caller repaints).
    pub fn lines(&mut self, scene: &Scene, ctx: &EvalCtx) -> (Vec<DrawLine>, bool) {
        let meshers: Vec<NodeId> = all_nodes(scene)
            .into_iter()
            .filter(|n| scene.node(*n).is_some_and(|n| n.type_id == MESHER))
            .filter(|n| forge_core::visibility::enabled(scene, *n))
            .collect();
        self.workers.retain(|id, _| meshers.contains(id));
        let mut out = Vec::new();
        let mut pending = false;
        for id in meshers {
            let preview: Option<Arc<Preview>> = if self.synchronous {
                forge_volume::build_preview(scene, id, ctx, EDITOR_PREVIEW_CELLS)
                    .ok()
                    .flatten()
                    .map(Arc::new)
            } else {
                let entry = self
                    .workers
                    .entry(id)
                    .or_insert_with(|| (PreviewWorker::standard(EDITOR_PREVIEW_CELLS), u64::MAX));
                if entry.1 != scene.revision() {
                    entry.0.request(scene, id);
                    entry.1 = scene.revision();
                }
                let snap = entry.0.snapshot();
                pending |= snap.busy;
                snap.preview
            };
            let (Some(preview), Some(world)) = (preview, scene.world_matrix(id)) else {
                continue;
            };
            let Some((lo, hi)) = preview.bounds else {
                continue;
            };
            out.extend(box_lines(lo, hi, BOX_COLOR).into_iter().map(|l| DrawLine {
                a: world.transform_point3(l.a),
                b: world.transform_point3(l.b),
                color: l.color,
            }));
        }
        (out, pending)
    }

    /// The whole overlay of a frame: [`SceneOverlay::collect`] plus the volume boxes.
    pub fn overlay(
        &mut self,
        scene: &Scene,
        selected: &BTreeSet<NodeId>,
        ctx: &EvalCtx,
    ) -> (SceneOverlay, bool) {
        let mut o = SceneOverlay::collect(scene, selected, ctx);
        let (volume, pending) = self.lines(scene, ctx);
        o.volume = volume;
        (o, pending)
    }
}
