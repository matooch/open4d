//! Embedded viewport: navigation mapping and wgpu-to-egui texture plumbing
//! (docs/specs/UI.md §6).

use std::collections::BTreeSet;

use eframe::egui_wgpu;
use forge_core::{EvalCtx, NodeId, Scene};
use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::Gizmo;
use forge_viewport::grid::grid_lines;
use forge_viewport::nav;
use forge_viewport::panes::Pane;
use forge_viewport::render::{
    DrawMesh, Frame, MeshBuffers, Palette, RenderTargets, Renderer, Settings, Shading,
};

use crate::components::{EditMode, display_selection, stored_selection};
use crate::theme::Theme;

/// What a held navigation key makes a primary drag do (UI.md §6.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavMode {
    Pan,
    Dolly,
    Orbit,
}

impl NavMode {
    /// The hold keys: X pan, C dolly, V orbit (own design, parity-pending G2).
    pub const KEYS: [(egui::Key, NavMode); 3] = [
        (egui::Key::X, NavMode::Pan),
        (egui::Key::C, NavMode::Dolly),
        (egui::Key::V, NavMode::Orbit),
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NavInput {
    pub primary: bool,
    pub middle: bool,
    pub secondary: bool,
    pub alt: bool,
    /// A navigation hold key that is down.
    pub held: Option<NavMode>,
    /// Pointer motion this frame, in points.
    pub drag: [f32; 2],
    /// Wheel motion this frame, in points (positive = away from the user).
    pub scroll: f32,
    /// Pointer position relative to the viewport's top-left corner, in points.
    pub cursor: [f32; 2],
}

impl NavInput {
    /// True while a navigation gesture owns the pointer (no selection or gizmo then).
    pub fn navigating(&self) -> bool {
        self.alt || self.held.is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavAction {
    None,
    Orbit(f64, f64),
    Pan(f64, f64),
    /// Drag distance in points; right moves closer.
    Dolly(f64),
    /// Wheel zoom: distance factor and the cursor it zooms toward.
    ZoomAt(f64, [f64; 2]),
}

/// Our default bindings (parity-pending G2; UI.md §6.1).
pub fn nav_action(i: &NavInput) -> NavAction {
    let (dx, dy) = (i.drag[0] as f64, i.drag[1] as f64);
    let held = |m: NavMode| i.primary && i.held == Some(m);
    if i.scroll != 0.0 {
        NavAction::ZoomAt(
            nav::wheel_factor(i.scroll as f64),
            [i.cursor[0] as f64, i.cursor[1] as f64],
        )
    } else if i.middle || held(NavMode::Pan) {
        NavAction::Pan(dx, dy)
    } else if (i.alt && i.primary) || held(NavMode::Orbit) {
        NavAction::Orbit(dx, dy)
    } else if (i.alt && i.secondary) || held(NavMode::Dolly) {
        NavAction::Dolly(dx)
    } else {
        NavAction::None
    }
}

/// What the viewport draws this frame besides the grid.
pub struct SceneView<'a> {
    pub scene: &'a Scene,
    pub selected: &'a BTreeSet<NodeId>,
    pub gizmo: Option<&'a Gizmo>,
    /// The component target and the mode being edited (EDITABLE.md §2.6).
    pub component: Option<(NodeId, EditMode)>,
    /// Evaluation context of the shown frame.
    pub ctx: EvalCtx,
}

/// Base colour of selected objects (our palette).
pub(crate) const SELECTED_TINT: [f32; 3] = [0.95, 0.62, 0.30];

/// Side of a drawn point in component mode, in pixels.
const POINT_SIZE: f32 = 7.0;

/// In a component mode edges must show: a shading without wire is drawn as its wire twin.
pub fn component_shading(shading: Shading) -> Shading {
    match shading {
        Shading::Smooth => Shading::SmoothWire,
        Shading::Flat => Shading::FlatWire,
        // faces plus edges, in the closest look to the chosen mode
        Shading::Quick | Shading::BoundingBox => Shading::SmoothWire,
        Shading::Constant => Shading::HiddenLine,
        other => other,
    }
}

/// An sRGB theme colour as the renderer's palette value (written unchanged to the target).
fn palette_color(c: egui::Color32) -> [f32; 3] {
    [c.r(), c.g(), c.b()].map(|v| v as f32 / 255.0)
}

/// Applies a gesture to the camera. `size` is the viewport in the same units as the drag.
pub fn apply_nav(cam: &mut Camera, action: NavAction, size: [f64; 2]) {
    *cam = match action {
        NavAction::None => return,
        NavAction::Orbit(dx, dy) => nav::orbit_drag(cam, dx, dy),
        NavAction::Pan(dx, dy) => nav::pan_drag(cam, dx, dy, size[1]),
        NavAction::Dolly(dx) => nav::dolly_drag(cam, dx),
        NavAction::ZoomAt(f, cursor) => nav::zoom_at(cam, cursor, size, f),
    };
}

struct Target {
    size: [u32; 2],
    targets: RenderTargets,
    texture_id: egui::TextureId,
}

/// Offscreen render targets registered with egui (one per pane), recreated on resize or when
/// the sample count changes.
#[derive(Default)]
pub struct ViewportPanel {
    renderer: Option<Renderer>,
    targets: [Option<Target>; 4],
    /// GPU meshes of the scene's generator nodes, keyed by scene revision and selection.
    meshes: Option<SceneMeshes>,
}

struct SceneMeshes {
    revision: u64,
    selection: BTreeSet<NodeId>,
    component: Option<(NodeId, EditMode)>,
    items: Vec<(MeshBuffers, [f32; 3])>,
}

/// World-space meshes of every generator node, with whether each is selected.
/// Also returns each node's smoothing angle (`None` = faceted).
pub fn scene_meshes(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
) -> Vec<(NodeId, PolyMesh, bool, Option<f64>)> {
    scene_meshes_at(scene, selected, &EvalCtx::default())
}

/// [`scene_meshes`] evaluated with `ctx` (time, fps, seed).
pub fn scene_meshes_at(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
    ctx: &EvalCtx,
) -> Vec<(NodeId, PolyMesh, bool, Option<f64>)> {
    let mut out = Vec::new();
    for root in scene.roots() {
        for id in std::iter::once(*root).chain(scene.descendants(*root)) {
            // a generator's input child is drawn through the generator (SUBDIVISION.md §4)
            if forge_generators::is_consumed(scene, id) {
                continue;
            }
            // hidden or disabled objects are not drawn (UI.md §13.1)
            if !forge_core::visibility::editor_visible(scene, id) {
                continue;
            }
            let (Ok(Some(mut mesh)), Some(m)) = (
                forge_generators::evaluate(scene, id, ctx),
                scene.world_matrix(id),
            ) else {
                continue;
            };
            for p in &mut mesh.points {
                *p = m.transform_point3(*p);
            }
            let smoothing = forge_core::smoothing::smoothing_angle(scene, id);
            out.push((id, mesh, selected.contains(&id), smoothing));
        }
    }
    out
}

impl ViewportPanel {
    /// Renders the scene into `pane`'s target at `size` pixels and returns the egui texture to
    /// show. `base` carries the pane's shading, lighting, shadows and anti-aliasing
    /// (`panes::display_settings`); the panel adds the component-mode overrides.
    pub fn render(
        &mut self,
        rs: &egui_wgpu::RenderState,
        pane: Pane,
        size: [u32; 2],
        cam: &Camera,
        base: Settings,
        view: SceneView,
    ) -> egui::TextureId {
        let SceneView {
            scene,
            selected,
            gizmo,
            component,
            ctx,
        } = view;
        let device = &rs.device;
        let samples = base.msaa.samples();
        if self
            .renderer
            .as_ref()
            .is_none_or(|r| r.samples() != samples)
        {
            self.renderer = Some(Renderer::with_samples(
                device,
                wgpu::TextureFormat::Rgba8Unorm,
                samples,
            ));
        }
        let renderer = self.renderer.as_ref().expect("renderer created above");
        let size = [size[0].max(1), size[1].max(1)];
        let slot = &mut self.targets[pane.index()];
        if slot
            .as_ref()
            .is_none_or(|t| t.size != size || t.targets.samples != samples)
        {
            let targets = RenderTargets::new(
                device,
                size[0],
                size[1],
                samples,
                wgpu::TextureUsages::TEXTURE_BINDING,
            );
            // egui samples the unorm view as-is, so palette values display as authored
            // (matching the offscreen snapshot PNGs).
            let view = targets.color.create_view(&Default::default());
            let mut egui_renderer = rs.renderer.write();
            let texture_id = match &*slot {
                Some(old) => {
                    egui_renderer.update_egui_texture_from_wgpu_texture(
                        device,
                        &view,
                        wgpu::FilterMode::Nearest,
                        old.texture_id,
                    );
                    old.texture_id
                }
                None => {
                    egui_renderer.register_native_texture(device, &view, wgpu::FilterMode::Nearest)
                }
            };
            *slot = Some(Target {
                size,
                targets,
                texture_id,
            });
        }
        let mut palette = Palette::default();
        if component.is_some() {
            palette.selection = palette_color(Theme::default().component_selection);
        }
        if self.meshes.as_ref().is_none_or(|m| {
            m.revision != scene.revision() || m.selection != *selected || m.component != component
        }) {
            let items = scene_meshes_at(scene, selected, &ctx)
                .into_iter()
                .map(|(id, mesh, sel, smoothing)| {
                    match component.filter(|(target, _)| *target == id) {
                        // the component target: plain colour, its components highlighted
                        Some((_, mode)) => {
                            let shown =
                                display_selection(&mesh, &stored_selection(scene, id), mode);
                            let build = if mode == EditMode::Points {
                                MeshBuffers::with_device
                            } else {
                                MeshBuffers::with_device_no_points
                            };
                            (build(device, &mesh, &shown, smoothing), palette.object)
                        }
                        None => {
                            let color = if sel { SELECTED_TINT } else { palette.object };
                            let buffers = MeshBuffers::with_device_no_points(
                                device,
                                &mesh,
                                &Selection::default(),
                                smoothing,
                            );
                            (buffers, color)
                        }
                    }
                })
                .collect();
            self.meshes = Some(SceneMeshes {
                revision: scene.revision(),
                selection: selected.clone(),
                component,
                items,
            });
        }
        let draw: Vec<DrawMesh> = self
            .meshes
            .iter()
            .flat_map(|m| &m.items)
            .map(|(mesh, color)| DrawMesh {
                mesh,
                color: *color,
            })
            .collect();
        let t = self.targets[pane.index()]
            .as_ref()
            .expect("target created above");
        let grid = grid_lines(cam, size[1] as f64);
        let frame = Frame {
            meshes: &draw,
            grid: Some(&grid),
            gizmo,
            settings: Settings {
                shading: if component.is_some() {
                    component_shading(base.shading)
                } else {
                    base.shading
                },
                show_points: component.is_some_and(|(_, m)| m == EditMode::Points),
                point_size: POINT_SIZE,
                palette,
                ..base
            },
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.render_to(device, &mut encoder, &t.targets, cam, &frame);
        rs.queue.submit([encoder.finish()]);
        t.texture_id
    }
}
