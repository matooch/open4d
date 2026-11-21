//! Shader behaviour that the golden samples do not reach (docs/specs/SHADERS.md).

use std::sync::Arc;

use forge_core::{Registry, Value};
use forge_image::{AlphaMode, Image};
use forge_material::{ShadeCtx, TextureSampler};
use forge_shaders::{
    BlendMode, Gradient, Inputs, Interp, Shader, ShaderError, ShaderSet, ShadersPlugin, Values,
    build,
};

fn at(u: f64, v: f64) -> ShadeCtx {
    ShadeCtx {
        uvw: [u, v, 0.0],
        ..ShadeCtx::default()
    }
}

fn color(c: [f64; 3]) -> Shader {
    Shader::Color {
        color: c,
        brightness: 1.0,
    }
}

#[test]
fn plugin_registers_every_shader_type() {
    let mut r = Registry::with_builtins();
    r.install(&ShadersPlugin).unwrap();
    assert_eq!(r.types_in(forge_core::Category::Shader).count(), 17);
}

#[test]
fn colour_parameters_are_decoded_from_srgb() {
    let mut v = Values::new();
    v.insert("color".into(), Value::Color([0.5, 1.0, 0.0]));
    let s = build("forge.shader.color", &v, &Inputs::default()).unwrap();
    let c = s.sample(&at(0.0, 0.0));
    assert!((c[0] - 0.21404114).abs() < 1e-7 && c[1] == 1.0 && c[2] == 0.0);
}

#[test]
fn gradient_interpolations() {
    let g = |interp| Gradient {
        knots: vec![(0.0, [0.0; 3]), (1.0, [1.0; 3])],
        interp,
    };
    assert_eq!(g(Interp::Linear).eval(0.25)[0], 0.25);
    assert_eq!(g(Interp::None).eval(0.99)[0], 0.0);
    assert!((g(Interp::Cubic).eval(0.25)[0] - 0.15625).abs() < 1e-12);
    let three = Gradient {
        knots: vec![
            (0.0, [0.0; 3]),
            (0.5, [1.0, 0.0, 0.0]),
            (1.0, [0.0, 0.0, 1.0]),
        ],
        interp: Interp::Linear,
    };
    assert_eq!(three.eval(0.75), [0.5, 0.0, 0.5]);
    assert_eq!(three.eval(-1.0), [0.0; 3]);
    assert_eq!(three.eval(2.0), [0.0, 0.0, 1.0]);
}

#[test]
fn layer_blends_bottom_to_top() {
    let mut v = Values::new();
    v.insert("blend_mode".into(), Value::Enum("multiply".into()));
    let inputs = Inputs {
        shaders: vec![color([0.5; 3]), color([0.5; 3])],
        image: None,
    };
    let s = build("forge.shader.layer", &v, &inputs).unwrap();
    assert_eq!(s.sample(&at(0.0, 0.0)), [0.25; 3]);
    assert_eq!(BlendMode::Screen.apply([0.5; 3], [0.5; 3], 1.0), [0.75; 3]);
    let d = BlendMode::Difference.apply([0.2; 3], [0.5; 3], 0.5);
    assert!((d[0] - 0.25).abs() < 1e-12);
}

#[test]
fn colorizer_maps_the_input_through_the_gradient() {
    let mut v = Values::new();
    v.insert("color_end".into(), Value::Color([1.0, 0.0, 0.0]));
    v.insert("interpolation".into(), Value::Enum("linear".into()));
    let inputs = Inputs {
        shaders: vec![color([0.5; 3])],
        image: None,
    };
    let s = build("forge.shader.colorizer", &v, &inputs).unwrap();
    let c = s.sample(&at(0.0, 0.0));
    assert!(c[0] > 0.2 && c[1] == 0.0);
}

#[test]
fn fresnel_uses_geometry_when_present() {
    let s = build("forge.shader.fresnel", &Values::new(), &Inputs::default()).unwrap();
    let head_on = ShadeCtx {
        normal: [0.0, 1.0, 0.0],
        view: [0.0, 1.0, 0.0],
        ..ShadeCtx::default()
    };
    let grazing = ShadeCtx {
        normal: [0.0, 1.0, 0.0],
        view: [1.0, 0.01, 0.0],
        ..ShadeCtx::default()
    };
    let (a, b) = (s.sample(&head_on)[0], s.sample(&grazing)[0]);
    let f0 = (0.33283f64 / 2.33283).powi(2);
    assert!((a - f0).abs() < 1e-9);
    assert!(b > 0.9);
}

#[test]
fn filter_normalizer_fusion_and_maps() {
    let inputs = Inputs {
        shaders: vec![color([0.25; 3])],
        image: None,
    };
    let mut v = Values::new();
    v.insert("high".into(), Value::Float(0.5));
    let n = build("forge.shader.normalizer", &v, &inputs).unwrap();
    assert_eq!(n.sample(&at(0.0, 0.0)), [0.5; 3]);
    let mut v = Values::new();
    v.insert("invert".into(), Value::Bool(true));
    let f = build("forge.shader.filter", &v, &inputs).unwrap();
    assert_eq!(f.sample(&at(0.0, 0.0)), [0.75; 3]);
    let fu = build(
        "forge.shader.fusion",
        &Values::new(),
        &Inputs {
            shaders: vec![color([0.1; 3]), color([0.9; 3])],
            image: None,
        },
    )
    .unwrap();
    assert_eq!(fu.sample(&at(0.0, 0.0)), [0.9; 3]);
    let vm = build(
        "forge.shader.vertex_map",
        &Values::new(),
        &Inputs::default(),
    )
    .unwrap();
    let ctx = ShadeCtx {
        vertex_weight: Some(0.25),
        ..ShadeCtx::default()
    };
    assert_eq!(vm.sample(&ctx), [0.25; 3]);
    assert_eq!(vm.sample(&ShadeCtx::default()), [0.0; 3]);
    let ao = build("forge.shader.ao", &Values::new(), &Inputs::default()).unwrap();
    assert_eq!(ao.sample(&ShadeCtx::default()), [1.0; 3]);
    assert_eq!(
        ao.sample(&ShadeCtx {
            ao: Some(0.5),
            ..ShadeCtx::default()
        }),
        [0.5; 3]
    );
}

#[test]
fn bitmap_samples_with_v_up_and_repeat() {
    let mut img = Image::new(2, 2, AlphaMode::Straight);
    img.set_pixel(0, 0, [1.0, 0.0, 0.0, 1.0]); // top left
    img.set_pixel(1, 0, [0.0, 1.0, 0.0, 1.0]);
    img.set_pixel(0, 1, [0.0, 0.0, 1.0, 1.0]); // bottom left
    img.set_pixel(1, 1, [1.0; 4]);
    let mut v = Values::new();
    v.insert("interpolate".into(), Value::Bool(false));
    let s = build(
        "forge.shader.bitmap",
        &v,
        &Inputs {
            shaders: vec![],
            image: Some(Arc::new(img)),
        },
    )
    .unwrap();
    assert_eq!(s.sample(&at(0.25, 0.75)), [1.0, 0.0, 0.0]);
    assert_eq!(s.sample(&at(0.25, 0.25)), [0.0, 0.0, 1.0]);
    assert_eq!(s.sample(&at(1.25, 0.75)), [1.0, 0.0, 0.0], "repeat");
    assert!(matches!(
        build("forge.shader.bitmap", &v, &Inputs::default()),
        Err(ShaderError::Inputs(_))
    ));
}

#[test]
fn unsupported_shaders_say_so() {
    for t in ["brick", "field"] {
        assert!(matches!(
            build(
                &format!("forge.shader.{t}"),
                &Values::new(),
                &Inputs::default()
            ),
            Err(ShaderError::Unsupported(_))
        ));
    }
    let mut v = Values::new();
    v.insert("noise".into(), Value::Enum("buya".into()));
    assert!(matches!(
        build("forge.shader.noise", &v, &Inputs::default()),
        Err(ShaderError::Unsupported(_))
    ));
}

#[test]
fn shader_set_is_a_texture_sampler() {
    let mut set = ShaderSet::default();
    set.shaders.insert("red".into(), color([1.0, 0.0, 0.0]));
    assert_eq!(set.sample("red", &ShadeCtx::default()), [1.0, 0.0, 0.0]);
    assert_eq!(set.sample("missing", &ShadeCtx::default()), [1.0; 3]);
}

#[test]
fn noise_is_deterministic_and_animates() {
    let s = build("forge.shader.noise", &Values::new(), &Inputs::default()).unwrap();
    assert_eq!(s.sample(&at(0.3, 0.4)), s.sample(&at(0.3, 0.4)));
    let mut v = Values::new();
    v.insert("animation_speed".into(), Value::Float(1.0));
    let a = build("forge.shader.noise", &v, &Inputs::default()).unwrap();
    let t1 = ShadeCtx {
        time: 1.0,
        ..at(0.3, 0.4)
    };
    assert_ne!(a.sample(&at(0.3, 0.4)), a.sample(&t1));
}
