//! The Weight Manager panel and the weight paint stroke (docs/specs/RIG.md section 9.4).
//! Every edit is a `weights.*` command; a drag of dabs is grouped into one undo step.

use forge_core::{EvalCtx, NodeId};
use forge_rig::weights::{find_weight_tag, read_weights, tag_joints};
use forge_viewport::camera::Ray;
use forge_viewport::picking::pick_poly;
use serde_json::json;

use crate::object_manager::Cmd;
use crate::state::EditorState;

/// Brush mode names, as the `weights.paint` command spells them.
pub const MODES: [&str; 4] = ["add", "subtract", "set", "smooth"];
/// Falloff names.
pub const FALLOFFS: [&str; 3] = ["smooth", "linear", "constant"];

#[derive(Debug, Clone, PartialEq)]
pub struct WeightPanel {
    /// Viewport clicks and drags paint instead of selecting.
    pub painting: bool,
    /// The joint painted (the tag's first joint when unset or not listed).
    pub joint: Option<NodeId>,
    pub mode: usize,
    pub falloff: usize,
    pub radius: f64,
    pub strength: f64,
    pub prune_threshold: f64,
    /// Undo depth before the current stroke's first dab.
    pub stroke_depth: Option<usize>,
}

impl Default for WeightPanel {
    fn default() -> Self {
        WeightPanel {
            painting: false,
            joint: None,
            mode: 0,
            falloff: 0,
            radius: 50.0,
            strength: 0.5,
            prune_threshold: 0.01,
            stroke_depth: None,
        }
    }
}

/// The active object and its weight tag, when it has one.
pub fn paint_target(s: &EditorState) -> Option<(NodeId, NodeId)> {
    let obj = s.selection.active?;
    let tag = find_weight_tag(s.doc.scene(), obj)?;
    Some((obj, tag))
}

/// The joint a dab paints: the panel's, if the tag lists it, else the tag's first.
pub fn active_joint(s: &EditorState) -> Option<NodeId> {
    let (_, tag) = paint_target(s)?;
    let joints = tag_joints(s.doc.scene(), tag);
    s.weight_panel
        .joint
        .filter(|j| joints.contains(j))
        .or_else(|| joints.first().copied())
}

/// One dab at pixel `cursor` of a viewport of `size`: the surface point under the cursor of the
/// active object's mesh, painted with the panel's brush. Returns whether anything was hit. The
/// first dab of a stroke notes the undo depth ([`end_stroke`] groups the stroke).
///
/// # Errors
/// The command's error text.
pub fn paint_dab(s: &mut EditorState, cursor: [f64; 2], size: [f64; 2]) -> Result<bool, String> {
    let Some((obj, _)) = paint_target(s) else {
        return Ok(false);
    };
    let Some(joint) = active_joint(s) else {
        return Ok(false);
    };
    let scene = s.doc.scene();
    let Some(mesh) = forge_generators::evaluate(scene, obj, &EvalCtx::default())
        .ok()
        .flatten()
    else {
        return Ok(false);
    };
    let Some(world) = scene.world_matrix(obj) else {
        return Ok(false);
    };
    let inv = world.inverse();
    let ray = s.camera.ray(cursor[0], cursor[1], size[0], size[1]);
    let local = Ray {
        origin: inv.transform_point3(ray.origin),
        dir: inv.transform_vector3(ray.dir).normalize_or_zero(),
    };
    let Some(hit) = pick_poly(&mesh, &local) else {
        return Ok(false);
    };
    let p = &s.weight_panel;
    let args = json!({
        "object": obj.0, "joint": joint.0, "center": hit.point.to_array(),
        "radius": p.radius, "strength": p.strength,
        "mode": MODES[p.mode.min(3)], "falloff": FALLOFFS[p.falloff.min(2)],
    });
    if s.weight_panel.stroke_depth.is_none() {
        s.weight_panel.stroke_depth = Some(s.doc.undo_depth());
    }
    crate::commands::execute(s, "weights.paint", args)?;
    Ok(true)
}

/// Ends a stroke: all its dabs become one undo step.
pub fn end_stroke(s: &mut EditorState) {
    if let Some(depth) = s.weight_panel.stroke_depth.take()
        && s.doc.undo_depth() > depth + 1
    {
        s.doc.group_undo_since(depth, "Paint Weights");
    }
}

/// The panel. Returns the commands its buttons ask for.
pub fn weight_manager_ui(ui: &mut egui::Ui, st: &mut EditorState) -> Vec<Cmd> {
    let mut cmds: Vec<Cmd> = Vec::new();
    let Some((obj, tag)) = paint_target(st) else {
        ui.weak("Select an object with a weight tag");
        return cmds;
    };
    let scene = st.doc.scene();
    let name = scene.node(obj).map(|n| n.name.clone()).unwrap_or_default();
    let joints = tag_joints(scene, tag);
    let weights = read_weights(scene, tag);
    let counts: Vec<usize> = (0..joints.len())
        .map(|slot| {
            weights
                .points()
                .filter(|p| weights.get(*p).iter().any(|(s, _)| *s as usize == slot))
                .count()
        })
        .collect();
    let labels: Vec<String> = joints
        .iter()
        .zip(&counts)
        .map(|(j, n)| {
            let jn = scene.node(*j).map(|n| n.name.clone()).unwrap_or_default();
            format!("{jn} ({n})")
        })
        .collect();
    let active = active_joint(st);
    let p = &mut st.weight_panel;
    ui.label(format!("Weights of {name}"));
    egui::ScrollArea::vertical()
        .max_height(160.0)
        .id_salt("weight_joints")
        .show(ui, |ui| {
            // the joint rows are selected things: the selection colour (UI.md §24.1)
            crate::theme::Theme::current(ui.ctx()).selection_scope(ui, |ui| {
                for (j, label) in joints.iter().zip(&labels) {
                    if ui.selectable_label(active == Some(*j), label).clicked() {
                        p.joint = Some(*j);
                    }
                }
            });
        });
    ui.separator();
    ui.checkbox(&mut p.painting, "Paint");
    ui.horizontal(|ui| {
        for (i, m) in MODES.iter().enumerate() {
            ui.selectable_value(&mut p.mode, i, *m);
        }
    });
    ui.horizontal(|ui| {
        for (i, f) in FALLOFFS.iter().enumerate() {
            ui.selectable_value(&mut p.falloff, i, *f);
        }
    });
    ui.horizontal(|ui| {
        let l = ui.label("Radius");
        ui.add(
            egui::DragValue::new(&mut p.radius)
                .range(0.001..=1e9)
                .speed(1.0),
        )
        .labelled_by(l.id);
        let l = ui.label("Strength");
        ui.add(
            egui::DragValue::new(&mut p.strength)
                .range(0.0..=1.0)
                .speed(0.01),
        )
        .labelled_by(l.id);
    });
    ui.separator();
    ui.horizontal_wrapped(|ui| {
        if ui.button("Normalise").clicked() {
            cmds.push(("weights.normalise", json!({"object": obj.0})));
        }
        if ui.button("Prune").clicked() {
            cmds.push((
                "weights.prune",
                json!({"object": obj.0, "threshold": p.prune_threshold}),
            ));
        }
        if ui.button("Smooth").clicked() {
            cmds.push(("weights.smooth", json!({"object": obj.0})));
        }
        if ui.button("Mirror").clicked() {
            cmds.push(("weights.mirror", json!({"object": obj.0})));
        }
    });
    cmds
}
