//! docs/specs/REALTIME.md §1: IR codegen, naga validation, cache and timing (no GPU).

use std::time::Instant;

use forge_material::ir::{
    Diffuse, DiffuseModel, Diffusion, Fog, IrNode, MixMode, NormalMap, Outputs, ShadingGraph,
    Specular, Transmission,
};
use forge_material::wgsl::{CODEGEN_VERSION, ShaderCache, compile, hash_graph};

fn sample_graph() -> ShadingGraph {
    let nodes = vec![
        IrNode::Color([0.1, 0.2, 0.3]),
        IrNode::Texture("noise".into()),
        IrNode::Scale {
            input: 0,
            factor: 2.0,
        },
        IrNode::Mix {
            base: 0,
            top: 1,
            mode: MixMode::Add,
            strength: 0.5,
        },
    ];
    let outputs = Outputs {
        diffuse: Some(Diffuse {
            color: 3,
            level: 1.0,
            falloff: 0.5,
            model: DiffuseModel::OrenNayar,
            roughness: 0.2,
        }),
        specular: Some(Specular {
            kind: "ggx".into(),
            width: 0.3,
            falloff: 0.1,
            inner_width: 0.2,
            strength: 1.0,
            color: 0,
            metal: true,
            colored: false,
        }),
        emission: Some(3),
        transmission: Some(Transmission {
            color: 0,
            ior: 1.5,
            absorption: [0.1, 0.2, 0.3],
            absorption_distance: 10.0,
            fresnel_reflectivity: 1.0,
            total_internal_reflection: true,
            exit_reflections: true,
            additive: false,
        }),
        environment: Some(1),
        fog: Some(Fog {
            color: 2,
            distance: 100.0,
        }),
        alpha: Some(forge_material::ir::Alpha {
            texture: Some(0),
            color: [1.0, 1.0, 1.0],
            delta: [0.1, 0.1, 0.1],
            invert: false,
            soft: true,
            image_alpha: false,
            premultiplied: false,
        }),
        bump: Some(forge_material::ir::Bump {
            texture: 0,
            strength: 1.0,
        }),
        normal: Some(NormalMap {
            texture: 1,
            strength: 0.5,
            method: "object".into(),
            flip: [true, false, true],
            swap_yz: false,
        }),
        displacement: Some(forge_material::ir::Displacement {
            texture: 0,
            strength: 2.0,
            height: 1.0,
            kind: "rgb_object".into(),
        }),
        diffusion: Some(Diffusion {
            brightness: 1.0,
            affect_luminance: true,
            affect_specular: false,
            affect_reflection: true,
        }),
        ..Default::default()
    };
    ShadingGraph { nodes, outputs }
}

fn validate(source: &str) {
    let module = naga::front::wgsl::parse_str(source).expect("WGSL parses");
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .expect("naga validates the module");
}

#[test]
fn every_ir_node_and_channel_compiles_and_validates() {
    let module = compile(&sample_graph());
    assert_eq!(module.fallbacks, ["noise"]);
    assert!(module.source.contains("fn forge_surface"));
    for name in [
        "fn ir0",
        "fn ir1",
        "fn ir2",
        "fn ir3",
        "forge_mix",
        "s.transmission_present",
        "s.diffusion_present",
    ] {
        assert!(module.source.contains(name), "missing {name}");
    }
    validate(&module.source);
}

#[test]
fn a_graph_without_channels_still_validates() {
    let module = compile(&ShadingGraph::default());
    validate(&module.source);
    assert!(module.fallbacks.is_empty());
}

#[test]
fn the_cache_reuses_the_module_and_recompiles_an_edit() {
    let mut cache = ShaderCache::new();
    let graph = sample_graph();
    let (a, compiled_a) = cache.module(&graph);
    assert!(compiled_a);
    let (b, compiled_b) = cache.module(&graph);
    assert!(!compiled_b);
    assert!(std::sync::Arc::ptr_eq(&a, &b));
    assert_eq!(cache.compiles(), 1);
    assert_eq!(a.hash, hash_graph(&graph));

    let mut edited = graph.clone();
    edited.nodes.push(IrNode::Color([0.9, 0.9, 0.9]));
    let (c, compiled_c) = cache.module(&edited);
    assert!(compiled_c);
    assert_ne!(a.hash, c.hash);
    assert_eq!(cache.compiles(), 2);
    assert_eq!(cache.len(), 2);
}

#[test]
fn a_fifty_node_material_compiles_under_100_ms() {
    let mut nodes = vec![IrNode::Color([0.2, 0.4, 0.6])];
    for i in 1..50 {
        nodes.push(IrNode::Mix {
            base: i - 1,
            top: 0,
            mode: if i % 2 == 0 {
                MixMode::Add
            } else {
                MixMode::Multiply
            },
            strength: 0.5,
        });
    }
    let graph = ShadingGraph {
        nodes,
        outputs: Outputs {
            diffuse: Some(Diffuse {
                color: 49,
                ..sample_graph().outputs.diffuse.unwrap()
            }),
            ..Default::default()
        },
    };
    let start = Instant::now();
    let m = compile(&graph);
    let elapsed = start.elapsed();
    assert!(elapsed.as_millis() < 100, "compiled in {elapsed:?}");
    assert_eq!(m.hash, hash_graph(&graph));
    assert_eq!(CODEGEN_VERSION, 4);
    validate(&m.source);
}

#[test]
fn fresnel_drives_a_ramp_and_scalar_surface_inputs_per_hit() {
    use forge_material::node_material::{self, FRESNEL, GRADIENT, LUMINANCE, VECTOR};
    use forge_material::{ShadeCtx, TextureSampler};
    use forge_nodes::PortRef;

    struct White;
    impl TextureSampler for White {
        fn sample(&self, _: &str, _: &ShadeCtx) -> [f64; 3] {
            [1.0; 3]
        }
    }
    let mut graph = node_material::default_graph();
    let surface = graph.nodes().next().unwrap().id;
    let registry = node_material::registry();
    let view = graph.add_node(registry, FRESNEL).unwrap();
    let ramp = graph.add_node(registry, GRADIENT).unwrap();
    let luma = graph.add_node(registry, LUMINANCE).unwrap();
    let vector = graph.add_node(registry, VECTOR).unwrap();
    for (a, output, b, input) in [
        (view, "factor", ramp, "position"),
        (ramp, "color", luma, "color"),
        (ramp, "color", vector, "vector"),
        (ramp, "color", surface, "base_color"),
        (luma, "value", surface, "roughness"),
        (vector, "x", surface, "metallic"),
    ] {
        graph
            .connect(registry, PortRef::new(a, output), PortRef::new(b, input))
            .unwrap();
    }

    let ir = node_material::compile_graph(&graph).unwrap();
    let diffuse = ir.outputs.diffuse.as_ref().unwrap().color;
    let roughness = ir.outputs.roughness_input.unwrap();
    let metallic = ir.outputs.metallic_input.unwrap();
    for (view, expected) in [([0.0, 0.0, 1.0], 0.04), ([1.0, 0.0, 0.0], 1.0)] {
        let ctx = ShadeCtx {
            normal: [0.0, 0.0, 1.0],
            view,
            ..Default::default()
        };
        for port in [diffuse, roughness, metallic] {
            assert!((ir.eval(port, &White, &ctx)[0] - expected).abs() < 1e-12);
        }
    }
    let source = compile(&ir).source;
    validate(&source);
}
