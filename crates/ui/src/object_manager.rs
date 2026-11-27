//! The object manager panel: columns, dots, tick, tags, drag and drop, rename, search and
//! context menu (docs/specs/UI.md §13). The logic that decides things is in `tree`; this
//! file draws and turns gestures into registry commands.

use egui::{Align, Color32, Layout, Pos2, Rect, RichText, Sense, Stroke, Ui, vec2};
use forge_core::NodeId;
use forge_core::visibility::Vis;
use serde_json::{Value as Json, json};

use crate::state::EditorState;
use crate::theme::Theme;
use crate::tree::{
    Connector, DropMark, DropTarget, EvalCell, METRICS, RowColours, TreeRow, click_selection,
    connector_segments, connectors, context_menu, disclosure_box, disclosure_glyph, drop_mark,
    drop_target, drop_zone, filter_rows, layer_menu, menu_target, moved_set, name_colour,
    row_colours, row_layout, tree_rows,
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

/// The two stacked visibility dots (observed, G1 item 2): the upper one is the editor
/// visibility, the lower one the render visibility. Each is its own click target with an
/// accessibility label; the result says which one was clicked.
fn dot_pair(ui: &mut Ui, row: NodeId, editor: Vis, render: Vis, name: &str) -> (bool, bool) {
    let (rect, _) = ui.allocate_exact_size(vec2(DOT, METRICS.row_height), Sense::hover());
    let half = METRICS.row_height / 2.0;
    let mut clicked = [false; 2];
    for (k, (vis, what)) in [(editor, "Editor"), (render, "Render")]
        .into_iter()
        .enumerate()
    {
        let cell = Rect::from_min_size(
            Pos2::new(rect.left(), rect.top() + half * k as f32),
            vec2(DOT, half),
        );
        let r = ui.interact(
            cell,
            ui.make_persistent_id(("object_dot", row.0, what)),
            Sense::click(),
        );
        let label = format!("{what} visibility: {name}");
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
        ui.painter()
            .circle_filled(cell.center(), DOT * 0.22, vis_colour(vis));
        if r.hovered() {
            ui.painter().circle_stroke(
                cell.center(),
                DOT * 0.32,
                Stroke::new(1.0, ui.visuals().strong_text_color()),
            );
        }
        clicked[k] = r.clicked();
    }
    (clicked[0], clicked[1])
}

/// The active-camera target: a ring with a dot.
fn target(ui: &mut Ui, label: String) -> egui::Response {
    let r = ui.allocate_response(vec2(DOT, DOT), Sense::click());
    r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label.clone()));
    let ink = ui.visuals().text_color();
    ui.painter()
        .circle_stroke(r.rect.center(), DOT * 0.3, Stroke::new(1.2, ink));
    ui.painter().circle_filled(r.rect.center(), DOT * 0.12, ink);
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
        let colours = row_colours(&Theme::current(&ctx));
        let lines = connectors(&rows);

        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                // rows touch: every y of the list belongs to a row (UI.md §13.1a)
                ui.spacing_mut().item_spacing.y = 0.0;
                for (index, row) in rows.iter().enumerate() {
                    let look = RowLook {
                        index,
                        selected: st.selection.nodes.contains(&row.id),
                        colours,
                        connector: &lines[index],
                    };
                    let rect = self.row_ui(ui, st, row, filtering, &order, &look, &mut cmds);
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
    #[allow(clippy::too_many_arguments)]
    fn row_ui(
        &mut self,
        ui: &mut Ui,
        st: &mut EditorState,
        row: &TreeRow,
        filtering: bool,
        order: &[NodeId],
        look: &RowLook,
        cmds: &mut Vec<Cmd>,
    ) -> Rect {
        // the background, separator and connector lines go behind the widgets: reserve
        // their place in the paint order now, fill it once the row's size is known
        let backdrop = ui.painter().add(egui::Shape::Noop);
        let full_width = ui.available_width();
        let resp = ui.horizontal(|ui| {
            ui.set_min_height(METRICS.row_height);
            // indent and fold arrow cell sit flush, at the numbers of `row_layout`
            let layout = row_layout(row.depth);
            let gap = ui.spacing().item_spacing.x;
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(layout.start);
            if row.has_children && !filtering {
                let (rect, r) = ui.allocate_exact_size(
                    vec2(layout.arrow.1 - layout.arrow.0, METRICS.row_height),
                    Sense::click(),
                );
                let name = &row.name;
                let verb = if row.expanded { "Collapse" } else { "Expand" };
                r.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::Role::Button, true, format!("{verb}: {name}"))
                });
                disclosure(ui, rect, row.depth, row.expanded, r.hovered());
                if r.clicked() {
                    cmds.push(("view.toggle_expanded", json!({"node": row.id.0})));
                }
            } else {
                ui.add_space(layout.arrow.1 - layout.arrow.0);
            }
            ui.spacing_mut().item_spacing.x = gap;
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
            let theme = Theme::current(ui.ctx());
            let mut text = RichText::new(&row.name);
            if row.missing_type {
                text = text.italics();
            }
            if let Some(colour) = name_colour(&theme, selected) {
                text = text.color(colour).strong();
            } else if row.dim || !row.enabled {
                text = text.weak();
            }
            let r = ui
                .add(egui::Button::new(text).frame(false))
                .interact(Sense::click_and_drag());
            r.dnd_set_drag_payload(row.id);
            // egui takes the payload before checking its type. Leave object reordering alone.
            if egui::DragAndDrop::has_payload_of_type::<crate::material_manager::MaterialDrag>(
                ui.ctx(),
            ) && let Some(material) =
                r.dnd_release_payload::<crate::material_manager::MaterialDrag>()
            {
                cmds.push(crate::material_manager::assign_to(*material, row.id));
            }
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
            let layer_items = layer_menu(st.doc.scene(), &st.selection.nodes);
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
                ui.menu_button("Layer", |ui| {
                    for item in layer_items {
                        if ui.selectable_label(item.checked, &item.label).clicked() {
                            cmds.push((item.command, item.args));
                            ui.close();
                        }
                    }
                });
            });
            // the type label stands in for the per-type icon of brief 227
            ui.weak(&row.type_label);
            // Layer glyph, the stacked visibility dots, the evaluation cell, then the tags,
            // right aligned (UI.md §13.1; observed order, G1 item 2). The layout is right to
            // left, so the last cell is added first.
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let target_nodes = |st: &EditorState| {
                    if st.selection.nodes.contains(&row.id) {
                        st.selection.nodes.iter().map(|n| n.0).collect::<Vec<_>>()
                    } else {
                        vec![row.id.0]
                    }
                };
                let name = &row.name;
                for (tag_id, tag) in row.tags.iter().rev() {
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
                match row.eval {
                    EvalCell::Check => {
                        if tick(ui, row.enabled, format!("Enabled: {name}")).clicked() {
                            cmds.push((
                                "object.enable",
                                json!({"nodes": target_nodes(st), "enabled": !row.enabled}),
                            ));
                        }
                    }
                    EvalCell::ActiveCamera => {
                        if target(ui, format!("Active camera: {name}")).clicked() {
                            cmds.push(("view.look_through_camera", json!({"node": row.id.0})));
                        }
                    }
                    EvalCell::None => {}
                }
                let (editor_clicked, render_clicked) =
                    dot_pair(ui, row.id, row.editor_vis, row.render_vis, name);
                if editor_clicked {
                    cmds.push((
                        "object.visibility",
                        json!({"nodes": target_nodes(st), "editor": row.editor_vis.next().id()}),
                    ));
                }
                if render_clicked {
                    cmds.push((
                        "object.visibility",
                        json!({"nodes": target_nodes(st), "render": row.render_vis.next().id()}),
                    ));
                }
                // the layer glyph (UI.md §22.2)
                if let Some((_, layer_name, c)) = &row.layer {
                    let (rect, r) = ui.allocate_exact_size(vec2(5.0, DOT), Sense::hover());
                    let col = Color32::from_rgb(
                        (c[0] * 255.0).round() as u8,
                        (c[1] * 255.0).round() as u8,
                        (c[2] * 255.0).round() as u8,
                    );
                    ui.painter().rect_filled(rect, 1.0, col);
                    r.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::Label,
                            true,
                            format!("Layer: {layer_name}"),
                        )
                    });
                }
            });
        });
        let row_rect = Rect::from_min_size(
            resp.response.rect.min,
            vec2(
                full_width.max(resp.response.rect.width()),
                resp.response.rect.height().max(METRICS.row_height),
            ),
        );
        paint_row_backdrop(ui, backdrop, row_rect, row, look);
        row_rect
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
        let (target, mark): (Option<DropTarget>, Option<(Rect, DropMark)>) = match hit {
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
                let mark = rows
                    .iter()
                    .find(|row| row.id == *id)
                    .filter(|_| t.is_some())
                    .map(|row| (*r, drop_mark(row, zone)));
                (t, mark)
            }
        };
        if let Some((r, mark)) = mark {
            paint_drop_mark(ui, r, mark);
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

/// What a row needs besides its data to be drawn.
struct RowLook<'a> {
    /// Position in the visible list (odd rows get the alternating shade).
    index: usize,
    selected: bool,
    colours: RowColours,
    connector: &'a Connector,
}

/// The disclosure: a boxed minus (expanded) or boxed plus (collapsed) in the gutter cell
/// (observed, G1 item 7).
fn disclosure(ui: &Ui, cell: Rect, depth: usize, expanded: bool, hovered: bool) {
    let origin = cell.left_top() - vec2(row_layout(depth).start, 0.0);
    let (l, t, r, b) = disclosure_box(depth);
    let rect = Rect::from_min_max(origin + vec2(l, t), origin + vec2(r, b));
    let colour = if hovered {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().text_color()
    };
    let stroke = Stroke::new(1.0, colour);
    ui.painter()
        .rect_stroke(rect, 1.0, stroke, egui::StrokeKind::Inside);
    for (a, b) in disclosure_glyph(depth, expanded) {
        ui.painter().line_segment(
            [origin + vec2(a[0], a[1]), origin + vec2(b[0], b[1])],
            stroke,
        );
    }
}

/// Fills the reserved paint slot of a row: alternating shade, connector lines, separator.
fn paint_row_backdrop(
    ui: &Ui,
    slot: egui::layers::ShapeIdx,
    rect: Rect,
    row: &TreeRow,
    look: &RowLook,
) {
    let mut shapes: Vec<egui::Shape> = Vec::new();
    if look.index % 2 == 1 {
        shapes.push(egui::Shape::rect_filled(rect, 0.0, look.colours.alt));
    }
    if look.selected {
        shapes.push(egui::Shape::rect_filled(rect, 0.0, look.colours.selected));
    }
    let guide = Stroke::new(1.0, look.colours.guide);
    for (a, b) in connector_segments(
        row.depth,
        row.has_children,
        row.expanded,
        &look.connector.trunks,
    ) {
        // half a pixel in, so the 1 px lines are crisp
        let at = |p: [f32; 2]| Pos2::new(rect.left() + p[0] + 0.5, rect.top() + p[1]);
        shapes.push(egui::Shape::line_segment([at(a), at(b)], guide));
    }
    shapes.push(egui::Shape::line_segment(
        [
            Pos2::new(rect.left(), rect.bottom() - 0.5),
            Pos2::new(rect.right(), rect.bottom() - 0.5),
        ],
        Stroke::new(1.0, look.colours.separator),
    ));
    ui.painter().set(slot, egui::Shape::Vec(shapes));
}

/// The drag hover: the whole row for "inside", a line with a dot at the landing indent for
/// "between rows" (UI.md §13.3).
fn paint_drop_mark(ui: &Ui, row: Rect, mark: DropMark) {
    let colour = ui.visuals().selection.bg_fill;
    let p = ui.painter();
    match mark {
        DropMark::Into => {
            p.rect_filled(row, 2.0, colour.gamma_multiply(0.25));
            p.rect_stroke(row, 2.0, Stroke::new(1.5, colour), egui::StrokeKind::Inside);
        }
        DropMark::Line { depth, below } => {
            let y = if below { row.bottom() } else { row.top() };
            let x = row.left() + row_layout(depth).start;
            p.line_segment(
                [Pos2::new(x, y), Pos2::new(row.right(), y)],
                Stroke::new(2.0, colour),
            );
            p.circle_filled(Pos2::new(x, y), 3.0, colour);
        }
    }
}
