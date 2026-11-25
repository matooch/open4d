//! Assembling the WGSL module: the frame constants, the compiled materials and the tracer
//! (docs/specs/RENDER.md R20.1).

use std::any::Any;
use std::sync::Arc;

use forge_material::TextureSampler;
use forge_material::ir::{IrNode, ShadingGraph};
use forge_material::wgsl::{ModuleBuilder, NoShaders, SURFACE_WGSL, compile_into};
use forge_render_api::RenderError;

/// The assembled compute module and the bitmaps its `@group(1)` bindings need.
pub struct Assembled {
    pub source: String,
    /// Entry `k` is bound at `@group(1) @binding(k)` (REALTIME.md §2.8).
    pub resources: Vec<Arc<dyn Any + Send + Sync>>,
}

/// The `F_*` constants of the frame uniform, generated from [`crate::frame`].
pub fn frame_constants() -> String {
    let mut s = String::new();
    for (name, value) in crate::frame::ALL {
        s.push_str(&format!("const F_{name}: u32 = {value}u;\n"));
    }
    s
}

/// Renames one compiled material so several can share a module.
fn rename(src: &str, i: usize, nodes: usize) -> String {
    let mut out = src.replace("fn forge_surface(", &format!("fn forge_surface_{i}("));
    for k in 0..nodes {
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

/// The whole compute module for `materials` (graph and the sampler that ports its shader
/// nodes, R20.1b): every material is compiled into one shared [`ModuleBuilder`].
pub fn assemble(
    materials: &[(&ShadingGraph, &dyn TextureSampler)],
) -> Result<Assembled, RenderError> {
    let mut module = ModuleBuilder::default();
    let mut compiled: Vec<String> = Vec::new();
    for (g, sampler) in materials {
        let has_texture = g.nodes.iter().any(|n| matches!(n, IrNode::Texture(_)));
        let lib = sampler.codegen();
        if has_texture && lib.is_none() {
            return Err(RenderError::Unsupported(
                "texture sampler without a WGSL codegen (RENDER.md R20.1b)".into(),
            ));
        }
        let (src, _) = match &lib {
            Some(l) => compile_into(g, l.as_ref(), &mut module),
            None => compile_into(g, &NoShaders, &mut module),
        };
        let body = src.strip_prefix(SURFACE_WGSL).unwrap_or(&src);
        compiled.push(rename(body, compiled.len(), g.nodes.len()));
    }
    let mut out = String::from(SURFACE_WGSL);
    for body in &compiled {
        out.push_str(body);
    }
    out.push_str(module.declarations());
    out.push_str(&dispatch(compiled.len()));
    out.push_str(&frame_constants());
    out.push_str(crate::RNG_WGSL);
    out.push('\n');
    out.push_str(crate::TRACE_WGSL);
    Ok(Assembled {
        source: out,
        resources: module.resources().to_vec(),
    })
}

/// Whether a material graph reads shader nodes (an IR `Texture` node).
pub fn needs_shader_nodes(g: &ShadingGraph) -> bool {
    g.nodes.iter().any(|n| matches!(n, IrNode::Texture(_)))
}

/// The surface struct and every material as `forge_surface_<i>` plus the `forge_shade`
/// dispatcher, for a WGSL module that shades through the IR without shader-node bindings
/// (R20.1, R21.3: the real-time raster engine). Materials with shader nodes are
/// `Unsupported` here; the path tracer compiles them through [`assemble`] (R20.1b).
pub fn materials(graphs: &[&ShadingGraph]) -> Result<String, RenderError> {
    if graphs.iter().any(|g| needs_shader_nodes(g)) {
        return Err(RenderError::Unsupported(
            "shader-node material channels in this engine (R21; the path tracer has them, R20.1b)"
                .into(),
        ));
    }
    let mut module = ModuleBuilder::default();
    let mut out = String::from(SURFACE_WGSL);
    let mut n = 0;
    for g in graphs {
        let (src, _) = compile_into(g, &NoShaders, &mut module);
        let body = src.strip_prefix(SURFACE_WGSL).unwrap_or(&src);
        out.push_str(&rename(body, n, g.nodes.len()));
        n += 1;
    }
    out.push_str(module.declarations());
    out.push_str(&dispatch(n));
    Ok(out)
}
