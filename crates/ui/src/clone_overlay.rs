//! Clone display overlay: the index and the axes of each clone, projected to the viewport
//! (docs/specs/UI.md §16.3). Pure: the editor paints the marks with the egui painter.

use forge_clone_engine::CloneData;
use forge_viewport::camera::Camera;
use glam::{DMat4, DVec3};

/// Most marks one cloner contributes (lowest clone indices first).
pub const MAX_MARKS: usize = 2000;

/// Length of a drawn axis in pixels.
pub const AXIS_PX: f64 = 24.0;

/// What to draw for each clone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OverlayOptions {
    pub indices: bool,
    pub axes: bool,
}

/// One clone as the overlay shows it, in pixels relative to the viewport's top left.
#[derive(Debug, Clone, PartialEq)]
pub struct CloneMark {
    pub index: u32,
    pub origin: [f32; 2],
    /// Screen end points of the X, Y, Z axes when axes are shown.
    pub axes: Option<[[f32; 2]; 3]>,
}

/// The marks of `clones` (matrices in cloner space, `world` places the cloner) seen by
/// `camera` in a viewport of `size` pixels. Hidden clones, clones behind the camera and
/// clones whose origin is outside the viewport are skipped; at most [`MAX_MARKS`].
pub fn clone_marks(
    clones: &CloneData,
    world: &DMat4,
    camera: &Camera,
    size: [f64; 2],
    opts: OverlayOptions,
) -> Vec<CloneMark> {
    if (!opts.indices && !opts.axes) || size[0] <= 0.0 || size[1] <= 0.0 {
        return Vec::new();
    }
    let axis_world = AXIS_PX * camera.world_per_pixel(size[1]);
    let project = |p: DVec3| {
        camera
            .project(p, size[0], size[1])
            .map(|s| [s.x as f32, s.y as f32])
    };
    let inside = |p: [f32; 2]| {
        p[0] >= 0.0 && p[1] >= 0.0 && f64::from(p[0]) <= size[0] && f64::from(p[1]) <= size[1]
    };
    let mut out = Vec::new();
    for (i, m) in clones.matrices.iter().enumerate() {
        if out.len() >= MAX_MARKS {
            break;
        }
        if !clones.visible.get(i).copied().unwrap_or(true) {
            continue;
        }
        let m = *world * *m;
        let o = m.w_axis.truncate();
        let Some(origin) = project(o).filter(|p| inside(*p)) else {
            continue;
        };
        let axes = opts.axes.then(|| {
            [m.x_axis, m.y_axis, m.z_axis].map(|a| {
                let dir = a.truncate().normalize_or_zero();
                project(o + dir * axis_world).unwrap_or(origin)
            })
        });
        out.push(CloneMark {
            index: clones.index.get(i).copied().unwrap_or(i as u32),
            origin,
            axes,
        });
    }
    out
}

/// The marks of every selected cloner. A cloner whose `viewport_mode` is `matrix` shows its
/// axes even when the view option is off.
pub fn selected_cloner_marks(
    scene: &forge_core::Scene,
    selected: &std::collections::BTreeSet<forge_core::NodeId>,
    ctx: &forge_core::EvalCtx,
    camera: &Camera,
    size: [f64; 2],
    view: OverlayOptions,
) -> Vec<CloneMark> {
    let mut out = Vec::new();
    for id in selected {
        let Some(node) = scene
            .node(*id)
            .filter(|n| n.type_id == forge_clone_engine::CLONER)
        else {
            continue;
        };
        let matrix_mode = matches!(
            node.params.get("viewport_mode"),
            Some(forge_core::Value::Enum(m)) if m == "matrix"
        );
        let opts = OverlayOptions {
            indices: view.indices,
            axes: view.axes || matrix_mode,
        };
        if !opts.indices && !opts.axes {
            continue;
        }
        let Some(world) = scene.world_matrix(*id) else {
            continue;
        };
        if let Ok(Some(clones)) = forge_generators::clone_data(scene, *id, ctx) {
            out.extend(clone_marks(&clones, &world, camera, size, opts));
        }
    }
    out
}
