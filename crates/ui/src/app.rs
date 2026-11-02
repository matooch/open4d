//! The egui shell: menus, dock areas, panels and shortcuts (docs/specs/UI.md §9).
//! All state changes go through registry commands (`commands::execute`) or
//! `attributes::apply_edit`.

use eframe::egui_wgpu;
use egui::{RichText, Sense, Ui};
use forge_command::UiRequest;
use forge_core::transform::{Transform, rotation_part};
use forge_core::{NodeId, Value};
use forge_viewport::gizmo::{Drag, Gizmo, GizmoMode, Space as GizmoSpace};
use forge_viewport::panes::{Layout as PaneLayout, Pane, arrange};
use forge_viewport::render::Shading;
use glam::DVec3;
use serde_json::{Value as Json, json};

use crate::animate::{AnimEdit, KeyState, key_frames, key_states};
use crate::attributes::{Gesture, Widget, attribute_rows_multi};
use crate::clone_overlay::{OverlayOptions, selected_cloner_marks};
use crate::commands::{category_label, clone_toolset_menu, create_menu, execute};
use crate::component_edit::{ComponentDrag, pivot_view, pivot_world};
use crate::components::{
    ClickModifiers, EditMode, SelectCmd, target as component_target, viewport_click, viewport_rect,
};
use crate::coordinates::{Row, Space, Third, coord_view, set_coord_anim};
use crate::dock::{Area, Panel};
use crate::keymap_editor::KeyMapEditor;
use crate::layout::{ButtonGroup, Layout, Sizes, command_bar, tool_palette};
use crate::manipulate::apply_drag_anim;
use crate::object_manager::ObjectManager;
use crate::panes::{PANE_GAP, display_settings};
use crate::state::EditorState;
use crate::theme::Theme;
use crate::viewport_panel::{
    NavAction, NavInput, NavMode, SceneView, ViewportPanel, apply_nav, nav_action,
};

mod files;
mod recovery;

pub use files::{Dialogs, FileKind, NativeDialogs, Unsaved};

const SHADINGS: [(Shading, &str); 9] = [
    (Shading::Smooth, "Smooth"),
    (Shading::Flat, "Flat"),
    (Shading::Wire, "Wireframe"),
    (Shading::SmoothWire, "Smooth + wire"),
    (Shading::FlatWire, "Flat + wire"),
    (Shading::Quick, "Quick"),
    (Shading::Constant, "Constant"),
    (Shading::HiddenLine, "Hidden line"),
    (Shading::BoundingBox, "Bounding box"),
];

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

/// The `view.set_view` entries of the View menu.
const VIEWS: [(&str, &str); 7] = [
    ("Perspective", "perspective"),
    ("Front", "front"),
    ("Right", "right"),
    ("Top", "top"),
    ("Back", "back"),
    ("Left", "left"),
    ("Bottom", "bottom"),
];

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
    /// Initial sizes of the resizable areas (docs/specs/UI.md §11).
    sizes: Sizes,
    viewport: ViewportPanel,
    /// The object manager's panel state: rename buffer, search text (UI.md §13).
    objects: ObjectManager,
    /// The key map editor window (UI.md §14.5).
    keymap_editor: KeyMapEditor,
    /// Name field buffer in the attribute panel.
    name_edit: Option<(NodeId, String)>,
    /// The attribute manager tab shown (UI.md §12).
    attr_tab: String,
    /// Counter for drag/typing gestures (each is one undo step).
    gesture_seq: u64,
    gesture: Option<u64>,
    /// The pointer was over the viewport last frame (Space plays only then).
    viewport_hot: bool,
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

    /// Clean exit: the session's autosaves and marker go (docs/specs/PLATFORM.md §7.6).
    fn on_exit(&mut self) {
        self.state.autosave.finish();
    }
}

impl Editor {
    /// The key map editor window's state.
    pub fn keymap_editor(&self) -> &KeyMapEditor {
        &self.keymap_editor
    }

    pub fn keymap_editor_mut(&mut self) -> &mut KeyMapEditor {
        &mut self.keymap_editor
    }

    /// The object manager's panel state.
    pub fn objects(&self) -> &ObjectManager {
        &self.objects
    }

    pub fn new(state: EditorState) -> Editor {
        Editor {
            state,
            sizes: Sizes::default(),
            viewport: ViewportPanel::default(),
            objects: ObjectManager::default(),
            keymap_editor: KeyMapEditor::default(),
            name_edit: None,
            attr_tab: String::new(),
            gesture_seq: 0,
            gesture: None,
            viewport_hot: false,
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

    /// The current arrangement as a serialisable layout.
    pub fn layout(&self) -> Layout {
        Layout {
            version: crate::layout::LAYOUT_VERSION,
            dock: self.state.dock.clone(),
            sizes: self.sizes,
        }
    }

    /// Switches to `layout`. Area sizes apply to areas that have not been drawn yet.
    pub fn set_layout(&mut self, layout: Layout) {
        self.state.dock = layout.dock;
        self.sizes = layout.sizes;
    }

    /// Runs a command; an error goes to the status bar. True if it succeeded.
    fn run(&mut self, (id, args): Cmd) -> bool {
        self.run_id(id, args)
    }

    /// Like [`Editor::run`] for a command id that is not a literal (key map bindings).
    fn run_id(&mut self, id: &str, args: Json) -> bool {
        // A command may leave a message of its own (e.g. skipped nodes) in the status.
        self.state.status.clear();
        let ok = match execute(&mut self.state, id, args) {
            Ok(_) => true,
            Err(e) => {
                self.state.status = e;
                false
            }
        };
        self.serve_ui_requests();
        ok
    }

    /// Does what commands asked of the window: dialogs and inline rename.
    fn serve_ui_requests(&mut self) {
        for request in std::mem::take(&mut self.state.ui_requests) {
            match request {
                UiRequest::Open => self.file_action(FileAction::Open),
                UiRequest::Save => self.file_action(FileAction::Save),
                UiRequest::SaveAs => self.file_action(FileAction::SaveAs),
                UiRequest::RenameActive => {
                    if let Some(id) = self.state.selection.active
                        && let Some(n) = self.state.doc.scene().node(id)
                    {
                        self.objects.renaming = Some((id, n.name.clone()));
                    }
                }
            }
        }
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
        // Closing the window with unsaved changes asks first (UI.md §15).
        if ui.ctx().input(|i| i.viewport().close_requested()) && !self.confirm_discard() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        self.update_playback(ui.ctx());
        self.autosave_frame(ui.ctx());
        self.shortcuts(ui.ctx());
        let title = self.window_title();
        if title != self.sent_title {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.sent_title = title;
        }
        egui::Panel::top("menu").show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui));
        let sizes = self.sizes;
        if !self.state.dock.tabs(Area::Top).is_empty() {
            egui::Panel::top("area_top")
                .resizable(false)
                .show(ui, |ui| self.area(ui, Area::Top, rs));
        }
        if !self.state.dock.tabs(Area::Bottom).is_empty() {
            egui::Panel::bottom("area_bottom")
                .resizable(true)
                .default_size(sizes.bottom_height)
                .show(ui, |ui| self.area(ui, Area::Bottom, rs));
        }
        if !self.state.dock.tabs(Area::Left).is_empty() {
            egui::Panel::left("area_left")
                .resizable(true)
                .default_size(sizes.left_width)
                .show(ui, |ui| self.area(ui, Area::Left, rs));
        }
        let (top, bottom) = (
            !self.state.dock.tabs(Area::RightTop).is_empty(),
            !self.state.dock.tabs(Area::RightBottom).is_empty(),
        );
        if top || bottom {
            egui::Panel::right("area_right")
                .resizable(true)
                .default_size(sizes.right_width)
                .show(ui, |ui| {
                    if top && bottom {
                        let h = ui.available_height();
                        egui::Panel::top("area_right_top")
                            .resizable(true)
                            .default_size(h * sizes.right_top_fraction)
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
        self.keymap_editor.show(ui.ctx(), &mut self.state);
    }

    /// Advances the current frame from the frame clock while playing (ANIM.md §2.6).
    fn update_playback(&mut self, ctx: &egui::Context) {
        self.state.sync_timeline();
        if !self.state.timeline.playing {
            return;
        }
        let now = ctx.input(|i| i.time);
        let scene = self.state.doc.scene();
        let playback = forge_command::Playback::of(&scene.settings(), scene.time());
        self.state.timeline.update_with(now, &playback);
        ctx.request_repaint();
    }

    /// Key presses run the registry command their chord is bound to (UI.md §14). Nothing
    /// is handled here but the lookup: the key map decides, `run_id` runs.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.keymap_editor.is_capturing() {
            if let Some(chord) = ctx.input_mut(crate::keymap_editor::take_captured) {
                self.keymap_editor.captured(chord, &mut self.state.keys);
            }
            return;
        }
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let hot = self.viewport_hot;
        let presses = ctx.input_mut(|i| self.state.keys.map.take_presses(i, hot));
        for (id, args) in presses {
            // Delete over the node editor deletes nodes, not objects (UI.md §17.4).
            if id == "object.delete" && self.state.node_editor.canvas.hot {
                self.state.node_editor.canvas.delete_requested = true;
                ctx.request_repaint();
                continue;
            }
            self.run_id(&id, args);
        }
    }

    /// The platform spelling of the first chord bound to the command, for menus.
    fn chord_text(&self, ctx: &egui::Context, (id, args): &Cmd) -> Option<String> {
        self.state
            .keys
            .map
            .chord_of(id, args)
            .map(|c| c.display(ctx))
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
                    (
                        "Open\u{2026}",
                        Some("file.open_dialog"),
                        FileAction::Open,
                        true,
                    ),
                    ("Save", Some("file.save_dialog"), FileAction::Save, true),
                    (
                        "Save As\u{2026}",
                        Some("file.save_as_dialog"),
                        FileAction::SaveAs,
                        true,
                    ),
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
                    if let Some(sc) = shortcut.and_then(|id| self.chord_text(ui.ctx(), &cmd(id))) {
                        button = button.shortcut_text(sc);
                    }
                    if ui.add_enabled(enabled, button).clicked() {
                        ui.close();
                        self.file_action(action);
                    }
                }
            });
            ui.menu_button("Edit", |ui| {
                for (label, command) in [
                    ("Undo", cmd("edit.undo")),
                    ("Redo", cmd("edit.redo")),
                    ("Delete", cmd("object.delete")),
                ] {
                    if ui.button(label).clicked() {
                        self.run(command);
                        ui.close();
                    }
                }
                ui.separator();
                let select_items = [
                    ("Select All", select_cmd(SelectCmd::All)),
                    ("Deselect All", select_cmd(SelectCmd::None)),
                    ("Invert Selection", select_cmd(SelectCmd::Invert)),
                ];
                for (label, command) in select_items {
                    let mut button = egui::Button::new(label);
                    if let Some(sc) = self.chord_text(ui.ctx(), &command) {
                        button = button.shortcut_text(sc);
                    }
                    if ui.add(button).clicked() {
                        self.run(command);
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
                ui.separator();
                let extra: [(&str, Cmd); 7] = [
                    ("Select Connected", cmd("select.connected")),
                    ("Loop Selection", cmd("select.loop")),
                    ("Ring Selection", cmd("select.ring")),
                    (
                        "Convert to Points",
                        ("select.convert", json!({"to": "points"})),
                    ),
                    (
                        "Convert to Edges",
                        ("select.convert", json!({"to": "edges"})),
                    ),
                    (
                        "Convert to Polygons",
                        ("select.convert", json!({"to": "polys"})),
                    ),
                    (
                        "Convert to Polygons (tolerant)",
                        ("select.convert", json!({"to": "polys", "mode": "tolerant"})),
                    ),
                ];
                for (label, command) in extra {
                    if ui.button(label).clicked() {
                        self.run(command);
                        ui.close();
                    }
                }
            });
            ui.menu_button("Mesh", |ui| {
                for (i, tool) in crate::tools::tools().into_iter().enumerate() {
                    // the three interactive tools, then the mesh commands (MODELING.md §4)
                    if i == 3 {
                        ui.separator();
                    }
                    if ui.button(tool.label()).clicked() {
                        crate::tools::activate_tool(&mut self.state, tool.id());
                        ui.close();
                    }
                }
                // Loop/Path Cut, the Knife modes and Slide (MODELING.md §5)
                ui.separator();
                for tool in crate::tools::cutting_tools() {
                    if ui.button(tool.label()).clicked() {
                        crate::tools::activate_tool(&mut self.state, tool.id());
                        ui.close();
                    }
                }
                // Bridge, Close Polygon Hole, Stitch and Sew, Polygon Pen (MODELING.md §6)
                ui.separator();
                for tool in crate::tools::building_tools() {
                    if ui.button(tool.label()).clicked() {
                        crate::tools::activate_tool(&mut self.state, tool.id());
                        ui.close();
                    }
                }
            });
            ui.menu_button("Create", |ui| {
                ui.menu_button("Clone toolset", |ui| {
                    let entries = clone_toolset_menu(self.state.doc.scene().registry());
                    for entry in entries {
                        if ui.button(&entry.label).clicked() {
                            self.run((entry.command, entry.args));
                            ui.close();
                        }
                    }
                });
                ui.separator();
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
                let items = [
                    ("Record Active Objects", cmd("key.record")),
                    ("Delete Keys", cmd("key.delete_at_frame")),
                    ("Play / Stop", cmd("timeline.toggle_play")),
                    ("Previous Frame", ("timeline.step", json!({"delta": -1}))),
                    ("Next Frame", ("timeline.step", json!({"delta": 1}))),
                    ("Previous Key", ("key.jump", json!({"forward": false}))),
                    ("Next Key", ("key.jump", json!({"forward": true}))),
                ];
                for (label, command) in items {
                    let mut button = egui::Button::new(label);
                    if let Some(sc) = self.chord_text(ui.ctx(), &command) {
                        button = button.shortcut_text(sc);
                    }
                    if ui.add(button).clicked() {
                        self.run(command);
                        ui.close();
                    }
                }
                ui.separator();
                let mut auto = self.state.autokey;
                let label = match self.chord_text(ui.ctx(), &cmd("key.autokey")) {
                    Some(sc) => format!("Auto-Key  ({sc})"),
                    None => "Auto-Key".to_string(),
                };
                if ui.checkbox(&mut auto, label).clicked() {
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
                self.view_options_menu(ui);
                ui.separator();
                ui.radio_value(
                    &mut self.state.gizmo_space,
                    GizmoSpace::Object,
                    "Gizmo axes: object",
                );
                ui.radio_value(
                    &mut self.state.gizmo_space,
                    GizmoSpace::World,
                    "Gizmo axes: world",
                );
                ui.separator();
                for (label, command) in [
                    ("Frame all", cmd("view.frame_all")),
                    ("Frame selected", cmd("view.frame_selected")),
                ] {
                    let mut button = egui::Button::new(label);
                    if let Some(sc) = self.chord_text(ui.ctx(), &command) {
                        button = button.shortcut_text(sc);
                    }
                    if ui.add(button).clicked() {
                        self.run(command);
                        ui.close();
                    }
                }
                ui.menu_button("Camera view", |ui| {
                    for (label, view) in VIEWS {
                        let command: Cmd = ("view.set_view", json!({"view": view}));
                        let mut button = egui::Button::new(label);
                        if let Some(sc) = self.chord_text(ui.ctx(), &command) {
                            button = button.shortcut_text(sc);
                        }
                        if ui.add(button).clicked() {
                            self.run(command);
                            ui.close();
                        }
                    }
                });
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
                if ui.button("Key map\u{2026}").clicked() {
                    self.keymap_editor.open = true;
                    ui.close();
                }
                if ui.button("Reset layout").clicked() {
                    self.set_layout(Layout::standard());
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
        // single-panel Centre, Top and Left areas have no tab header
        if tabs.len() > 1 || !matches!(area, Area::Center | Area::Top | Area::Left) {
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
            Panel::CommandBar => self.command_bar_ui(ui),
            Panel::ToolPalette => self.tool_palette_ui(ui),
            Panel::Viewport => self.viewport_ui(ui, rs),
            Panel::ObjectTree => self.tree_ui(ui),
            Panel::Attributes => self.attributes_ui(ui),
            Panel::Timeline => self.timeline_ui(ui),
            Panel::Coordinates => self.coordinates_ui(ui),
            Panel::Script => {
                crate::script_panel::script_panel_ui(ui, &mut self.state, self.dialogs.as_mut())
            }
            Panel::NodeEditor => crate::node_editor::node_editor_ui(ui, &mut self.state),
        }
    }

    /// The command bar: grouped buttons, each running one registry command.
    fn command_bar_ui(&mut self, ui: &mut Ui) {
        let mut clicked: Option<Cmd> = None;
        ui.horizontal_wrapped(|ui| {
            for (i, group) in command_bar().iter().enumerate() {
                if i > 0 {
                    ui.separator();
                }
                if let Some(c) = self.button_group(ui, group) {
                    clicked = Some(c);
                }
            }
        });
        if let Some(c) = clicked {
            self.run(c);
        }
    }

    /// The tool palette: the groups stacked in a column.
    fn tool_palette_ui(&mut self, ui: &mut Ui) {
        let mut clicked: Option<Cmd> = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for group in tool_palette() {
                ui.weak(group.name);
                ui.horizontal_wrapped(|ui| {
                    if let Some(c) = self.button_group(ui, &group) {
                        clicked = Some(c);
                    }
                });
                ui.separator();
            }
        });
        if let Some(c) = clicked {
            self.run(c);
        }
    }

    /// Draws a group's buttons; the command of the one clicked, if any.
    fn button_group(&self, ui: &mut Ui, group: &ButtonGroup) -> Option<Cmd> {
        let mut clicked = None;
        for b in &group.buttons {
            let selected = b.command == "select.mode"
                && b.args["mode"] == json!(forge_command::ModeArg::from(self.state.mode))
                || (b.command == "view.shading"
                    && b.args["mode"]
                        == json!(crate::commands::shading_mode_of(self.state.shading)));
            let response = ui
                .add(egui::Button::new(b.text()).selected(selected))
                .on_hover_text(b.tip);
            if response.clicked() {
                // the registry owns the ids; `Cmd` borrows them as 'static
                clicked = Some((b.command, b.args.clone()));
            }
        }
        clicked
    }

    /// Soft selection and symmetry controls of component modes (EDITABLE.md §5.6).
    fn component_bar(&mut self, ui: &mut Ui) {
        use forge_modeling::{Axis, Distance, Falloff};
        let st = &mut self.state.app.component_settings;
        ui.checkbox(&mut st.soft.enabled, "Soft");
        if st.soft.enabled {
            ui.add(
                egui::DragValue::new(&mut st.soft.radius)
                    .range(0.001..=1e9)
                    .speed(1.0)
                    .prefix("R "),
            );
            egui::ComboBox::from_id_salt("soft_falloff")
                .selected_text(st.soft.falloff.id())
                .show_ui(ui, |ui| {
                    for f in Falloff::ALL {
                        ui.selectable_value(&mut st.soft.falloff, f, f.id());
                    }
                });
            egui::ComboBox::from_id_salt("soft_distance")
                .selected_text(st.soft.distance.id())
                .show_ui(ui, |ui| {
                    for d in Distance::ALL {
                        ui.selectable_value(&mut st.soft.distance, d, d.id());
                    }
                });
        }
        ui.checkbox(&mut st.symmetry.enabled, "Sym");
        if st.symmetry.enabled {
            egui::ComboBox::from_id_salt("sym_axis")
                .selected_text(st.symmetry.axis.id())
                .show_ui(ui, |ui| {
                    for a in Axis::ALL {
                        ui.selectable_value(&mut st.symmetry.axis, a, a.id());
                    }
                });
        }
    }

    /// View menu: layout, maximise, lighting, shadows, anti-aliasing (docs/specs/UI.md §6.2).
    /// Each entry runs its registry command.
    fn view_options_menu(&mut self, ui: &mut Ui) {
        let v = self.state.views.clone();
        let four = v.layout == PaneLayout::Four;
        let mut runs: Vec<Cmd> = Vec::new();
        let mut item = |ui: &mut Ui, this: &Self, on: bool, label: &str, command: Cmd| {
            let mut button = egui::Button::new(label).selected(on);
            if let Some(sc) = this.chord_text(ui.ctx(), &command) {
                button = button.shortcut_text(sc);
            }
            if ui.add(button).clicked() {
                runs.push(command);
                ui.close();
            }
        };
        item(
            ui,
            self,
            !four,
            "Layout: single view",
            ("view.layout", json!({"layout": "single"})),
        );
        item(
            ui,
            self,
            four,
            "Layout: four views",
            ("view.layout", json!({"layout": "four"})),
        );
        ui.add_enabled_ui(four, |ui| {
            item(
                ui,
                self,
                v.maximized.is_some(),
                "Maximize pane",
                ("view.maximize", json!({})),
            );
        });
        ui.separator();
        for (mode, label) in [
            (forge_command::LightingArg::Headlight, "Lighting: headlight"),
            (forge_command::LightingArg::Rig, "Lighting: rig"),
        ] {
            item(
                ui,
                self,
                v.lighting == mode,
                label,
                ("view.lighting", json!({"mode": mode})),
            );
        }
        item(
            ui,
            self,
            v.shadows,
            "Shadows",
            ("view.shadows", json!({"enabled": !v.shadows})),
        );
        item(
            ui,
            self,
            v.antialias,
            "Anti-aliasing",
            ("view.antialias", json!({"enabled": !v.antialias})),
        );
        ui.separator();
        item(
            ui,
            self,
            v.clone_indices,
            "Clone Indices",
            ("view.clone_display", json!({"indices": !v.clone_indices})),
        );
        item(
            ui,
            self,
            v.clone_axes,
            "Clone Axes",
            ("view.clone_display", json!({"axes": !v.clone_axes})),
        );
        for c in runs {
            self.run(c);
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
            if self.state.mode != EditMode::Model {
                ui.separator();
                self.component_bar(ui);
            }
        });
        let area = ui.available_rect_before_wrap();
        let rects = arrange(
            self.state.views.layout,
            self.state.views.active,
            self.state.views.maximized,
            area.width(),
            area.height(),
            PANE_GAP,
        );
        self.viewport_hot = false;
        let four = rects.len() > 1;
        let active = self.state.views.active;
        for r in rects {
            let rect =
                egui::Rect::from_min_size(area.min + egui::vec2(r.x, r.y), egui::vec2(r.w, r.h));
            self.pane_ui(ui, rs, r.pane, rect);
            if four && r.pane == active {
                // the active pane's outline
                ui.painter().rect_stroke(
                    rect,
                    0.0,
                    egui::Stroke::new(1.5, Theme::default().component_selection),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    /// One viewport pane in `rect`: the pointer in it makes it active, then navigation,
    /// picking and gizmos act on it (docs/specs/UI.md §6.2).
    fn pane_ui(
        &mut self,
        ui: &mut Ui,
        rs: Option<&egui_wgpu::RenderState>,
        pane: Pane,
        rect: egui::Rect,
    ) {
        let response = ui.allocate_rect(rect, Sense::click_and_drag());
        self.viewport_hot |= response.hovered() || response.has_focus();
        if response.hovered() || response.dragged() || response.is_pointer_button_down_on() {
            self.state.activate(pane);
        }
        let typing = ui.ctx().egui_wants_keyboard_input();
        // the response reads the input itself, so ask it before locking the input
        let drag = if response.dragged() {
            response.drag_delta().into()
        } else {
            [0.0, 0.0]
        };
        let over = response.hovered();
        let input = ui.ctx().input(|i| NavInput {
            primary: i.pointer.primary_down(),
            middle: i.pointer.middle_down(),
            secondary: i.pointer.secondary_down(),
            alt: i.modifiers.alt,
            held: NavMode::KEYS
                .iter()
                .find(|(k, _)| {
                    !typing && i.key_down(*k) && !i.modifiers.command && !i.modifiers.alt
                })
                .map(|(_, m)| *m),
            drag,
            scroll: if over { i.smooth_scroll_delta.y } else { 0.0 },
            cursor: i
                .pointer
                .hover_pos()
                .map_or([0.0, 0.0], |p| (p - rect.min).into()),
        });
        let action = nav_action(&input);
        let view_size = [rect.width() as f64, rect.height() as f64];
        if pane == self.state.views.active {
            if action != NavAction::None {
                self.state.touch_view();
            }
            apply_nav(&mut self.state.camera, action, view_size);
            if view_size[1] > 0.0 {
                self.state.view_aspect = view_size[0] / view_size[1];
            }
        }
        let Some(rs) = rs else {
            // not selectable: a selectable label would take the pointer from the navigation
            ui.put(
                rect,
                egui::Label::new("Viewport unavailable").selectable(false),
            );
            return;
        };

        let ppp = ui.ctx().pixels_per_point();
        let size = [
            (rect.width() * ppp).round() as u32,
            (rect.height() * ppp).round() as u32,
        ];
        if input.navigating() {
            self.rect_drag = None;
        } else {
            self.viewport_interaction(ui, &response, rect, ppp, size);
        }
        let gizmo = self.current_gizmo();
        let shown = self.state.evaluated();
        // a live tool preview replaces the target's mesh in the drawn scene only
        let shown = crate::tools::with_preview(&self.state, &shown).unwrap_or(shown);
        let lights = crate::panes::scene_lights(&shown);
        let tex = self.viewport.render(
            rs,
            pane,
            size,
            &self.state.pane_camera(pane),
            display_settings(&self.state.views, self.state.pane_shading(pane), &lights),
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
        self.paint_clone_overlay(ui, &shown, pane, rect);
        let label = SHADINGS
            .iter()
            .find(|(s, _)| *s == self.state.pane_shading(pane))
            .map_or("", |(_, l)| l);
        ui.painter().text(
            rect.left_top() + egui::vec2(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            match self.state.mode {
                EditMode::Model => format!("{} · {label}", pane.label()),
                mode => format!("{} · {label} · {} mode", pane.label(), mode.label()),
            },
            egui::FontId::proportional(12.0),
            Theme::default().weak_text,
        );
        // cutting tools: the knife line being dragged and the heads-up line (MODELING.md §5.5)
        if let Some(hud) =
            crate::cutting::hud(&self.state).or_else(|| crate::building::hud(&self.state))
        {
            ui.painter().text(
                rect.left_bottom() + egui::vec2(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                hud,
                egui::FontId::proportional(12.0),
                Theme::default().component_selection,
            );
        }
        if let Some(([x0, y0, x1, y1], _)) =
            self.state.tool.as_ref().and_then(|t| t.interaction.drag)
        {
            let to_points = |x: f64, y: f64| {
                rect.min + egui::vec2((x / ppp as f64) as f32, (y / ppp as f64) as f32)
            };
            ui.painter().line_segment(
                [to_points(x0, y0), to_points(x1, y1)],
                egui::Stroke::new(1.5, Theme::default().component_selection),
            );
        }
        // Polygon Pen: the corners placed so far (MODELING.md §6.7)
        let size = rect.size() * ppp;
        for [x, y] in crate::building::pen_corner_pixels(&self.state, size.x as f64, size.y as f64)
        {
            ui.painter().circle_filled(
                rect.min + egui::vec2((x / ppp as f64) as f32, (y / ppp as f64) as f32),
                4.0,
                Theme::default().component_selection,
            );
        }
    }

    /// Clone indices and axes of the selected cloners (UI.md §16.3), painted over the image.
    fn paint_clone_overlay(
        &self,
        ui: &Ui,
        scene: &forge_core::Scene,
        pane: Pane,
        rect: egui::Rect,
    ) {
        let v = &self.state.views;
        let marks = selected_cloner_marks(
            scene,
            &self.state.selection.nodes,
            &forge_core::EvalCtx::at_frame(
                self.state.timeline.current,
                f64::from(scene.settings().fps),
            ),
            &self.state.pane_camera(pane),
            [f64::from(rect.width()), f64::from(rect.height())],
            OverlayOptions {
                indices: v.clone_indices,
                axes: v.clone_axes,
            },
        );
        let painter = ui.painter().with_clip_rect(rect);
        let at = |p: [f32; 2]| rect.min + egui::vec2(p[0], p[1]);
        let colours = [
            egui::Color32::from_rgb(230, 70, 70),
            egui::Color32::from_rgb(80, 200, 90),
            egui::Color32::from_rgb(80, 130, 240),
        ];
        for mark in &marks {
            if let Some(axes) = mark.axes {
                for (end, colour) in axes.iter().zip(colours) {
                    painter
                        .line_segment([at(mark.origin), at(*end)], egui::Stroke::new(1.5, colour));
                }
            }
            if v.clone_indices {
                painter.text(
                    at(mark.origin) + egui::vec2(3.0, -3.0),
                    egui::Align2::LEFT_BOTTOM,
                    mark.index.to_string(),
                    egui::FontId::proportional(11.0),
                    Theme::default().weak_text,
                );
            }
        }
    }

    /// The pointer on an active topology tool: hover previews, a click picks or builds.
    fn build_interaction(
        &mut self,
        response: &egui::Response,
        rect: egui::Rect,
        ppp: f32,
        size: [u32; 2],
    ) {
        let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) else {
            return;
        };
        let (w, h) = (size[0].max(1) as f64, size[1].max(1) as f64);
        let x = ((pos.x - rect.min.x) * ppp) as f64;
        let y = ((pos.y - rect.min.y) * ppp) as f64;
        crate::building::hover(&mut self.state, x, y, w, h);
        if response.clicked_by(egui::PointerButton::Primary) {
            self.state.status.clear();
            if let Err(e) = crate::building::click(&mut self.state) {
                self.state.status = e;
            }
        }
    }

    /// The pointer on an active cutting tool: hover previews, a click or a drag cuts.
    fn cut_interaction(
        &mut self,
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
        let Some(pos) = response.hover_pos().or(response.interact_pointer_pos()) else {
            return;
        };
        let (x, y) = to_pixels(pos);
        let id = self.state.tool.as_ref().map(|t| t.tool().id());
        if matches!(id, Some("loop_cut" | "knife_loop")) {
            crate::cutting::hover(&mut self.state, x, y, w, h);
            if response.clicked_by(egui::PointerButton::Primary) {
                self.state.status.clear();
                if let Err(e) = crate::cutting::click(&mut self.state) {
                    self.state.status = e;
                }
            }
            return;
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            crate::cutting::drag_begin(&mut self.state, x, y, [w, h]);
        }
        if response.dragged_by(egui::PointerButton::Primary) {
            crate::cutting::drag_update(&mut self.state, x, y);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            self.state.status.clear();
            if let Err(e) = crate::cutting::drag_finish(&mut self.state, x, y) {
                self.state.status = e;
            }
        }
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
        let object = self.state.gizmo_space == GizmoSpace::Object
            || self.state.gizmo_mode == GizmoMode::Scale;
        let axes = if object {
            let r = rotation_part(&m);
            [r.x_axis, r.y_axis, r.z_axis]
        } else {
            [DVec3::X, DVec3::Y, DVec3::Z]
        };
        Some(Gizmo {
            origin,
            mode: self.state.gizmo_mode,
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
        let cutting = self.state.tool.as_ref().is_some_and(|t| {
            crate::cutting::is_cut_tool(t.tool().id())
                && crate::cutting::cut_target(&self.state).is_some()
        });
        if cutting {
            self.cut_interaction(response, rect, ppp, size);
            return;
        }
        let building = self.state.tool.as_ref().is_some_and(|t| {
            crate::building::is_build_tool(t.tool().id())
                && crate::cutting::cut_target(&self.state).is_some()
        });
        if building {
            self.build_interaction(response, rect, ppp, size);
            return;
        }
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
                let delta = drag.update(&cam, x, y, w, h, snap_step(self.state.gizmo_mode));
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
                let delta = drag.update(&cam, x, y, w, h, snap_step(self.state.gizmo_mode));
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
        let cmds = self.objects.show(ui, &mut self.state);
        if let Some(tag) = self.objects.open_tag.take() {
            self.state.open_node_editor(tag);
        }
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
                                (Widget::Vec3Drag { .. }, Value::Vec3(v)) => {
                                    let name = ui.label(&row.label);
                                    let mut d = *v;
                                    let changed = ui
                                        .horizontal(|ui| {
                                            let mut changed = false;
                                            for (axis, c) in
                                                ["X", "Y", "Z"].iter().zip(d.iter_mut())
                                            {
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(c)
                                                            .speed(0.1)
                                                            .prefix(format!("{axis} ")),
                                                    )
                                                    .labelled_by(name.id)
                                                    .changed();
                                            }
                                            changed
                                        })
                                        .inner;
                                    if changed {
                                        edits.push((row.param.clone(), Value::Vec3(d)));
                                    }
                                }
                                (Widget::Combo { options }, Value::Enum(cur)) => {
                                    let name = ui.label(&row.label);
                                    let current = options
                                        .iter()
                                        .find(|o| o.id == *cur)
                                        .map_or(cur.as_str(), |o| o.label.as_str());
                                    egui::ComboBox::from_id_salt(("tool_combo", &row.param))
                                        .selected_text(current)
                                        .show_ui(ui, |ui| {
                                            for o in options {
                                                if ui
                                                    .selectable_label(o.id == *cur, &o.label)
                                                    .clicked()
                                                {
                                                    edits.push((
                                                        row.param.clone(),
                                                        Value::Enum(o.id.clone()),
                                                    ));
                                                }
                                            }
                                        })
                                        .response
                                        .labelled_by(name.id);
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
        // The active object first, then the other selected ones (multi-edit).
        let mut ids = vec![id];
        ids.extend(
            self.state
                .selection
                .nodes
                .iter()
                .copied()
                .filter(|n| *n != id),
        );
        let Some(view) = attribute_rows_multi(&shown, &ids) else {
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
            if view.count > 1 {
                ui.weak(format!("{} objects", view.count));
            }
        });
        if let Some(name) = rename {
            self.run(("object.rename", json!({"node": id.0, "name": name})));
        }
        let tabs = view.tabs();
        if !tabs.contains(&self.attr_tab) {
            self.attr_tab = tabs.first().cloned().unwrap_or_default();
        }
        if tabs.len() > 1 {
            ui.horizontal(|ui| {
                for t in &tabs {
                    if ui.selectable_label(*t == self.attr_tab, t).clicked() {
                        self.attr_tab = t.clone();
                    }
                }
            });
        }
        ui.separator();
        let tab = self.attr_tab.clone();
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_enabled_ui(view.editable, |ui| {
                    for (group, rows) in view.groups_of(&tab) {
                        egui::CollapsingHeader::new(group)
                            .default_open(true)
                            .show(ui, |ui| {
                                egui::Grid::new(("attrs", &tab, group))
                                    .num_columns(2)
                                    .spacing([12.0, 4.0])
                                    .show(ui, |ui| {
                                        for row in rows {
                                            self.attribute_row(ui, &shown, id, row, &marks);
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                });
            });
    }

    /// One row: key marker slot + label, then the widget of its kind. A change runs
    /// `param.edit` on every selected object (one undo step; a scrub merges into one).
    fn attribute_row(
        &mut self,
        ui: &mut Ui,
        scene: &forge_core::Scene,
        id: NodeId,
        row: &crate::attributes::AttrRow,
        marks: &std::collections::BTreeMap<String, KeyState>,
    ) {
        use crate::widgets::{self, Cx, Phase};
        let cx_label = if row.widget == Widget::Checkbox {
            ui.make_persistent_id(("attr", id, &row.param))
        } else {
            // Brief 097 attaches the keyframe click and context menu to this slot's response.
            ui.horizontal(|ui| {
                if row.animatable
                    && matches!(
                        row.widget,
                        Widget::FloatDrag { .. } | Widget::Vec3Drag { .. }
                    )
                {
                    key_marker(ui, marks.get(&row.param).copied(), &row.label);
                }
                ui.label(&row.label)
            })
            .inner
            .id
        };
        let cx = Cx {
            scene,
            node: id,
            label: cx_label,
        };
        if let Some(edit) = widgets::show(ui, &cx, row) {
            let gesture = match edit.phase {
                Phase::Commit => None,
                Phase::Drag { started } => match self.gesture_for(started) {
                    Gesture::Drag(n) => Some(n),
                    Gesture::Commit => None,
                },
            };
            let mut nodes: Vec<NodeId> = self.state.selection.nodes.iter().copied().collect();
            if !nodes.contains(&id) {
                nodes.insert(0, id);
            }
            self.run(("param.edit", edit.args(&row.param, &nodes, gesture)));
        }
        if ui.input(|i| i.pointer.any_released()) && ui.ctx().dragged_id().is_none() {
            self.gesture = None;
        }
    }

    fn timeline_ui(&mut self, ui: &mut Ui) {
        let settings = self.state.settings();
        let ids: Vec<NodeId> = self.state.selection.nodes.iter().copied().collect();
        let mut cmds: Vec<Cmd> = Vec::new();
        ui.horizontal_wrapped(|ui| {
            let playing = self.state.timeline.playing;
            if ui.button(if playing { "Stop" } else { "Play" }).clicked() {
                cmds.push(cmd("timeline.toggle_play"));
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
            ui.separator();
            crate::timeline::playback_controls(ui, self.state.doc.scene(), &mut cmds);
        });
        ui.horizontal_wrapped(|ui| {
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
            if next.fps != settings.fps {
                cmds.push(("timeline.set_fps", json!({"fps": next.fps})));
            } else if next != settings {
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
            ui.weak(crate::timeline::time_label(frame, self.state.doc.scene()));
            ui.separator();
            crate::timeline::range_controls(ui, self.state.doc.scene(), &mut cmds);
        });
        let keys = key_frames(self.state.doc.scene(), &ids);
        crate::timeline::strip(
            ui,
            self.state.doc.scene(),
            self.state.timeline.current,
            &keys,
            &mut cmds,
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
