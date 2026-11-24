//! Parity: docs/specs/MODIFIERS.md §22 against tests/golden/{smoothing-deformer,displacer,
//! polygon-reduction,bevel-deformer} (O19).

use std::path::Path;
use std::sync::Arc;

mod common;

use common::resolved_params;
use forge_core::{Document, NodeId, Registry, Value};
use forge_generators::{Primitives, evaluate, from_poly_mesh, make_editable, to_poly_mesh};
use forge_modifiers::Deformers;
use forge_parity_harness::{GoldenCase, Mesh, compare_volume_mesh, discover_cases, export_weld};

const FEATURES: &[&str] = &[
    "smoothing-deformer",
    "displacer",
    "polygon-reduction",
    "bevel-deformer",
];

/// Cases pinned because the derived rule does not reproduce the reference.
const KNOWN_MISMATCH: &[(&str, &str)] = &[
    (
        "smoothing-deformer/defaults",
        "the reference Laplacian shrinks much more (area 141k vs 238k, volume 4.3M vs 9.3M); the strength/stiffness weighting is not derived",
    ),
    (
        "smoothing-deformer/iterations-10",
        "same as defaults (10 is already the default); shrink weighting not derived",
    ),
    (
        "displacer/height-noise",
        "no UVW tag is observed for the procedural cube; object-box UVW gives Hausdorff 23.8 and volume -2.7 %",
    ),
    (
        "polygon-reduction/defaults",
        "the quality-factor stopping rule is not derived: 13/22 expected, our target leaves 104/108",
    ),
    (
        "polygon-reduction/50",
        "the quality-factor stopping rule is not derived: 58/112 expected, our target leaves 104/108",
    ),
    (
        "bevel-deformer/offset-20",
        "Subdivision/rounding is not derived; the full-edge chamfer is flat (24/26) against 96/98",
    ),
];

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&Deformers).unwrap();
    reg.install(&forge_shaders::ShadersPlugin).unwrap();
    reg.install(&forge_material::MaterialPlugin).unwrap();
    Arc::new(reg)
}

fn f(t: &toml::map::Map<String, toml::Value>, k: &str) -> f64 {
    t.get(k)
        .and_then(|v| v.as_float().or(v.as_integer().map(|i| i as f64)))
        .unwrap_or_else(|| panic!("missing {k}"))
}

fn s<'a>(t: &'a toml::map::Map<String, toml::Value>, k: &str) -> &'a str {
    t.get(k).and_then(|v| v.as_str()).unwrap()
}

fn bools(t: &toml::map::Map<String, toml::Value>, k: &str) -> bool {
    t.get(k).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Label → enum id for the O19 table (the scripted form stores the labels).
fn id(label: &str) -> String {
    match label {
        "Intensity (Centered)" => "centered".into(),
        "Intensity" => "intensity".into(),
        "Red / Green" => "red".into(),
        "Green" => "green".into(),
        "RGB (XYZ Object)" => "rgb_object".into(),
        "RGB (XYZ World)" => "rgb_world".into(),
        "Vertex normal" => "normal".into(),
        "Spherical" => "spherical".into(),
        "Planar" => "planar".into(),
        "Custom Shader" => "custom".into(),
        "Fixed Distance" => "fixed_distance".into(),
        "Standard" => "standard".into(),
        other => {
            // CamelCase labels become snake_case ids ("Smooth" → "smooth").
            let mut out = String::new();
            for (i, c) in other.chars().enumerate() {
                if c.is_uppercase() {
                    if i > 0 {
                        out.push('_');
                    }
                    out.extend(c.to_lowercase());
                } else {
                    out.push(c);
                }
            }
            out
        }
    }
}

fn cube(tx: &mut forge_core::Tx, t: &toml::map::Map<String, toml::Value>) -> NodeId {
    let id = tx.create("forge.cube", None, None).unwrap();
    let sy = f(t, "size_y");
    let _ = tx.set_param(id, "size_y", Value::Float(sy));
    for k in ["size_x", "size_z"] {
        if t.contains_key(k) {
            let _ = tx.set_param(id, k, Value::Float(f(t, k)));
        }
    }
    for k in ["segments_x", "segments_y", "segments_z"] {
        if let Some(v) = t.get(k).and_then(|v| v.as_integer()) {
            let _ = tx.set_param(id, k, Value::Int(v));
        }
    }
    if let Some(v) = t.get("fillet").and_then(|v| v.as_bool()) {
        let _ = tx.set_param(id, "fillet", Value::Bool(v));
    }
    id
}

/// The object of the case's `[params.input]` table.
fn input(tx: &mut forge_core::Tx, t: &toml::map::Map<String, toml::Value>) -> NodeId {
    match s(t, "type") {
        "cube_editable" => {
            let id = cube(tx, t);
            let id = make_editable(tx, id).unwrap();
            let Value::Mesh(m) = tx.scene().param(id, "mesh").unwrap().clone() else {
                panic!("editable mesh");
            };
            let mut mesh = to_poly_mesh(&m);
            let moved = t.get("point_0_moved").and_then(|v| v.as_array()).unwrap();
            let d = glam::DVec3::new(
                moved[0].as_float().unwrap(),
                moved[1].as_float().unwrap(),
                moved[2].as_float().unwrap(),
            );
            mesh.points[0] += d;
            tx.set_param(
                id,
                "mesh",
                Value::Mesh(std::sync::Arc::new(from_poly_mesh(&mesh))),
            )
            .unwrap();
            id
        }
        "sphere" => {
            let node = tx.create("forge.sphere", None, None).unwrap();
            let _ = tx.set_param(node, "radius", Value::Float(f(t, "radius")));
            let _ = tx.set_param(
                node,
                "segments",
                Value::Int(t["segments"].as_integer().unwrap()),
            );
            let _ = tx.set_param(node, "type", Value::Enum(id(s(t, "sphere_type"))));
            node
        }
        _ => cube(tx, t),
    }
}

fn build(case: &GoldenCase) -> (Mesh, Mesh) {
    let reg = registry();
    let m = case.manifest().unwrap();
    let params = resolved_params(case, &m);
    let input_t = params["input"].as_table().unwrap();
    let mut doc = Document::new(reg.clone());
    let object = doc
        .transact("case", |tx| {
            let object = input(tx, input_t);
            let ty = match case.feature.as_str() {
                "smoothing-deformer" => "forge.smoothing_deformer",
                "bevel-deformer" => "forge.bevel_deformer",
                other => match other {
                    "displacer" => "forge.displacer",
                    "polygon-reduction" => "forge.polygon_reduction",
                    _ => unreachable!(),
                },
            };
            let d = tx.create(ty, Some(object), None).unwrap();
            let set = |tx: &mut forge_core::Tx, k: &str, v: Value| {
                let _ = tx.set_param(d, k, v);
            };
            match case.feature.as_str() {
                "smoothing-deformer" => {
                    set(
                        tx,
                        "strength",
                        Value::Float(f(&params, "strength_percent") / 100.0),
                    );
                    set(
                        tx,
                        "stiffness",
                        Value::Float(f(&params, "stiffness_percent") / 100.0),
                    );
                    set(
                        tx,
                        "iterations",
                        Value::Int(params["iterations"].as_integer().unwrap()),
                    );
                    set(tx, "type", Value::Enum(id(s(&params, "type"))));
                    set(
                        tx,
                        "keep_displacements",
                        Value::Bool(bools(&params, "keep_displacements")),
                    );
                }
                "displacer" => {
                    set(
                        tx,
                        "strength",
                        Value::Float(f(&params, "strength_percent") / 100.0),
                    );
                    set(tx, "height", Value::Float(f(&params, "height")));
                    set(tx, "type", Value::Enum(id(s(&params, "type"))));
                    set(tx, "direction", Value::Enum(id(s(&params, "direction"))));
                    set(tx, "channel", Value::Enum(id(s(&params, "channel"))));
                    set(tx, "use", Value::Enum(id(s(&params, "use"))));
                    set(tx, "offsetu", Value::Float(f(&params, "offsetu")));
                    set(tx, "offsetv", Value::Float(f(&params, "offsetv")));
                    set(
                        tx,
                        "lengthu",
                        Value::Float(f(&params, "lengthu_percent") / 100.0),
                    );
                    set(
                        tx,
                        "lengthv",
                        Value::Float(f(&params, "lengthv_percent") / 100.0),
                    );
                    set(tx, "tile", Value::Bool(bools(&params, "tile")));
                    set(tx, "invert", Value::Bool(bools(&params, "invert")));
                    if let Some(sh) = params.get("shader").and_then(|v| v.as_table()) {
                        // Shaders are hosted under a material, not at the scene root.
                        let mat = tx.create_material("forge.material.standard", None).unwrap();
                        let noise = tx.create_shader("forge.shader.noise", mat, None).unwrap();
                        if let Some(n) = sh.get("seed").and_then(|v| v.as_integer()) {
                            let _ = tx.set_param(noise, "seed", Value::Int(n));
                        }
                        let _ = tx.set_param(d, "shader", Value::Link(Some(noise)));
                    }
                }
                "polygon-reduction" => {
                    set(
                        tx,
                        "reduction_strength",
                        Value::Float(f(&params, "reduction_strength_percent") / 100.0),
                    );
                    set(
                        tx,
                        "mesh_quality_factor",
                        Value::Float(f(&params, "mesh_quality_factor")),
                    );
                    set(
                        tx,
                        "co_planar_optimization",
                        Value::Bool(bools(&params, "co_planar_optimization")),
                    );
                    set(
                        tx,
                        "bounday_curve_preservation",
                        Value::Bool(bools(&params, "bounday_curve_preservation")),
                    );
                    set(
                        tx,
                        "polygon_quality_preservation",
                        Value::Bool(bools(&params, "polygon_quality_preservation")),
                    );
                }
                _ => {
                    set(
                        tx,
                        "component_mode",
                        Value::Enum(id(s(&params, "component_mode"))),
                    );
                    set(tx, "use_angle", Value::Bool(bools(&params, "use_angle")));
                    set(
                        tx,
                        "angle_threshold",
                        Value::Float(f(&params, "angle_threshold_deg").to_radians()),
                    );
                    set(tx, "bevel_mode", Value::Enum(id(s(&params, "bevel_mode"))));
                    set(
                        tx,
                        "offset_mode",
                        Value::Enum(id(s(&params, "offset_mode"))),
                    );
                    set(tx, "offset", Value::Float(f(&params, "offset")));
                    set(
                        tx,
                        "subdivision",
                        Value::Int(params["subdivision"].as_integer().unwrap()),
                    );
                    set(
                        tx,
                        "depth",
                        Value::Float(f(&params, "depth_percent") / 100.0),
                    );
                    set(tx, "shape", Value::Enum(id(s(&params, "shape"))));
                    set(
                        tx,
                        "tension",
                        Value::Float(f(&params, "tension_percent") / 100.0),
                    );
                    set(tx, "mitering", Value::Enum(id(s(&params, "mitering"))));
                    set(tx, "ending", Value::Enum(id(s(&params, "ending"))));
                    set(
                        tx,
                        "partial_rounding",
                        Value::Enum(id(s(&params, "partial_rounding"))),
                    );
                }
            }
            Ok(object)
        })
        .unwrap();
    let mesh = evaluate(doc.scene(), object, &forge_core::EvalCtx::default())
        .unwrap()
        .unwrap();
    let world = doc.scene().local_transform(object).unwrap().matrix();
    let ours = Mesh {
        positions: mesh
            .points
            .iter()
            .map(|p| world.transform_point3(*p).to_array())
            .collect(),
        faces: mesh.polys.clone(),
    };
    let expected = case.expected_mesh().unwrap().unwrap();
    (expected, ours)
}

#[test]
fn o19_deformers_match_or_are_pinned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut pinned = std::collections::BTreeSet::new();
    let mut failed = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if !FEATURES.contains(&case.feature.as_str()) {
            continue;
        }
        let key = format!("{}/{}", case.feature, case.name);
        let (expected, ours) = build(&case);
        let (e, o) = (export_weld(&expected), export_weld(&ours));
        let voxel = match case.feature.as_str() {
            "bevel-deformer" => 2.0,
            _ => 10.0,
        };
        let r = compare_volume_mesh(&e, &o, voxel);
        println!(
            "{key}: counts {} / {} faces {} / {} volume {} {:?}",
            e.positions.len(),
            o.positions.len(),
            e.faces.len(),
            o.faces.len(),
            r.passed(),
            r.failures
        );
        if let Some((_, _reason)) = KNOWN_MISMATCH.iter().find(|(k, _)| *k == key) {
            assert!(!r.passed(), "{key} matches now");
            pinned.insert(key);
        } else if r.passed() {
            continue;
        } else {
            failed.push(key);
        }
    }
    assert!(failed.is_empty(), "unlisted mismatches: {failed:?}");
}
