//! Embedded viewport: navigation mapping and wgpu-to-egui texture plumbing
//! (docs/specs/UI.md §6).

use std::collections::BTreeSet;
use std::sync::Arc;

use eframe::egui_wgpu;
use std::collections::BTreeMap;

use forge_app::schedule;
use forge_core::{EvalCtx, NodeId, Scene};
use forge_generators::{ClonerInstances, POLYGON};
use forge_geometry::PolyMesh;
use forge_geometry::selection::Selection;
use forge_viewport::cache::{
    RevisionCache, combine, instances_revision, mesh_revision, selection_revision,
};
use forge_viewport::camera::Camera;
use forge_viewport::gizmo::Gizmo;
use forge_viewport::grid::grid_lines;
use forge_viewport::instancing::InstanceInput;
use forge_viewport::lod::lod_proxy;
use forge_viewport::nav;
use forge_viewport::panes::Pane;
use forge_viewport::render::{
    DrawInstanced, DrawMesh, Frame, InstanceSet, MeshBuffers, Palette, RenderTargets, Renderer,
    Settings, Shading,
};
use glam::DMat4;

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
    /// The scheduler's pass for `scene` (viewport instancing mode): when present the panel
    /// draws from its outputs and rebuilds only the nodes whose output changed
    /// (docs/specs/VIEWPORT.md §10.8). `None` evaluates `scene` from scratch (a live tool
    /// preview replaces a mesh in the scene).
    pub frame: Option<&'a schedule::Frame>,
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
    /// The drawn items, rebuilt when the scene revision, selection or component state changes.
    items: Option<SceneItems>,
    /// The scheduled path: one uploaded item per drawn node, kept while its output is.
    slots: SlotCache<GpuItem>,
    /// Uploaded meshes and instance sets, reused while a node's content is unchanged.
    cache: GpuCache,
}

struct SceneItems {
    revision: u64,
    selection: BTreeSet<NodeId>,
    component: Option<(NodeId, EditMode)>,
    items: Vec<GpuItem>,
}

/// An uploaded drawable with its base colour.
#[derive(Clone)]
pub enum GpuItem {
    Mesh(Arc<MeshBuffers>, [f32; 3]),
    Instanced(Arc<InstanceSet>, [f32; 3]),
}

/// Uploaded meshes and instance sets per node, keyed by the content revision
/// (docs/specs/VIEWPORT.md §10.2).
#[derive(Default)]
pub struct GpuCache {
    pub meshes: RevisionCache<NodeId, Arc<MeshBuffers>>,
    pub instances: RevisionCache<NodeId, Arc<InstanceSet>>,
}

/// Meshes with more triangles than this get a LOD proxy when proxies are on.
pub const LOD_MIN_TRIS: usize = 50_000;
/// Clustering cells per axis of the proxies.
pub const LOD_CELLS: u32 = 24;

/// One drawable object of the scene.
pub enum DrawItem {
    /// World-space geometry.
    Mesh {
        id: NodeId,
        mesh: PolyMesh,
        selected: bool,
        smoothing: Option<f64>,
    },
    /// A cloner drawn by GPU instancing: prototypes and clones in cloner space, `world` places
    /// the cloner.
    Instanced {
        id: NodeId,
        instances: Box<ClonerInstances>,
        world: DMat4,
        selected: bool,
        smoothing: Option<f64>,
    },
}

impl DrawItem {
    pub fn id(&self) -> NodeId {
        match self {
            DrawItem::Mesh { id, .. } | DrawItem::Instanced { id, .. } => *id,
        }
    }
}

/// The per-clone arrays of a cloner as the viewport reads them.
pub fn instance_input(i: &ClonerInstances) -> InstanceInput<'_> {
    InstanceInput {
        matrices: &i.clones.matrices,
        colors: &i.clones.colors,
        visible: &i.clones.visible,
        child: &i.clones.child,
    }
}

/// World-space meshes of every generator node, with whether each is selected.
/// Also returns each node's smoothing angle (`None` = faceted).
pub fn scene_meshes(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
) -> Vec<(NodeId, PolyMesh, bool, Option<f64>)> {
    scene_meshes_at(scene, selected, &EvalCtx::default())
}

/// The nodes drawn as objects, in scene order: generator inputs consumed by their parent and
/// hidden or disabled objects are left out (SUBDIVISION.md §4, UI.md §13.1).
fn drawn_nodes(scene: &Scene) -> Vec<NodeId> {
    forge_app::schedule::drawn_nodes(scene)
}

/// [`scene_meshes`] evaluated with `ctx` (time, fps, seed). Cloners are expanded to geometry
/// (export, bounds, tests); the viewport itself uses [`scene_draw_items_at`].
pub fn scene_meshes_at(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
    ctx: &EvalCtx,
) -> Vec<(NodeId, PolyMesh, bool, Option<f64>)> {
    let mut out = Vec::new();
    for id in drawn_nodes(scene) {
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
    out
}

/// What the viewport draws: [`scene_meshes_at`] with every cloner that
/// `forge_generators::cloner_instances` can show returned as instances instead of expanded
/// geometry (docs/specs/VIEWPORT.md §10.8).
pub fn scene_draw_items_at(
    scene: &Scene,
    selected: &BTreeSet<NodeId>,
    ctx: &EvalCtx,
) -> Vec<DrawItem> {
    let mut out = Vec::new();
    for id in drawn_nodes(scene) {
        let Some(world) = scene.world_matrix(id) else {
            continue;
        };
        let smoothing = forge_core::smoothing::smoothing_angle(scene, id);
        let is_selected = selected.contains(&id);
        if let Ok(Some(instances)) = forge_generators::cloner_instances(scene, id, ctx) {
            out.push(DrawItem::Instanced {
                id,
                instances: Box::new(instances),
                world,
                selected: is_selected,
                smoothing,
            });
            continue;
        }
        let Ok(Some(mut mesh)) = forge_generators::evaluate(scene, id, ctx) else {
            continue;
        };
        for p in &mut mesh.points {
            *p = world.transform_point3(*p);
        }
        out.push(DrawItem::Mesh {
            id,
            mesh,
            selected: is_selected,
            smoothing,
        });
    }
    out
}

/// The draw item of one scheduled output, `None` when it shows nothing. The same item
/// [`scene_draw_items_at`] makes for the node (bit for bit, tested).
pub fn draw_item_of(
    scene: &Scene,
    id: NodeId,
    out: &schedule::NodeOutput,
    selected: bool,
) -> Option<DrawItem> {
    let world = scene.world_matrix(id)?;
    let smoothing = forge_core::smoothing::smoothing_angle(scene, id);
    if let Some(instances) = &out.instances {
        return Some(DrawItem::Instanced {
            id,
            instances: Box::new((**instances).clone()),
            world,
            selected,
            smoothing,
        });
    }
    Some(DrawItem::Mesh {
        id,
        mesh: out.mesh.clone()?,
        selected,
        smoothing,
    })
}

/// [`scene_draw_items_at`] from a pass of a scheduler made `with_instancing(true)`.
pub fn scheduled_draw_items(frame: &schedule::Frame, selected: &BTreeSet<NodeId>) -> Vec<DrawItem> {
    frame
        .drawn
        .iter()
        .filter_map(|id| {
            let out = frame.outputs.get(id)?;
            draw_item_of(&frame.scene, *id, out, selected.contains(id))
        })
        .collect()
}

struct Slot<T> {
    out: Arc<schedule::NodeOutput>,
    selected: bool,
    smoothing: u64,
    /// Built as the component target (its buffers differ from a plain object's).
    target: bool,
    item: Option<T>,
}

/// Pass generation, selection, component target and LOD flag of the last refresh.
type SlotKey = (u64, BTreeSet<NodeId>, Option<(NodeId, EditMode)>, bool);

/// What the viewport keeps per drawn node between passes of the scheduler: the built item
/// of a node is reused while its output (by identity), selection colour and smoothing are
/// unchanged, so a frame of playback rebuilds only the animated nodes and their dependants.
pub struct SlotCache<T> {
    slots: BTreeMap<NodeId, Slot<T>>,
    key: Option<SlotKey>,
    items: Vec<T>,
    /// Nodes built by the last refresh.
    pub rebuilt: usize,
}

impl<T> Default for SlotCache<T> {
    fn default() -> Self {
        SlotCache {
            slots: BTreeMap::new(),
            key: None,
            items: Vec::new(),
            rebuilt: 0,
        }
    }
}

impl<T: Clone> SlotCache<T> {
    /// The items to draw, in scene order, for the pass `frame`. `build` makes the item of a
    /// changed node. Nothing is done when the pass, selection, component target and LOD
    /// flag are the same as in the last call.
    pub fn refresh(
        &mut self,
        frame: &schedule::Frame,
        selected: &BTreeSet<NodeId>,
        component: Option<(NodeId, EditMode)>,
        lod: bool,
        mut build: impl FnMut(DrawItem) -> T,
    ) -> &[T] {
        let same = self.key.as_ref().is_some_and(|k| {
            k.0 == frame.generation && k.1 == *selected && k.2 == component && k.3 == lod
        });
        if same {
            self.rebuilt = 0;
            return &self.items;
        }
        if self.key.as_ref().is_some_and(|k| k.3 != lod) {
            self.slots.clear();
        }
        self.key = Some((frame.generation, selected.clone(), component, lod));
        self.rebuilt = 0;
        // every selected polygon object is a component target next to the primary one
        // (EDITABLE.md §6.1)
        let is_target = |id: &NodeId| {
            component.is_some_and(|(t, _)| {
                t == *id
                    || (selected.contains(id)
                        && frame.scene.node(*id).is_some_and(|n| n.type_id == POLYGON))
            })
        };
        let mut slots = std::mem::take(&mut self.slots);
        let mut next = BTreeMap::new();
        let mut items = Vec::new();
        for id in &frame.drawn {
            let Some(out) = frame.outputs.get(id) else {
                continue;
            };
            let is_selected = selected.contains(id);
            let smoothing =
                smoothing_bits(forge_core::smoothing::smoothing_angle(&frame.scene, *id));
            let kept = slots.remove(id).filter(|s| {
                Arc::ptr_eq(&s.out, out)
                    && s.selected == is_selected
                    && s.smoothing == smoothing
                    && !s.target
                    && !is_target(id)
            });
            let slot = match kept {
                Some(slot) => slot,
                None => {
                    self.rebuilt += 1;
                    let item = draw_item_of(&frame.scene, *id, out, is_selected).map(&mut build);
                    Slot {
                        out: out.clone(),
                        selected: is_selected,
                        smoothing,
                        target: is_target(id),
                        item,
                    }
                }
            };
            items.extend(slot.item.clone());
            next.insert(*id, slot);
        }
        self.slots = next;
        self.items = items;
        &self.items
    }

    /// The items of the last refresh.
    pub fn items(&self) -> &[T] {
        &self.items
    }

    /// The nodes that currently have an item.
    pub fn live(&self) -> BTreeSet<NodeId> {
        self.slots
            .iter()
            .filter(|(_, s)| s.item.is_some())
            .map(|(id, _)| *id)
            .collect()
    }
}

/// Options of [`upload_items`].
#[derive(Debug, Clone, Copy, Default)]
pub struct UploadSettings {
    /// Build LOD proxies for large meshes (docs/specs/VIEWPORT.md §10.6).
    pub lod_proxies: bool,
}

fn triangle_estimate(mesh: &PolyMesh) -> usize {
    mesh.polys.iter().map(|p| p.len().saturating_sub(2)).sum()
}

fn smoothing_bits(s: Option<f64>) -> u64 {
    s.map_or(u64::MAX, f64::to_bits)
}

/// Uploads `items`, reusing the cache entries whose content revision is unchanged
/// (docs/specs/VIEWPORT.md §10.2); entries of nodes that are no longer drawn are dropped.
pub fn upload_items(
    cache: &mut GpuCache,
    device: &wgpu::Device,
    scene: &Scene,
    items: Vec<DrawItem>,
    component: Option<(NodeId, EditMode)>,
    palette: &Palette,
    opts: UploadSettings,
) -> Vec<GpuItem> {
    let live: BTreeSet<NodeId> = items.iter().map(DrawItem::id).collect();
    let out = items
        .into_iter()
        .map(|item| upload_item(cache, device, scene, item, component, palette, opts))
        .collect();
    cache.meshes.retain(|k| live.contains(k));
    cache.instances.retain(|k| live.contains(k));
    out
}

/// Uploads one item through the cache (no clean-up of other entries).
pub fn upload_item(
    cache: &mut GpuCache,
    device: &wgpu::Device,
    scene: &Scene,
    item: DrawItem,
    component: Option<(NodeId, EditMode)>,
    palette: &Palette,
    opts: UploadSettings,
) -> GpuItem {
    let lod_flag = u64::from(opts.lod_proxies);
    match item {
        DrawItem::Mesh {
            id,
            mesh,
            selected,
            smoothing,
        } => {
            let target = component.filter(|(t, _)| {
                *t == id || (selected && scene.node(id).is_some_and(|n| n.type_id == POLYGON))
            });
            let shown = target.map(|(_, mode)| {
                (
                    mode,
                    display_selection(&mesh, &stored_selection(scene, id), mode),
                )
            });
            // the colour is not part of the buffers: selecting an object uploads nothing
            let mut rev = combine(mesh_revision(&mesh), smoothing_bits(smoothing));
            rev = combine(rev, lod_flag);
            if let Some((mode, sel)) = &shown {
                rev = combine(combine(rev, *mode as u64 + 1), selection_revision(sel));
            }
            let buffers = cache.meshes.get_or_insert_with(id, rev, || {
                Arc::new(match &shown {
                    Some((mode, sel)) => {
                        let build = if *mode == EditMode::Points {
                            MeshBuffers::with_device
                        } else {
                            MeshBuffers::with_device_no_points
                        };
                        build(device, &mesh, sel, smoothing)
                    }
                    None => {
                        let mut b = MeshBuffers::with_device_no_points(
                            device,
                            &mesh,
                            &Selection::default(),
                            smoothing,
                        );
                        if opts.lod_proxies && triangle_estimate(&mesh) > LOD_MIN_TRIS {
                            b.set_lod(MeshBuffers::with_device_no_points(
                                device,
                                &lod_proxy(&mesh, LOD_CELLS),
                                &Selection::default(),
                                smoothing,
                            ));
                        }
                        b
                    }
                })
            });
            let color = if selected && target.is_none() {
                SELECTED_TINT
            } else {
                palette.object
            };
            GpuItem::Mesh(buffers, color)
        }
        DrawItem::Instanced {
            id,
            instances,
            world,
            selected,
            smoothing,
        } => {
            let input = instance_input(&instances);
            let protos: Vec<u64> = instances.prototypes.iter().map(mesh_revision).collect();
            let rev = combine(
                combine(
                    instances_revision(&protos, &input, &world),
                    smoothing_bits(smoothing),
                ),
                lod_flag,
            );
            let set = cache.instances.get_or_insert_with(id, rev, || {
                let mut set =
                    InstanceSet::new(device, &instances.prototypes, smoothing, &input, &world);
                if opts.lod_proxies {
                    for (i, p) in instances.prototypes.iter().enumerate() {
                        if triangle_estimate(p) > LOD_MIN_TRIS {
                            set.set_proto_lod(device, i, &lod_proxy(p, LOD_CELLS), smoothing);
                        }
                    }
                }
                Arc::new(set)
            });
            let color = if selected {
                SELECTED_TINT
            } else {
                palette.object
            };
            GpuItem::Instanced(set, color)
        }
    }
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
            frame,
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
        let opts = UploadSettings {
            lod_proxies: base.lod_px > 0.0,
        };
        let scheduled = frame.is_some();
        if let Some(frame) = frame {
            let cache = &mut self.cache;
            self.slots
                .refresh(frame, selected, component, opts.lod_proxies, |item| {
                    upload_item(cache, device, &frame.scene, item, component, &palette, opts)
                });
            if self.slots.rebuilt > 0 {
                let live = self.slots.live();
                self.cache.meshes.retain(|k| live.contains(k));
                self.cache.instances.retain(|k| live.contains(k));
            }
        }
        if !scheduled
            && self.items.as_ref().is_none_or(|m| {
                m.revision != scene.revision()
                    || m.selection != *selected
                    || m.component != component
            })
        {
            let items = upload_items(
                &mut self.cache,
                device,
                scene,
                scene_draw_items_at(scene, selected, &ctx),
                component,
                &palette,
                opts,
            );
            self.items = Some(SceneItems {
                revision: scene.revision(),
                selection: selected.clone(),
                component,
                items,
            });
            // the scheduled path starts over when it is used again
            self.slots = SlotCache::default();
        }
        let (mut draw, mut draw_instanced) = (Vec::new(), Vec::new());
        let shown: &[GpuItem] = if scheduled {
            self.slots.items()
        } else {
            self.items.as_ref().map_or(&[], |m| &m.items)
        };
        for item in shown {
            match item {
                GpuItem::Mesh(mesh, color) => draw.push(DrawMesh {
                    mesh,
                    color: *color,
                }),
                GpuItem::Instanced(set, color) => {
                    draw_instanced.push(DrawInstanced { set, color: *color })
                }
            }
        }
        let t = self.targets[pane.index()]
            .as_ref()
            .expect("target created above");
        let grid = grid_lines(cam, size[1] as f64);
        let frame = Frame {
            meshes: &draw,
            instanced: &draw_instanced,
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
