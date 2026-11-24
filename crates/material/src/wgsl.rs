//! Shading IR to WGSL (docs/specs/REALTIME.md §1): one specialised `forge_surface`
//! function per material, plus a cache keyed by the IR hash. Shader-node channels compile
//! to the documented fallback until brief 207.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::sync::Arc;

use crate::ir::{DiffuseModel, IrNode, MixMode, ShadingGraph};

/// Bumped whenever the emitted source changes; part of the cache key.
pub const CODEGEN_VERSION: u32 = 1;

/// The colour a shader-node (`Texture`) channel compiles to in part 1.
pub const FALLBACK_COLOR: [f64; 3] = [0.8, 0.8, 0.8];

/// One generated module.
#[derive(Debug, Clone)]
pub struct WgslMaterial {
    pub source: String,
    /// The cache key (codegen version plus the canonical IR).
    pub hash: u64,
    /// Shader-node names that compiled to [`FALLBACK_COLOR`] (brief 207 replaces them).
    pub fallbacks: Vec<String>,
}

/// The material surface, mirroring [`crate::ir::Outputs`] (REALTIME.md §1.1).
pub const SURFACE_WGSL: &str = r#"
struct ShadeCtx {
    uvw: vec3<f32>,
    position: vec3<f32>,
    normal: vec3<f32>,
    view: vec3<f32>,
    time: f32,
};
struct Surface {
    diffuse: vec4<f32>,
    diffuse_params: vec4<f32>,
    specular: vec4<f32>,
    specular_params: vec4<f32>,
    specular_flags: vec4<f32>,
    emission: vec4<f32>,
    transmission: vec4<f32>,
    transmission_params: vec4<f32>,
    transmission_flags: vec4<f32>,
    transmission_present: vec4<f32>,
    environment: vec4<f32>,
    fog: vec4<f32>,
    fog_present: vec4<f32>,
    alpha: vec4<f32>,
    alpha_params: vec4<f32>,
    alpha_flags: vec4<f32>,
    bump: vec4<f32>,
    bump_present: vec4<f32>,
    normal: vec4<f32>,
    normal_params: vec4<f32>,
    displacement: vec4<f32>,
    displacement_params: vec4<f32>,
    diffusion: vec4<f32>,
    diffusion_present: vec4<f32>,
};
fn forge_mix(base: vec3<f32>, top: vec3<f32>, mode: u32, strength: f32) -> vec3<f32> {
    if (mode == 0u) {
        return mix(base, top, strength);
    }
    var op = top;
    if (mode == 1u) {
        op = base + top;
    } else if (mode == 2u) {
        op = base - top;
    } else if (mode == 3u) {
        op = base * top;
    }
    return base + (op - base) * strength;
}
"#;

fn f(v: f64) -> String {
    let v = v as f32;
    if v.is_finite() {
        format!("{v:?}")
    } else {
        "0.0".into()
    }
}

fn vec3(c: [f64; 3]) -> String {
    format!("vec3<f32>({}, {}, {})", f(c[0]), f(c[1]), f(c[2]))
}

fn mix_mode(mode: MixMode) -> u32 {
    match mode {
        MixMode::Normal => 0,
        MixMode::Add => 1,
        MixMode::Subtract => 2,
        MixMode::Multiply => 3,
    }
}

fn specular_kind(kind: &str) -> u32 {
    match kind {
        "beckmann" => 0,
        "ggx" => 1,
        "phong" => 2,
        "ward" => 3,
        "anisotropic" => 4,
        "lambertian" => 5,
        "oren_nayar" => 6,
        "irawan" => 7,
        "reflection_legacy" => 8,
        "blinn_legacy" => 9,
        "phong_legacy" => 10,
        _ => 0,
    }
}

fn normal_method(method: &str) -> u32 {
    match method {
        "tangent" => 0,
        "object" => 1,
        "world" => 2,
        _ => 0,
    }
}

fn displacement_kind(kind: &str) -> u32 {
    match kind {
        "intensity" => 0,
        "intensity_centered" => 1,
        "red_green" => 2,
        "rgb_tangent" => 3,
        "rgb_object" => 4,
        "rgb_world" => 5,
        _ => 0,
    }
}

/// The key of a graph: codegen version plus its canonical debug form.
pub fn hash_graph(graph: &ShadingGraph) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    let mut feed = |bytes: &[u8]| {
        for b in bytes {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0000_0100_0000_01b3);
        }
    };
    feed(&CODEGEN_VERSION.to_le_bytes());
    feed(format!("{graph:?}").as_bytes());
    h
}

/// Compiles the IR to a WGSL module.
pub fn compile(graph: &ShadingGraph) -> WgslMaterial {
    let mut out = String::from(SURFACE_WGSL);
    let mut fallbacks = Vec::new();
    for (ix, node) in graph.nodes.iter().enumerate() {
        let (expr, fallback) = match node {
            IrNode::Color(c) => (vec3(*c), None),
            IrNode::Texture(name) => (vec3(FALLBACK_COLOR), Some(name.clone())),
            IrNode::Scale { input, factor } => (format!("ir{input}(ctx) * {}", f(*factor)), None),
            IrNode::Mix {
                base,
                top,
                mode,
                strength,
            } => (
                format!(
                    "forge_mix(ir{base}(ctx), ir{top}(ctx), {}u, {})",
                    mix_mode(*mode),
                    f(*strength)
                ),
                None,
            ),
        };
        if let Some(name) = fallback
            && !fallbacks.contains(&name)
        {
            fallbacks.push(name);
        }
        let _ = writeln!(
            out,
            "fn ir{ix}(ctx: ShadeCtx) -> vec3<f32> {{ return {expr}; }}"
        );
    }
    let eval = |node: Option<usize>| match node {
        Some(ix) => format!("ir{ix}(ctx)"),
        None => "vec3<f32>(0.0)".into(),
    };
    out.push_str("fn forge_surface(ctx: ShadeCtx) -> Surface {\n    var s: Surface;\n");
    out.push_str("    s.alpha = vec4<f32>(1.0, 1.0, 1.0, 0.0);\n");
    let o = &graph.outputs;
    if let Some(d) = &o.diffuse {
        let model = match d.model {
            DiffuseModel::Lambertian => 0u32,
            DiffuseModel::OrenNayar => 1,
        };
        let _ = writeln!(
            out,
            "    s.diffuse = vec4<f32>({}, {});",
            eval(Some(d.color)),
            f(d.level)
        );
        let _ = writeln!(
            out,
            "    s.diffuse_params = vec4<f32>({}, {}, {}.0, 1.0);",
            f(d.falloff),
            f(d.roughness),
            model
        );
    }
    if let Some(sp) = &o.specular {
        let _ = writeln!(
            out,
            "    s.specular = vec4<f32>({}, {});",
            eval(Some(sp.color)),
            f(sp.strength)
        );
        let _ = writeln!(
            out,
            "    s.specular_params = vec4<f32>({}, {}, {}, {}.0);",
            f(sp.width),
            f(sp.falloff),
            f(sp.inner_width),
            u32::from(sp.metal)
        );
        let _ = writeln!(
            out,
            "    s.specular_flags = vec4<f32>(1.0, {}.0, {}.0, 0.0);",
            specular_kind(&sp.kind),
            u32::from(sp.colored)
        );
    }
    if let Some(e) = o.emission {
        let _ = writeln!(out, "    s.emission = vec4<f32>({}, 1.0);", eval(Some(e)));
    }
    if let Some(t) = &o.transmission {
        let _ = writeln!(
            out,
            "    s.transmission = vec4<f32>({}, {});",
            eval(Some(t.color)),
            f(t.ior)
        );
        let _ = writeln!(
            out,
            "    s.transmission_params = vec4<f32>({}, {}, {}, {});",
            f(t.absorption[0]),
            f(t.absorption[1]),
            f(t.absorption[2]),
            f(t.absorption_distance)
        );
        let _ = writeln!(
            out,
            "    s.transmission_flags = vec4<f32>({}, {}.0, {}.0, {}.0);",
            f(t.fresnel_reflectivity),
            u32::from(t.total_internal_reflection),
            u32::from(t.exit_reflections),
            u32::from(t.additive)
        );
        out.push_str("    s.transmission_present = vec4<f32>(1.0, 0.0, 0.0, 0.0);\n");
    }
    if let Some(e) = o.environment {
        let _ = writeln!(
            out,
            "    s.environment = vec4<f32>({}, 1.0);",
            eval(Some(e))
        );
    }
    if let Some(fo) = &o.fog {
        let _ = writeln!(
            out,
            "    s.fog = vec4<f32>({}, {});",
            eval(Some(fo.color)),
            f(fo.distance)
        );
        out.push_str("    s.fog_present = vec4<f32>(1.0, 0.0, 0.0, 0.0);\n");
    }
    if let Some(a) = &o.alpha {
        let _ = writeln!(
            out,
            "    s.alpha = vec4<f32>({}, {}, {}, {}.0);",
            f(a.color[0]),
            f(a.color[1]),
            f(a.color[2]),
            u32::from(a.soft)
        );
        let _ = writeln!(
            out,
            "    s.alpha_params = vec4<f32>({}, {}, {}, {}.0);",
            f(a.delta[0]),
            f(a.delta[1]),
            f(a.delta[2]),
            u32::from(a.invert)
        );
        let _ = writeln!(
            out,
            "    s.alpha_flags = vec4<f32>(1.0, {}.0, {}.0, 0.0);",
            u32::from(a.image_alpha),
            u32::from(a.premultiplied)
        );
    }
    if let Some(b) = &o.bump {
        let _ = writeln!(
            out,
            "    s.bump = vec4<f32>({}, {});",
            eval(Some(b.texture)),
            f(b.strength)
        );
        out.push_str("    s.bump_present = vec4<f32>(1.0, 0.0, 0.0, 0.0);\n");
    }
    if let Some(n) = &o.normal {
        let _ = writeln!(
            out,
            "    s.normal = vec4<f32>({}, {});",
            eval(Some(n.texture)),
            f(n.strength)
        );
        let flips =
            u32::from(n.flip[0]) | (u32::from(n.flip[1]) << 1) | (u32::from(n.flip[2]) << 2);
        let _ = writeln!(
            out,
            "    s.normal_params = vec4<f32>(1.0, {}.0, {}.0, {}.0);",
            normal_method(&n.method),
            flips,
            u32::from(n.swap_yz)
        );
    }
    if let Some(d) = &o.displacement {
        let _ = writeln!(
            out,
            "    s.displacement = vec4<f32>({}, {});",
            eval(Some(d.texture)),
            f(d.strength)
        );
        let _ = writeln!(
            out,
            "    s.displacement_params = vec4<f32>(1.0, {}, {}.0, 0.0);",
            f(d.height),
            displacement_kind(&d.kind)
        );
    }
    if let Some(d) = &o.diffusion {
        let _ = writeln!(
            out,
            "    s.diffusion = vec4<f32>({}, {}.0, {}.0, {}.0);",
            f(d.brightness),
            u32::from(d.affect_luminance),
            u32::from(d.affect_specular),
            u32::from(d.affect_reflection)
        );
        out.push_str("    s.diffusion_present = vec4<f32>(1.0, 0.0, 0.0, 0.0);\n");
    }
    out.push_str("    return s;\n}\n");
    WgslMaterial {
        source: out,
        hash: hash_graph(graph),
        fallbacks,
    }
}

/// A per-session cache of compiled IR modules.
#[derive(Default)]
pub struct ShaderCache {
    modules: HashMap<u64, Arc<WgslMaterial>>,
    compiles: u64,
}

impl ShaderCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// The module for `graph`, compiling it once per hash. The bool is true when a new
    /// module was compiled.
    pub fn module(&mut self, graph: &ShadingGraph) -> (Arc<WgslMaterial>, bool) {
        let key = hash_graph(graph);
        if let Some(m) = self.modules.get(&key) {
            return (m.clone(), false);
        }
        let m = Arc::new(compile(graph));
        self.modules.insert(key, m.clone());
        self.compiles += 1;
        (m, true)
    }

    /// Number of compilations (the tests check edits compile only one material).
    pub fn compiles(&self) -> u64 {
        self.compiles
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}
