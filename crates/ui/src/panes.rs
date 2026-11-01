//! The viewport's panes: per-pane cameras and shading, the 4-view layout state, the lighting
//! options and the settings handed to the renderer (docs/specs/UI.md §6.2, VIEWPORT.md §8-§9).
//! Own design, parity-pending G8.

use forge_command::{LayoutArg, LightingArg, PaneArg};
use forge_core::Scene;
use forge_viewport::camera::Camera;
use forge_viewport::lights::{Light, Lighting, SceneLights};
use forge_viewport::panes::{Layout, Pane, pane_camera};
use forge_viewport::render::{Msaa, Settings, Shading};

use crate::state::EditorState;

/// Gap between panes in the 4-view layout, in points.
pub const PANE_GAP: f32 = 2.0;

/// Shading of the perspective pane at the start and after File > New.
pub const DEFAULT_SHADING: Shading = Shading::SmoothWire;
/// Shading of the top, front and right panes at the start: line drawings (our choice).
pub const ORTHO_SHADING: Shading = Shading::Wire;

/// What a pane remembers while another pane is active.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaneState {
    pub camera: Camera,
    pub shading: Shading,
}

/// Layout and display state of the viewport. `EditorState.camera` / `.shading` are the working
/// copy of the **active** pane; `saved` holds the others (and a stale copy of the active one).
#[derive(Debug, Clone, PartialEq)]
pub struct Views {
    pub layout: Layout,
    pub active: Pane,
    /// The pane shown alone in the four-view layout.
    pub maximized: Option<Pane>,
    pub lighting: LightingArg,
    pub shadows: bool,
    pub antialias: bool,
    saved: [PaneState; 4],
    /// Panes whose camera was never changed: they follow the perspective pane when the
    /// four-view layout opens.
    pristine: [bool; 4],
}

impl Views {
    pub fn new(base: &Camera) -> Views {
        Views {
            layout: Layout::Single,
            active: Pane::Perspective,
            maximized: None,
            lighting: LightingArg::Headlight,
            shadows: true,
            antialias: true,
            saved: Pane::ALL.map(|p| PaneState {
                camera: pane_camera(p, base),
                shading: if p == Pane::Perspective {
                    DEFAULT_SHADING
                } else {
                    ORTHO_SHADING
                },
            }),
            pristine: [true; 4],
        }
    }

    /// The panes drawn now.
    pub fn visible(&self) -> Vec<Pane> {
        forge_viewport::panes::visible(self.layout, self.active, self.maximized)
    }
}

pub fn pane_of(p: PaneArg) -> Pane {
    match p {
        PaneArg::Perspective => Pane::Perspective,
        PaneArg::Top => Pane::Top,
        PaneArg::Front => Pane::Front,
        PaneArg::Right => Pane::Right,
    }
}

pub fn layout_of(l: LayoutArg) -> Layout {
    match l {
        LayoutArg::Single => Layout::Single,
        LayoutArg::Four => Layout::Four,
    }
}

impl EditorState {
    /// Makes `pane` the active one: the working camera and shading go back to the pane they
    /// belong to and the new pane's are loaded.
    pub fn activate(&mut self, pane: Pane) {
        let v = &mut self.views;
        if v.active == pane {
            return;
        }
        v.saved[v.active.index()] = PaneState {
            camera: self.camera,
            shading: self.shading,
        };
        let next = v.saved[pane.index()];
        self.camera = next.camera;
        self.shading = next.shading;
        v.active = pane;
    }

    pub fn pane_camera(&self, pane: Pane) -> Camera {
        if pane == self.views.active {
            self.camera
        } else {
            self.views.saved[pane.index()].camera
        }
    }

    pub fn pane_shading(&self, pane: Pane) -> Shading {
        if pane == self.views.active {
            self.shading
        } else {
            self.views.saved[pane.index()].shading
        }
    }

    /// Runs `f` with `pane` active (`None`: the current one) and then restores the active pane.
    pub fn with_pane<R>(&mut self, pane: Option<Pane>, f: impl FnOnce(&mut EditorState) -> R) -> R {
        let previous = self.views.active;
        if let Some(p) = pane {
            self.activate(p);
        }
        let out = f(self);
        self.activate(previous);
        out
    }

    /// The active pane's camera was changed by the user or a command.
    pub fn touch_view(&mut self) {
        self.views.pristine[self.views.active.index()] = false;
    }

    /// `view.layout`; `None` toggles. Opening the four-view layout lets untouched
    /// orthographic panes follow the perspective pane's pivot and size; going back to one pane
    /// forgets the maximised pane.
    pub fn set_view_layout(&mut self, layout: Option<Layout>) {
        let layout = layout.unwrap_or(match self.views.layout {
            Layout::Single => Layout::Four,
            Layout::Four => Layout::Single,
        });
        if layout == Layout::Four && self.views.layout == Layout::Single {
            let base = self.pane_camera(Pane::Perspective);
            for p in [Pane::Top, Pane::Front, Pane::Right] {
                if self.views.pristine[p.index()] {
                    let derived = pane_camera(p, &base);
                    if p == self.views.active {
                        self.camera = derived;
                    } else {
                        self.views.saved[p.index()].camera = derived;
                    }
                }
            }
        }
        if layout == Layout::Single {
            self.views.maximized = None;
        }
        self.views.layout = layout;
    }

    /// `view.maximize`: toggles `pane` (default: the active one) in the four-view layout.
    pub fn toggle_maximize(&mut self, pane: Option<Pane>) {
        if self.views.layout != Layout::Four {
            return;
        }
        let pane = pane.unwrap_or(self.views.active);
        if self.views.maximized == Some(pane) {
            self.views.maximized = None;
        } else {
            self.views.maximized = Some(pane);
            self.activate(pane);
        }
    }

    /// File > New: every pane starts over.
    pub fn reset_views(&mut self) {
        let base = EditorState::default_camera();
        self.views = Views::new(&base);
        self.camera = base;
        self.shading = DEFAULT_SHADING;
    }
}

/// The lights of the scene's light objects, in world space (VIEWPORT.md §8.1). No light node
/// type exists yet, so this is empty; when light objects arrive, map each to a
/// [`Light`] here (omni -> `Light::point`, spot -> `Light::spot`, infinite -> `Light::directional`
/// along the node's world -Z axis) and nothing else has to change.
pub fn scene_lights(_scene: &Scene) -> Vec<Light> {
    Vec::new()
}

/// The renderer settings for a pane with `shading`: lighting, shadows and anti-aliasing from
/// `views`; scene lights override the lighting mode.
pub fn display_settings(views: &Views, shading: Shading, lights: &[Light]) -> Settings {
    let lighting = if lights.is_empty() {
        match views.lighting {
            LightingArg::Headlight => Lighting::Headlight,
            LightingArg::Rig => Lighting::Rig,
        }
    } else {
        Lighting::Scene(SceneLights::new(lights))
    };
    Settings {
        shading,
        lighting,
        shadows: views.shadows,
        msaa: if views.antialias { Msaa::X4 } else { Msaa::Off },
        ..Settings::default()
    }
}
