//! Editor state and node selection (docs/specs/UI.md §1).

use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use forge_app::App;
use forge_command::UiRequest;
use forge_core::{NodeId, Registry};
use forge_viewport::camera::{Camera, Handedness};
use forge_viewport::gizmo::{GizmoMode, Space as GizmoSpace};
use forge_viewport::panes::Pane;
use forge_viewport::render::Shading;

use crate::autosave::Autosave;
use crate::console::ConsoleState;
use crate::crash_notice::CrashNotice;
use crate::dock::Dock;
use crate::hud::{FrameTimer, HudPrim, HudSettings};
use crate::keymap::KeyMapState;
use crate::layer_manager::LayerPanel;
use crate::node_editor::NodeEditorState;
use crate::panes::{DEFAULT_SHADING, Views};
use crate::prefs::UiPrefs;
use crate::screenshot::OffscreenGpu;
use crate::script_panel::ScriptPanelState;
use crate::tools::ToolState;

pub use forge_command::NodeSelection;

/// The application core plus the state only a window needs.
pub struct EditorState {
    /// Document, selection, timeline, commands, evaluation cache (docs/specs/APP.md).
    pub app: App,
    pub expanded: BTreeSet<NodeId>,
    pub dock: Dock,
    /// The active pane's camera and shading are these two fields; the other panes' are in
    /// `views` (docs/specs/UI.md §6.2).
    pub camera: Camera,
    /// Width / height of the viewport as last drawn; the frame commands fit to it.
    pub view_aspect: f64,
    pub shading: Shading,
    /// Viewport layout, active and maximised pane, lighting options and the other panes.
    pub views: Views,
    /// Script Manager source, console and recent list (docs/specs/UI.md §10).
    pub script: ScriptPanelState,
    /// The Console panel: log filter and the REPL (docs/specs/UI.md §19).
    pub console: ConsoleState,
    /// The Layers panel: selected rows and rename (docs/specs/UI.md §22).
    pub layer_panel: LayerPanel,
    /// The Node Editor's tag, canvas and cache (docs/specs/UI.md §17).
    pub node_editor: NodeEditorState,
    /// The headless GPU device `viewport.screenshot` renders with (RPC.md §10.2).
    pub offscreen: OffscreenGpu,
    /// The active modeling tool and its values (docs/specs/MODELING.md §0.2).
    pub tool: Option<ToolState>,
    /// Key chords to commands, defaults and overrides (docs/specs/UI.md §14).
    pub keys: KeyMapState,
    /// Theme and UI scale (UI.md §24).
    pub prefs: UiPrefs,
    /// The viewport gizmo's mode and axes (`view.gizmo_mode`, `view.gizmo_space`).
    pub gizmo_mode: GizmoMode,
    pub gizmo_space: GizmoSpace,
    /// Window actions that commands asked for; the editor serves them after the command.
    pub ui_requests: Vec<UiRequest>,
    /// Periodic autosave and the recovery list (docs/specs/PLATFORM.md §7).
    pub autosave: Autosave,
    /// The "crashed last time" notice (docs/specs/UI.md §15.2).
    pub crash_notice: CrashNotice,
    /// Which heads-up items each pane shows (docs/specs/UI.md §23). Saved with layouts.
    pub hud: HudSettings,
    /// Smoothed editor frame time for the frame rate item. Not saved.
    pub hud_timer: FrameTimer,
    /// The HUD primitives painted in the last frame, per pane (for tests). Not saved.
    pub hud_last: Vec<(Pane, Vec<HudPrim>)>,
    /// The snap target under the pointer during a drag or tool hover, for the marker
    /// (MANIPULATE.md §6.7). Not saved.
    pub snap_hit: Option<forge_modeling::snap::SnapHit>,
}

impl Deref for EditorState {
    type Target = App;

    fn deref(&self) -> &App {
        &self.app
    }
}

impl DerefMut for EditorState {
    fn deref_mut(&mut self) -> &mut App {
        &mut self.app
    }
}

impl EditorState {
    /// Left-handed view matching the reference default directions; the exact default
    /// pose is not shown by the reference app (docs/specs/SMOOTHING.md §4).
    pub fn default_camera() -> Camera {
        Camera {
            yaw: 0.68,
            pitch: 0.52,
            distance: 1000.0,
            handedness: Handedness::Left,
            ..Camera::default()
        }
    }

    pub fn new(registry: Arc<Registry>) -> EditorState {
        EditorState {
            app: App::new(registry),
            expanded: BTreeSet::new(),
            dock: Dock::default(),
            camera: Self::default_camera(),
            view_aspect: 16.0 / 9.0,
            shading: DEFAULT_SHADING,
            views: Views::new(&Self::default_camera()),
            script: ScriptPanelState::default(),
            console: ConsoleState::default(),
            layer_panel: LayerPanel::default(),
            node_editor: NodeEditorState::default(),
            offscreen: OffscreenGpu::default(),
            tool: None,
            keys: KeyMapState::default(),
            prefs: UiPrefs::default(),
            gizmo_mode: GizmoMode::Translate,
            gizmo_space: GizmoSpace::Object,
            ui_requests: Vec::new(),
            autosave: Autosave::default(),
            crash_notice: CrashNotice::default(),
            hud: HudSettings::default(),
            hud_timer: FrameTimer::default(),
            hud_last: Vec::new(),
            snap_hit: None,
        }
    }
}
