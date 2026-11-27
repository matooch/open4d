//! UV Editor workspace tab (UV.md U7). The canvas previews; registry commands edit.

use std::collections::BTreeSet;

use egui::{PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use forge_app::uv_commands::{UvMesh, read};
use forge_core::NodeId;
use forge_geometry::mesh::Edge;
use glam::DVec2;
use serde_json::json;

use crate::object_manager::Cmd;
use crate::state::EditorState;

struct Move {
    origin: Pos2,
    delta: DVec2,
}

pub struct UvEditor {
    object: Option<NodeId>,
    navigation: crate::navigation::CanvasNavigation,
    pan: Vec2,
    zoom: f64,
    selected: BTreeSet<usize>,
    gesture: Option<Move>,
    seams: bool,
    scale: f64,
    angle: f64,
    padding: f64,
}

impl Default for UvEditor {
    fn default() -> Self {
        Self {
            object: None,
            navigation: Default::default(),
            pan: Vec2::ZERO,
            zoom: 1.0,
            selected: BTreeSet::new(),
            gesture: None,
            seams: false,
            scale: 1.0,
            angle: 0.0,
            padding: 0.01,
        }
    }
}

struct View {
    centre: DVec2,
    origin: Pos2,
    zoom: f64,
}

impl View {
    fn fit(rect: Rect, coords: &[DVec2]) -> Self {
        let (mut lo, mut hi) = (DVec2::ZERO, DVec2::ONE);
        for &p in coords {
            lo = lo.min(p);
            hi = hi.max(p);
        }
        let span = hi - lo;
        Self {
            centre: (lo + hi) * 0.5,
            origin: rect.center(),
            zoom: (f64::from((rect.width() - 24.0).max(1.0)) / span.x)
                .min(f64::from((rect.height() - 24.0).max(1.0)) / span.y),
        }
    }

    fn point(&self, p: DVec2) -> Pos2 {
        let [x, y] = ((p - self.centre) * self.zoom).to_array();
        self.origin + Vec2::new(x as f32, -y as f32)
    }

    fn offset(&self, d: Vec2) -> DVec2 {
        DVec2::new(f64::from(d.x), -f64::from(d.y)) / self.zoom
    }
}

fn point_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let v = b - a;
    let t = if v.length_sq() > 0.0 {
        ((p - a).dot(v) / v.length_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + v * t)
}

fn edges(source: &UvMesh) -> impl Iterator<Item = (u32, usize, usize)> + '_ {
    let edge_ids = source.mesh.edge_ids();
    let mut start = 0;
    source.mesh.polys.iter().flat_map(move |poly| {
        let base = start;
        start += poly.len();
        (0..poly.len())
            .map({
                let edge_ids = &edge_ids;
                move |i| {
                    let j = (i + 1) % poly.len();
                    (edge_ids[&Edge::new(poly[i], poly[j])], base + i, base + j)
                }
            })
            .collect::<Vec<_>>()
    })
}

impl UvEditor {
    pub fn show(&mut self, ui: &mut Ui, state: &EditorState) -> Vec<Cmd> {
        let mut cmds = Vec::new();
        let object = state.selection.active;
        if object != self.object {
            self.object = object;
            self.pan = Vec2::ZERO;
            self.zoom = 1.0;
            self.selected.clear();
            self.gesture = None;
        }
        let Some((id, source)) =
            object.and_then(|id| read(state.doc.scene(), id).ok().map(|s| (id, s)))
        else {
            ui.weak("Select an editable polygon object.");
            return cmds;
        };
        let coords: Vec<DVec2> = source
            .uv
            .corners
            .iter()
            .flatten()
            .copied()
            .map(DVec2::from)
            .collect();
        self.selected.retain(|&i| i < coords.len());
        ui.horizontal_wrapped(|ui| {
            if ui.button("Select all UVs").clicked() {
                self.selected = (0..coords.len()).collect();
            }
            ui.checkbox(&mut self.seams, "Mark seams");
            if ui.button("Relax").clicked() {
                cmds.push(("uv.relax", json!({"object":id.0})));
            }
            let padding = ui.label("Padding");
            ui.add(
                egui::DragValue::new(&mut self.padding)
                    .speed(0.001)
                    .range(0.0..=0.49),
            )
            .labelled_by(padding.id);
            if ui.button("Pack").clicked() {
                cmds.push(("uv.pack", json!({"object":id.0,"padding":self.padding})));
            }
            ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
                let scale = ui.label("Scale");
                ui.add(egui::DragValue::new(&mut self.scale).speed(0.01))
                    .labelled_by(scale.id);
                let angle = ui.label("Angle");
                ui.add(egui::DragValue::new(&mut self.angle).speed(1.0).suffix("°"))
                    .labelled_by(angle.id);
                if ui.button("Transform").clicked() {
                    cmds.push((
                        "uv.transform",
                        json!({
                            "object":id.0, "corners":self.selected, "scale":[self.scale,self.scale],
                            "rotation_deg":self.angle,
                        }),
                    ));
                }
            });
        });
        let (rect, response) = ui.allocate_exact_size(
            ui.available_size().max(Vec2::splat(16.0)),
            Sense::click_and_drag(),
        );
        use crate::navigation::CanvasMotion;
        let (navigating, motion) = self.navigation.read(&response);
        if let Some(motion) = motion {
            match motion {
                CanvasMotion::Pan(delta) => self.pan += delta,
                CanvasMotion::Zoom { anchor, factor } => {
                    let zoom = (self.zoom * f64::from(factor)).clamp(0.1, 64.0);
                    let origin = rect.center() + self.pan;
                    self.pan += (origin - anchor) * ((zoom / self.zoom) as f32 - 1.0);
                    self.zoom = zoom;
                }
            }
        }
        let mut view = View::fit(rect, &coords);
        view.origin += self.pan;
        view.zoom *= self.zoom;
        let base_positions: Vec<_> = coords.iter().copied().map(|p| view.point(p)).collect();
        let lines = edges(&source).collect::<Vec<_>>();
        if navigating {
            self.gesture = None;
        } else if self.seams {
            if response.clicked_by(PointerButton::Primary)
                && let Some(p) = response.interact_pointer_pos()
                && let Some(edge) = lines
                    .iter()
                    .filter_map(|&(edge, i, j)| {
                        let d = point_segment(p, base_positions[i], base_positions[j]);
                        (d <= 8.0).then_some((edge, d))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
                    .map(|(edge, _)| edge)
            {
                cmds.push((
                    "uv.seam",
                    json!({"object":id.0, "edge":edge, "marked":!source.seams.contains(&edge)}),
                ));
            }
            self.gesture = None;
        } else {
            let start = response.drag_started_by(PointerButton::Primary);
            let click = response.clicked_by(PointerButton::Primary);
            if start || click {
                let position = if start {
                    ui.input(|i| i.pointer.press_origin())
                } else {
                    response.interact_pointer_pos()
                };
                if let Some(pos) = position {
                    self.pick_vertex(ui, &source, &coords, &base_positions, pos, start);
                }
            }
            if let Some(gesture) = &mut self.gesture {
                if let Some(p) = ui.input(|i| i.pointer.interact_pos()) {
                    gesture.delta = view.offset(p - gesture.origin);
                }
                if response.drag_stopped_by(PointerButton::Primary)
                    && let Some(m) = self.gesture.take()
                    && m.delta.length_squared() > 0.0
                {
                    cmds.push((
                        "uv.transform",
                        json!({"object":id.0,"corners":self.selected,"offset":m.delta.to_array()}),
                    ));
                } else if !response.dragged_by(PointerButton::Primary)
                    && !ui.input(|i| i.pointer.primary_down())
                {
                    self.gesture = None;
                }
            }
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
        let tile = Rect::from_two_pos(view.point(DVec2::ZERO), view.point(DVec2::ONE));
        painter.rect_stroke(
            tile,
            0.0,
            Stroke::new(1.0, ui.visuals().text_color()),
            StrokeKind::Inside,
        );
        let delta = self.gesture.as_ref().map_or(DVec2::ZERO, |m| m.delta);
        let position = |i: usize| {
            if self.selected.contains(&i) && delta != DVec2::ZERO {
                view.point(coords[i] + delta)
            } else {
                base_positions[i]
            }
        };
        for (edge, a, b) in lines {
            let stroke = if source.seams.contains(&edge) {
                Stroke::new(2.0, crate::theme::Theme::current(ui.ctx()).record)
            } else {
                Stroke::new(1.0, ui.visuals().weak_text_color())
            };
            painter.line_segment([position(a), position(b)], stroke);
        }
        for i in 0..coords.len() {
            let colour = if self.selected.contains(&i) {
                crate::theme::Theme::current(ui.ctx()).selection
            } else {
                ui.visuals().text_color()
            };
            painter.circle_filled(position(i), 2.4, colour);
        }
        cmds
    }

    fn pick_vertex(
        &mut self,
        ui: &Ui,
        source: &UvMesh,
        coords: &[DVec2],
        positions: &[Pos2],
        p: Pos2,
        start: bool,
    ) {
        let hit = positions
            .iter()
            .enumerate()
            .filter_map(|(i, pos)| {
                let d = p.distance(*pos);
                (d <= 8.0).then_some((i, d))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)))
            .map(|(i, _)| i);
        let shift = ui.input(|i| i.modifiers.shift);
        if let Some(i) = hit {
            let mut mesh_points = source.mesh.polys.iter().flatten();
            let point = *mesh_points.nth(i).expect("corner and UV counts agree");
            if !start || !self.selected.contains(&i) {
                if !shift {
                    self.selected.clear();
                }
                let remove = shift && !start && self.selected.contains(&i);
                for (j, &v) in source.mesh.polys.iter().flatten().enumerate() {
                    if v == point && coords[i].abs_diff_eq(coords[j], 1e-9) {
                        if remove {
                            self.selected.remove(&j);
                        } else {
                            self.selected.insert(j);
                        }
                    }
                }
            }
            if start {
                self.gesture = Some(Move {
                    origin: p,
                    delta: DVec2::ZERO,
                });
            }
        } else if !shift {
            self.selected.clear();
        }
    }
}
