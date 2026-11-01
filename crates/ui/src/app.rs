//! The egui shell: menus, dock areas, panels and shortcuts (docs/specs/UI.md §9).
//! All state changes go through registry commands (`commands::execute`) or
//! `attributes::apply_edit`.

use eframe::egui_wgpu;
use egui::{Key, KeyboardShortcut, Modifiers, RichText, Sense, Ui};
use forge_core::transform::{Transform, rotation_part};
use forge_core::{NodeId, Value};
use forge_viewport::gizmo::{Drag, Gizmo, GizmoMode, Space as GizmoSpace};
use forge_viewport::render::Shading;
use glam::DVec3;
use serde_json::{Value as Json, json};

use crate::animate::{AnimEdit, KeyState, key_frames, key_states};
use crate::attributes::{
    Gesture, Widget, apply_edit_anim, attribute_rows, from_display, to_display, unit_suffix,
};
use crate::commands::{category_label, create_menu, execute};
use crate::component_edit::{ComponentDrag, pivot_view, pivot_world};
use crate::components::{
    ClickModifiers, EditMode, SelectCmd, target as component_target, viewport_click, viewport_rect,
};
use crate::coordinates::{Row, Space, Third, coord_view, set_coord_anim};
use crate::dock::{Area, Dock, Panel};
use crate::manipulate::apply_drag_anim;
use crate::state::EditorState;
use crate::theme::Theme;
use crate::tree::tree_rows;
use crate::viewport_panel::{NavInput, SceneView, ViewportPanel, apply_nav, nav_action};

mod files;

pub use files::{Dialogs, FileKind, NativeDialogs, Unsaved};

const SHADINGS: [(Shading, &str); 5] = [
    (Shading::Smooth, "Smooth"),
    (Shading::Flat, "Flat"),
    (Shading::Wire, "Wireframe"),
    (Shading::SmoothWire, "Smooth + wire"),
    (Shading::FlatWire, "Flat + wire"),
];

const OPEN: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::O);
const SAVE: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);
const SAVE_AS: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::S);
const SELECT_ALL: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
const DESELECT_ALL: KeyboardShortcut =
    KeyboardShortcut::new(Modifiers::COMMAND.plus(Modifiers::SHIFT), Key::A);

/// A command to run: registry id and JSON arguments (docs/specs/COMMANDS.md).
type Cmd = (&'static str, Json);

fn cmd(id: &'static str) -> Cmd {
    (id, json!({}))
}

fn select_cmd(op: SelectCmd) -> Cmd {
    cmd(match op {
        SelectCmd::All => "select.all",
        SelectCmd::None => "select.none",
        SelectCmd::Invert => "select.invert",
        SelectCmd::Grow => "select.grow",
        SelectCmd::Shrink => "select.shrink",
    })
}

/// Play or stop, whichever the current state calls for.
fn play_stop(state: &EditorState) -> Cmd {
    cmd(if state.timeline.playing {
        "timeline.stop"
    } else {
        "timeline.play"
    })
}

fn shading_cmd(shading: Shading) -> Cmd {
    (
        "view.shading",
        json!({"mode": crate::commands::shading_mode_of(shading)}),
    )
}

#[derive(Clone, Copy)]
enum FileAction {
    New,
    Open,
    Save,
    SaveAs,
    ImportObj,
    ExportObj,
    ExportSelectedObj,
}

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
    /// The pointer was over the viewport last frame (Space plays only then).
    viewport_hot: bool,
    gizmo_mode: GizmoMode,
    gizmo_space: GizmoSpace,
    coord_space: Space,
    coord_third: Third,
    /// The gizmo drag in progress: drag state, node, its transform at the start, gesture.
    gizmo_drag: Option<(Drag, NodeId, Transform, u64)>,
    /// The gizmo drag on components in progress: drag state, component drag, gesture.
    component_drag: Option<(Drag, ComponentDrag, u64)>,
    /// The rectangle select in progress, `[x0, y0, x1, y1]` in render pixels.
    rect_drag: Option<[f64; 4]>,
    /// The interactive drag of the active modeling tool (docs/specs/MODELING.md §0.2).
    tool_drag: Option<crate::tools::ToolDrag>,
    /// File menu (docs/specs/IO.md §2): dialogs, current file, revision at the last
    /// save or load, and the last window title sent.
    dialogs: Box<dyn Dialogs>,
    sent_title: String,
    /// Requests from the RPC server, run between frames (docs/specs/RPC.md §5).
    rpc: Option<forge_rpc::Inbox>,
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
            viewport_hot: false,
            gizmo_mode: GizmoMode::Translate,
            gizmo_space: GizmoSpace::Object,
            coord_space: Space::Local,
            coord_third: Third::Scale,
            gizmo_drag: None,
            component_drag: None,
            rect_drag: None,
            tool_drag: None,
            dialogs: Box::new(NativeDialogs),
            sent_title: String::new(),
            rpc: None,
        }
    }

    /// Serves the RPC server's queued requests from now on, one batch per frame.
    pub fn attach_rpc(&mut self, inbox: forge_rpc::Inbox) {
        self.rpc = Some(inbox);
    }

    /// Runs a command; an error goes to the status bar. True if it succeeded.
    fn run(&mut self, (id, args): Cmd) -> bool {
        // A command may leave a message of its own (e.g. skipped nodes) in the status.
        self.state.status.clear();
        match execute(&mut self.state, id, args) {
            Ok(_) => true,
            Err(e) => {
                self.state.status = e;
                false
            }
        }
    }

    fn edit(&mut self, id: NodeId, param: &str, value: Value, gesture: Gesture) {
        let ctx = self.state.anim_ctx();
        self.state.status =
            match apply_edit_anim(&mut self.state.doc, id, param, value, gesture, Some(&ctx)) {
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
        // RPC jobs run here, before anything is drawn: one command is one undo step.
        if let Some(inbox) = self.rpc.take() {
            if inbox.drain(&mut self.state) > 0 {
                ui.ctx().request_repaint();
            }
            self.rpc = Some(inbox);
        }
        self.update_playback(ui.ctx());
        self.shortcuts(ui.ctx());
        let title = self.window_title();
        if title != self.sent_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.sent_title = title;
        }
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

    /// Advances the current frame from the frame clock while playing (ANIM.md §2.6).
    fn update_playback(&mut self, ctx: &egui::Context) {
        self.state.sync_timeline();
        if !self.state.timeline.playing {
            return;
        }
        let now = ctx.input(|i| i.time);
        let settings = self.state.settings();
        self.state.timeline.update(now, &settings);
        ctx.request_repaint();
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let redo = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        let redo_y = KeyboardShortcut::new(Modifiers::COMMAND, Key::Y);
        let undo = KeyboardShortcut::new(Modifiers::COMMAND, Key::Z);
        let mut cmds: Vec<Cmd> = Vec::new();
        let mut file_action = None;
        ctx.input_mut(|i| {
            // Save As first: Ctrl+S would also match with Shift held
            if i.consume_shortcut(&SAVE_AS) {
                file_action = Some(FileAction::SaveAs);
            } else if i.consume_shortcut(&SAVE) {
                file_action = Some(FileAction::Save);
            } else if i.consume_shortcut(&OPEN) {
                file_action = Some(FileAction::Open);
            }
            // Deselect All first: Ctrl+A would also match with Shift held
            if i.consume_shortcut(&DESELECT_ALL) {
                cmds.push(select_cmd(SelectCmd::None));
            } else if i.consume_shortcut(&SELECT_ALL) {
                cmds.push(select_cmd(SelectCmd::All));
            }
            if i.consume_shortcut(&redo) || i.consume_shortcut(&redo_y) {
                cmds.push(cmd("edit.redo"));
            } else if i.consume_shortcut(&undo) {
                cmds.push(cmd("edit.undo"));
            }
            if i.key_pressed(Key::Delete) {
                cmds.push(cmd("object.delete"));
            }
            // Animation (ours, parity-pending; ANIM.md §2.7). Shift variants first.
            if i.consume_key(Modifiers::SHIFT, Key::K) {
                cmds.push(cmd("key.autokey"));
            } else if i.consume_key(Modifiers::NONE, Key::K) {
                cmds.push(cmd("key.record"));
            }
            if i.consume_key(Modifiers::SHIFT, Key::Comma) {
                cmds.push(("key.jump", json!({"forward": false})));
            } else if i.consume_key(Modifiers::NONE, Key::Comma) {
                cmds.push(("timeline.step", json!({"delta": -1})));
            }
            if i.consume_key(Modifiers::SHIFT, Key::Period) {
                cmds.push(("key.jump", json!({"forward": true})));
            } else if i.consume_key(Modifiers::NONE, Key::Period) {
                cmds.push(("timeline.step", json!({"delta": 1})));
            }
            if self.viewport_hot && i.consume_key(Modifiers::NONE, Key::Space) {
                cmds.push(play_stop(&self.state));
            }
            let keys = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5];
            for (k, (shading, _)) in keys.iter().zip(SHADINGS) {
                if i.key_pressed(*k) && i.modifiers.is_none() {
                    cmds.push(shading_cmd(shading));
                }
            }
        });
        // Gizmo modes and space (ours, parity-pending): W move, E rotate, R scale, Q axes.
        ctx.input(|i| {
            if i.modifiers.is_none() {
                if i.key_pressed(Key::W) {
                    self.gizmo_mode = GizmoMode::Translate;
                }
                if i.key_pressed(Key::E) {
                    self.gizmo_mode = GizmoMode::Rotate;
                }
                if i.key_pressed(Key::R) {
                    self.gizmo_mode = GizmoMode::Scale;
                }
                if i.key_pressed(Key::Q) {
                    self.gizmo_space = match self.gizmo_space {
                        GizmoSpace::Object => GizmoSpace::World,
                        GizmoSpace::World => GizmoSpace::Object,
                    };
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
        if let Some(a) = file_action {
            self.file_action(a);
        }
    }

    fn file_action(&mut self, action: FileAction) {
        match action {
            FileAction::New => self.new_scene(),
            FileAction::Open => self.open_scene(),
            FileAction::Save => {
                self.save();
            }
            FileAction::SaveAs => {
                self.save_as();
            }
            FileAction::ImportObj => self.import_obj(),
            FileAction::ExportObj => self.export_obj(false),
            FileAction::ExportSelectedObj => self.export_obj(true),
        }
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                let has_selection = !self.state.selection.nodes.is_empty();
                let items = [
                    ("New", None, FileAction::New, true),
                    ("Open\u{2026}", Some(OPEN), FileAction::Open, true),
                    ("Save", Some(SAVE), FileAction::Save, true),
                    ("Save As\u{2026}", Some(SAVE_AS), FileAction::SaveAs, true),
                    ("Import OBJ\u{2026}", None, FileAction::ImportObj, true),
                    ("Export OBJ\u{2026}", None, FileAction::ExportObj, true),
                    (
                        "Export Selected as OBJ\u{2026}",
                        None,
                        FileAction::ExportSelectedObj,
                        has_selection,
                    ),
                ];
                for (label, shortcut, action, enabled) in items {
                    if matches!(action, FileAction::ImportObj) {
                        ui.separator();
                    }
                    let mut button = egui::Button::new(label);
                    if let Some(sc) = shortcut {
                        button = button.shortcut_text(ui.ctx().format_shortcut(&sc));
                    }
                    if ui.add_enabled(enabled, button).clicked() {
                        ui.close();
                        self.file_action(action);
                    }
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui.button("Undo").clicked() {
                    self.run(cmd("edit.undo"));
                    ui.close();
                }
                if ui.button("Redo").clicked() {
                    self.run(cmd("edit.redo"));
                    ui.close();
                }
                if ui.button("Delete").clicked() {
                    self.run(cmd("object.delete"));
                    ui.close();
                }
                ui.separator();
                let select_items = [
                    ("Select All", Some(SELECT_ALL), SelectCmd::All),
                    ("Deselect All", Some(DESELECT_ALL), SelectCmd::None),
                    ("Invert Selection", None, SelectCmd::Invert),
                ];
                for (label, shortcut, cmd) in select_items {
                    let mut button = egui::Button::new(label);
                    if let Some(sc) = shortcut {
                        button = button.shortcut_text(ui.ctx().format_shortcut(&sc));
                    }
                    if ui.add(button).clicked() {
                        self.run(select_cmd(cmd));
                        ui.close();
                    }
                }
                ui.separator();
                if ui.button("Make Editable").clicked() {
                    self.run(cmd("mesh.make_editable"));
                    ui.close();
                }
                if ui.button("Current State to Object").clicked() {
                    self.run(cmd("mesh.current_state_to_object"));
                    ui.close();
                }
            });
            ui.menu_button("Select", |ui| {
                if ui.button("Grow Selection").clicked() {
                    self.run(select_cmd(SelectCmd::Grow));
                    ui.close();
                }
                if ui.button("Shrink Selection").clicked() {
                    self.run(select_cmd(SelectCmd::Shrink));
                    ui.close();
                }
            });
            ui.menu_button("Mesh", |ui| {
                for tool in crate::tools::tools() {
                    if ui.button(tool.label()).clicked() {
                        crate::tools::activate_tool(&mut self.state, tool.id());
                        ui.close();
                    }
                }
            });
            ui.menu_button("Create", |ui| {
                for (cat, items) in create_menu(self.state.doc.scene().registry()) {
                    ui.menu_button(category_label(cat), |ui| {
                        for (type_id, label) in items {
                            if ui.button(label).clicked() {
                                self.run(("object.create", json!({"type": type_id})));
                                ui.close();
                            }
                        }
                    });
                }
            });
            ui.menu_button("Animate", |ui| {
                let fmt_ctx = ui.ctx().clone();
                let k = move |m: Modifiers, key: Key| {
                    fmt_ctx.format_shortcut(&KeyboardShortcut::new(m, key))
                };
                let items = [
                    (
                        "Record Active Objects",
                        k(Modifiers::NONE, Key::K),
                        cmd("key.record"),
                    ),
                    ("Delete Keys", String::new(), cmd("key.delete_at_frame")),
                    ("Play / Stop", "Space".to_string(), play_stop(&self.state)),
                    (
                        "Previous Frame",
                        k(Modifiers::NONE, Key::Comma),
                        ("timeline.step", json!({"delta": -1})),
                    ),
                    (
                        "Next Frame",
                        k(Modifiers::NONE, Key::Period),
                        ("timeline.step", json!({"delta": 1})),
                    ),
                    (
                        "Previous Key",
                        k(Modifiers::SHIFT, Key::Comma),
                        ("key.jump", json!({"forward": false})),
                    ),
                    (
                        "Next Key",
                        k(Modifiers::SHIFT, Key::Period),
                        ("key.jump", json!({"forward": true})),
                    ),
                ];
                for (label, shortcut, cmd) in items {
                    let mut button = egui::Button::new(label);
                    if !shortcut.is_empty() {
                        button = button.shortcut_text(shortcut);
                    }
                    if ui.add(button).clicked() {
                        self.run(cmd);
                        ui.close();
                    }
                }
                ui.separator();
                let mut auto = self.state.autokey;
                let sc = k(Modifiers::SHIFT, Key::K);
                if ui
                    .checkbox(&mut auto, format!("Auto-Key  ({sc})"))
                    .clicked()
                {
                    self.run(cmd("key.autokey"));
                    ui.close();
                }
            });
            ui.menu_button("View", |ui| {
                for (shading, label) in SHADINGS {
                    if ui.radio(self.state.shading == shading, label).clicked() {
                        self.run(shading_cmd(shading));
                        ui.close();
                    }
                }
                ui.separator();
                ui.radio_value(
                    &mut self.gizmo_space,
                    GizmoSpace::Object,
                    "Gizmo axes: object",
                );
                ui.radio_value(
                    &mut self.gizmo_space,
                    GizmoSpace::World,
                    "Gizmo axes: world",
                );
                ui.separator();
                if ui.button("Frame all").clicked() {
                    self.run(cmd("view.frame_all"));
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
            Panel::Coordinates => self.coordinates_ui(ui),
            Panel::Script => {
                crate::script_panel::script_panel_ui(ui, &mut self.state, self.dialogs.as_mut())
            }
        }
    }

    fn viewport_ui(&mut self, ui: &mut Ui, rs: Option<&egui_wgpu::RenderState>) {
        // header: what a click selects (EDITABLE.md §2.6)
        ui.horizontal(|ui| {
            for mode in EditMode::ALL {
                let on = self.state.mode == mode;
                if ui
                    .add(egui::Button::new(mode.label()).selected(on))
                    .clicked()
                {
                    self.run((
                        "select.mode",
                        json!({"mode": forge_command::ModeArg::from(mode)}),
                    ));
                }
            }
        });
        let rect = ui.available_rect_before_wrap();
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        self.viewport_hot = response.hovered() || response.has_focus();
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
        if input.alt {
            self.rect_drag = None;
        } else {
            self.viewport_interaction(ui, &response, rect, ppp, size);
        }
        let gizmo = self.current_gizmo();
        let shown = self.state.evaluated();
        // a live tool preview replaces the target's mesh in the drawn scene only
        let shown = crate::tools::with_preview(&self.state, &shown).unwrap_or(shown);
        let tex = self.viewport.render(
            rs,
            size,
            &self.state.camera,
            self.state.shading,
            SceneView {
                scene: &shown,
                selected: &self.state.selection.nodes,
                gizmo: gizmo.as_ref(),
                component: component_target(&self.state).map(|id| (id, self.state.mode)),
                ctx: forge_core::EvalCtx::at_frame(
                    self.state.timeline.current,
                    f64::from(shown.settings().fps),
                ),
            },
        );
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        ui.painter().image(tex, rect, uv, egui::Color32::WHITE);
        if let Some([x0, y0, x1, y1]) = self.rect_drag
            && response.dragged()
        {
            let to_points = |x: f64, y: f64| {
                rect.min + egui::vec2((x / ppp as f64) as f32, (y / ppp as f64) as f32)
            };
            let colour = Theme::default().component_selection;
            ui.painter().rect_stroke(
                egui::Rect::from_two_pos(to_points(x0, y0), to_points(x1, y1)),
                0.0,
                egui::Stroke::new(1.0, colour),
                egui::StrokeKind::Inside,
            );
        }
        let label = SHADINGS
            .iter()
            .find(|(s, _)| *s == self.state.shading)
            .map_or("", |(_, l)| l);
        ui.painter().text(
            rect.left_top() + egui::vec2(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            match self.state.mode {
                EditMode::Model => format!("Perspective · {label}"),
                mode => format!("Perspective · {label} · {} mode", mode.label()),
            },
            egui::FontId::proportional(12.0),
            Theme::default().weak_text,
        );
    }

    /// Gizmo at the active node, in its object frame or world axes (MANIPULATE.md §2, §5).
    fn current_gizmo(&self) -> Option<Gizmo> {
        let id = self.state.selection.active?;
        let m = self.state.evaluated().world_matrix(id)?;
        // component modes: the gizmo sits on the selected components (EDITABLE.md §3.6)
        let origin = if component_target(&self.state).is_some() {
            pivot_world(&self.state)?
        } else {
            m.w_axis.truncate()
        };
        let object = self.gizmo_space == GizmoSpace::Object || self.gizmo_mode == GizmoMode::Scale;
        let axes = if object {
            let r = rotation_part(&m);
            [r.x_axis, r.y_axis, r.z_axis]
        } else {
            [DVec3::X, DVec3::Y, DVec3::Z]
        };
        Some(Gizmo {
            origin,
            mode: self.gizmo_mode,
            axes,
            ..Gizmo::default()
        })
    }

    /// Left-button picking and gizmo drags (positions in render pixels).
    fn viewport_interaction(
        &mut self,
        ui: &Ui,
        response: &egui::Response,
        rect: egui::Rect,
        ppp: f32,
        size: [u32; 2],
    ) {
        let (w, h) = (size[0].max(1) as f64, size[1].max(1) as f64);
        let to_pixels = |pos: egui::Pos2| {
            (
                ((pos.x - rect.min.x) * ppp) as f64,
                ((pos.y - rect.min.y) * ppp) as f64,
            )
        };
        let modifiers = ui.input(|i| i.modifiers);
        let mods = ClickModifiers {
            shift: modifiers.shift,
            command: modifiers.command,
        };
        // End of a rectangle select: the release may not report a pointer position
        if response.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(mut r) = self.rect_drag.take()
        {
            if let Some(pos) = response.interact_pointer_pos() {
                (r[2], r[3]) = to_pixels(pos);
            }
            if let Err(e) = viewport_rect(&mut self.state, r, [w, h], mods) {
                self.state.status = e;
            }
            return;
        }
        let Some(pos) = response.interact_pointer_pos() else {
            return;
        };
        let (x, y) = to_pixels(pos);
        if self.state.tool.is_some() && crate::tools::tool_target(&self.state).is_some() {
            // an active tool takes the primary drag: vertical distance sets the offset
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.tool_drag = crate::tools::ToolDrag::begin(&self.state, y, h);
            }
            if let Some(drag) = self.tool_drag {
                if response.dragged_by(egui::PointerButton::Primary) {
                    drag.update(&mut self.state, y);
                }
                if response.drag_stopped() {
                    self.tool_drag = None;
                    self.state.status.clear();
                    if let Err(e) = drag.finish(&mut self.state) {
                        self.state.status = e;
                    }
                }
                return;
            }
        }
        let cam = self.state.camera;
        let snap_step = |mode: GizmoMode| {
            modifiers.command.then_some(match mode {
                GizmoMode::Translate => 10.0,
                GizmoMode::Rotate => 15f64.to_radians(),
                GizmoMode::Scale => 0.1,
            })
        };
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(g) = self.current_gizmo()
            && let Some(handle) = g.hit_test(&cam, x, y, w, h)
            && let Some(drag) = Drag::begin(&g, handle, &cam, x, y, w, h)
        {
            if component_target(&self.state).is_some() {
                // a gizmo drag in a component mode edits the components (EDITABLE.md §3.5)
                if let Some(cd) = ComponentDrag::begin(&self.state) {
                    self.gesture_seq += 1;
                    self.component_drag = Some((drag, cd, self.gesture_seq));
                }
            } else if let Some(id) = self.state.selection.active
                && let Some(start) = self.state.evaluated().local_transform(id)
            {
                self.gesture_seq += 1;
                self.gizmo_drag = Some((drag, id, start, self.gesture_seq));
            }
        }
        if let Some((drag, cd, n)) = &mut self.component_drag {
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = drag.update(&cam, x, y, w, h, snap_step(self.gizmo_mode));
                self.state.status = match cd.apply(&mut self.state.doc, delta, Gesture::Drag(*n)) {
                    Ok(()) => String::new(),
                    Err(e) => e.to_string(),
                };
            }
            if response.drag_stopped() {
                self.component_drag = None;
            }
            return;
        }
        if let Some((drag, id, start, n)) = &mut self.gizmo_drag {
            if response.dragged_by(egui::PointerButton::Primary) {
                let delta = drag.update(&cam, x, y, w, h, snap_step(self.gizmo_mode));
                let (id, start, n) = (*id, *start, *n);
                let (ctx, eval) = (self.state.anim_ctx(), self.state.evaluated());
                let anim = AnimEdit { ctx, eval: &eval };
                self.state.status = match apply_drag_anim(
                    &mut self.state.doc,
                    id,
                    &start,
                    delta,
                    Gesture::Drag(n),
                    Some(&anim),
                ) {
                    Ok(()) => String::new(),
                    Err(e) => e.to_string(),
                };
            }
            if response.drag_stopped() {
                self.gizmo_drag = None;
            }
            return;
        }
        // a primary drag that is not on a gizmo handle selects components by rectangle
        if component_target(&self.state).is_some() {
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.rect_drag = Some([x, y, x, y]);
            }
            if let Some(r) = &mut self.rect_drag
                && response.dragged_by(egui::PointerButton::Primary)
            {
                (r[2], r[3]) = (x, y);
            }
        }
        if response.clicked()
            && let Err(e) = viewport_click(&mut self.state, [x, y], [w, h], mods)
        {
            self.state.status = e;
        }
    }

    fn coordinates_ui(&mut self, ui: &mut Ui) {
        let Some(id) = self.state.selection.active else {
            ui.weak("Nothing selected");
            return;
        };
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.coord_space, Space::Local, "Local");
            ui.radio_value(&mut self.coord_space, Space::World, "World");
            ui.separator();
            ui.radio_value(&mut self.coord_third, Third::Scale, "Scale");
            ui.radio_value(&mut self.coord_third, Third::Size, "Size");
        });
        let eval_scene = self.state.evaluated();
        let Some(view) = coord_view(&eval_scene, id, self.coord_space, self.coord_third) else {
            return;
        };
        let third_label = match self.coord_third {
            Third::Scale => "Scale",
            Third::Size => "Size",
        };
        let rows = [
            (
                Row::Position,
                "Position",
                ["X", "Y", "Z"],
                Some(view.pos),
                false,
            ),
            (
                Row::Rotation,
                "Rotation",
                ["H", "P", "B"],
                Some(view.rot),
                true,
            ),
            (
                Row::Third(self.coord_third),
                third_label,
                ["X", "Y", "Z"],
                view.third,
                false,
            ),
        ];
        egui::Grid::new(("coords", id))
            .num_columns(4)
            .show(ui, |ui| {
                for (row, label, axes, value, degrees) in rows {
                    let l = ui.label(label);
                    let Some(value) = value else {
                        ui.weak("no mesh");
                        ui.end_row();
                        continue;
                    };
                    let mut shown = if degrees {
                        value.map(f64::to_degrees)
                    } else {
                        value
                    };
                    let mut gesture = None;
                    for (k, axis) in axes.iter().enumerate() {
                        let r = ui
                            .add(
                                egui::DragValue::new(&mut shown[k])
                                    .speed(0.1)
                                    .prefix(format!("{axis} ")),
                            )
                            .labelled_by(l.id);
                        if r.changed() {
                            gesture = Some(if r.dragged() {
                                self.gesture_for(r.drag_started())
                            } else {
                                Gesture::Commit
                            });
                        }
                    }
                    if let Some(g) = gesture {
                        let value = if degrees {
                            shown.map(f64::to_radians)
                        } else {
                            shown
                        };
                        let anim = AnimEdit {
                            ctx: self.state.anim_ctx(),
                            eval: &eval_scene,
                        };
                        self.state.status = match set_coord_anim(
                            &mut self.state.doc,
                            id,
                            self.coord_space,
                            row,
                            value,
                            g,
                            Some(&anim),
                        ) {
                            Ok(()) => String::new(),
                            Err(e) => e.to_string(),
                        };
                    }
                    ui.end_row();
                }
                // component modes: the pivot of the selected components (EDITABLE.md §3.6)
                if component_target(&self.state).is_some() {
                    ui.label("Pivot");
                    match pivot_view(&self.state, self.coord_space == Space::World) {
                        Some(p) => {
                            for (k, axis) in ["X", "Y", "Z"].iter().enumerate() {
                                ui.label(format!("{axis} {:.3}", p[k]));
                            }
                        }
                        None => {
                            ui.weak("no selection");
                        }
                    }
                    ui.end_row();
                }
            });
    }

    fn tree_ui(&mut self, ui: &mut Ui) {
        let rows = tree_rows(self.state.doc.scene(), &self.state.expanded);
        let order: Vec<NodeId> = rows.iter().map(|r| r.id).collect();
        let mut cmds: Vec<Cmd> = Vec::new();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                for row in &rows {
                    ui.horizontal(|ui| {
                        ui.add_space(row.depth as f32 * 14.0);
                        if row.has_children {
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
                            r.request_focus();
                            if ui.input(|i| i.key_pressed(Key::Escape)) {
                                self.renaming = None;
                            } else if r.lost_focus() {
                                cmds.push((
                                    "object.rename",
                                    json!({"node": row.id.0, "name": buf}),
                                ));
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
                            cmds.push((
                                "object.reparent",
                                json!({"node": dragged.0, "parent": row.id.0}),
                            ));
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
                                cmds.push(cmd("object.delete"));
                                ui.close();
                            }
                            if ui.button("Move to root").clicked() {
                                cmds.push((
                                    "object.reparent",
                                    json!({"node": row.id.0, "parent": null}),
                                ));
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
                    cmds.push((
                        "object.reparent",
                        json!({"node": dragged.0, "parent": null}),
                    ));
                }
            });
        for c in cmds {
            self.run(c);
        }
    }

    /// The active modeling tool: its settings, Reset Values, Apply (MODELING.md §0.2).
    fn tool_ui(&mut self, ui: &mut Ui) {
        let Some(ts) = &self.state.tool else {
            return;
        };
        let label = ts.tool().label();
        let rows = crate::attributes::tool_rows(ts.tool().settings(), ts.values());
        ui.heading(label);
        let (mut reset, mut apply, mut done) = (false, false, false);
        ui.horizontal(|ui| {
            reset = ui.button("Reset Values").clicked();
            apply = ui.button("Apply").clicked();
            done = ui.button("Done").clicked();
        });
        let mut edits: Vec<(String, Value)> = Vec::new();
        egui::ScrollArea::vertical()
            .id_salt("tool_scroll")
            .max_height(240.0)
            .show(ui, |ui| {
                egui::Grid::new("tool_settings")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for row in &rows {
                            match (&row.widget, &row.value) {
                                (Widget::Checkbox, Value::Bool(b)) => {
                                    ui.label("");
                                    let mut on = *b;
                                    if ui.checkbox(&mut on, &row.label).changed() {
                                        edits.push((row.param.clone(), Value::Bool(on)));
                                    }
                                }
                                (Widget::IntDrag { min, max }, Value::Int(v)) => {
                                    let name = ui.label(&row.label);
                                    let mut x = *v;
                                    if ui
                                        .add(egui::DragValue::new(&mut x).range(*min..=*max))
                                        .labelled_by(name.id)
                                        .changed()
                                    {
                                        edits.push((row.param.clone(), Value::Int(x)));
                                    }
                                }
                                (Widget::FloatDrag { .. }, Value::Float(v)) => {
                                    let name = ui.label(&row.label);
                                    let mut x = *v;
                                    if ui
                                        .add(egui::DragValue::new(&mut x).speed(0.1))
                                        .labelled_by(name.id)
                                        .changed()
                                    {
                                        edits.push((row.param.clone(), Value::Float(x)));
                                    }
                                }
                                _ => continue,
                            }
                            ui.end_row();
                        }
                    });
            });
        let mut error = None;
        if let Some(ts) = self.state.tool.as_mut() {
            for (id, v) in edits {
                if let Err(e) = ts.set(&id, v) {
                    error = Some(e);
                }
            }
            if reset {
                ts.reset();
            }
        }
        if let Some(e) = error {
            self.state.status = e;
        }
        if apply {
            self.run_tool();
        }
        if done {
            crate::tools::deactivate_tool(&mut self.state);
            self.tool_drag = None;
        }
        ui.separator();
    }

    /// Applies the active tool through its command; an error goes to the status bar.
    fn run_tool(&mut self) {
        self.state.status.clear();
        if let Err(e) = crate::tools::apply_tool(&mut self.state) {
            self.state.status = e;
        }
    }

    fn attributes_ui(&mut self, ui: &mut Ui) {
        if self.state.tool.is_some() {
            self.tool_ui(ui);
        }
        let Some(id) = self.state.selection.active else {
            ui.weak("Nothing selected");
            return;
        };
        let shown = self.state.evaluated();
        let Some(view) = attribute_rows(&shown, id) else {
            ui.weak("Nothing selected");
            return;
        };
        let marks = key_states(&shown, id, self.state.timeline.current);
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
            self.run(("object.rename", json!({"node": id.0, "name": name})));
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
                                            self.attribute_row(ui, id, row, &marks);
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            });
    }

    fn attribute_row(
        &mut self,
        ui: &mut Ui,
        id: NodeId,
        row: &crate::attributes::AttrRow,
        marks: &std::collections::BTreeMap<String, KeyState>,
    ) {
        if row.widget == Widget::Checkbox {
            ui.label("");
            let mut b = row.value == Value::Bool(true);
            if ui.checkbox(&mut b, &row.label).changed() {
                self.edit(id, &row.param, Value::Bool(b), Gesture::Commit);
            }
            return;
        }
        let animatable = matches!(
            row.widget,
            Widget::FloatDrag { .. } | Widget::Vec3Drag { .. }
        );
        let label = ui
            .horizontal(|ui| {
                if animatable {
                    key_marker(ui, marks.get(&row.param).copied(), &row.label);
                }
                ui.label(&row.label)
            })
            .inner;
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
        let settings = self.state.settings();
        let theme = Theme::default();
        let ids: Vec<NodeId> = self.state.selection.nodes.iter().copied().collect();
        let mut cmds: Vec<Cmd> = Vec::new();
        ui.horizontal_wrapped(|ui| {
            let playing = self.state.timeline.playing;
            if ui.button(if playing { "Stop" } else { "Play" }).clicked() {
                cmds.push(play_stop(&self.state));
            }
            if ui.button("Record").clicked() {
                cmds.push(cmd("key.record"));
            }
            let mut auto = self.state.autokey;
            if ui.checkbox(&mut auto, "Auto-Key").clicked() {
                cmds.push(cmd("key.autokey"));
            }
            ui.separator();
            if ui.button("Previous Key").clicked() {
                cmds.push(("key.jump", json!({"forward": false})));
            }
            if ui.button("Next Key").clicked() {
                cmds.push(("key.jump", json!({"forward": true})));
            }
            if ui.button("Delete Keys").clicked() {
                cmds.push(cmd("key.delete_at_frame"));
            }
        });
        ui.horizontal(|ui| {
            let mut next = settings;
            let l = ui.label("Start");
            ui.add(egui::DragValue::new(&mut next.start).range(i64::MIN..=settings.end))
                .labelled_by(l.id);
            let l = ui.label("End");
            ui.add(egui::DragValue::new(&mut next.end).range(settings.start..=i64::MAX))
                .labelled_by(l.id);
            let l = ui.label("FPS");
            ui.add(egui::DragValue::new(&mut next.fps).range(1..=240))
                .labelled_by(l.id);
            if next != settings {
                cmds.push((
                    "timeline.settings",
                    json!({"fps": next.fps, "start": next.start, "end": next.end}),
                ));
            }
            ui.separator();
            let mut frame = self.state.timeline.current;
            let l = ui.label("Frame");
            let r = ui
                .add(egui::DragValue::new(&mut frame).range(settings.start..=settings.end))
                .labelled_by(l.id);
            if r.changed() {
                cmds.push(("timeline.set_frame", json!({"frame": frame})));
            }
            ui.weak(format!(
                "{:.3} s",
                crate::animate::frame_time(frame, settings.fps)
            ));
        });
        // frame strip: click or drag scrubs; ticks mark the selected objects' keys
        let width = ui.available_width().max(60.0);
        let (rect, response) =
            ui.allocate_exact_size(egui::vec2(width, 30.0), Sense::click_and_drag());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Slider, true, "Frame strip"));
        let span = (settings.end - settings.start).max(1) as f32;
        let (x0, x1) = (rect.left() + 8.0, rect.right() - 8.0);
        let to_x = |f: f64| x0 + (f as f32 - settings.start as f32) / span * (x1 - x0);
        if (response.dragged() || response.clicked())
            && let Some(pos) = response.interact_pointer_pos()
        {
            let f = settings.start as f32 + (pos.x - x0) / (x1 - x0) * span;
            cmds.push(("timeline.set_frame", json!({"frame": f.round() as i64})));
        }
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 2.0, theme.faint);
        let step = [1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 5000]
            .into_iter()
            .find(|s| (*s as f32 / span) * (x1 - x0) >= 40.0)
            .unwrap_or(10000);
        let first = settings.start.div_euclid(step) * step;
        for f in (first..=settings.end).step_by(step as usize) {
            if f < settings.start {
                continue;
            }
            let x = to_x(f as f64);
            painter.line_segment(
                [
                    egui::pos2(x, rect.bottom() - 6.0),
                    egui::pos2(x, rect.bottom()),
                ],
                egui::Stroke::new(1.0, theme.weak_text),
            );
            painter.text(
                egui::pos2(x + 2.0, rect.bottom() - 7.0),
                egui::Align2::LEFT_BOTTOM,
                f.to_string(),
                egui::FontId::proportional(10.0),
                theme.weak_text,
            );
        }
        for f in key_frames(self.state.doc.scene(), &ids) {
            if f < settings.start as f64 - 1e-6 || f > settings.end as f64 + 1e-6 {
                continue;
            }
            let x = to_x(f);
            painter.line_segment(
                [
                    egui::pos2(x, rect.top() + 2.0),
                    egui::pos2(x, rect.top() + 12.0),
                ],
                egui::Stroke::new(2.0, theme.accent),
            );
        }
        let x = to_x(self.state.timeline.current as f64);
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(1.5, theme.component_selection),
        );
        for c in cmds {
            self.run(c);
        }
    }
}

/// A diamond left of an animatable parameter: filled = key on this frame, outline =
/// animated without a key here (ANIM.md §2.5). Drawn by us.
fn key_marker(ui: &mut Ui, state: Option<KeyState>, name: &str) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), Sense::hover());
    let Some(state) = state else { return };
    let (text, fill) = match state {
        KeyState::Keyed => ("keyed", Theme::default().accent),
        KeyState::Animated => ("animated", egui::Color32::TRANSPARENT),
    };
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::Role::Label, true, format!("{name} {text}"))
    });
    let c = rect.center();
    let r = 4.5;
    let points = vec![
        egui::pos2(c.x, c.y - r),
        egui::pos2(c.x + r, c.y),
        egui::pos2(c.x, c.y + r),
        egui::pos2(c.x - r, c.y),
    ];
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        fill,
        egui::Stroke::new(1.5, Theme::default().accent),
    ));
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
