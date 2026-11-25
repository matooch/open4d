//! Assembling the WGSL module: the frame constants, the compiled materials and the tracer
//! (docs/specs/RENDER.md R20.1).

use forge_material::ir::{IrNode, ShadingGraph};
use forge_material::wgsl::{SURFACE_WGSL, compile};
use forge_render_api::RenderError;

/// The `F_*` constants of the frame uniform, generated from [`crate::frame`].
pub fn frame_constants() -> String {
    let mut s = String::new();
    for (name, value) in crate::frame::ALL {
        s.push_str(&format!("const F_{name}: u32 = {value}u;\n"));
    }
    s
}

/// True when a material needs a shader node, which the codegen does not port yet.
pub fn needs_shader_nodes(g: &ShadingGraph) -> bool {
    g.nodes.iter().any(|n| matches!(n, IrNode::Texture(_)))
}

/// Renames one compiled material so several can share a module.
fn rename(src: &str, i: usize) -> String {
    let mut out = src.replace("fn forge_surface(", &format!("fn forge_surface_{i}("));
    for k in 0..64 {
        out = out.replace(&format!("ir{k}(ctx)"), &format!("ir{i}_{k}(ctx)"));
        out = out.replace(&format!("fn ir{k}("), &format!("fn ir{i}_{k}("));
    }
    out
}

/// The material dispatcher `forge_shade(mat, ctx)`.
fn dispatch(n: usize) -> String {
    if n == 0 {
        return "fn forge_shade(mat: u32, ctx: ShadeCtx) -> Surface {\n    \
                var s: Surface;\n    s.alpha = vec4<f32>(1.0, 1.0, 1.0, 0.0);\n    \
                return s;\n}\n"
            .to_string();
    }
    let mut s = String::from("fn forge_shade(mat: u32, ctx: ShadeCtx) -> Surface {\n");
    s.push_str("    switch mat {\n");
    for i in 0..n {
        s.push_str(&format!(
            "        case {i}u: {{ return forge_surface_{i}(ctx); }}\n"
        ));
    }
    s.push_str("        default: { return forge_surface_0(ctx); }\n");
    s.push_str("    }\n}\n");
    s
}

/// The whole compute module for `graphs`.
pub fn assemble(graphs: &[&ShadingGraph]) -> Result<String, RenderError> {
    if graphs.iter().any(|g| needs_shader_nodes(g)) {
        return Err(RenderError::Unsupported(
            "shader-node material channels on the GPU (REALTIME.md §2, brief 207)".into(),
        ));
    }
    let compiled: Vec<String> = graphs.iter().map(|g| compile(g).source).collect();
    let mut out = String::from(SURFACE_WGSL);
    for (i, src) in compiled.iter().enumerate() {
        let body = src.strip_prefix(SURFACE_WGSL).unwrap_or(src);
        out.push_str(&rename(body, i));
    }
    out.push_str(&dispatch(compiled.len()));
    out.push_str(&frame_constants());
    out.push_str(crate::RNG_WGSL);
    out.push('\n');
    out.push_str(crate::TRACE_WGSL);
    Ok(out)
}
