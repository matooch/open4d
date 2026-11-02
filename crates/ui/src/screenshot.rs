//! `viewport.screenshot`: the scene rendered offscreen with the viewport renderer and
//! encoded as PNG (docs/specs/RPC.md §10.2).

use forge_core::EvalCtx;
use forge_viewport::camera::Camera;
use forge_viewport::grid::grid_lines;
use forge_viewport::panes::{PaneRect, arrange};
use forge_viewport::render::{
    DrawInstanced, DrawMesh, Frame, Gpu, Palette, Settings, render_to_rgba,
};

use crate::panes::{PANE_GAP, display_settings, scene_lights};
use crate::state::EditorState;
use crate::viewport_panel::{GpuCache, GpuItem, UploadSettings, scene_draw_items_at, upload_items};

/// A headless wgpu device made on first use. A failure to find an adapter is remembered,
/// so a machine without a GPU does not retry on every call.
#[derive(Default)]
pub struct OffscreenGpu(Option<Result<Gpu, String>>);

impl OffscreenGpu {
    pub fn get(&mut self) -> Result<&Gpu, String> {
        self.0
            .get_or_insert_with(|| Gpu::new_headless().map_err(|e| e.to_string()))
            .as_ref()
            .map_err(Clone::clone)
    }
}

/// Colour of the gaps between panes in a four-view screenshot.
const GAP_COLOR: [u8; 4] = [18, 18, 20, 255];

/// Renders the editor's view of the current frame at `width` x `height` pixels and
/// returns an RGBA8 PNG. What the viewport shows is rendered: the active pane, the maximised
/// pane, or the four panes side by side with the editor's lighting and anti-aliasing; the
/// grid is drawn, selected objects are tinted, the gizmo is left out (UI.md §6.2).
pub fn render_png(state: &mut EditorState, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let scene = state.app.evaluated();
    let selected = state.app.selection.nodes.clone();
    let ctx = EvalCtx::at_frame(state.app.timeline.current, f64::from(scene.settings().fps));
    let lights = scene_lights(&scene);
    let views = state.views.clone();
    let panes: Vec<(PaneRect, Camera, Settings)> = arrange(
        views.layout,
        views.active,
        views.maximized,
        width as f32,
        height as f32,
        PANE_GAP,
    )
    .into_iter()
    .map(|r| {
        let settings = display_settings(&views, state.pane_shading(r.pane), &lights);
        (r, state.pane_camera(r.pane), settings)
    })
    .collect();
    let gpu = state.offscreen.get()?;
    let palette = Palette::default();
    let items = upload_items(
        &mut GpuCache::default(),
        &gpu.device,
        &scene,
        scene_draw_items_at(&scene, &selected, &ctx),
        None,
        &palette,
        UploadSettings::default(),
    );
    let (mut draw, mut draw_instanced) = (Vec::new(), Vec::new());
    for item in &items {
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
    let rgba = if let [(_, camera, settings)] = &panes[..] {
        let grid = grid_lines(camera, f64::from(height));
        let frame = Frame {
            instanced: &draw_instanced,
            meshes: &draw,
            grid: Some(&grid),
            gizmo: None,
            settings: *settings,
        };
        render_to_rgba(gpu, width, height, camera, &frame)
    } else {
        let mut canvas = GAP_COLOR.repeat((width * height) as usize);
        for (r, camera, settings) in &panes {
            let (x0, y0) = (r.x as u32, r.y as u32);
            let (w, h) = ((r.w as u32).max(1), (r.h as u32).max(1));
            let grid = grid_lines(camera, f64::from(h));
            let frame = Frame {
                instanced: &draw_instanced,
                meshes: &draw,
                grid: Some(&grid),
                gizmo: None,
                settings: *settings,
            };
            let pane = render_to_rgba(gpu, w, h, camera, &frame);
            for row in 0..h.min(height.saturating_sub(y0)) {
                let n = w.min(width.saturating_sub(x0)) as usize * 4;
                let src = (row * w) as usize * 4;
                let dst = ((y0 + row) * width + x0) as usize * 4;
                canvas[dst..dst + n].copy_from_slice(&pane[src..src + n]);
            }
        }
        canvas
    };
    let mut png_bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut png_bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(&rgba))
        .map_err(|e| format!("PNG encoding failed: {e}"))?;
    Ok(png_bytes)
}
