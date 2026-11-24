//! Shader nodes to WGSL (docs/specs/REALTIME.md §2). `ShaderSetCodegen` plugs a
//! [`ShaderSet`] into `forge_material::wgsl::compile_with`: every shader tree becomes
//! generated functions that call one library function per family (emitted once per module).

use std::fmt::Write as _;

use forge_material::ir::ShadingGraph;
use forge_material::wgsl::{ModuleBuilder, TextureCodegen, WgslMaterial, compile_with};
use forge_noise::NoiseKind;

use crate::blend::BlendMode;
use crate::gradient::{Gradient, Interp};
use crate::shader::{
    ColorizeInput, GradientKind, GradientShader, NoiseShader, NoiseSpace, Shader, ShaderSet, Tiles,
};
use crate::wgsl_noise::{NOISE_WGSL, U64_WGSL};

/// Colour ramps, smoothstep and tiles (REALTIME.md §2.3, §2.4).
const RAMP_WGSL: &str = r#"
fn forge_smooth(x: f32) -> f32 {
    let c = clamp(x, 0.0, 1.0);
    return c * c * (3.0 - 2.0 * c);
}
// interp: 0 none, 1 linear, 2 cubic (Gradient::eval)
fn forge_ramp_seg(c0: vec3<f32>, c1: vec3<f32>, p0: f32, p1: f32, t: f32, interp: u32) -> vec3<f32> {
    var x = 1.0;
    if (p1 > p0) { x = (t - p0) / (p1 - p0); }
    if (interp == 0u) {
        x = 0.0;
    } else if (interp == 2u) {
        x = x * x * (3.0 - 2.0 * x);
    }
    return c0 + (c1 - c0) * x;
}
"#;

/// Layer/Fusion blend and the HSL helpers of Filter (REALTIME.md §2.5, §2.6).
const BLEND_WGSL: &str = r#"
// mode: the BlendMode declaration index (normal 0 ... difference 8)
fn forge_blend(a: vec3<f32>, b: vec3<f32>, mode: u32, opacity: f32) -> vec3<f32> {
    var r = b;
    switch (mode) {
        case 1u: { r = a + b; }
        case 2u: { r = a - b; }
        case 3u: { r = a * b; }
        case 4u: { r = a + b - a * b; }
        case 5u: {
            r = select(vec3<f32>(1.0) - 2.0 * (vec3<f32>(1.0) - a) * (vec3<f32>(1.0) - b),
                       2.0 * a * b, a < vec3<f32>(0.5));
        }
        case 6u: { r = min(a, b); }
        case 7u: { r = max(a, b); }
        case 8u: { r = abs(a - b); }
        default: {}
    }
    return a + (r - a) * opacity;
}
"#;

const HSL_WGSL: &str = r#"
fn forge_rgb_to_hsl(c: vec3<f32>) -> vec3<f32> {
    let mx = max(c.x, max(c.y, c.z));
    let mn = min(c.x, min(c.y, c.z));
    let l = (mx + mn) * 0.5;
    if (mx == mn) { return vec3<f32>(0.0, 0.0, l); }
    let d = mx - mn;
    var s = d / (mx + mn);
    if (l > 0.5) { s = d / (2.0 - mx - mn); }
    var h = 0.0;
    if (mx == c.x) {
        h = (c.y - c.z) / d + select(0.0, 6.0, c.y < c.z);
    } else if (mx == c.y) {
        h = (c.z - c.x) / d + 2.0;
    } else {
        h = (c.x - c.y) / d + 4.0;
    }
    return vec3<f32>(h / 6.0, s, l);
}
fn forge_hue_channel(p: f32, q: f32, t0: f32) -> f32 {
    let t = t0 - floor(t0);
    if (t < 1.0 / 6.0) { return p + (q - p) * 6.0 * t; }
    if (t < 0.5) { return q; }
    if (t < 2.0 / 3.0) { return p + (q - p) * (2.0 / 3.0 - t) * 6.0; }
    return p;
}
fn forge_hsl_to_rgb(hsl: vec3<f32>) -> vec3<f32> {
    let l = hsl.z;
    if (hsl.y == 0.0) { return vec3<f32>(l); }
    var q = l + hsl.y - l * hsl.y;
    if (l < 0.5) { q = l * (1.0 + hsl.y); }
    let p = 2.0 * l - q;
    return vec3<f32>(
        forge_hue_channel(p, q, hsl.x + 1.0 / 3.0),
        forge_hue_channel(p, q, hsl.x),
        forge_hue_channel(p, q, hsl.x - 1.0 / 3.0));
}
"#;

const TILES_WGSL: &str = r#"
// Shader::Tiles cell_hash(seed, i, j) % 3
fn forge_tile_pick(seed: u32, i: i32, j: i32) -> u32 {
    var x = vec2<u32>(0u, seed)
        ^ forge_u64_mul(forge_sext(i), vec2<u32>(0x7f4a7c15u, 0x9e3779b9u))
        ^ forge_u64_mul(forge_sext(j), vec2<u32>(0x27d4eb4fu, 0xc2b2ae3du));
    x = x ^ forge_u64_shr_hi(x, 33u);
    x = forge_u64_mul(x, vec2<u32>(0xed558ccdu, 0xff51afd7u));
    x = x ^ forge_u64_shr_hi(x, 33u);
    // 2^32 = 1 (mod 3)
    return ((x.y % 3u) + (x.x % 3u)) % 3u;
}
fn forge_tiles(uv: vec2<f32>, pitch: vec2<f32>, grout_width: f32, bevel_width: f32,
               bevel: bool, randomize: bool, seed: u32,
               grout: vec3<f32>, c0: vec3<f32>, c1: vec3<f32>, c2: vec3<f32>) -> vec3<f32> {
    var p = 1.0;
    var cell = vec2<i32>(0);
    for (var k = 0u; k < 2u; k++) {
        let x = uv[k] / pitch[k];
        let c = floor(x);
        cell[k] = i32(c);
        let fr = x - c;
        let half = pitch[k] * 0.5;
        let d = min(fr, 1.0 - fr) * pitch[k];
        let g = grout_width * half;
        let b = bevel_width * (half - g);
        var pk = 1.0;
        if (bevel && b > 0.0) {
            pk = forge_smooth((d - g) / b);
        } else if (d < g) {
            pk = 0.0;
        }
        p = min(p, pk);
    }
    var tile = c1;
    if (randomize) {
        let pick = forge_tile_pick(seed, cell.x, cell.y);
        if (pick == 0u) { tile = c0; } else if (pick == 2u) { tile = c2; }
    } else if (((cell.x + cell.y) & 1) == 0) {
        tile = c0;
    }
    return grout + (tile - grout) * p;
}
"#;

/// Compiles `graph` with the shaders of `set` ported to WGSL.
pub fn compile(graph: &ShadingGraph, set: &ShaderSet) -> WgslMaterial {
    compile_with(graph, &ShaderSetCodegen { set })
}

/// The [`TextureCodegen`] of a [`ShaderSet`]; an unknown name is white (SHADERS.md S1).
pub struct ShaderSetCodegen<'a> {
    pub set: &'a ShaderSet,
}

impl TextureCodegen for ShaderSetCodegen<'_> {
    fn fingerprint(&self, name: &str) -> u64 {
        let mut h = Fnv::default();
        match self.set.shaders.get(name) {
            Some(s) => fingerprint(s, &mut h),
            None => h.feed(b"missing"),
        }
        h.0
    }

    fn emit(&self, name: &str, module: &mut ModuleBuilder) -> Option<String> {
        match self.set.shaders.get(name) {
            None => Some("vec3<f32>(1.0)".into()),
            Some(s) => emit_shader(s, module).map(|f| format!("{f}(ctx)")),
        }
    }
}

/// An `f32` literal of an `f64` parameter (the host computes in `f64`, the GPU runs `f32`).
fn f(v: f64) -> String {
    let v = v as f32;
    if v.is_finite() {
        format!("{v:?}")
    } else {
        "0.0".into()
    }
}

fn v3(c: [f64; 3]) -> String {
    format!("vec3<f32>({}, {}, {})", f(c[0]), f(c[1]), f(c[2]))
}

#[derive(Default)]
struct Fnv(u64);

impl Fnv {
    fn feed(&mut self, bytes: &[u8]) {
        if self.0 == 0 {
            self.0 = 0xcbf2_9ce4_8422_2325;
        }
        for b in bytes {
            self.0 ^= u64::from(*b);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

/// Hashes everything the generated code depends on (the whole tree, bitmap pixels included).
fn fingerprint(s: &Shader, h: &mut Fnv) {
    let mut text = |t: String| h.feed(t.as_bytes());
    match s {
        Shader::Noise(n) => text(format!("{n:?}")),
        _ => text(format!("{s:?}")),
    }
}

fn emit_shader(s: &Shader, m: &mut ModuleBuilder) -> Option<String> {
    let body = match s {
        Shader::Noise(n) => noise_body(n, m),
        Shader::Color { color, brightness } => {
            format!("    return {};", v3(color.map(|c| c * brightness)))
        }
        Shader::Gradient(g) => gradient_body(g, m),
        Shader::Fresnel {
            ior,
            invert,
            gradient,
        } => {
            let f0 = ((ior - 1.0) / (ior + 1.0)).powi(2);
            let ramp = ramp_fn(gradient, m);
            format!(
                "    let n = ctx.normal;\n    let v = ctx.view;\n    var f = forge_smooth(ctx.uvw.y);\n    if (dot(n, n) != 0.0 && dot(v, v) != 0.0) {{\n        let k = 1.0 - abs(dot(normalize(n), normalize(v)));\n        let k2 = k * k;\n        f = {f0} + (1.0 - {f0}) * (k2 * k2 * k);\n    }}\n    {}\n    return {ramp}(f);",
                if *invert { "f = 1.0 - f;" } else { "" },
                f0 = f(f0)
            )
        }
        Shader::Falloff { invert, gradient } => {
            let ramp = ramp_fn(gradient, m);
            format!(
                "    let n = ctx.normal;\n    let v = ctx.view;\n    var t = 0.0;\n    if (dot(n, n) != 0.0 && dot(v, v) != 0.0) {{\n        t = 1.0 - abs(dot(normalize(n), normalize(v)));\n    }}\n    {}\n    return {ramp}(t);",
                if *invert { "t = 1.0 - t;" } else { "" }
            )
        }
        Shader::Checkerboard {
            color_1,
            color_2,
            u_frequency,
            v_frequency,
        } => format!(
            "    let i = i32(floor(ctx.uvw.x * 2.0 * {}));\n    let j = i32(floor(ctx.uvw.y * 2.0 * {}));\n    if (((i + j) & 1) == 0) {{ return {}; }}\n    return {};",
            f(*u_frequency),
            f(*v_frequency),
            v3(*color_1),
            v3(*color_2)
        ),
        Shader::Tiles(t) => tiles_body(t, m),
        Shader::Layer { base, layers } => {
            m.once("blend", BLEND_WGSL);
            let mut b = format!("    var acc = {};\n", sub(base, m)?);
            for l in layers.iter().filter(|l| l.enabled) {
                let top = emit_shader(&l.shader, m)?;
                let _ = writeln!(
                    b,
                    "    acc = forge_blend(acc, {top}(ctx), {}u, {});",
                    blend_index(l.mode),
                    f(l.opacity)
                );
            }
            b.push_str("    return acc;");
            b
        }
        Shader::Fusion {
            base,
            blend,
            mode,
            opacity,
        } => {
            m.once("blend", BLEND_WGSL);
            let (base, blend) = (emit_shader(base, m)?, emit_shader(blend, m)?);
            format!(
                "    return forge_blend({base}(ctx), {blend}(ctx), {}u, {});",
                blend_index(*mode),
                f(*opacity)
            )
        }
        Shader::Colorizer {
            input,
            cycle,
            gradient,
            texture,
        } => {
            let ramp = ramp_fn(gradient, m);
            let c = sub(texture, m)?;
            let t = match input {
                ColorizeInput::Luminance => {
                    "0.2126 * c.x + 0.7152 * c.y + 0.0722 * c.z".to_string()
                }
                ColorizeInput::Red => "c.x".to_string(),
                ColorizeInput::Green => "c.y".to_string(),
                ColorizeInput::Blue => "c.z".to_string(),
            };
            let wrap = if *cycle {
                "t - floor(t)"
            } else {
                "clamp(t, 0.0, 1.0)"
            };
            format!("    let c = {c};\n    let t = {t};\n    return {ramp}({wrap});")
        }
        Shader::Filter {
            texture,
            hue,
            saturation,
            lightness,
            contrast,
            gamma,
            invert,
        } => {
            m.once("hsl", HSL_WGSL);
            let c = sub(texture, m)?;
            let mut b = format!(
                "    var hsl = forge_rgb_to_hsl({c});\n    hsl.x = hsl.x + {};\n    hsl.y = clamp(hsl.y * {}, 0.0, 1.0);\n    var c = forge_hsl_to_rgb(hsl);\n    c = c + vec3<f32>({});\n    c = (c - vec3<f32>(0.5)) * {} + vec3<f32>(0.5);\n    c = pow(max(c, vec3<f32>(0.0)), vec3<f32>({}));\n",
                f(*hue),
                f(1.0 + saturation),
                f(*lightness),
                f(1.0 + contrast),
                f(1.0 / gamma.max(1e-9))
            );
            if *invert {
                b.push_str("    c = vec3<f32>(1.0) - c;\n");
            }
            b.push_str("    return c;");
            b
        }
        Shader::Normalizer { texture, low, high } => {
            let c = sub(texture, m)?;
            format!(
                "    return clamp(({c} - vec3<f32>({})) / {}, vec3<f32>(0.0), vec3<f32>(1.0));",
                f(*low),
                f((high - low).max(1e-9))
            )
        }
        Shader::VertexMap { low, high } => format!(
            "    return {low} + ({high} - {low}) * clamp(ctx.vertex_weight, 0.0, 1.0);",
            low = v3(*low),
            high = v3(*high)
        ),
        Shader::Ao { low, high } => format!(
            "    return {low} + ({high} - {low}) * clamp(ctx.ao, 0.0, 1.0);",
            low = v3(*low),
            high = v3(*high)
        ),
        _ => return None,
    };
    Some(m.function("ctx: ShadeCtx", "vec3<f32>", &body))
}

fn noise_library() -> String {
    let nf = 4.0_f64;
    let skew = ((nf + 1.0).sqrt() - 1.0) / nf;
    let unskew = (1.0 - 1.0 / (nf + 1.0).sqrt()) / nf;
    NOISE_WGSL
        .replace("SIMPLEX_SKEW", &f(skew))
        .replace("SIMPLEX_UNSKEW", &f(unskew))
}

fn use_noise(m: &mut ModuleBuilder) {
    m.once("u64", U64_WGSL);
    m.once("noise", &noise_library());
}

fn noise_kind_index(kind: NoiseKind) -> usize {
    NoiseKind::ALL
        .iter()
        .position(|k| *k == kind)
        .expect("kind in ALL")
}

fn noise_body(n: &NoiseShader, m: &mut ModuleBuilder) -> String {
    use_noise(m);
    let p = match n.space {
        NoiseSpace::Texture | NoiseSpace::Uv => "ctx.uvw",
        NoiseSpace::Object | NoiseSpace::World => "ctx.position",
    };
    let scale = (0.1 * n.global_scale).max(1e-9);
    let span = (n.high_clip - n.low_clip).max(1e-9);
    let mut b = String::new();
    let _ = writeln!(
        b,
        "    let q = ({p} / {}) / {};",
        v3(n.relative_scale),
        f(scale)
    );
    let _ = writeln!(
        b,
        "    let n = forge_noise_eval({}u, {}u, {}u, vec4<f32>(q, {} * ctx.time));",
        noise_kind_index(n.kind),
        n.seed,
        n.octaves.max(1),
        f(n.animation_speed)
    );
    if n.absolute {
        b.push_str("    var v = abs(n);\n");
    } else {
        b.push_str("    var v = 0.5 + 0.5 * n;\n");
    }
    let _ = writeln!(
        b,
        "    v = clamp((v - {}) / {}, 0.0, 1.0);",
        f(n.low_clip),
        f(span)
    );
    let _ = writeln!(b, "    v = v + {};", f(n.brightness));
    let _ = writeln!(b, "    v = (v - 0.5) * {} + 0.5;", f(1.0 + n.contrast));
    b.push_str("    v = clamp(v, 0.0, 1.0);\n");
    let _ = write!(
        b,
        "    return {c1} + ({c2} - {c1}) * v;",
        c1 = v3(n.color_1),
        c2 = v3(n.color_2)
    );
    b
}

/// The ramp of `g` as a generated function `fn(t: f32) -> vec3<f32>` (knots unrolled,
/// `Gradient::eval`); returns its name.
fn ramp_fn(g: &Gradient, m: &mut ModuleBuilder) -> String {
    m.once("ramp", RAMP_WGSL);
    let k = &g.knots;
    let body = match k.len() {
        0 => "    return vec3<f32>(0.0);".to_string(),
        1 => format!("    return {};", v3(k[0].1)),
        n => {
            let interp = match g.interp {
                Interp::None => 0,
                Interp::Linear => 1,
                Interp::Cubic => 2,
            };
            let mut b = format!(
                "    if (t <= {}) {{ return {}; }}\n    if (t >= {}) {{ return {}; }}\n",
                f(k[0].0),
                v3(k[0].1),
                f(k[n - 1].0),
                v3(k[n - 1].1)
            );
            let seg = |i: usize| {
                format!(
                    "forge_ramp_seg({}, {}, {}, {}, t, {interp}u)",
                    v3(k[i].1),
                    v3(k[i + 1].1),
                    f(k[i].0),
                    f(k[i + 1].0)
                )
            };
            // the last knot with position <= t selects the segment
            for i in (1..n - 1).rev() {
                let _ = writeln!(b, "    if (t >= {}) {{ return {}; }}", f(k[i].0), seg(i));
            }
            let _ = write!(b, "    return {};", seg(0));
            b
        }
    };
    m.function("t: f32", "vec3<f32>", &body)
}

fn gradient_body(g: &GradientShader, m: &mut ModuleBuilder) -> String {
    let ramp = ramp_fn(&g.gradient, m);
    let two_d = matches!(
        g.kind,
        GradientKind::U
            | GradientKind::V
            | GradientKind::Diagonal
            | GradientKind::Circular
            | GradientKind::Radial
    );
    let mut b = String::new();
    let (a, r) = (v3(g.start), f(g.radius.max(1e-9)));
    let t = match g.kind {
        GradientKind::U => "ctx.uvw.x".to_string(),
        GradientKind::V => "1.0 - ctx.uvw.y".to_string(),
        GradientKind::Diagonal => "(ctx.uvw.x + 1.0 - ctx.uvw.y) * 0.5".to_string(),
        GradientKind::Circular => {
            let _ = writeln!(
                b,
                "    let c = vec2<f32>(ctx.uvw.x - 0.5, ctx.uvw.y - 0.5);"
            );
            format!(
                "sqrt(c.x * c.x + c.y * c.y) * {}",
                f(std::f64::consts::SQRT_2)
            )
        }
        GradientKind::Radial => format!(
            "atan2(ctx.uvw.y - 0.5, ctx.uvw.x - 0.5) / {} + 0.5",
            f(std::f64::consts::TAU)
        ),
        GradientKind::Linear3d => {
            let d = [
                g.end[0] - g.start[0],
                g.end[1] - g.start[1],
                g.end[2] - g.start[2],
            ];
            let l2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
            if l2 > 0.0 {
                format!("dot(ctx.position - {a}, {}) / {}", v3(d), f(l2))
            } else {
                "0.0".to_string()
            }
        }
        GradientKind::Spherical3d => format!("length(ctx.position - {a}) / {r}"),
        GradientKind::Cylindrical3d => {
            let _ = writeln!(b, "    let c = ctx.position - {a};");
            format!("sqrt(c.x * c.x + c.z * c.z) / {r}")
        }
    };
    let _ = writeln!(b, "    var t = {t};");
    if g.turbulence != 0.0 {
        use_noise(m);
        let q = if two_d { "ctx.uvw" } else { "ctx.position" };
        let _ = writeln!(
            b,
            "    t = t + {} * forge_noise_eval({}u, {}u, {}u, vec4<f32>({q} / {}, 0.0));",
            f(g.turbulence),
            noise_kind_index(NoiseKind::Fbm),
            g.seed,
            g.octaves.max(1),
            f((0.1 * g.scale).max(1e-9))
        );
    }
    if g.cycle {
        b.push_str("    t = t - floor(t);\n");
    } else {
        b.push_str("    t = clamp(t, 0.0, 1.0);\n");
    }
    let _ = write!(b, "    return {ramp}(t);");
    b
}

fn tiles_body(t: &Tiles, m: &mut ModuleBuilder) -> String {
    m.once("u64", U64_WGSL);
    m.once("ramp", RAMP_WGSL);
    m.once("tiles", TILES_WGSL);
    format!(
        "    return forge_tiles(ctx.uvw.xy, vec2<f32>({}, {}), {}, {}, {}, {}, {}u, {}, {}, {}, {});",
        f(t.pitch[0]),
        f(t.pitch[1]),
        f(t.grout_width),
        f(t.bevel_width),
        t.bevel,
        t.randomize,
        t.seed,
        v3(t.grout_color),
        v3(t.colors[0]),
        v3(t.colors[1]),
        v3(t.colors[2])
    )
}

fn blend_index(mode: BlendMode) -> u32 {
    match mode {
        BlendMode::Normal => 0,
        BlendMode::Add => 1,
        BlendMode::Subtract => 2,
        BlendMode::Multiply => 3,
        BlendMode::Screen => 4,
        BlendMode::Overlay => 5,
        BlendMode::Darken => 6,
        BlendMode::Lighten => 7,
        BlendMode::Difference => 8,
    }
}

/// The colour of an optional child shader: black without one (the CPU `sub`).
fn sub(child: &Option<Box<Shader>>, m: &mut ModuleBuilder) -> Option<String> {
    match child {
        Some(c) => emit_shader(c, m).map(|f| format!("{f}(ctx)")),
        None => Some("vec3<f32>(0.0)".into()),
    }
}
