//! Editor state and node selection (docs/specs/UI.md §1).

use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

use forge_app::App;
use forge_core::{NodeId, Registry};
use forge_viewport::camera::{Camera, Handedness};
use forge_viewport::render::Shading;

use crate::dock::Dock;

pub use forge_command::NodeSelection;

/// The application core plus the state only a window needs.
pub struct EditorState {
    /// Document, selection, timeline, commands, evaluation cache (docs/specs/APP.md).
    pub app: App,
    pub expanded: BTreeSet<NodeId>,
    pub dock: Dock,
    pub camera: Camera,
    pub shading: Shading,
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
            shading: Shading::SmoothWire,
        }
    }
}
