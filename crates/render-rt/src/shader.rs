//! Assembling the raster module: the frame constants, the compiled materials and `rt.wgsl`
//! (docs/specs/RENDER.md R21).

use forge_material::ir::ShadingGraph;
use forge_render_api::RenderError;

/// The passes of R21.2–R21.5.
pub const RT_WGSL: &str = include_str!("rt.wgsl");

/// The `F_*` constants of the frame uniform, generated from [`crate::frame`].
pub fn frame_constants() -> String {
    let mut s = String::new();
    for (name, value) in crate::frame::ALL {
        s.push_str(&format!("const F_{name}: u32 = {value}u;\n"));
    }
    s
}

/// The whole module for `graphs`: the shared IR surface functions plus every pass.
pub fn assemble(graphs: &[&ShadingGraph]) -> Result<String, RenderError> {
    let mut out = forge_render_gpu::shader::materials(graphs)?;
    out.push_str(&frame_constants());
    out.push('\n');
    out.push_str(RT_WGSL);
    Ok(out)
}
