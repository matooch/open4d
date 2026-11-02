//! The object manager panel: columns, dots, tick, tags, drag and drop, rename, search and
//! context menu (docs/specs/UI.md §13). The logic that decides things is in `tree`; this
//! file draws and turns gestures into registry commands.

use egui::{Align, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, vec2};
use forge_core::NodeId;
use forge_core::visibility::Vis;
use serde_json::{Value as Json, json};

use crate::state::EditorState;
use crate::tree::{
    DropTarget, DropZone, TreeRow, click_selection, context_menu, drop_target, drop_zone,
    filter_rows, menu_target, moved_set, tree_rows,
};

/// A command to run: registry id and JSON arguments.
pub type Cmd = (&'static str, Json);

/// Panel state that is not document state.
#[derive(Default)]
pub struct ObjectManager {
    /// Row being renamed, with its edit buffer.
    pub renaming: Option<(NodeId, String)>,
    /// The search text (UI.md §13.5).
    pub filter: String,
    /// An expression tag that was double-clicked; the editor opens the Node Editor on it.
    pub open_tag: Option<NodeId>,
    /// Last pointer position seen during a drag (the release frame may have none).
    last_pointer: Option<Pos2>,
}

const INDENT: f32 = 14.0;
const DOT: f32 = 14.0;
const RED: Color32 = Color32::from_rgb(214, 74, 66);
const GREEN: Color32 = Color32::from_rgb(86, 178, 98);
const GREY: Color32 = Color32::from_gray(120);

fn vis_colour(v: Vis) -> Color32 {
    match v {
        Vis::Default => GREY,
        Vis::Off => RED,
        Vis::On => GREEN,
    }
}

/// A round clickable dot with an accessibility label.
fn dot(ui: &mut Ui, colour: Color32, label: String) -> egui::Response {
    let r = ui.allocate_response(vec2(DOT, DOT), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
    ui.painter()
        .circle_filled(r.rect.center(), DOT * 0.32, colour);
    if r.hovered() {
        ui.painter().circle_stroke(
            r.rect.center(),
            DOT * 0.42,
            Stroke::new(1.0, ui.visuals().strong_text_color()),
        );
    }
    r
}

/// The enable tick: a box, ticked when the object is enabled.
fn tick(ui: &mut Ui, on: bool, label: String) -> egui::Response {
    let r = ui.allocate_response(vec2(DOT, DOT), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
    let b = r.rect.shrink(2.0);
    let ink = ui.visuals().text_color();
    ui.painter()
        .rect_stroke(b, 2.0, Stroke::new(1.0, ink), egui::StrokeKind::Inside);
    if on {
        let s = Stroke::new(1.6, GREEN);
        let (p0, p1, p2) = (
            b.left_top() + vec2(b.width() * 0.2, b.height() * 0.55),
            b.left_top() + vec2(b.width() * 0.42, b.height() * 0.8),
            b.left_top() + vec2(b.width() * 0.82, b.height() * 0.2),
        );
        ui.painter().line_segment([p0, p1], s);
        ui.painter().line_segment([p1, p2], s);
    }
    r
}

impl ObjectManager {
    /// Draws the panel and returns the commands to run (the caller runs them in order).
    pub fn show(&mut self, ui: &mut Ui, st: &mut EditorState) -> Vec<Cmd> {
        let mut cmds: Vec<Cmd> = Vec::new();
        let ctx = ui.ctx().clone();
        let panel = ui.max_rect();

        ui.horizontal(|ui| {
            let name = ui.label("Find");
            ui.add(
                egui::TextEdit::singleline(&mut self.filter)
                    .hint_text("name, type or tag")
                    .desired_width(f32::INFINITY),
            )
            .labelled_by(name.id);
        });
        let filtering = !self.filter.trim().is_empty();
        let rows = if filtering {
            filter_rows(st.doc.scene(), &self.filter)
        } else {
            tree_rows(st.doc.scene(), &st.expanded)
        };
        let order: Vec<NodeId> = rows.iter().map(|r| r.id).collect();
        let mut rects: Vec<(NodeId, Rect)> = Vec::new();

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for row in &rows {
                    let rect = self.row_ui(ui, st, row, filtering, &order, &mut cmds);
                    rects.push((row.id, rect));
                }
                // The empty area below the rows: drop = end of the root list.
                let size = egui::vec2(ui.available_width(), ui.available_height().max(40.0));
                let rest = ui.allocate_response(size, Sense::hover());
                rects.push((NodeId(0), rest.rect));
            });

        if !filtering {
            self.drag_and_drop(&ctx, ui, st, &rows, &rects, panel, &mut cmds);
        }
        cmds
    }

    /// Draws one row and handles its clicks; returns the row's rectangle.
    fn row_ui(
        &mut self,
        ui: &mut Ui,
        st: &mut EditorState,
        row: &TreeRow,
        filtering: bool,
        order: &[NodeId],
        cmds: &mut Vec<Cmd>,
    ) -> Rect {
        let resp = ui.horizontal(|ui| {
            ui.add_space(row.depth as f32 * INDENT);
            if row.has_children && !filtering {
                let arrow = if row.expanded { "-" } else { "+" };
                if ui.small_button(arrow).clicked() {
                    cmds.push(("view.toggle_expanded", json!({"node": row.id.0})));
                }
            } else {
                ui.add_space(18.0);
            }
            if let Some((id, buf)) = &mut self.renaming
                && *id == row.id
            {
                let r = ui.text_edit_singleline(buf);
                r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, "Rename"));
                if !r.has_focus() && !r.lost_focus() {
                    r.request_focus();
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.renaming = None;
                } else if r.lost_focus() {
                    if !buf.trim().is_empty() && *buf != row.name {
                        cmds.push(("object.rename", json!({"node": row.id.0, "name": buf})));
                    }
                    self.renaming = None;
                }
                return;
            }
            let selected = st.selection.nodes.contains(&row.id);
            let mut text = RichText::new(&row.name);
            if row.missing_type {
                text = text.italics();
            }
            if row.dim || !row.enabled {
                text = text.weak();
            }
            let r = ui
                .selectable_label(selected, text)
                .interact(Sense::click_and_drag());
            r.dnd_set_drag_payload(row.id);
            if r.double_clicked() {
                self.renaming = Some((row.id, row.name.clone()));
            } else if r.clicked() {
                let m = ui.input(|i| i.modifiers);
                click_selection(&mut st.selection, order, row.id, m.shift, m.command);
            }
            if r.secondary_clicked() {
                menu_target(&mut st.selection, row.id);
            }
            let items = context_menu(st.doc.scene(), &st.selection.nodes);
            r.context_menu(|ui| {
                for item in items {
                    if ui.button(item.label).clicked() {
                        match item.command {
                            Some(c) => cmds.push((c, item.args)),
                            None => self.renaming = Some((row.id, row.name.clone())),
                        }
                        ui.close();
                    }
                }
            });
            ui.weak(&row.type_label);
            for (tag_id, tag) in &row.tags {
                let text = RichText::new(format!("[{tag}]")).weak();
                let r = ui.add(egui::Label::new(text).sense(Sense::click()));
                if r.double_clicked()
                    && st
                        .doc
                        .scene()
                        .node(*tag_id)
                        .is_some_and(|n| n.type_id == forge_nodes::TAG)
                {
                    self.open_tag = Some(*tag_id);
                }
            }
            // Dots and tick, right aligned (UI.md §13.1).
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let target = |st: &EditorState| {
                    if st.selection.nodes.contains(&row.id) {
                        st.selection.nodes.iter().map(|n| n.0).collect::<Vec<_>>()
                    } else {
                        vec![row.id.0]
                    }
                };
                let name = &row.name;
                if tick(ui, row.enabled, format!("Enabled: {name}")).clicked() {
                    cmds.push((
                        "object.enable",
                        json!({"nodes": target(st), "enabled": !row.enabled}),
                    ));
                }
                if dot(
                    ui,
                    vis_colour(row.render_vis),
                    format!("Render visibility: {name}"),
                )
                .clicked()
                {
                    cmds.push((
                        "object.visibility",
                        json!({"nodes": target(st), "render": row.render_vis.next().id()}),
                    ));
                }
                if dot(
                    ui,
                    vis_colour(row.editor_vis),
                    format!("Editor visibility: {name}"),
                )
                .clicked()
                {
                    cmds.push((
                        "object.visibility",
                        json!({"nodes": target(st), "editor": row.editor_vis.next().id()}),
                    ));
                }
            });
        });
        resp.response.rect
    }

    /// Drop indicator while dragging a row, and the move on release (UI.md §13.3).
    #[allow(clippy::too_many_arguments)]
    fn drag_and_drop(
        &mut self,
        ctx: &egui::Context,
        ui: &mut Ui,
        st: &EditorState,
        rows: &[TreeRow],
        rects: &[(NodeId, Rect)],
        panel: Rect,
        cmds: &mut Vec<Cmd>,
    ) {
        let Some(dragged) = egui::DragAndDrop::payload::<NodeId>(ctx) else {
            self.last_pointer = None;
            return;
        };
        let pos = ui.input(|i| i.pointer.interact_pos()).or(self.last_pointer);
        self.last_pointer = pos;
        let Some(pos) = pos.filter(|p| panel.contains(*p)) else {
            return;
        };
        let scene = st.doc.scene();
        let moved = moved_set(scene, rows, &st.selection.nodes, *dragged);
        // The hovered row and zone; the empty area (id 0) = root end.
        let hit = rects.iter().find(|(_, r)| r.y_range().contains(pos.y));
        let (target, line): (Option<DropTarget>, Option<(Rect, bool)>) = match hit {
            Some((NodeId(0), _)) | None => (
                Some(DropTarget {
                    parent: None,
                    index: None,
                }),
                None,
            ),
            Some((id, r)) => {
                let zone = drop_zone((pos.y - r.top()) / r.height().max(1.0));
                let t = drop_target(scene, rows, *id, zone, &moved);
                let line = t.map(|_| match zone {
                    DropZone::Into => (*r, true),
                    DropZone::Before => (*r, false),
                    DropZone::After => (*r, false),
                });
                (t, line)
            }
        };
        if let (Some(_), Some((r, into))) = (target, line) {
            let colour = ui.visuals().selection.stroke.color;
            let p = ui.painter();
            if into {
                p.rect_stroke(r, 2.0, Stroke::new(1.5, colour), egui::StrokeKind::Inside);
            } else {
                let y = if pos.y < r.center().y {
                    r.top()
                } else {
                    r.bottom()
                };
                p.line_segment(
                    [Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
                    Stroke::new(2.0, colour),
                );
            }
        }
        if ui.input(|i| i.pointer.any_released())
            && let Some(t) = target
            && !moved.is_empty()
        {
            let mut args = json!({
                "nodes": moved.iter().map(|n| n.0).collect::<Vec<_>>(),
                "parent": t.parent.map(|p| p.0),
            });
            if let Some(i) = t.index {
                args["index"] = json!(i);
            }
            cmds.push(("object.move", args));
        }
    }
}
