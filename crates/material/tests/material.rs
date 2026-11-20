//! Standard material, texture tag and IR (docs/specs/MATERIAL.md).

use std::collections::BTreeMap;

use forge_core::{Registry, Value};
use forge_material::{
    IrNode, Material, MaterialPlugin, MixMode, ShadeCtx, TextureSampler, compile, mix,
};

struct Const(f64);
impl TextureSampler for Const {
    fn sample(&self, _: &str, _: &ShadeCtx) -> [f64; 3] {
        [self.0; 3]
    }
}

#[test]
fn registers_both_types() {
    let mut r = Registry::with_builtins();
    r.install(&MaterialPlugin).unwrap();
    assert!(r.get("forge.material.standard").is_some());
    assert!(r.get("forge.tag.texture").is_some());
}

#[test]
fn defaults_match_o29() {
    let m = Material::new();
    assert_eq!(m.c("icon_color"), [1.0, 0.9, 0.4]);
    assert_eq!(m.c("color.color"), [0.8; 3]);
    assert!(m.enabled("color") && m.enabled("reflectance"));
    for c in [
        "diffusion",
        "luminance",
        "transparency",
        "environment",
        "fog",
        "bump",
        "normal",
        "alpha",
        "glow",
        "displacement",
    ] {
        assert!(!m.enabled(c), "{c}");
    }
    assert_eq!(m.f("color.brightness"), 1.0);
    assert_eq!(m.f("color.roughness"), 0.5);
    assert_eq!(m.f("transparency.refraction"), 1.517);
    assert_eq!(m.e("transparency.refraction_preset"), "glass");
    assert_eq!(m.e("reflectance.spec.type"), "blinn_legacy");
    assert_eq!(m.f("reflectance.spec.strength"), 0.2);
    assert_eq!(m.f("reflectance.spec.width"), 0.5);
    assert_eq!(m.e("environment.mix_mode"), "multiply");
    assert_eq!(m.f("bump.strength"), 0.2);
    assert_eq!(m.f("glow.outer_strength"), 5.0);
    assert_eq!(m.f("displacement.height"), 10.0);
    assert_eq!(m.e("displacement.type"), "intensity_centered");
}

#[test]
fn default_material_compiles_to_diffuse_and_specular() {
    let g = compile(&Material::new());
    let d = g.outputs.diffuse.as_ref().unwrap();
    let c = g.eval(d.color, &Const(0.0), &ShadeCtx::default());
    assert_eq!(c, [0.8; 3]);
    let s = g.outputs.specular.as_ref().unwrap();
    assert!((s.strength - 0.2).abs() < 1e-12);
    assert!(g.outputs.emission.is_none() && g.outputs.transmission.is_none());
    assert!(g.outputs.bump.is_none());
}

#[test]
fn texture_slot_mixes_with_the_colour() {
    let mut ov = BTreeMap::new();
    ov.insert("color.texture".to_string(), Value::String("noise".into()));
    ov.insert("color.mix_mode".to_string(), Value::Enum("multiply".into()));
    ov.insert("color.brightness".to_string(), Value::Float(0.5));
    let g = compile(&Material::from_values(&ov));
    let d = g.outputs.diffuse.as_ref().unwrap();
    assert!(matches!(
        g.nodes[d.color],
        IrNode::Mix {
            mode: MixMode::Multiply,
            ..
        }
    ));
    // base 0.8 * 0.5 = 0.4, top 0.6 * 0.5 = 0.3, multiply -> 0.12
    let v = g.eval(d.color, &Const(0.6), &ShadeCtx::default());
    assert!((v[0] - 0.12).abs() < 1e-12);
}

#[test]
fn mix_modes_and_strength() {
    let a = [0.5; 3];
    let b = [0.25; 3];
    assert_eq!(mix(a, b, MixMode::Normal, 1.0), b);
    assert_eq!(mix(a, b, MixMode::Add, 1.0), [0.75; 3]);
    assert_eq!(mix(a, b, MixMode::Subtract, 1.0), [0.25; 3]);
    assert_eq!(mix(a, b, MixMode::Multiply, 1.0), [0.125; 3]);
    assert_eq!(mix(a, b, MixMode::Normal, 0.0), a);
    assert_eq!(mix(a, b, MixMode::Normal, 0.5), [0.375; 3]);
}

#[test]
fn channels_follow_their_enable_boxes() {
    let mut m = Material::new();
    m.set("enable.luminance", Value::Bool(true));
    m.set("enable.transparency", Value::Bool(true));
    m.set("enable.color", Value::Bool(false));
    m.set("enable.bump", Value::Bool(true));
    let g = compile(&m);
    assert!(g.outputs.diffuse.is_none());
    assert!(g.outputs.emission.is_some());
    assert_eq!(g.outputs.transmission.as_ref().unwrap().ior, 1.517);
    assert!(
        g.outputs.bump.is_none(),
        "bump without a texture has nothing to perturb"
    );
    m.set("bump.texture", Value::String("noise".into()));
    assert!(compile(&m).outputs.bump.is_some());
}

#[test]
fn compile_is_deterministic() {
    assert_eq!(compile(&Material::new()), compile(&Material::new()));
}

#[test]
fn texture_tag_defaults() {
    let t = forge_material::texture_tag_type();
    let d = |id: &str| t.find_param(id).unwrap().default.clone();
    assert_eq!(d("projection"), Value::Enum("spherical".into()));
    assert_eq!(d("tiles_u"), Value::Float(1.0));
    assert_eq!(d("length_u"), Value::Float(1.0));
    assert_eq!(d("scale"), Value::Vec3([1.0; 3]));
}
