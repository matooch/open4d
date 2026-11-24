//! Shader nodes to WGSL (docs/specs/REALTIME.md §2). `ShaderSetCodegen` plugs a
//! [`ShaderSet`] into `forge_material::wgsl::compile_with`: every shader tree becomes
//! generated functions that call one library function per family (emitted once per module).

use std::fmt::Write as _;

use forge_material::ir::ShadingGraph;
use forge_material::wgsl::{ModuleBuilder, TextureCodegen, WgslMaterial, compile_with};
use forge_noise::NoiseKind;

use crate::shader::{NoiseShader, NoiseSpace, Shader, ShaderSet};
use crate::wgsl_noise::{NOISE_WGSL, U64_WGSL};

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
