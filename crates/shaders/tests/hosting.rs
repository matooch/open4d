//! SHADERS.md S11 / MATERIAL.md M6 (brief 194): shaders hosted below materials in a document.

use std::sync::Arc;

use forge_core::{Document, Registry, Value};
use forge_image::{AlphaMode, Image};
use forge_material::{Material, ShadeCtx, ShadingGraph, compile, shader_name};
use forge_shaders::{ShaderError, ShaderSet, ShadersPlugin, build_node, no_images};

fn doc() -> Document {
    let mut r = Registry::with_builtins();
    r.install(&forge_material::MaterialPlugin).unwrap();
    r.install(&ShadersPlugin).unwrap();
    Document::new(Arc::new(r))
}

fn sample(graph: &ShadingGraph, set: &ShaderSet, u: f64) -> [f64; 3] {
    let ctx = ShadeCtx {
        uvw: [u, 0.5, 0.0],
        ..ShadeCtx::default()
    };
    let ix = graph.outputs.diffuse.as_ref().unwrap().color;
    graph.eval(ix, set, &ctx)
}

#[test]
fn a_gradient_on_the_color_channel_drives_the_compiled_material() {
    let mut d = doc();
    let (m, g) = d
        .transact("m", |tx| {
            let m = tx.create_material("forge.material.standard", None)?;
            let g = tx.create_shader("forge.shader.gradient", m, None)?;
            tx.set_param(m, "color.texture", Value::Link(Some(g)))?;
            // brightness 100 %, mix mode normal, strength 100 %: the texture replaces the colour
            Ok((m, g))
        })
        .unwrap();
    let node = d.scene().node(m).unwrap();
    let mat = Material::from_node(node);
    assert_eq!(mat.e("color.texture"), shader_name(g));
    let (set, failed) = ShaderSet::from_material(d.scene(), m, &no_images);
    assert!(failed.is_empty(), "{failed:?}");
    assert!(set.shaders.contains_key(&shader_name(g)));
    let graph = compile(&mat);
    // the default gradient runs black to white along u (SHADERS.md S2: smoothstep)
    let (a, b, c) = (
        sample(&graph, &set, 0.0),
        sample(&graph, &set, 0.5),
        sample(&graph, &set, 0.99),
    );
    // cycle is on by default (t wraps at 1), so sample just below 1
    assert!(
        a[0].abs() < 1e-9 && (c[0] - 1.0).abs() < 1e-3,
        "{a:?} {c:?}"
    );
    assert!((b[0] - 0.5).abs() < 1e-6, "{b:?}");
    // an empty slot samples nothing: the plain colour
    let mat = Material::from_node(&{
        let mut n = node.clone();
        n.params.insert("color.texture".into(), Value::Link(None));
        n
    });
    assert_eq!(mat.e("color.texture"), "");
}

#[test]
fn children_are_the_input_shaders_in_order() {
    let mut d = doc();
    let (m, layer) = d
        .transact("m", |tx| {
            let m = tx.create_material("forge.material.standard", None)?;
            let layer = tx.create_shader("forge.shader.layer", m, None)?;
            let base = tx.create_shader("forge.shader.color", layer, None)?;
            tx.set_param(base, "color", Value::Color([1.0, 1.0, 1.0]))?;
            let top = tx.create_shader("forge.shader.color", layer, None)?;
            tx.set_param(top, "color", Value::Color([0.0, 0.0, 0.0]))?;
            tx.set_param(layer, "opacity", Value::Float(0.5))?;
            Ok((m, layer))
        })
        .unwrap();
    let s = build_node(d.scene(), layer, &no_images).unwrap();
    // white base with 50 % black on top: mid grey in sRGB-decoded linear terms is 0.5 of 1
    let c = s.sample(&ShadeCtx::default());
    assert!((c[0] - 0.5).abs() < 1e-9, "{c:?}");
    let _ = m;
}

#[test]
fn errors_are_reported_per_slot_and_the_slot_samples_white() {
    let mut d = doc();
    let (m, noise, obj) = d
        .transact("m", |tx| {
            let m = tx.create_material("forge.material.standard", None)?;
            let noise = tx.create_shader("forge.shader.noise", m, None)?;
            tx.set_param(noise, "noise", Value::Enum("buya".into()))?;
            tx.set_param(m, "color.texture", Value::Link(Some(noise)))?;
            let obj = tx.create("forge.null", None, None)?;
            Ok((m, noise, obj))
        })
        .unwrap();
    assert!(matches!(
        build_node(d.scene(), noise, &no_images),
        Err(ShaderError::Unsupported(_))
    ));
    assert!(matches!(
        build_node(d.scene(), obj, &no_images),
        Err(ShaderError::UnknownType(_))
    ));
    let (set, failed) = ShaderSet::from_material(d.scene(), m, &no_images);
    assert_eq!(failed.len(), 1);
    assert!(set.shaders.is_empty());
    let graph = compile(&Material::from_node(d.scene().node(m).unwrap()));
    assert_eq!(sample(&graph, &set, 0.3), [1.0; 3]);
}

#[test]
fn a_bitmap_takes_its_image_from_the_loader() {
    let mut d = doc();
    let b = d
        .transact("m", |tx| {
            let m = tx.create_material("forge.material.standard", None)?;
            let b = tx.create_shader("forge.shader.bitmap", m, None)?;
            tx.set_param(b, "file", Value::String("tex.png".into()))?;
            Ok(b)
        })
        .unwrap();
    assert!(matches!(
        build_node(d.scene(), b, &no_images),
        Err(ShaderError::Inputs(_))
    ));
    let img = Arc::new(Image::from_fn(2, 2, AlphaMode::Straight, |_, _| {
        [1.0, 0.0, 0.0, 1.0]
    }));
    let loader = move |p: &str| (p == "tex.png").then(|| img.clone());
    let s = build_node(d.scene(), b, &loader).unwrap();
    let c = s.sample(&ShadeCtx {
        uvw: [0.5, 0.5, 0.0],
        ..ShadeCtx::default()
    });
    assert!((c[0] - 1.0).abs() < 1e-6 && c[1].abs() < 1e-6, "{c:?}");
}
