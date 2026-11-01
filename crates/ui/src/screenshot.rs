//! `viewport.screenshot`: the scene rendered offscreen with the viewport renderer and
//! encoded as PNG (docs/specs/RPC.md §10.2).

use forge_core::EvalCtx;
use forge_viewport::grid::grid_lines;
use forge_viewport::render::{
    DrawMesh, Frame, Gpu, MeshBuffers, Palette, Settings, render_to_rgba,
};

use crate::state::EditorState;
use crate::viewport_panel::{SELECTED_TINT, scene_meshes_at};
use forge_geometry::selection::Selection;

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

/// Renders the editor's view of the current frame at `width` x `height` pixels and
/// returns an RGBA8 PNG. The editor's camera and shading are used; the grid is drawn,
/// selected objects are tinted, the gizmo is left out.
pub fn render_png(state: &mut EditorState, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let scene = state.app.evaluated();
    let selected = state.app.selection.nodes.clone();
    let camera = state.camera;
    let shading = state.shading;
    let ctx = EvalCtx::at_frame(state.app.timeline.current, f64::from(scene.settings().fps));
    let gpu = state.offscreen.get()?;
    let palette = Palette::default();
    let buffers: Vec<(MeshBuffers, [f32; 3])> = scene_meshes_at(&scene, &selected, &ctx)
        .into_iter()
        .map(|(_, mesh, sel, smoothing)| {
            let color = if sel { SELECTED_TINT } else { palette.object };
            let buffers = MeshBuffers::with_device_no_points(
                &gpu.device,
                &mesh,
                &Selection::default(),
                smoothing,
            );
            (buffers, color)
        })
        .collect();
    let draw: Vec<DrawMesh> = buffers
        .iter()
        .map(|(mesh, color)| DrawMesh {
            mesh,
            color: *color,
        })
        .collect();
    let grid = grid_lines(&camera, f64::from(height));
    let frame = Frame {
        meshes: &draw,
        grid: Some(&grid),
        gizmo: None,
        settings: Settings {
            shading,
            palette,
            ..Settings::default()
        },
    };
    let rgba = render_to_rgba(gpu, width, height, &camera, &frame);
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
