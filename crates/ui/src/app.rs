//! The egui shell: menus, dock areas, panels and shortcuts (docs/specs/UI.md §9).
//! All state changes go through `commands::execute` or `attributes::apply_edit`.

use eframe::egui_wgpu;
use egui::{Key, KeyboardShortcut, Modifiers, RichText, Sense, Ui};
use forge_core::{NodeId, Value};
use forge_viewport::render::Shading;

use crate::attributes::{
    Gesture, Widget, apply_edit, attribute_rows, from_display, to_display, unit_suffix,
};
use crate::commands::{Command, category_label, create_menu, execute};
use crate::dock::{Area, Dock, Panel};
use crate::state::EditorState;
use crate::theme::Theme;
use crate::tree::tree_rows;
use crate::viewport_panel::{NavInput, ViewportPanel, apply_nav, nav_action};

const SHADINGS: [(Shading, &str); 5] = [
    (Shading::Smooth, "Smooth"),
    (Shading::Flat, "Flat"),
    (Shading::Wire, "Wireframe"),
    (Shading::SmoothWire, "Smooth + wire"),
    (Shading::FlatWire, "Flat + wire"),
];

pub struct Editor {
    pub state: EditorState,
    viewport: ViewportPanel,
    /// Row being renamed in the tree, with its edit buffer.
    renaming: Option<(NodeId, String)>,
    /// Name field buffer in the attribute panel.
    name_edit: Option<(NodeId, String)>,
    /// Counter for drag/typing gestures (each is one undo step).
    gesture_seq: u64,
    gesture: Option<u64>,
    play_accum: f32,
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        self.show(ui, frame.wgpu_render_state());
    }
}

impl Editor {
    pub fn new(state: EditorState) -> Editor {
        Editor {
            state,
            viewport: ViewportPanel::default(),
            renaming: None,
            name_edit: None,
            gesture_seq: 0,
            gesture: None,
            play_accum: 0.0,
        }
    }

    fn run(&mut self, cmd: Command) {
        self.state.status = match execute(&mut self.state, cmd) {
            Ok(()) => String::new(),
            Err(e) => e,
        };
    }

    fn edit(&mut self, id: NodeId, param: &str, value: Value, gesture: Gesture) {
        self.state.status = match apply_edit(&mut self.state.doc, id, param, value, gesture) {
            Ok(()) => String::new(),
            Err(e) => e.to_string(),
        };
    }

    /// A gesture id for a continuous interaction: new on start, reused while it lasts.
    fn gesture_for(&mut self, started: bool) -> Gesture {
        if started || self.gesture.is_none() {
            self.gesture_seq += 1;
            self.gesture = Some(self.gesture_seq);
        }
        Gesture::Drag(self.gesture.expect("set above"))
    }

    /// Draws the whole editor. `rs` is `None` without a GPU (tests).
    pub fn show(&mut self, ui: &mut Ui, rs: Option<&egui_wgpu::RenderState>) {
        self.shortcuts(ui.ctx());
        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        if !self.state.dock.tabs(Area::Bottom).is_empty() {
            egui::Panel::bottom("area_bottom")
                .resizable(true)
                .default_size(120.0)
                .show(ui, |ui| self.area(ui, Area::Bottom, rs));
        }
        if !self.state.dock.tabs(Area::Left).is_empty() {
            egui::Panel::left("area_left")
                .resizable(true)
                .default_size(280.0)
                .show(ui, |ui| self.area(ui, Area::Left, rs));
        }
        let (top, bottom) = (
            !self.state.dock.tabs(Area::RightTop).is_empty(),
            !self.state.dock.tabs(Area::RightBottom).is_empty(),
        );
        if top || bottom {
            egui::Panel::right("area_right")
                .resizable(true)
                .default_size(340.0)
                .show(ui, |ui| {
                    if top && bottom {
                        let h = ui.available_height();
                        egui::Panel::top("area_right_top")
                            .resizable(true)
                            .default_size(h * 0.45)
                            .show(ui, |ui| self.area(ui, Area::RightTop, rs));
                        self.area(ui, Area::RightBottom, rs);
                    } else if top {
                        self.area(ui, Area::RightTop, rs);
                    } else {
                        self.area(ui, Area::RightBottom, rs);
                    }
                });
        }
        egui::CentralPanel::no_frame().show(ui, |ui| self.area(ui, Area::Center, rs));
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let redo = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        let redo_y = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
        let undo = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
        let mut cmds = Vec::new();
        ctx.input_mut(|i| {
            if i.consume_shortcut(&redo) || i.consume_shortcut(&redo_y) {
                cmds.push(Command::Redo);
            } else if i.consume_shortcut(&undo) {
                cmds.push(Command::Undo);
            }
            if i.key_pressed(Key::Delete) {
                cmds.push(Command::DeleteSelected);
            }
            let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];
            for (k, (shading, _)) in keys.iter().zip(SHADINGS) {
                if i.key_pressed(*k) && i.modifiers.is_none() {
                    cmds.push(Command::SetShading(shading));
                }
            }
        });
        if ctx.input(|i| i.key_pressed(Key::F2))
            && let Some(id) = self.state.selection.active
            && let Some(n) = self.state.doc.scene().node(id)
        {
            self.renaming = Some((id, n.name.clone()));
        }
        for c in cmds {
            self.run(c);
        }
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New").clicked() {
                    let registry = self.state.doc.scene().registry().clone();
                    let dock = self.state.dock.clone();
                    self.state = EditorState::new(registry);
                    self.state.dock = dock;
                    ui.close();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() {
                    self.run(Command::Undo);
                    ui.close();
                }
                if ui.button("Redo").clicked() {
                    self.run(Command::Redo);
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    self.run(Command::DeleteSelected);
                    ui.close();
                }
            });
            ui.menu_button("Create", |ui| {
                for (cat, items) in create_menu(self.state.doc.scene().registry()) {
                    ui.menu_button(category_label(cat), |ui| {
                        for (type_id, label) in items {
                            if ui.button(label).clicked() {
                                self.run(Command::Create(type_id));
                                ui.close();
                            }
                        }
                    });
                }
            });
            ui.menu_button("View", |ui| {
                for (shading, label) in SHADINGS {
                    if ui.radio(self.state.shading == shading, label).clicked() {
                        self.run(Command::SetShading(shading));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Frame all").clicked() {
                    self.run(Command::FrameAll);
                    ui.close();
                }
            });
            ui.menu_button("Window", |ui| {
                for p in Panel::ALL {
                    let mut open = self.state.dock.area_of(p).is_some();
                    if ui
                        .checkbox(&mut open, format!("{} panel", p.title()))
                        .clicked()
                    {
                        if open {
                            self.state.dock.open(p);
                        } else {
                            self.state.dock.close(p);
                        }
                        ui.close();
                    }
                }
                ui.separator();
                for p in Panel::ALL {
                    ui.menu_button(format!("Move {}", p.title()), |ui| {
                        for a in Area::ALL {
                            if self.state.dock.area_of(p) != Some(a)
                                && ui
                                    .button(format!("{} to {}", p.title(), a.label()))
                                    .clicked()
                            {
                                self.state.dock.move_panel(p, a);
                                ui.close();
                            }
                        }
                    });
                }
                ui.separator();
                if ui.button("Reset layout").clicked() {
                    self.state.dock = Dock::default();
                    ui.close();
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if self.state.status.is_empty() {
                ui.weak("Ready");
            } else {
                ui.colored_label(Theme::default().error, &self.state.status);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("{} nodes", self.state.doc.scene().len()));
            });
        });
    }

    fn area(&mut self, ui: &mut Ui, area: Area, rs: Option<&egui_wgpu::RenderState>) {
        let tabs = self.state.dock.tabs(area).to_vec();
        let Some(active) = self.state.dock.active(area) else {
            return;
        };
        if area != Area::Center || tabs.len() > 1 {
            ui.horizontal(|ui| {
                for (i, p) in tabs.iter().enumerate() {
                    let title = RichText::new(p.title()).strong();
                    if ui.selectable_label(*p == active, title).clicked() {
                        self.state.dock.set_active(area, i);
                    }
                }
            });
            ui.separator();
        }
        match active {
            Panel::Viewport => self.viewport_ui(ui, rs),
            Panel::ObjectTree => self.tree_ui(ui),
            Panel::Attributes => self.attributes_ui(ui),
            Panel::Timeline => self.timeline_ui(ui),
        }
    }

    fn viewport_ui(&mut self, ui: &mut Ui, rs: Option<&egui_wgpu::RenderState>) {
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        let Some(rs) = rs else {
            ui.put(rect, egui::Label::new("Viewport unavailable"));
            return;
        };
        let input = ui.ctx().input(|i| NavInput {
            primary: i.pointer.primary_down(),
            middle: i.pointer.middle_down(),
            secondary: i.pointer.secondary_down(),
            alt: i.modifiers.alt,
            drag: if response.dragged() {
                response.drag_delta().into()
            } else {
                [0.0, 0.0]
            },
            scroll: if response.hovered() {
                i.smooth_scroll_delta.y
            } else {
                0.0
            },
        });
        let action = nav_action(&input);
        apply_nav(&mut self.state.camera, action, rect.height() as f64);

        let ppp = ui.ctx().pixels_per_point();
        let size = [
            (rect.width() * ppp).round() as u32,
            (rect.height() * ppp).round() as u32,
        ];
        let tex = self
            .viewport
            .render(rs, size, &self.state.camera, self.state.shading);
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(tex, rect, uv, egui::Color32::WHITE);
        let label = SHADINGS
            .iter()
            .find(|(s, _)| *s == self.state.shading)
            .map_or("", |(_, l)| l);
        ui.painter().text(
            rect.left_top() + egui::vec2(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            format!("Perspective · {label}"),
            egui::FontId::proportional(12.0),
            Theme::default().weak_text,
        );
    }

    fn tree_ui(&mut self, ui: &mut Ui) {
        let rows = tree_rows(self.state.doc.scene(), &self.state.expanded);
        let order: Vec<NodeId> = rows.iter().map(|r| r.id).collect();
        let mut cmds = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for row in &rows {
                    ui.horizontal(|ui| {
                        ui.add_space(row.depth as f32 * 14.0);
                        if row.has_children {
                            let arrow = if row.expanded { "-" } else { "+" };
                            if ui.small_button(arrow).clicked() {
                                cmds.push(Command::ToggleExpanded(row.id));
                            }
                        } else {
                            ui.add_space(18.0);
                        }
                        if let Some((id, buf)) = &mut self.renaming
                            && *id == row.id
                        {
                            let r = ui.text_edit_singleline(buf);
                            r.request_focus();
                            if ui.input(|i| i.key_pressed(Key::Escape)) {
                                self.renaming = None;
                            } else if r.lost_focus() {
                                cmds.push(Command::Rename(row.id, buf.clone()));
                                self.renaming = None;
                            }
                            return;
                        }
                        let selected = self.state.selection.nodes.contains(&row.id);
                        let mut text = RichText::new(&row.name);
                        if row.missing_type {
                            text = text.italics();
                        }
                        let r = ui
                            .selectable_label(selected, text)
                            .interact(Sense::click_and_drag());
                        r.dnd_set_drag_payload(row.id);
                        if let Some(dragged) = r.dnd_release_payload::<NodeId>()
                            && *dragged != row.id
                        {
                            cmds.push(Command::Reparent(*dragged, Some(row.id)));
                        }
                        if r.double_clicked() {
                            self.renaming = Some((row.id, row.name.clone()));
                        } else if r.clicked() {
                            let m = ui.input(|i| i.modifiers);
                            if m.shift {
                                self.state.selection.range(row.id, &order);
                            } else if m.command {
                                self.state.selection.toggle(row.id);
                            } else {
                                self.state.selection.click(row.id);
                            }
                        }
                        r.context_menu(|ui| {
                            if ui.button("Rename").clicked() {
                                self.renaming = Some((row.id, row.name.clone()));
                                ui.close();
                            }
                            if ui.button("Delete").clicked() {
                                self.state.selection.click(row.id);
                                cmds.push(Command::DeleteSelected);
                                ui.close();
                            }
                            if ui.button("Move to root").clicked() {
                                cmds.push(Command::Reparent(row.id, None));
                                ui.close();
                            }
                        });
                        ui.weak(&row.type_label);
                        for (_, tag) in &row.tags {
                            ui.weak(format!("[{tag}]"));
                        }
                    });
                }
                // Dropping below the rows moves the node to the end of the root list.
                let size = egui::vec2(ui.available_width(), ui.available_height().max(40.0));
                let rest = ui.allocate_response(size, Sense::hover());
                if let Some(dragged) = rest.dnd_release_payload::<NodeId>() {
                    cmds.push(Command::Reparent(*dragged, None));
                }
            });
        for c in cmds {
            self.run(c);
        }
    }

    fn attributes_ui(&mut self, ui: &mut Ui) {
        let Some(id) = self.state.selection.active else {
            ui.weak("Nothing selected");
            return;
        };
        let Some(view) = attribute_rows(self.state.doc.scene(), id) else {
            ui.weak("Nothing selected");
            return;
        };
        let theme = Theme::default();
        let mut rename = None;
        ui.horizontal(|ui| {
            let type_label = ui.label(RichText::new(&view.type_label).color(theme.weak_text));
            let buf = match &mut self.name_edit {
                Some((nid, b)) if *nid == id => b,
                slot => &mut slot.insert((id, view.name.clone())).1,
            };
            let r = ui.text_edit_singleline(buf).labelled_by(type_label.id);
            if r.lost_focus() && *buf != view.name {
                rename = Some(buf.clone());
            } else if !r.has_focus() {
                *buf = view.name.clone();
            }
        });
        if let Some(name) = rename {
            self.run(Command::Rename(id, name));
        }
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_enabled_ui(view.editable, |ui| {
                    for (group, rows) in &view.groups {
                        egui::CollapsingHeader::new(group)
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new(("attrs", group))
                                    .num_columns(2)
                                    .spacing([12.0, 4.0])
                                    .show(ui, |ui| {
                                        for row in rows {
                                            self.attribute_row(ui, id, row);
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            });
    }

    fn attribute_row(&mut self, ui: &mut Ui, id: NodeId, row: &crate::attributes::AttrRow) {
        if row.widget == Widget::Checkbox {
            ui.label("");
            let mut b = row.value == Value::Bool(true);
            if ui.checkbox(&mut b, &row.label).changed() {
                self.edit(id, &row.param, Value::Bool(b), Gesture::Commit);
            }
            return;
        }
        let label = ui.label(&row.label);
        let mut edited: Option<(Value, Gesture)> = None;
        match (&row.widget, &row.value) {
            (Widget::IntDrag { min, max }, Value::Int(v)) => {
                let mut x = *v;
                let r = ui
                    .add(egui::DragValue::new(&mut x).range(*min..=*max))
                    .labelled_by(label.id);
                if r.changed() {
                    let g = if r.dragged() {
                        self.gesture_for(r.drag_started())
                    } else {
                        Gesture::Commit
                    };
                    edited = Some((Value::Int(x), g));
                }
            }
            (Widget::FloatDrag { unit, .. }, Value::Float(v)) => {
                let mut d = to_display(*unit, *v);
                let r = ui
                    .add(
                        egui::DragValue::new(&mut d)
                            .speed(0.01)
                            .suffix(unit_suffix(*unit)),
                    )
                    .labelled_by(label.id);
                if r.changed() {
                    let g = if r.dragged() {
                        self.gesture_for(r.drag_started())
                    } else {
                        Gesture::Commit
                    };
                    edited = Some((Value::Float(from_display(*unit, d)), g));
                }
            }
            (Widget::Vec3Drag { unit, .. }, Value::Vec3(v)) => {
                let mut d = v.map(|c| to_display(*unit, c));
                ui.horizontal(|ui| {
                    for (axis, c) in ["X", "Y", "Z"].iter().zip(d.iter_mut()) {
                        let r = ui
                            .add(
                                egui::DragValue::new(c)
                                    .speed(0.01)
                                    .prefix(format!("{axis} ")),
                            )
                            .labelled_by(label.id);
                        if r.changed() {
                            let g = if r.dragged() {
                                self.gesture_for(r.drag_started())
                            } else {
                                Gesture::Commit
                            };
                            edited = Some((Value::Vec3([0.0; 3]), g));
                        }
                    }
                });
                if let Some((_, g)) = edited {
                    edited = Some((Value::Vec3(d.map(|c| from_display(*unit, c))), g));
                }
            }
            (Widget::Color, Value::Color(c)) => {
                let mut rgb = c.map(|x| x as f32);
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    let g = self.gesture_for(false);
                    edited = Some((Value::Color(rgb.map(f64::from)), g));
                }
            }
            (Widget::Combo { options }, Value::Enum(cur)) => {
                let current = options
                    .iter()
                    .find(|o| o.id == *cur)
                    .map_or(cur.as_str(), |o| o.label.as_str());
                egui::ComboBox::from_id_salt(("combo", id, &row.param))
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for o in options {
                            if ui.selectable_label(o.id == *cur, &o.label).clicked() {
                                edited = Some((Value::Enum(o.id.clone()), Gesture::Commit));
                            }
                        }
                    })
                    .response
                    .labelled_by(label.id);
            }
            (Widget::Text, Value::String(s)) => {
                let mut buf = s.clone();
                let r = ui.text_edit_singleline(&mut buf).labelled_by(label.id);
                if r.changed() {
                    let g = self.gesture_for(r.gained_focus());
                    edited = Some((Value::String(buf), g));
                }
            }
            (Widget::Link { accepts }, Value::Link(target)) => {
                let scene = self.state.doc.scene();
                let name_of = |t: NodeId| {
                    scene
                        .node(t)
                        .map_or(format!("(missing {t})"), |n| n.name.clone())
                };
                let current = target.map_or("None".to_string(), name_of);
                let candidates: Vec<(NodeId, String)> =
                    tree_rows(scene, &scene_all_expanded(scene))
                        .into_iter()
                        .filter(|r| r.id != id)
                        .filter(|r| {
                            accepts.is_empty()
                                || scene
                                    .node_type(r.id)
                                    .is_some_and(|t| accepts.contains(&t.category))
                        })
                        .map(|r| (r.id, r.name))
                        .collect();
                egui::ComboBox::from_id_salt(("link", id, &row.param))
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(target.is_none(), "None").clicked() {
                            edited = Some((Value::Link(None), Gesture::Commit));
                        }
                        for (cid, name) in candidates {
                            if ui.selectable_label(*target == Some(cid), name).clicked() {
                                edited = Some((Value::Link(Some(cid)), Gesture::Commit));
                            }
                        }
                    })
                    .response
                    .labelled_by(label.id);
            }
            (_, value) => {
                ui.weak(format!("{value:?}"));
            }
        }
        if let Some((value, gesture)) = edited {
            self.edit(id, &row.param, value, gesture);
        }
        if ui.input(|i| i.pointer.any_released()) && !ui.ctx().dragged_id().is_some() {
            self.gesture = None;
        }
    }

    fn timeline_ui(&mut self, ui: &mut Ui) {
        let t = &mut self.state.timeline;
        if t.playing {
            self.play_accum += ui.input(|i| i.stable_dt);
            let step = 1.0 / t.fps as f32;
            while self.play_accum >= step {
                self.play_accum -= step;
                t.tick();
            }
            ui.ctx().request_repaint();
        }
        ui.horizontal(|ui| {
            let label = if t.playing { "Pause" } else { "Play" };
            if ui.button(label).clicked() {
                t.playing = !t.playing;
                self.play_accum = 0.0;
            }
            let (mut start, mut end) = (t.start, t.end);
            let l = ui.label("Start");
            ui.add(egui::DragValue::new(&mut start)).labelled_by(l.id);
            let l = ui.label("End");
            ui.add(egui::DragValue::new(&mut end)).labelled_by(l.id);
            if (start, end) != (t.start, t.end) {
                t.set_range(start, end);
            }
            ui.weak(format!("{} fps", t.fps));
        });
        let mut current = t.current;
        let slider = egui::Slider::new(&mut current, t.start..=t.end).text("Frame");
        ui.spacing_mut().slider_width = (ui.available_width() - 140.0).max(60.0);
        if ui.add(slider).changed() {
            t.set_current(current);
        }
    }
}

/// Every node expanded, for listing link candidates.
fn scene_all_expanded(scene: &forge_core::Scene) -> std::collections::BTreeSet<NodeId> {
    let mut out = std::collections::BTreeSet::new();
    for r in scene.roots() {
        out.insert(*r);
        out.extend(scene.descendants(*r));
    }
    out
}
