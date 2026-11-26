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

/// Compiles every material into one shared [`ModuleBuilder`]: the surface struct, each
/// `forge_surface_<i>`, the library and generated shader functions, the `forge_shade`
/// dispatcher, and the bitmaps the declarations bind (R20.1b, R21.3). Both GPU engines build
/// their materials here. The codegen declares bitmap `k` at `@group(1) @binding(k)`; the
/// declarations are moved to `bitmap_group` when an engine uses group 1 itself.
pub fn materials(
    materials: &[(&ShadingGraph, &dyn TextureSampler)],
    bitmap_group: u32,
) -> Result<Assembled, RenderError> {
    let mut module = ModuleBuilder::default();
    let mut compiled: Vec<String> = Vec::new();
    for (g, sampler) in materials {
        let has_texture = needs_shader_nodes(g);
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
    out.push_str(&module.declarations().replace(
        "@group(1) @binding(",
        &format!("@group({bitmap_group}) @binding("),
    ));
    out.push_str(&dispatch(compiled.len()));
    Ok(Assembled {
        source: out,
        resources: module.resources().to_vec(),
    })
}

/// The whole compute module for `materials` (graph and the sampler that ports its shader
/// nodes, R20.1b): the shared material code, the frame constants and the tracer.
pub fn assemble(
    materials_in: &[(&ShadingGraph, &dyn TextureSampler)],
) -> Result<Assembled, RenderError> {
    assemble_with_scene(materials_in, "", include_str!("flat_scene.wgsl"))
}

/// Builds the R20 transport with a replacement scene module (R25.1).
///
/// `directives` contains WGSL enable directives, if any. `scene` provides
/// `scene_intersect`, `scene_occluded`, `scene_geometry` and `scene_ids`
/// (material index and object ID, in that order), plus their scene bindings.
pub fn assemble_with_scene(
    materials_in: &[(&ShadingGraph, &dyn TextureSampler)],
    directives: &str,
    scene: &str,
) -> Result<Assembled, RenderError> {
    let mut assembled = materials(materials_in, 1)?;
    if !directives.is_empty() {
        assembled.source.insert(0, '\n');
        assembled.source.insert_str(0, directives);
    }
    assembled.source.push_str(&frame_constants());
    assembled.source.push_str(crate::RNG_WGSL);
    assembled.source.push('\n');
    assembled.source.push_str(crate::LIGHT_SAMPLING_WGSL);
    assembled.source.push('\n');
    assembled.source.push_str(crate::GEOMETRY_WGSL);
    assembled.source.push('\n');
    assembled.source.push_str(include_str!("trace.wgsl"));
    assembled.source.push('\n');
    assembled.source.push_str(scene);
    Ok(assembled)
}

/// Whether a material graph reads shader nodes (an IR `Texture` node).
pub fn needs_shader_nodes(g: &ShadingGraph) -> bool {
    g.nodes.iter().any(|n| matches!(n, IrNode::Texture(_)))
}
