//! Embedded viewport: navigation mapping and wgpu-to-egui texture plumbing
//! (docs/specs/UI.md §6).

use std::collections::BTreeSet;

use eframe::egui_wgpu;
use forge_core::{NodeId, Scene};
use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use forge_viewport::camera::Camera;
use forge_viewport::grid::grid_lines;
use forge_viewport::render::{DrawMesh, Frame, MeshBuffers, Palette, Renderer, Settings, Shading};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NavInput {
    pub primary: bool,
    pub middle: bool,
    pub secondary: bool,
    pub alt: bool,
    /// Pointer motion this frame, in points.
    pub drag: [f32; 2],
    /// Wheel motion this frame, in points (positive = away from the user).
    pub scroll: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NavAction {
    None,
    Orbit(f64, f64),
    Pan(f64, f64),
    Dolly(f64),
}

/// Our default bindings (parity-pending).
pub fn nav_action(i: &NavInput) -> NavAction {
    let (dx, dy) = (i.drag[0] as f64, i.drag[1] as f64);
    if i.scroll != 0.0 {
        NavAction::Dolly(0.999f64.powf(i.scroll as f64))
    } else if i.middle {
        NavAction::Pan(dx, dy)
    } else if i.alt && i.primary {
        NavAction::Orbit(-0.01 * dx, 0.01 * dy)
    } else if i.alt && i.secondary {
        NavAction::Dolly((-0.01 * dx).exp())
    } else {
        NavAction::None
    }
}

/// Base colour of selected objects (our palette).
const SELECTED_TINT: [f32; 3] = [0.95, 0.62, 0.30];

pub fn apply_nav(cam: &mut Camera, action: NavAction, viewport_h: f64) {
    match action {
        NavAction::None => {}
        NavAction::Orbit(yaw, pitch) => cam.orbit(yaw, pitch),
        NavAction::Pan(dx, dy) => cam.pan(dx, dy, viewport_h),
        NavAction::Dolly(f) => cam.dolly(f),
    }
}

struct Target {
    size: [u32; 2],
    color: wgpu::Texture,
    depth: wgpu::Texture,
    texture_id: egui::TextureId,
}

/// Offscreen render target registered with egui, recreated on resize.
#[derive(Default)]
pub struct ViewportPanel {
    renderer: Option<Renderer>,
    target: Option<Target>,
    /// GPU meshes of the scene's generator nodes, keyed by scene revision and selection.
    meshes: Option<SceneMeshes>,
}

struct SceneMeshes {
    revision: u64,
    selection: BTreeSet<NodeId>,
    items: Vec<(MeshBuffers, [f32; 3])>,
}

/// World-space meshes of every generator node, with whether each is selected.
pub fn scene_meshes(scene: &Scene, selected: &BTreeSet<NodeId>) -> Vec<(NodeId, PolyMesh, bool)> {
    let mut out = Vec::new();
    for root in scene.roots() {
        for id in std::iter::once(*root).chain(scene.descendants(*root)) {
            let Some(node) = scene.node(id) else { continue };
            let (Ok(mut mesh), Some(m)) = (
                forge_generators::build(&node.type_id, &node.params),
                scene.world_matrix(id),
            ) else {
                continue;
            };
            for p in &mut mesh.points {
                *p = m.transform_point3(*p);
            }
            out.push((id, mesh, selected.contains(&id)));
        }
    }
    out
}

impl ViewportPanel {
    /// Renders the scene at `size` pixels and returns the egui texture to show.
    pub fn render(
        &mut self,
        rs: &egui_wgpu::RenderState,
        size: [u32; 2],
        cam: &Camera,
        shading: Shading,
        scene: &Scene,
        selected: &BTreeSet<NodeId>,
    ) -> egui::TextureId {
        let device = &rs.device;
        let renderer = self
            .renderer
            .get_or_insert_with(|| Renderer::new(device, wgpu::TextureFormat::Rgba8Unorm));
        let size = [size[0].max(1), size[1].max(1)];
        if self.target.as_ref().is_none_or(|t| t.size != size) {
            let extent = wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            };
            let make = |label, format, usage, view_formats: &'static [wgpu::TextureFormat]| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats,
                })
            };
            let color = make(
                "viewport color",
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                &[],
            );
            let depth = make(
                "viewport depth",
                wgpu::TextureFormat::Depth32Float,
                wgpu::TextureUsages::RENDER_ATTACHMENT,
                &[],
            );
            // egui samples the unorm view as-is, so palette values display as authored
            // (matching the offscreen snapshot PNGs).
            let view = color.create_view(&Default::default());
            let mut egui_renderer = rs.renderer.write();
            let texture_id = match &self.target {
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
            self.target = Some(Target {
                size,
                color,
                depth,
                texture_id,
            });
        }
        let palette = Palette::default();
        if self
            .meshes
            .as_ref()
            .is_none_or(|m| m.revision != scene.revision() || m.selection != *selected)
        {
            let items = scene_meshes(scene, selected)
                .into_iter()
                .map(|(_, mesh, sel)| {
                    let color = if sel { SELECTED_TINT } else { palette.object };
                    (
                        MeshBuffers::with_device(device, &mesh, &Selection::default()),
                        color,
                    )
                })
                .collect();
            self.meshes = Some(SceneMeshes {
                revision: scene.revision(),
                selection: selected.clone(),
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
        let t = self.target.as_ref().expect("target created above");
        let color_view = t.color.create_view(&Default::default());
        let depth_view = t.depth.create_view(&Default::default());
        let grid = grid_lines(cam, size[1] as f64);
        let frame = Frame {
            meshes: &draw,
            grid: Some(&grid),
            gizmo: None,
            settings: Settings {
                shading,
                show_points: false,
                palette,
            },
        };
        let mut encoder = device.create_command_encoder(&Default::default());
        renderer.render(
            device,
            &mut encoder,
            &color_view,
            &depth_view,
            size[0],
            size[1],
            cam,
            &frame,
        );
        rs.queue.submit([encoder.finish()]);
        t.texture_id
    }
}
