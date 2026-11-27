//! The egui shell: menus, dock areas, panels and shortcuts (docs/specs/UI.md §9).
//! All state changes go through registry commands (`commands::execute`) or
//! `attributes::apply_edit`.

use eframe::egui_wgpu;
use egui::{RichText, Sense, Ui};
use forge_command::UiRequest;
use forge_core::transform::{Transform, rotation_part};
use forge_core::{NodeId, Value};
use forge_generators::axis::AxisEdit;
use forge_viewport::gizmo::{Drag, Gizmo, GizmoMode, Space as GizmoSpace};
use forge_viewport::panes::{Layout as PaneLayout, Pane, arrange};
use forge_viewport::render::Shading;
use glam::DVec3;
use serde_json::{Value as Json, json};

use crate::animate::key_frames;
use crate::attributes::{Gesture, Widget, attribute_rows_multi};
use crate::clone_overlay::{OverlayOptions, selected_cloner_marks};
use crate::commands::{
    CHARACTER_TYPES, character_menu, clone_toolset_menu, create_menu, execute, scene_menu,
};
use crate::component_edit::{
    ComponentDrag, component_coords, component_transform_args, pivot_world,
};
use crate::components::{
    ClickModifiers, EditMode, SelectCmd, target as component_target, viewport_click, viewport_rect,
};
use crate::content_browser::{BrowserAction, menu_entries};
use crate::coordinates::{CoordDraft, Mixed, Space, Third, coord_view_multi};
use crate::dock::{Area, Panel};
use crate::keymap_editor::KeyMapEditor;
use crate::layout::{ButtonGroup, Layout, Sizes, command_bar, tool_palette};
use crate::manipulate::{apply_axis_drag, apply_drag_edit, apply_transform_edits, edit_drafts};
use crate::object_manager::ObjectManager;
use crate::palette::Palette;
use crate::panes::{PANE_GAP, display_settings};
use crate::snapping::{SnapScene, collect_sources, collect_sources_multi, drag_step};
use crate::state::{DisplayMode, EditorState};
use crate::theme::{Theme, ThemeKind};
use crate::viewport_panel::{
    NavAction, NavInput, NavMode, SceneView, ViewportPanel, apply_nav, nav_action,
};
use crate::workspaces::{LayoutStore, Workspace};

mod files;
mod recovery;

pub use files::{Dialogs, FileKind, IoDialog, NativeDialogs, Unsaved};

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
    /// The Render display mode's frames and their cache (UI.md §6.3, R22.7).
    render_view: crate::render_mode::RenderView,
    /// The object manager's panel state: rename buffer, search text (UI.md §13).
    objects: ObjectManager,
    material_previews: crate::material_manager::MaterialPreviews,
    picture_viewer: crate::picture_viewer::PictureViewer,
    interactive_render: crate::interactive_render::InteractiveRender,
    uv_editor: crate::uv_editor::UvEditor,
    preferences_draft: Option<crate::prefs::UiPrefs>,
    /// The key map editor window (UI.md §14.5).
    keymap_editor: KeyMapEditor,
    /// The command palette window (UI.md §18).
    palette: Palette,
    /// Name field buffer in the attribute panel.
    name_edit: Option<(NodeId, String)>,
    /// The attribute manager tab shown (UI.md §12).
    attr_tabs: crate::attributes::TabSelection,
    /// Counter for drag/typing gestures (each is one undo step).
    gesture_seq: u64,
    gesture: Option<u64>,
    /// The pointer was over the viewport last frame (Space plays only then).
    viewport_hot: bool,
    /// The workspace the arrangement started from, shown in the Workspace menu (UI.md §21).
    workspace: String,
    /// Where saved layouts live; empty by default so tests never touch the user's folder.
    layout_store: LayoutStore,
    /// Bumped on every `set_layout`: egui forgets the remembered panel sizes (UI.md §21.2).
    layout_gen: u32,
    /// The name field of the "Save layout as" window while it is open.
    save_layout_name: Option<String>,
    coord_space: Space,
    coord_third: Third,
    /// Manual apply mode, the typed values waiting for Apply, and what they belong to (UI.md §20).
    coord_manual: bool,
    coord_draft: CoordDraft,
    coord_draft_key: String,
    /// The gizmo drag in progress: drag state, node, its transform at the start, gesture.
    gizmo_drag: Option<(Drag, NodeId, Transform, u64)>,
    /// The gizmo drag on components in progress: drag state, component drag, gesture.
    component_drag: Option<(Drag, ComponentDrag, u64)>,
    /// A drag of spline control points or a handle (VIEWPORT.md section 11.2).
    spline_drag: Option<crate::spline_edit::SplineDrag>,
    /// The axis-mode gizmo drag in progress: drag state, the captured axis, gesture.
    axis_drag: Option<(Drag, AxisEdit, u64)>,
    /// What the snapping of the drag in progress can land on, collected when it began.
    drag_sources: SnapScene,
    /// Where the viewport was drawn last (egui points); tests aim pointers with it.
    view_rect: egui::Rect,
    /// The rectangle select in progress, `[x0, y0, x1, y1]` in render pixels.
    rect_drag: Option<[f64; 4]>,
    /// The interactive drag of the active modeling tool (docs/specs/MODELING.md §0.2).
    tool_drag: Option<crate::tools::ToolDrag>,
    /// File menu (docs/specs/IO.md §2): dialogs, current file, revision at the last
    /// save or load, and the last window title sent.
    dialogs: Box<dyn Dialogs>,
    /// The import/export options window while open (docs/specs/UI.md §27).
    io_dialog: Option<IoDialog>,
    dynamics_panel: crate::dynamics_panel::DynamicsPanel,
    sent_title: String,
    applied_prefs: Option<(ThemeKind, f32)>,
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
        // remembered for the next start; an unwritable folder is not worth a message now
        let _ = self.layout_store.save_last(&self.layout());
    }
}

impl Editor {
    /// The rectangle of the last drawn viewport pane (egui points).
    pub fn view_rect(&self) -> egui::Rect {
        self.view_rect
    }

    /// The key map editor window's state.
    pub fn keymap_editor(&self) -> &KeyMapEditor {
        &self.keymap_editor
    }

    pub fn keymap_editor_mut(&mut self) -> &mut KeyMapEditor {
        &mut self.keymap_editor
    }

    /// The command palette's state.
    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    pub fn palette_mut(&mut self) -> &mut Palette {
        &mut self.palette
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
            render_view: crate::render_mode::RenderView::default(),
            objects: ObjectManager::default(),
            material_previews: crate::material_manager::MaterialPreviews::default(),
            picture_viewer: crate::picture_viewer::PictureViewer::default(),
            interactive_render: crate::interactive_render::InteractiveRender::default(),
            uv_editor: crate::uv_editor::UvEditor::default(),
            preferences_draft: None,
            keymap_editor: KeyMapEditor::default(),
            palette: Palette::default(),
            name_edit: None,
            attr_tabs: crate::attributes::TabSelection::default(),
            gesture_seq: 0,
            gesture: None,
            viewport_hot: false,
            workspace: "Standard".into(),
            layout_store: LayoutStore::default(),
            layout_gen: 0,
            save_layout_name: None,
            coord_space: Space::Local,
            coord_third: Third::Scale,
            coord_manual: false,
            coord_draft: CoordDraft::default(),
            coord_draft_key: String::new(),
            gizmo_drag: None,
            component_drag: None,
            spline_drag: None,
            axis_drag: None,
            drag_sources: SnapScene::default(),
            view_rect: egui::Rect::NOTHING,
            rect_drag: None,
            tool_drag: None,
            dialogs: Box::new(NativeDialogs),
            io_dialog: None,
            dynamics_panel: Default::default(),
            sent_title: String::new(),
            applied_prefs: None,
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
            workspace: self.workspace.clone(),
            hud: (self.state.hud != crate::hud::HudSettings::default()).then_some(self.state.hud),
        }
    }

    /// Switches to `layout`. Area sizes apply to areas that have not been drawn yet.
    pub fn set_layout(&mut self, layout: Layout) {
        self.state.dock = layout.dock;
        self.sizes = layout.sizes;
        self.workspace = layout.workspace;
        if let Some(hud) = layout.hud {
            self.state.hud = hud;
        }
        self.layout_gen += 1;
    }

    /// Replaces where saved layouts are kept (tests use a temporary folder).
    pub fn set_layout_store(&mut self, store: LayoutStore) {
        self.layout_store = store;
    }

    /// The saved layout names.
    pub fn saved_layouts(&self) -> Vec<String> {
        self.layout_store.list()
    }

    /// The workspace name shown in the Workspace menu.
    pub fn workspace(&self) -> &str {
        &self.workspace
    }

    /// Restores the arrangement remembered at the last exit, if any; a broken file gives the
    /// standard layout and a warning in the status bar (UI.md §21.3).
    pub fn restore_last_layout(&mut self) {
        match self.layout_store.load_last() {
            Ok(Some(l)) => {
                self.set_layout(l.layout);
                if let Some(w) = l.warning {
                    self.state.status = w;
                }
            }
            Ok(None) => {}
            Err(e) => self.state.status = e,
        }
    }

    /// View > Theme and View > UI scale (UI.md §24.3); both run commands.
    fn appearance_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Theme", |ui| {
            for kind in ThemeKind::ALL {
                if ui
                    .radio(self.state.prefs.theme == kind, kind.name())
                    .clicked()
                {
                    self.run(("ui.theme", json!({"name": kind.name()})));
                    ui.close();
                }
            }
        });
        ui.menu_button("UI scale", |ui| {
            for percent in [50, 75, 100, 125, 150, 200, 300] {
                let factor = percent as f32 / 100.0;
                if ui
                    .radio(
                        (self.state.prefs.scale - factor).abs() < 1e-3,
                        format!("{percent} %"),
                    )
                    .clicked()
                {
                    self.run(("ui.scale", json!({"factor": factor})));
                    ui.close();
                }
            }
        });
    }

    /// Does one layout request of a command (UI.md §21.4); the error is for the status bar.
    fn layout_request(&mut self, request: UiRequest) -> Result<(), String> {
        match request {
            UiRequest::Workspace(name) => {
                let w = Workspace::from_name(&name).ok_or_else(|| {
                    let all: Vec<&str> = Workspace::ALL.iter().map(|w| w.name()).collect();
                    format!("no workspace named \"{name}\" (use {})", all.join(", "))
                })?;
                self.set_layout(w.layout());
            }
            UiRequest::LayoutSave(name) => {
                self.layout_store.save(&name, &self.layout())?;
            }
            UiRequest::LayoutLoad(name) => {
                let loaded = self.layout_store.load(&name)?;
                self.set_layout(loaded.layout);
                if let Some(w) = loaded.warning {
                    return Err(w);
                }
            }
            UiRequest::LayoutDelete(name) => self.layout_store.delete(&name)?,
            UiRequest::Theme(name) => {
                let kind = ThemeKind::from_name(&name).ok_or_else(|| {
                    let all: Vec<&str> = ThemeKind::ALL.iter().map(|k| k.name()).collect();
                    format!("no theme named \"{name}\" (use {})", all.join(", "))
                })?;
                self.state.prefs.theme = kind;
                self.state.prefs.save()?;
            }
            UiRequest::Scale(factor) => {
                if !crate::prefs::scale_in_range(factor) {
                    return Err("UI scale must be between 0.5 and 3".into());
                }
                self.state.prefs.scale = factor as f32;
                self.state.prefs.save()?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Applies the theme and scale of the preferences when they changed (UI.md §24.3).
    fn apply_prefs(&mut self, ctx: &egui::Context) {
        let defaults = self
            .state
            .prefs
            .use_render_defaults
            .then(|| self.state.prefs.render_defaults.clone());
        self.picture_viewer.set_defaults(defaults.clone());
        self.interactive_render.set_defaults(defaults);
        let wanted = (self.state.prefs.theme, self.state.prefs.scale);
        if self.applied_prefs == Some(wanted) {
            return;
        }
        Theme::of(wanted.0).apply_visuals(ctx);
        if (ctx.zoom_factor() - wanted.1).abs() > f32::EPSILON {
            ctx.set_zoom_factor(wanted.1);
        }
        self.applied_prefs = Some(wanted);
    }

    fn preferences_window(&mut self, ctx: &egui::Context) {
        let Some(draft) = &mut self.preferences_draft else {
            return;
        };
        let mut open = true;
        let mut apply = false;
        let mut cancel = false;
        egui::Window::new("Preferences")
            .open(&mut open)
            .resizable(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(510.0)
                    .show(ui, |ui| draft.form(ui));
                ui.separator();
                ui.horizontal(|ui| {
                    apply = ui.button("Apply preferences").clicked();
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if apply {
            match draft.save_all() {
                Ok(()) => {
                    self.state.autosave.settings.enabled = draft.autosave.enabled;
                    self.state.autosave.settings.interval_seconds = draft.autosave.interval_seconds;
                    self.state.prefs = draft.clone();
                    self.state.status = "Preferences saved".into();
                    self.preferences_draft = None;
                    ctx.request_repaint();
                }
                Err(e) => self.state.status = e,
            }
        } else if cancel || !open {
            self.preferences_draft = None;
        }
    }

    /// Runs a command; an error goes to the status bar. True if it succeeded.
    /// The stored point, edge and polygon selection tags of the active node, as (id, name).
    fn stored_selection_tags(&self) -> Vec<(u64, String)> {
        let scene = self.state.doc.scene();
        let Some(node) = self.state.selection.active.and_then(|a| scene.node(a)) else {
            return Vec::new();
        };
        node.tags
            .iter()
            .filter_map(|t| scene.node(*t))
            .filter(|t| forge_core::tags::SelectionKind::of_tag_type(&t.type_id).is_some())
            .map(|t| (t.id.0, t.name.clone()))
            .collect()
    }

    fn run(&mut self, (id, args): Cmd) -> bool {
        self.run_id(id, args)
    }

    /// Like [`Editor::run`] for a command id that is not a literal (key map bindings).
    fn run_id(&mut self, id: &str, args: Json) -> bool {
        // A command may leave a message of its own (e.g. skipped nodes) in the status.
        self.state.status.clear();
        let noted = args.clone();
        let ok = match execute(&mut self.state, id, args) {
            Ok(_) => true,
            Err(e) => {
                self.state.status = e;
                false
            }
        };
        if ok {
            self.palette.note_run(id, &noted);
        }
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
                UiRequest::CommandPalette => self.palette.open_palette(),
                UiRequest::ImportDialog(format) => self.begin_import(&format),
                UiRequest::ExportDialog { format, selected } => {
                    self.begin_export(&format, selected);
                }
                r @ (UiRequest::Workspace(_)
                | UiRequest::LayoutSave(_)
                | UiRequest::LayoutLoad(_)
                | UiRequest::LayoutDelete(_)
                | UiRequest::Theme(_)
                | UiRequest::Scale(_)) => {
                    if let Err(e) = self.layout_request(r) {
                        self.state.status = e;
                    }
                }
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
        self.picture_viewer.tick(ui.ctx());
        if self
            .state
            .dock
            .area_of(Panel::InteractiveRender)
            .and_then(|area| self.state.dock.active(area))
            != Some(Panel::InteractiveRender)
        {
            self.interactive_render.suspend();
        }
        // RPC jobs run here, before anything is drawn: one command is one undo step.
        if let Some(inbox) = self.rpc.take() {
            if inbox.drain(&mut self.state) > 0 {
                ui.ctx().request_repaint();
            }
            self.rpc = Some(inbox);
        }
        // requests left by commands run outside `run_id` (a script, a test) (UI.md §21.4)
        self.serve_ui_requests();
        self.apply_prefs(ui.ctx());
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
        // the document / layout row and the status bar have the observed heights (UI.md §29.1)
        egui::Panel::top("menu")
            .min_size(crate::layout::WINDOW.document_row)
            .show(ui, |ui| self.menu_bar(ui));
        egui::Panel::bottom("status")
            .exact_size(crate::layout::WINDOW.status_bar)
            .show(ui, |ui| self.status_bar(ui));
        let sizes = self.sizes;
        let salt = self.layout_gen;
        if !self.state.dock.tabs(Area::Top).is_empty() {
            egui::Panel::top("area_top")
                .resizable(false)
                .min_size(crate::layout::WINDOW.command_bar)
                .show(ui, |ui| self.area(ui, Area::Top, rs));
        }
        if !self.state.dock.tabs(Area::Bottom).is_empty() {
            egui::Panel::bottom(("area_bottom", salt))
                .resizable(true)
                .default_size(sizes.bottom_height)
                .show(ui, |ui| self.area(ui, Area::Bottom, rs));
        }
        if !self.state.dock.tabs(Area::Left).is_empty() {
            egui::Panel::left(("area_left", salt))
                .resizable(true)
                .default_size(sizes.left_width)
                .show(ui, |ui| self.area(ui, Area::Left, rs));
        }
        let (top, bottom) = (
            !self.state.dock.tabs(Area::RightTop).is_empty(),
            !self.state.dock.tabs(Area::RightBottom).is_empty(),
        );
        if top || bottom {
            egui::Panel::right(("area_right", salt))
                .resizable(true)
                .default_size(sizes.right_width)
                .show(ui, |ui| {
                    if top && bottom {
                        let h = ui.available_height();
                        egui::Panel::top(("area_right_top", salt))
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
        self.save_layout_window(ui.ctx());
        self.io_dialog_window(ui.ctx());
        self.preferences_window(ui.ctx());
        for cmd in self.dynamics_panel.show(ui.ctx(), &self.state) {
            self.run(cmd);
        }
        let ctx = ui.ctx().clone();
        let registry = self.state.app.commands.clone();
        if let Some((id, args)) = self.palette.show(&ctx, &registry, &self.state.keys.map) {
            self.run_id(&id, args);
        }
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
                let now = ctx.input(|i| i.time);
                self.keymap_editor
                    .captured(chord, &mut self.state.keys, now);
            }
            self.state.keys.armed = None;
            return;
        }
        if ctx.egui_wants_keyboard_input() || self.palette.open {
            self.state.keys.armed = None;
            return;
        }
        let hot = self.viewport_hot;
        let presses = ctx.input_mut(|i| self.state.keys.take_presses(i, hot));
        for (id, args) in presses {
            // Delete over the node editor deletes nodes, not objects (UI.md §17.4).
            if id == "object.delete" && self.state.node_editor.active_canvas().hot {
                self.state.node_editor.active_canvas().delete_requested = true;
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
            .binding_of(id, args)
            .map(|b| b.display(ctx))
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
            FileAction::ImportObj => self.begin_import("obj"),
            FileAction::ExportObj => self.begin_export("obj", false),
            FileAction::ExportSelectedObj => self.begin_export("obj", true),
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
                    if matches!(action, FileAction::Open) {
                        let entries = menu_entries(&self.state.app.browser);
                        ui.add_enabled_ui(!entries.is_empty(), |ui| {
                            ui.menu_button("Open Recent", |ui| {
                                for (i, name) in &entries {
                                    if ui.button(name).clicked() {
                                        ui.close();
                                        self.open_recent(*i);
                                    }
                                }
                            });
                        });
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
                ui.separator();
                if ui.button("Preferences").clicked() {
                    self.preferences_draft = Some(self.state.prefs.clone());
                    ui.close();
                }
                ui.separator();
                self.snapping_menu(ui);
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
                // Stored selections (EDITABLE.md section 7)
                ui.separator();
                if ui.button("Set Selection").clicked() {
                    self.run(cmd("select.store"));
                    ui.close();
                }
                let stored = self.stored_selection_tags();
                ui.add_enabled_ui(!stored.is_empty(), |ui| {
                    ui.menu_button("Restore Selection", |ui| {
                        for (tag, name) in &stored {
                            if ui.button(name).clicked() {
                                self.run(("select.restore", json!({ "tag": tag })));
                                ui.close();
                            }
                        }
                    });
                });
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
                // Array command, Duplicate, Arrange, Randomize, Transfer (MODELING.md §9)
                ui.separator();
                for tool in crate::tools::arrange_tools() {
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
                ui.menu_button("Scene", |ui| {
                    for entry in scene_menu(self.state.doc.scene().registry()) {
                        if ui.button(&entry.label).clicked() {
                            self.run((entry.command, entry.args));
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Character", |ui| {
                    for entry in character_menu(self.state.doc.scene().registry()) {
                        if ui.button(&entry.label).clicked() {
                            self.run((entry.command, entry.args));
                            ui.close();
                        }
                    }
                });
                ui.separator();
                // a long category (Generator) is taller than a small window: scroll it
                let max_height = (ui.ctx().content_rect().height() - 48.0).max(120.0);
                for (cat, items) in create_menu(self.state.doc.scene().registry()) {
                    ui.menu_button(&cat, |ui| {
                        egui::ScrollArea::vertical()
                            .id_salt(("create_menu", &cat))
                            .max_height(max_height)
                            .show(ui, |ui| {
                                for (type_id, label) in items {
                                    if CHARACTER_TYPES.contains(&type_id.as_str()) {
                                        continue;
                                    }
                                    if ui.button(label).clicked() {
                                        self.run(("object.create", json!({"type": type_id})));
                                        ui.close();
                                    }
                                }
                            });
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
            if self
                .state
                .doc
                .scene()
                .registry()
                .get("forge.dynamics_settings")
                .is_some()
            {
                ui.menu_button("Dynamics", |ui| {
                    if ui.button("Dynamics Settings").clicked() {
                        self.dynamics_panel.open(&self.state.settings());
                        ui.close();
                    }
                });
            }
            ui.menu_button("View", |ui| {
                for (shading, label) in SHADINGS {
                    if ui.radio(self.state.shading == shading, label).clicked() {
                        self.run(shading_cmd(shading));
                        ui.close();
                    }
                }
                ui.separator();
                self.view_options_menu(ui);
                self.hud_menu(ui);
                self.appearance_menu(ui);
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
                ui.menu_button("Move panel", |ui| {
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
                });
                ui.separator();
                if ui.button("Command palette\u{2026}").clicked() {
                    self.run(cmd("ui.command_palette"));
                    ui.close();
                }
                if ui.button("Key map\u{2026}").clicked() {
                    self.keymap_editor.open = true;
                    ui.close();
                }
                if ui.button("Reset layout").clicked() {
                    self.set_layout(Layout::standard());
                    ui.close();
                }
            });
            self.workspace_menu(ui);
        });
    }

    /// The workspace switcher (UI.md §21.5): every entry runs a registry command.
    fn workspace_menu(&mut self, ui: &mut Ui) {
        ui.menu_button("Workspace", |ui| {
            for w in Workspace::ALL {
                let on = self.workspace.eq_ignore_ascii_case(w.name());
                if ui.selectable_label(on, w.name()).clicked() {
                    self.run(("ui.workspace", json!({"name": w.name()})));
                    ui.close();
                }
            }
            let saved = self.layout_store.list();
            if !saved.is_empty() {
                ui.separator();
                for name in &saved {
                    let on = self.workspace == *name;
                    if ui.selectable_label(on, name).clicked() {
                        self.run(("ui.layout_load", json!({"name": name})));
                        ui.close();
                    }
                }
            }
            ui.separator();
            if ui.button("Save layout as\u{2026}").clicked() {
                self.save_layout_name = Some(String::new());
                ui.close();
            }
            if !saved.is_empty() {
                ui.menu_button("Delete layout", |ui| {
                    for name in &saved {
                        if ui.button(name).clicked() {
                            self.run(("ui.layout_delete", json!({"name": name})));
                            ui.close();
                        }
                    }
                });
            }
            if ui.button("Reset layout").clicked() {
                self.run(("ui.workspace", json!({"name": "Standard"})));
                ui.close();
            }
        });
    }

    /// The small "Save layout as" window.
    fn save_layout_window(&mut self, ctx: &egui::Context) {
        let Some(mut name) = self.save_layout_name.take() else {
            return;
        };
        let (mut save, mut cancel) = (false, false);
        egui::Window::new("Save layout as")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                let label = ui.label("Name");
                let r = ui.text_edit_singleline(&mut name).labelled_by(label.id);
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.horizontal(|ui| {
                    save = ui.button("Save").clicked() || enter;
                    cancel = ui.button("Cancel").clicked();
                });
            });
        if save {
            self.run(("ui.layout_save", json!({"name": name})));
        } else if !cancel {
            self.save_layout_name = Some(name);
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let now = ui.input(|i| i.time);
            if let Some(prefix) = self.state.keys.pending_prefix(now) {
                // a prefix key waits for its second key (UI.md §14.7)
                ui.strong(format!("{} \u{2026}", prefix.display(ui.ctx())));
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs_f64(
                        crate::keymap::SEQUENCE_TIMEOUT + 0.05,
                    ));
            }
            if self.state.status.is_empty() {
                // the active tool's help text (observed, G10 A8)
                ui.weak(crate::tools::help_text(
                    self.state.gizmo_mode,
                    forge_command::CommandContext::edit_mode(&self.state),
                ));
            } else {
                ui.colored_label(Theme::current(ui.ctx()).error, &self.state.status);
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
            Panel::Console => crate::console::console_ui(ui, &mut self.state),
            Panel::Layers => {
                for c in crate::layer_manager::layer_panel_ui(ui, &mut self.state) {
                    self.run(c);
                }
            }
            Panel::Materials => {
                for c in crate::material_manager::material_manager_ui(
                    ui,
                    &mut self.state,
                    &mut self.material_previews,
                ) {
                    self.run(c);
                }
            }
            Panel::PictureViewer => {
                self.picture_viewer
                    .show(ui, self.state.doc.scene(), self.state.timeline.current)
            }
            Panel::InteractiveRender => self.interactive_render.show(
                ui,
                self.state.doc.scene(),
                self.state.timeline.current,
            ),
            Panel::UvEditor => {
                for cmd in self.uv_editor.show(ui, &self.state) {
                    self.run(cmd);
                }
            }
            Panel::WeightManager => {
                for c in crate::weight_manager::weight_manager_ui(ui, &mut self.state) {
                    self.run(c);
                }
            }
            Panel::DopeSheet => {
                for c in crate::dopesheet::dope_sheet_ui(ui, &mut self.state) {
                    self.run(c);
                }
            }
            Panel::ValueGraph => {
                for c in crate::fcurve::value_graph_ui(ui, &mut self.state) {
                    self.run(c);
                }
            }
            Panel::ContentBrowser => {
                for a in crate::content_browser::content_browser_ui(ui, &mut self.state) {
                    match a {
                        BrowserAction::Run(c) => {
                            self.run(c);
                        }
                        BrowserAction::OpenRecent(i) => self.open_recent(i),
                    }
                }
            }
            Panel::History => {
                if let Some(position) = crate::history_panel::history_panel_ui(ui, &self.state.doc)
                {
                    self.run_id("edit.history_jump", json!({"position": position}));
                }
            }
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
                if let Some(c) = self.button_group(ui, group, crate::theme::BUTTONS.command) {
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
                    if let Some(c) = self.button_group(ui, &group, crate::theme::BUTTONS.palette) {
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
    fn button_group(&self, ui: &mut Ui, group: &ButtonGroup, min: [f32; 2]) -> Option<Cmd> {
        let mut clicked = None;
        for b in &group.buttons {
            let selected = b.command == "select.mode"
                && b.args["mode"] == json!(forge_command::ModeArg::from(self.state.mode))
                || (b.command == "view.shading"
                    && b.args["mode"]
                        == json!(crate::commands::shading_mode_of(self.state.shading)));
            let response = ui
                .add(
                    egui::Button::new(b.text())
                        .selected(selected)
                        .min_size(egui::vec2(min[0], min[1])),
                )
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

    /// Edit menu: snapping and axis mode (docs/specs/MANIPULATE.md §6, §7.4). Each entry runs
    /// its registry command.
    fn snapping_menu(&mut self, ui: &mut Ui) {
        let t = self.state.app.transform_settings.clone();
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
        item(ui, self, t.axis_mode, "Axis mode", cmd("edit.axis_mode"));
        ui.separator();
        item(ui, self, t.snap.enabled, "Snapping", cmd("snap.toggle"));
        ui.menu_button("Snap targets", |ui| {
            for kind in forge_modeling::snap::SnapKind::ALL {
                let on = t.snap.targets.get(kind);
                if ui
                    .add(egui::Button::new(kind.label()).selected(on))
                    .clicked()
                {
                    runs.push(("snap.set", json!({"targets": {kind.id(): !on}})));
                    ui.close();
                }
            }
        });
        let two_d = t.snap.mode == forge_modeling::snap::SnapMode::D2;
        let mode = if two_d { "3d" } else { "2d" };
        if ui
            .add(egui::Button::new("2D snapping").selected(two_d))
            .clicked()
        {
            runs.push(("snap.set", json!({"mode": mode})));
            ui.close();
        }
        let q = t.snap.quantize.enabled;
        if ui.add(egui::Button::new("Quantize").selected(q)).clicked() {
            runs.push(("snap.set", json!({"quantize": {"enabled": !q}})));
            ui.close();
        }
        for command in runs {
            self.run(command);
        }
    }

    /// View menu: layout, maximise, lighting, shadows, anti-aliasing (docs/specs/UI.md §6.2).
    /// Each entry runs its registry command.
    /// View > HUD: one sub menu per pane with a check box per item (UI.md §23.1).
    fn hud_menu(&mut self, ui: &mut Ui) {
        let mut runs: Vec<Cmd> = Vec::new();
        ui.menu_button("HUD", |ui| {
            for pane in Pane::ALL {
                ui.menu_button(pane.label(), |ui| {
                    for item in crate::hud::HudItem::ALL {
                        let on = self.state.hud.flags(pane).get(item);
                        if ui.selectable_label(on, item.label()).clicked() {
                            runs.push((
                                "view.hud",
                                json!({"item": item.id(), "pane": pane.id(), "enabled": !on}),
                            ));
                        }
                    }
                });
            }
        });
        for c in runs {
            self.run(c);
        }
    }

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
        let mut preview = v.field_preview;
        if ui.checkbox(&mut preview, "Field Preview").changed() {
            self.state.views.field_preview = preview;
        }
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
        ui.separator();
        item(
            ui,
            self,
            v.motion.paths,
            "Motion Paths",
            ("view.motion_paths", json!({"enabled": !v.motion.paths})),
        );
        item(
            ui,
            self,
            v.motion.onion,
            "Onion Skin",
            ("view.onion_skin", json!({"enabled": !v.motion.onion})),
        );
        for c in runs {
            self.run(c);
        }
    }

    fn viewport_ui(&mut self, ui: &mut Ui, rs: Option<&egui_wgpu::RenderState>) {
        // header: the soft-selection bar of the component modes and the render toggle. The
        // modes themselves are switched in the tool palette only (UI.md §6.1, owner note N6).
        ui.horizontal(|ui| {
            if self.state.mode != EditMode::Model {
                self.component_bar(ui);
                ui.separator();
            }
            // what the panes draw: the shaded scene or the real-time tier (UI.md §6.3).
            // UI state only: no undo entry, no document change.
            let render = self.state.display == DisplayMode::Render;
            if ui
                .add(egui::Button::new("Render").selected(render))
                .clicked()
            {
                self.state.display = if render {
                    DisplayMode::Shaded
                } else {
                    DisplayMode::Render
                };
            }
            ui.add_enabled_ui(render, |ui| {
                ui.menu_button("Post", |ui| {
                    for (label, on) in [
                        ("SSAO", &mut self.state.post.ssao),
                        ("SSR", &mut self.state.post.ssr),
                        ("Bloom", &mut self.state.post.bloom),
                        ("TAA", &mut self.state.post.taa),
                        ("Tone map", &mut self.state.post.tonemap),
                    ] {
                        ui.checkbox(on, label);
                    }
                });
            });
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
        // the frame time of the editor, once per frame (UI.md §23.5)
        let dt = f64::from(ui.input(|i| i.unstable_dt));
        self.state.hud_timer.push(dt);
        self.state.hud_last.clear();
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
                    egui::Stroke::new(1.5, Theme::current(ui.ctx()).component_selection),
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
        // the digits are hold keys only over the viewport, or while a drag started there runs
        // (UI.md §6.1, owner note N1)
        let hold_ok = !typing && (over || response.dragged());
        let input = ui.ctx().input(|i| NavInput {
            primary: i.pointer.primary_down(),
            middle: i.pointer.middle_down(),
            secondary: i.pointer.secondary_down(),
            alt: i.modifiers.alt,
            held: NavMode::KEYS
                .iter()
                .find(|(k, _)| {
                    hold_ok && i.key_down(*k) && !i.modifiers.command && !i.modifiers.alt
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
        if pane == self.state.views.active {
            self.view_rect = rect;
        }
        let ppp = ui.ctx().pixels_per_point();
        let size = [
            (rect.width() * ppp).round() as u32,
            (rect.height() * ppp).round() as u32,
        ];
        use crate::material_manager::{MaterialDrag, assign_to};
        if egui::DragAndDrop::has_payload_of_type::<MaterialDrag>(ui.ctx()) {
            if let Some(material) = response.dnd_release_payload::<MaterialDrag>()
                && let Some(pos) = ui.input(|i| i.pointer.latest_pos())
            {
                let frame = self.state.app.scheduled_draw(self.state.timeline.current);
                let local = (pos - rect.min) * ppp;
                if let Some(node) = crate::manipulate::pick_node(
                    &frame.scene,
                    &self.state.pane_camera(pane),
                    f64::from(local.x),
                    f64::from(local.y),
                    f64::from(size[0].max(1)),
                    f64::from(size[1].max(1)),
                ) {
                    self.run(assign_to(*material, node));
                }
            }
        } else if input.navigating() {
            self.rect_drag = None;
        } else {
            self.viewport_interaction(ui, &response, rect, ppp, size);
        }
        let Some(rs) = rs else {
            // not selectable: a selectable label would take the pointer from the navigation
            ui.put(
                rect,
                egui::Label::new("Viewport unavailable").selectable(false),
            );
            let frame = self.state.app.scheduled_draw(self.state.timeline.current);
            self.paint_hud(ui, pane, rect, &frame);
            return;
        };
        if self.state.display == DisplayMode::Render {
            self.paint_render_mode(ui, pane, rect, ppp);
            return;
        }
        let gizmo = self.current_gizmo();
        // the scheduler's pass: only what changed since the last frame is evaluated and
        // rebuilt (CORE.md section 5b); a live tool preview replaces the target's mesh in
        // the drawn scene only, which the scheduler does not know, so it draws from scratch
        let current = self.state.timeline.current;
        let scheduled = self.state.app.scheduled_draw(current);
        let previewed = crate::tools::with_preview(&self.state, &scheduled.scene);
        let shown = previewed.clone().unwrap_or_else(|| scheduled.scene.clone());
        let lights = crate::panes::scene_lights(&shown);
        let (motion, motion_pending) = self.state.motion.draw(
            self.state.app.doc.scene(),
            &shown,
            &self.state.app.selection.nodes,
            &self.state.views.motion,
            current,
        );
        if motion_pending {
            ui.ctx().request_repaint();
        }
        let eval_ctx = forge_core::EvalCtx::at_frame(
            self.state.timeline.current,
            f64::from(shown.settings().fps),
        );
        let selected_now = self.state.selection.nodes.clone();
        let (overlay, volumes_pending) =
            self.state.volumes.overlay(&shown, &selected_now, &eval_ctx);
        if volumes_pending {
            ui.ctx().request_repaint();
        }
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
                frame: previewed.is_none().then_some(&*scheduled),
                splines: &crate::spline_edit::editor_draws(&shown, &self.state),
                motion: &motion,
                overlay: &overlay,
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
            let colour = Theme::current(ui.ctx()).component_selection;
            ui.painter().rect_stroke(
                egui::Rect::from_two_pos(to_points(x0, y0), to_points(x1, y1)),
                0.0,
                egui::Stroke::new(1.0, colour),
                egui::StrokeKind::Inside,
            );
        }
        self.paint_clone_overlay(ui, &shown, pane, rect);
        self.paint_field_overlay(ui, &shown, pane, rect);
        if pane == self.state.views.active {
            self.paint_snap_marker(ui, rect, ppp, size);
        }
        let label = SHADINGS
            .iter()
            .find(|(s, _)| *s == self.state.pane_shading(pane))
            .map_or("", |(_, l)| l);
        ui.painter().text(
            rect.left_top() + egui::vec2(8.0, 6.0),
            egui::Align2::LEFT_TOP,
            {
                let axis = if self.state.app.transform_settings.axis_mode {
                    " · Axis mode"
                } else {
                    ""
                };
                match self.state.mode {
                    EditMode::Model => format!("{} · {label}{axis}", pane.label()),
                    mode => format!("{} · {label} · {} mode{axis}", pane.label(), mode.label()),
                }
            },
            egui::FontId::proportional(12.0),
            Theme::current(ui.ctx()).weak_text,
        );
        self.paint_hud(ui, pane, rect, &scheduled);
        // cutting tools: the knife line being dragged and the heads-up line (MODELING.md §5.5)
        if let Some(hud) =
            crate::cutting::hud(&self.state).or_else(|| crate::building::hud(&self.state))
        {
            ui.painter().text(
                rect.left_bottom() + egui::vec2(8.0, -6.0),
                egui::Align2::LEFT_BOTTOM,
                hud,
                egui::FontId::proportional(12.0),
                Theme::current(ui.ctx()).component_selection,
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
                egui::Stroke::new(1.5, Theme::current(ui.ctx()).component_selection),
            );
        }
        // Polygon Pen: the corners placed so far (MODELING.md §6.7)
        let size = rect.size() * ppp;
        for [x, y] in crate::building::pen_corner_pixels(&self.state, size.x as f64, size.y as f64)
        {
            ui.painter().circle_filled(
                rect.min + egui::vec2((x / ppp as f64) as f32, (y / ppp as f64) as f32),
                4.0,
                Theme::current(ui.ctx()).component_selection,
            );
        }
    }

    /// The Render display mode: the real-time tier's frame of this pane's camera, without the
    /// overlays (UI.md §6.3, RENDER.md R22.7). A missing device keeps a note instead of the
    /// shaded image, since the shaded image needs the same device.
    fn paint_render_mode(&mut self, ui: &mut Ui, pane: Pane, rect: egui::Rect, ppp: f32) {
        let size = [
            ((rect.width() * ppp).round() as u32).clamp(1, 640),
            ((rect.height() * ppp).round() as u32).clamp(1, 400),
        ];
        let current = self.state.timeline.current;
        let shown = self.state.app.scheduled_draw(current).scene.clone();
        let camera = self.state.pane_camera(pane);
        let post = self.state.post;
        match self
            .render_view
            .image(ui.ctx(), &shown, &camera, current, size, post)
        {
            Ok(texture) => {
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter()
                    .image(texture.id(), rect, uv, egui::Color32::WHITE);
                ui.painter().text(
                    rect.left_top() + egui::vec2(8.0, 6.0),
                    egui::Align2::LEFT_TOP,
                    format!("{} · Render", pane.label()),
                    egui::FontId::proportional(12.0),
                    Theme::current(ui.ctx()).weak_text,
                );
            }
            Err(_) => {
                ui.put(
                    rect,
                    egui::Label::new("Render unavailable").selectable(false),
                );
            }
        }
    }

    /// The HUD items of `pane` over its image (UI.md §23.6).
    fn paint_hud(
        &mut self,
        ui: &Ui,
        pane: Pane,
        rect: egui::Rect,
        frame: &forge_app::schedule::Frame,
    ) {
        use crate::hud::{Anchor, HudPrim, Tone};
        let flags = *self.state.hud.flags(pane);
        if !flags.any() {
            return;
        }
        let ctx = crate::hud::context(&self.state, pane, frame, &self.state.hud_timer);
        let list = crate::hud::draw_list(&ctx, &flags, [rect.width(), rect.height()]);
        let theme = Theme::current(ui.ctx());
        let painter = ui.painter().with_clip_rect(rect);
        let tone = |t: Tone| match t {
            Tone::Normal => theme.text,
            Tone::Weak => theme.weak_text,
            Tone::Guide => theme.accent,
        };
        let at = |p: [f32; 2]| rect.min + egui::vec2(p[0], p[1]);
        for prim in &list {
            match prim {
                HudPrim::Text {
                    pos,
                    anchor,
                    text,
                    tone: t,
                } => {
                    let align = match anchor {
                        Anchor::LeftTop => egui::Align2::LEFT_TOP,
                        Anchor::RightTop => egui::Align2::RIGHT_TOP,
                        Anchor::RightBottom => egui::Align2::RIGHT_BOTTOM,
                    };
                    painter.text(
                        at(*pos),
                        align,
                        text,
                        egui::FontId::monospace(11.0),
                        tone(*t),
                    );
                }
                HudPrim::Rect { min, max, tone: t } => {
                    painter.rect_stroke(
                        egui::Rect::from_min_max(at(*min), at(*max)),
                        0.0,
                        egui::Stroke::new(1.0, tone(*t)),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        self.state.hud_last.push((pane, list));
    }

    /// The snap target under the pointer: a ring at its projection and its kind (MANIPULATE.md §6.7).
    fn paint_snap_marker(&self, ui: &Ui, rect: egui::Rect, ppp: f32, size: [u32; 2]) {
        let Some(hit) = self.state.snap_hit else {
            return;
        };
        let (w, h) = (size[0].max(1) as f64, size[1].max(1) as f64);
        let Some(p) = crate::snapping::pixel_of(&self.state.camera, hit.position, w, h) else {
            return;
        };
        let at = rect.min + egui::vec2((p.0 / ppp as f64) as f32, (p.1 / ppp as f64) as f32);
        let colour = Theme::current(ui.ctx()).component_selection;
        ui.painter()
            .circle_stroke(at, 6.0, egui::Stroke::new(2.0, colour));
        ui.painter().text(
            at + egui::vec2(10.0, -10.0),
            egui::Align2::LEFT_BOTTOM,
            hit.kind.label(),
            egui::FontId::proportional(11.0),
            colour,
        );
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
                    Theme::current(ui.ctx()).weak_text,
                );
            }
        }
    }

    /// Gizmos of the selected fields, and with View > Field Preview the field value on the
    /// points of the selected meshes (UI.md §16.6).
    fn paint_field_overlay(
        &self,
        ui: &Ui,
        scene: &forge_core::Scene,
        pane: Pane,
        rect: egui::Rect,
    ) {
        use crate::field_overlay as fo;
        let selected = &self.state.selection.nodes;
        let fields = fo::shown_fields(scene, selected);
        let preview = self.state.views.field_preview;
        if fields.is_empty() && !preview {
            return;
        }
        let camera = self.state.pane_camera(pane);
        let size = [f64::from(rect.width()), f64::from(rect.height())];
        let painter = ui.painter().with_clip_rect(rect);
        let at = |p: [f32; 2]| rect.min + egui::vec2(p[0], p[1]);
        let colour = Theme::current(ui.ctx()).component_selection;
        for id in fields {
            for strip in fo::field_gizmo(scene, id) {
                for run in fo::project_strip(&strip, &camera, size) {
                    let pts: Vec<egui::Pos2> = run.into_iter().map(at).collect();
                    painter.add(egui::Shape::line(pts, egui::Stroke::new(1.5, colour)));
                }
            }
        }
        if !preview {
            return;
        }
        let frame = self.state.timeline.current;
        let ctx = forge_core::EvalCtx::at_frame(frame, f64::from(scene.settings().fps));
        let mut points = Vec::new();
        let mut meshes: Vec<NodeId> = selected.iter().copied().collect();
        meshes.extend(selected.iter().filter_map(|id| scene.node(*id)?.parent));
        meshes.dedup();
        for id in meshes {
            if let Ok(m) = self.state.app.eval_mesh(id, frame) {
                points.extend(m.points);
            }
        }
        for dot in fo::selected_preview(scene, selected, &points, &ctx, &camera, size) {
            let [r, g, b] = fo::value_colour(dot.value);
            painter.circle_filled(at(dot.at), 2.5, egui::Color32::from_rgb(r, g, b));
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
        let origin = if component_target(&self.state).is_some()
            && !self.state.app.transform_settings.axis_mode
        {
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
        // Escape over the viewport discards the keyframe drafts (UI.md §12, owner note N10)
        if response.hovered()
            && !self.state.app.drafts().is_empty()
            && ui
                .ctx()
                .input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            self.state.app.clear_drafts();
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
        // an editable spline in Points mode: its control points and handles take the pointer
        // (VIEWPORT.md section 11.2)
        if crate::spline_edit::edit_target(&self.state.app).is_some() {
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.gesture_seq += 1;
                self.spline_drag = crate::spline_edit::SplineDrag::begin(
                    &mut self.state,
                    [x, y],
                    [w, h],
                    mods,
                    self.gesture_seq,
                );
            }
            if let Some(drag) = &mut self.spline_drag {
                if response.dragged_by(egui::PointerButton::Primary) {
                    self.state.status =
                        match drag.update(&mut self.state, [x, y], [w, h], mods.command) {
                            Ok(()) => String::new(),
                            Err(e) => e,
                        };
                }
                if response.drag_stopped() {
                    self.spline_drag = None;
                }
                return;
            }
            if response.clicked() {
                crate::spline_edit::click(&mut self.state, [x, y], [w, h], mods);
            }
            return;
        }
        let cam = self.state.camera;
        let snap = self.state.app.transform_settings.snap.clone();
        let axis_mode = self.state.app.transform_settings.axis_mode;
        let mode = self.state.gizmo_mode;
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(g) = self.current_gizmo()
            && let Some(handle) = g.hit_test(&cam, x, y, w, h)
            && let Some(drag) = Drag::begin(&g, handle, &cam, x, y, w, h)
        {
            if axis_mode {
                // axis mode: the drag edits the axis of the active object (MANIPULATE.md §7.4)
                if let Some(id) = self.state.selection.active
                    && !forge_core::layers::locked(self.state.doc.scene(), id)
                    && let Ok(edit) = AxisEdit::capture(self.state.doc.scene(), id)
                {
                    self.gesture_seq += 1;
                    self.drag_sources = if snap.enabled {
                        collect_sources(&self.state, &[], None)
                    } else {
                        SnapScene::default()
                    };
                    self.axis_drag = Some((drag, edit, self.gesture_seq));
                }
            } else if component_target(&self.state).is_some() {
                // a gizmo drag in a component mode edits the components (EDITABLE.md §3.5)
                if let Some(cd) = ComponentDrag::begin(&self.state) {
                    self.gesture_seq += 1;
                    self.drag_sources = if snap.enabled {
                        collect_sources_multi(&self.state, &[], &cd.moved())
                    } else {
                        SnapScene::default()
                    };
                    self.component_drag = Some((drag, cd, self.gesture_seq));
                }
            } else if let Some(id) = self.state.selection.active
                && !forge_core::layers::locked(self.state.doc.scene(), id)
                && let Some(start) = self.state.evaluated().local_transform(id)
            {
                self.gesture_seq += 1;
                self.drag_sources = if snap.enabled {
                    collect_sources(&self.state, &[id], None)
                } else {
                    SnapScene::default()
                };
                self.gizmo_drag = Some((drag, id, start, self.gesture_seq));
            }
        }
        let ctrl = modifiers.command;
        if let Some((drag, edit, n)) = &mut self.axis_drag {
            if response.dragged_by(egui::PointerButton::Primary) {
                let (delta, hit) = drag_step(
                    &snap,
                    &self.drag_sources.sources,
                    drag,
                    &cam,
                    (x, y),
                    (w, h),
                    mode,
                    ctrl,
                );
                self.state.snap_hit = hit;
                self.state.status =
                    match apply_axis_drag(&mut self.state.doc, edit, delta, Gesture::Drag(*n)) {
                        Ok(()) => String::new(),
                        Err(e) => e.to_string(),
                    };
            }
            if response.drag_stopped() {
                self.axis_drag = None;
                self.state.snap_hit = None;
            }
            return;
        }
        if let Some((drag, cd, n)) = &mut self.component_drag {
            if response.dragged_by(egui::PointerButton::Primary) {
                let (delta, hit) = drag_step(
                    &snap,
                    &self.drag_sources.sources,
                    drag,
                    &cam,
                    (x, y),
                    (w, h),
                    mode,
                    ctrl,
                );
                self.state.snap_hit = hit;
                self.state.status = match cd.apply(&mut self.state.doc, delta, Gesture::Drag(*n)) {
                    Ok(()) => String::new(),
                    Err(e) => e.to_string(),
                };
            }
            if response.drag_stopped() {
                self.component_drag = None;
                self.state.snap_hit = None;
            }
            return;
        }
        if let Some((drag, id, start, n)) = &mut self.gizmo_drag {
            if response.dragged_by(egui::PointerButton::Primary) {
                let (delta, hit) = drag_step(
                    &snap,
                    &self.drag_sources.sources,
                    drag,
                    &cam,
                    (x, y),
                    (w, h),
                    mode,
                    ctrl,
                );
                self.state.snap_hit = hit;
                let (id, start, n) = (*id, *start, *n);
                self.state.status =
                    match apply_drag_edit(&mut self.state.app, id, &start, delta, Gesture::Drag(n))
                    {
                        Ok(()) => String::new(),
                        Err(e) => e.to_string(),
                    };
            }
            if response.drag_stopped() {
                self.gizmo_drag = None;
                self.state.snap_hit = None;
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
        if self.state.weight_panel.painting
            && crate::weight_manager::paint_target(&self.state).is_some()
        {
            if (response.clicked() || response.dragged_by(egui::PointerButton::Primary))
                && let Err(e) = crate::weight_manager::paint_dab(&mut self.state, [x, y], [w, h])
            {
                self.state.status = e;
            }
            if response.drag_stopped() || response.clicked() {
                crate::weight_manager::end_stroke(&mut self.state);
            }
            return;
        }
        if response.clicked()
            && let Err(e) = viewport_click(&mut self.state, [x, y], [w, h], mods)
        {
            self.state.status = e;
        }
    }

    fn coordinates_ui(&mut self, ui: &mut Ui) {
        let Some(active) = self.state.selection.active else {
            ui.weak("Nothing selected");
            return;
        };
        // component rows replace the object rows while components are selected (UI.md §20.3)
        let comp = if component_target(&self.state).is_some() {
            component_coords(&self.state, self.coord_space == Space::World)
        } else {
            None
        };
        let mut ids = vec![active];
        ids.extend(
            self.state
                .selection
                .nodes
                .iter()
                .copied()
                .filter(|n| *n != active),
        );
        // a draft belongs to one selection, space and row set
        let key = format!(
            "{ids:?}|{:?}|{:?}|{}",
            self.coord_space,
            self.coord_third,
            comp.is_some()
        );
        if key != self.coord_draft_key {
            self.coord_draft_key = key;
            self.coord_draft.clear();
        }
        ui.horizontal(|ui| {
            ui.radio_value(&mut self.coord_space, Space::Local, "Local");
            ui.radio_value(&mut self.coord_space, Space::World, "World");
            if comp.is_none() {
                ui.separator();
                ui.radio_value(&mut self.coord_third, Third::Scale, "Scale");
                ui.radio_value(&mut self.coord_third, Third::Size, "Size");
            }
            ui.separator();
            ui.checkbox(&mut self.coord_manual, "Manual");
            let pending = self.coord_manual && !self.coord_draft.is_empty();
            if ui
                .add_enabled(pending, egui::Button::new("Apply"))
                .clicked()
            {
                self.coord_apply(comp.is_some());
            }
            if ui
                .add_enabled(pending, egui::Button::new("Reset"))
                .clicked()
            {
                self.coord_draft.clear();
            }
            ui.separator();
            ui.weak(format!(
                "Units: {}",
                self.state.prefs.distance.suffix().trim()
            ));
        });
        let eval_scene = self.state.evaluated();
        let (labels, values, mixed, degrees): (_, [Option<DVec3>; 3], Mixed, _) = match comp {
            Some(c) => (
                ["Pivot", "Rotation", "Size"],
                [Some(c.pivot), Some(DVec3::ZERO), Some(c.size)],
                [[false; 3]; 3],
                [false, true, false],
            ),
            None => {
                let Some((view, mixed)) =
                    coord_view_multi(&eval_scene, &ids, self.coord_space, self.coord_third)
                else {
                    return;
                };
                (
                    [
                        "Position",
                        "Rotation",
                        match self.coord_third {
                            Third::Scale => "Scale",
                            Third::Size => "Size",
                        },
                    ],
                    [Some(view.pos), Some(view.rot), view.third],
                    mixed,
                    [false, true, false],
                )
            }
        };
        let axes = [["X", "Y", "Z"], ["H", "P", "B"], ["X", "Y", "Z"]];
        let distance = self.state.prefs.distance;
        let distance_row = [
            true,
            false,
            comp.is_some() || self.coord_third == Third::Size,
        ];
        let mut edit: Option<(usize, usize, f64, Gesture)> = None;
        egui::Grid::new(("coords", active))
            .num_columns(4)
            .show(ui, |ui| {
                for row in 0..3 {
                    let l = ui.label(labels[row]);
                    let Some(value) = values[row] else {
                        ui.weak("no mesh");
                        ui.end_row();
                        continue;
                    };
                    for k in 0..3 {
                        let draft = self.coord_draft.value(row, k);
                        let base = if degrees[row] {
                            value[k].to_degrees()
                        } else {
                            value[k]
                        };
                        let canonical = draft.unwrap_or(base);
                        let mut shown = if distance_row[row] {
                            distance.to_display(canonical)
                        } else {
                            canonical
                        };
                        let is_mixed = mixed[row][k] && draft.is_none();
                        let mut field = egui::DragValue::new(&mut shown)
                            .speed(0.1)
                            .prefix(format!("{} ", axes[row][k]));
                        if distance_row[row] {
                            field = field.speed(distance.to_display(0.1));
                        }
                        if is_mixed {
                            field = field.custom_formatter(|_, _| "mixed".into());
                        }
                        let r = ui.add(field).labelled_by(l.id);
                        if r.changed() {
                            let g = if r.dragged() {
                                self.gesture_for(r.drag_started())
                            } else {
                                Gesture::Commit
                            };
                            let value = if distance_row[row] {
                                distance.to_centimetres(shown)
                            } else {
                                shown
                            };
                            edit = Some((row, k, value, g));
                        }
                    }
                    ui.end_row();
                }
            });
        if let Some((row, k, v, g)) = edit {
            if self.coord_manual {
                self.coord_draft.set(row, k, v);
            } else {
                let mut one = CoordDraft::default();
                one.set(row, k, v);
                let gesture = match g {
                    Gesture::Commit => None,
                    Gesture::Drag(n) => Some(n),
                };
                self.coord_run(&one, comp.is_some(), gesture);
            }
        }
    }

    /// Writes the Manual-mode draft in one step and drops it.
    fn coord_apply(&mut self, component: bool) {
        let draft = std::mem::take(&mut self.coord_draft);
        self.coord_run(&draft, component, None);
    }

    /// Runs the registry command for a draft: `mesh.transform` for components, else
    /// `transform.edit` (UI.md §20.3).
    fn coord_run(&mut self, draft: &CoordDraft, component: bool, gesture: Option<u64>) {
        let world = self.coord_space == Space::World;
        if !component && self.coord_run_drafts(draft, gesture) {
            return;
        }
        let cmd = if component {
            component_transform_args(&self.state, draft, world, gesture)
                .map(|a| ("mesh.transform", a))
        } else {
            draft
                .transform_args(self.coord_space, self.coord_third, gesture)
                .map(|a| ("transform.edit", a))
        };
        if let Some((id, args)) = cmd {
            self.state.status = execute(&mut self.state, id, args).err().unwrap_or_default();
        }
    }

    /// The Coordinates edit of animated objects with Auto-Key off: the same transforms
    /// `transform.edit` would write, staged as drafts (UI.md §12, owner note N10). Returns
    /// false when no selected object has a track, so the command runs as before.
    fn coord_run_drafts(&mut self, draft: &CoordDraft, gesture: Option<u64>) -> bool {
        let ids: Vec<NodeId> = self.state.selection.nodes.iter().copied().collect();
        if !edit_drafts(&self.state.app, &ids) {
            return false;
        }
        let Some(args) = draft.transform_args(self.coord_space, self.coord_third, gesture) else {
            return true;
        };
        let eval = self.state.evaluated();
        let merge = gesture.map(|g| {
            let nodes: Vec<String> = ids.iter().map(ToString::to_string).collect();
            format!("coords:{}:{g}", nodes.join(","))
        });
        self.state.status = match forge_app::coords::edit_results(&eval, &ids, &args)
            .map_err(|e| e.to_string())
            .and_then(|edits| {
                apply_transform_edits(
                    &mut self.state.app,
                    &edits,
                    "Set Coordinates",
                    merge.as_deref(),
                )
                .map_err(|e| e.to_string())
            }) {
            Ok(()) => String::new(),
            Err(e) => e,
        };
        true
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
        let note = crate::tools::scope_note(&self.state);
        let rows = crate::attributes::tool_rows(ts.tool().settings(), ts.values());
        ui.heading(label);
        if let Some(note) = note {
            ui.weak(note);
        }
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
        let can_draft = !self.state.autokey;
        let theme = Theme::current(ui.ctx());
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
        self.attr_tabs.normalise(&tabs);
        if tabs.len() > 1 {
            ui.horizontal(|ui| {
                let shown = self.attr_tabs.shown(&tabs);
                for t in &tabs {
                    let r = ui.selectable_label(shown.contains(t), t);
                    if r.clicked() {
                        // Shift-click stacks a second tab body under the first (G1 item 3)
                        let shift = ui.input(|i| i.modifiers.shift);
                        self.attr_tabs.click(t, shift, &tabs);
                    }
                }
            });
        }
        ui.separator();
        let shown_tabs = self.attr_tabs.shown(&tabs);
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.add_enabled_ui(view.editable, |ui| {
                    for (n, tab) in shown_tabs.iter().enumerate() {
                        if n > 0 {
                            // stacked bodies are told apart by a line
                            ui.separator();
                        }
                        for (group, rows) in view.groups_of(tab) {
                            egui::CollapsingHeader::new(group)
                                .id_salt(("attr_group", tab, group))
                                .default_open(true)
                                .show(ui, |ui| {
                                    egui::Grid::new(("attrs", tab, group))
                                        .num_columns(2)
                                        .spacing([12.0, 4.0])
                                        .show(ui, |ui| {
                                            for row in rows {
                                                self.attribute_row(ui, &shown, id, row, can_draft);
                                                ui.end_row();
                                            }
                                        });
                                });
                        }
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
        can_draft: bool,
    ) {
        use crate::key_indicator::{RowKeys, draft_value};
        use crate::widgets::{self, Cx, Phase};
        let keys = RowKeys::of(scene, id, &row.param, self.state.timeline.current);
        // `scene` is the shown scene, so the row already carries the drafted value
        let changed = self.state.app.draft(id, &row.param).is_some();
        let shown = row;
        let mut key_cmd = None;
        let cx_label = ui
            .horizontal(|ui| {
                if let Some(keys) = &keys {
                    key_cmd = keys.show(ui, &row.label, &shown.value, changed);
                }
                if row.widget == Widget::Checkbox {
                    // The checkbox keeps its own label in the value column.
                    ui.make_persistent_id(("attr", id, &row.param))
                } else {
                    ui.label(&row.label).id
                }
            })
            .inner;
        if let Some(cmd) = key_cmd {
            if changed && cmd.0 == "key.set_parameter" {
                self.commit_row_drafts(&row.param);
            } else {
                self.run(cmd);
            }
        }
        let cx = Cx {
            scene,
            node: id,
            label: cx_label,
            distance: self.state.prefs.distance,
        };
        if let Some(edit) = widgets::show(ui, &cx, shown) {
            let mut nodes: Vec<NodeId> = self.state.selection.nodes.iter().copied().collect();
            if !nodes.contains(&id) {
                nodes.insert(0, id);
            }
            if can_draft && edit.create.is_none() {
                // every selected object whose row has a track stages a draft (UI.md §12,
                // owner note N10); the others are edited below as before
                let animated = self.state.animated();
                let mut direct = Vec::with_capacity(nodes.len());
                for node in nodes {
                    let staged =
                        forge_app::drafts::has_tracks(self.state.doc.scene(), node, &row.param)
                            && scene.param_desc(node, &row.param).is_some_and(|desc| {
                                let Some(current) = scene.param(node, &row.param) else {
                                    return false;
                                };
                                let Some(value) = draft_value(desc, current, &edit) else {
                                    return false;
                                };
                                if animated.param(node, &row.param) == Some(&value) {
                                    self.state.app.remove_draft(node, &row.param);
                                } else {
                                    let _ = self.state.app.set_draft(node, &row.param, value);
                                }
                                true
                            });
                    if !staged {
                        direct.push(node);
                    }
                }
                ui.ctx().request_repaint();
                if direct.is_empty() {
                    return;
                }
                nodes = direct;
            }
            let gesture = match edit.phase {
                Phase::Commit => None,
                Phase::Drag { started } => match self.gesture_for(started) {
                    Gesture::Drag(n) => Some(n),
                    Gesture::Commit => None,
                },
            };
            let mut edit = edit;
            if let Some(type_id) = edit.create.take() {
                // The field list's folder button (UI.md §16.6): create the group, put it
                // back as the active object, then add its row to the list.
                let blend = match &row.widget {
                    Widget::LinkRows { blends, .. } => {
                        blends.first().map(|b| b.id.clone()).unwrap_or_default()
                    }
                    _ => String::new(),
                };
                if self.run(("object.create", json!({"type": type_id})))
                    && let Some(new) = self.state.selection.active
                {
                    edit.value = widgets::link_list::with_new_row(&edit.value, new, &blend);
                    self.state.selection.click(id);
                } else {
                    return;
                }
            }
            let args = ("param.edit", edit.args(&row.param, &nodes, gesture));
            let kept = self.state.app.drafts();
            self.run(args);
            self.state.app.restore_drafts(kept);
        }
        if ui.input(|i| i.pointer.any_released()) && ui.ctx().dragged_id().is_none() {
            self.gesture = None;
        }
    }

    /// The key button of an edited row (UI.md §12, owner note N10): every selected object's
    /// draft of `param` becomes a key at the current frame, in one undo step. The drafts of
    /// other rows stay.
    fn commit_row_drafts(&mut self, param: &str) {
        let frame = self.state.timeline.current;
        let all = self.state.app.drafts();
        let (row, kept): (Vec<_>, Vec<_>) = all.into_iter().partition(|(_, p, _)| p == param);
        let depth = self.state.doc.undo_depth();
        let mut done = true;
        for (node, param, value) in &row {
            let unit = self
                .state
                .doc
                .scene()
                .param_desc(*node, param)
                .map_or(forge_core::Unit::None, |d| d.ui.unit);
            let Some(cmd) = crate::key_indicator::key_args(*node, param, frame, unit, value) else {
                continue;
            };
            done &= self.run(cmd);
        }
        self.state.doc.group_undo_since(depth, "Set Parameter Key");
        if done {
            self.state.app.restore_drafts(kept);
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
            // key-all toggles (UI.md §12, owner note N3): when every record key of the
            // selection is here already, the button is selected and removes them
            let recorded = forge_app::animate::recorded_keys_here(
                self.state.doc.scene(),
                &ids,
                self.state.timeline.current,
                settings.fps,
            );
            // a draft in the selection is an uncommitted edit: Record keys it (N10)
            let drafted = self
                .state
                .app
                .drafts()
                .iter()
                .any(|(node, _, _)| ids.contains(node));
            let remove = !recorded.is_empty() && !drafted;
            if ui
                .add(egui::Button::new("Record").selected(remove))
                .clicked()
            {
                if remove {
                    let fps = f64::from(settings.fps);
                    let keys: Vec<serde_json::Value> = recorded
                        .iter()
                        .map(|(id, track, time)| {
                            json!({"node": id.0, "track": track, "frame": time * fps})
                        })
                        .collect();
                    cmds.push(("key.delete", json!({"keys": keys})));
                } else {
                    cmds.push(cmd("key.record"));
                }
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
