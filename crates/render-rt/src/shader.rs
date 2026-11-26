//! Assembling the raster module: the frame constants, the compiled materials and `rt.wgsl`
//! (docs/specs/RENDER.md R21).

use forge_material::TextureSampler;
use forge_material::ir::ShadingGraph;
use forge_render_api::RenderError;
use forge_render_gpu::shader::Assembled;

/// The passes of R21.2–R21.5.
pub const RT_WGSL: &str = include_str!("rt.wgsl");
pub(crate) const INSTANCE_GEOMETRY_WGSL: &str = include_str!("instance_geometry.wgsl");

/// Raw R24.7 receiver helpers, with the attribute table at group 0, binding 0.
pub fn receiver_geometry_wgsl() -> String {
    let geometry = forge_render_gpu::GEOMETRY_WGSL;
    let receiver = include_str!("receiver_geometry.wgsl");
    format!(
        "{geometry}\n@group(0) @binding(0) var<storage, read> path_attributes: array<vec4<f32>>;\n\
         {INSTANCE_GEOMETRY_WGSL}\n{receiver}"
    )
}

/// Raw R24.6 visibility and reconstruction; spatial and temporal filtering are separate.
pub fn ambient_visibility_wgsl(backend: crate::ray::BackendKind) -> String {
    let (directives, scene) = crate::ray::inline_ray_scene(backend);
    let rng = forge_render_gpu::RNG_WGSL;
    let ao = include_str!("ambient_visibility.wgsl");
    format!("{directives}\n{scene}\n{rng}\n{ao}")
}

/// Raw R24.7 direct-shadow signal before spatial/temporal filtering.
pub fn direct_shadow_wgsl(
    backend: crate::ray::BackendKind,
    materials: &[(&ShadingGraph, &dyn TextureSampler)],
) -> Result<Assembled, RenderError> {
    use forge_render_gpu::{GEOMETRY_WGSL, LIGHT_SAMPLING_WGSL, RNG_WGSL, SURFACE_WGSL};
    let (directives, scene) = crate::ray::inline_ray_scene(backend);
    let mut assembled = forge_render_gpu::shader::materials(materials, 1)?;
    let receiver = include_str!("receiver_geometry.wgsl");
    let shadow = include_str!("direct_shadow.wgsl");
    assembled.source = format!(
        "{directives}\n{}\n{RNG_WGSL}\n{GEOMETRY_WGSL}\n{SURFACE_WGSL}\n{LIGHT_SAMPLING_WGSL}\n\
         {scene}\n{INSTANCE_GEOMETRY_WGSL}\n{receiver}\n{shadow}",
        assembled.source
    );
    Ok(assembled)
}

/// The `F_*` constants of the frame uniform, generated from [`crate::frame`].
pub fn frame_constants() -> String {
    let mut s = String::new();
    for (name, value) in crate::frame::ALL {
        s.push_str(&format!("const F_{name}: u32 = {value}u;\n"));
    }
    s
}

/// The `@group` the shader-node bitmaps bind at: groups 0 and 1 are the frame/geometry and the
/// lighting pass's textures (R21.3).
pub const BITMAP_GROUP: u32 = 2;

/// The whole module for `materials` (graph and the sampler that ports its shader nodes): the
/// shared material code of `render_gpu::shader::materials` (R21.3), the frame constants and
/// every pass. `resources` entry `k` is bound at `@group(2) @binding(k)`.
pub fn assemble(
    materials: &[(&ShadingGraph, &dyn TextureSampler)],
) -> Result<Assembled, RenderError> {
    let mut out = forge_render_gpu::shader::materials(materials, BITMAP_GROUP)?;
    out.source.push_str(&frame_constants());
    out.source.push('\n');
    out.source.push_str(RT_WGSL);
    Ok(out)
}
