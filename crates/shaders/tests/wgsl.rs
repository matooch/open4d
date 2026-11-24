//! docs/specs/REALTIME.md §2: shader-node codegen validated with naga, the once-per-module
//! rule, cache keys and the compile time (no GPU needed).

mod common;

use std::collections::BTreeMap;
use std::time::Instant;

use common::*;
use forge_material::ir::{IrNode, MixMode, Outputs, ShadingGraph};
use forge_material::wgsl::{ShaderCache, hash_graph_with};
use forge_shaders::shader::*;
use forge_shaders::wgsl::{ShaderSetCodegen, compile};

fn validate(source: &str) {
    let module = naga::front::wgsl::parse_str(source).unwrap_or_else(|e| {
        panic!("WGSL parses: {}", e.emit_to_string(source));
    });
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    validator
        .validate(&module)
        .expect("naga validates the module");
}

fn set_of(cases: &[Case]) -> ShaderSet {
    ShaderSet {
        shaders: cases
            .iter()
            .map(|c| (c.name.clone(), c.shader.clone()))
            .collect::<BTreeMap<_, _>>(),
    }
}

/// One texture node per case, the first as the emission.
fn graph_of(cases: &[Case]) -> ShadingGraph {
    ShadingGraph {
        nodes: cases
            .iter()
            .map(|c| IrNode::Texture(c.name.clone()))
            .collect(),
        outputs: Outputs {
            emission: Some(0),
            ..Default::default()
        },
    }
}

fn all_cases() -> Vec<Case> {
    let mut cases = noise_cases();
    cases.extend(extra_cases());
    cases
}

fn extra_cases() -> Vec<Case> {
    Vec::new()
}

#[test]
fn every_family_compiles_without_fallback_and_validates() {
    let cases = all_cases();
    let m = compile(&graph_of(&cases), &set_of(&cases));
    assert!(m.fallbacks.is_empty(), "fallbacks: {:?}", m.fallbacks);
    validate(&m.source);
}

#[test]
fn library_functions_are_emitted_once_per_module() {
    let cases = all_cases();
    let m = compile(&graph_of(&cases), &set_of(&cases));
    for lib in ["forge_noise_eval", "forge_mix64", "forge_perlin4"] {
        assert_eq!(
            m.source.matches(&format!("fn {lib}(")).count(),
            1,
            "{lib} emitted more than once"
        );
    }
}

#[test]
fn an_unknown_shader_name_is_white() {
    let g = ShadingGraph {
        nodes: vec![IrNode::Texture("missing".into())],
        outputs: Outputs {
            emission: Some(0),
            ..Default::default()
        },
    };
    let m = compile(&g, &ShaderSet::default());
    assert!(m.fallbacks.is_empty());
    assert!(m.source.contains("vec3<f32>(1.0)"));
    validate(&m.source);
}

#[test]
fn the_cache_key_covers_the_shader_code() {
    let mut cases = noise_cases();
    cases.truncate(2);
    let g = graph_of(&cases);
    let mut set = set_of(&cases);
    let mut cache = ShaderCache::new();
    let (a, fresh) = cache.module_with(&g, &ShaderSetCodegen { set: &set });
    assert!(fresh);
    let (b, fresh) = cache.module_with(&g, &ShaderSetCodegen { set: &set });
    assert!(!fresh && std::sync::Arc::ptr_eq(&a, &b));
    // an edit of one shader changes the key and recompiles
    if let Some(Shader::Noise(n)) = set.shaders.get_mut(&cases[0].name) {
        n.seed += 1;
    }
    let (c, fresh) = cache.module_with(&g, &ShaderSetCodegen { set: &set });
    assert!(fresh && c.hash != a.hash);
    assert_eq!(cache.compiles(), 2);
    assert_eq!(c.hash, hash_graph_with(&g, &ShaderSetCodegen { set: &set }));
}

/// 50 IR nodes, a quarter of them shader nodes of every family, compiled under 100 ms.
#[test]
fn a_fifty_node_material_compiles_under_100_ms() {
    let cases = all_cases();
    let set = set_of(&cases);
    let mut nodes = Vec::new();
    for i in 0..50 {
        if i % 2 == 0 || nodes.is_empty() {
            nodes.push(IrNode::Texture(cases[(i / 2) % cases.len()].name.clone()));
        } else {
            nodes.push(IrNode::Mix {
                base: i - 1,
                top: i - 2.min(i),
                mode: MixMode::Add,
                strength: 0.5,
            });
        }
    }
    let graph = ShadingGraph {
        outputs: Outputs {
            emission: Some(49),
            ..Default::default()
        },
        nodes,
    };
    let start = Instant::now();
    let m = compile(&graph, &set);
    let elapsed = start.elapsed();
    eprintln!("50-node material compiled in {elapsed:?}");
    assert!(elapsed.as_millis() < 100, "compiled in {elapsed:?}");
    assert!(m.fallbacks.is_empty());
    validate(&m.source);
}
