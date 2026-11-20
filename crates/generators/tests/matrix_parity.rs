//! Parity: clone-toolset spec M1 against tests/golden/matrix (level B clone data). The cases'
//! `mesh.obj` is the standalone 20 cm cube the observer exported next to the Matrix, which has
//! no geometry of its own.

use std::path::Path;
use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, ParamKind, Registry, Value};
use forge_generators::{BuildError, Primitives, evaluate, matrix_data};
use forge_parity_harness::{
    Mesh, Samples, compare_meshes, compare_samples, discover_cases, export_weld,
    samples::Clone as SampleClone,
};

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    Arc::new(reg)
}

fn floats(v: &toml::Value) -> Vec<f64> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|x| x.as_float().or(x.as_integer().map(|i| i as f64)).unwrap())
        .collect()
}

/// The Matrix parameter a case key sets in `mode` (the Linear tab and the Grid tab reuse
/// labels such as `p`), and the factor from the case's unit to the stored one.
fn target(mode: &str, key: &str) -> Option<(&'static str, f64)> {
    let deg = std::f64::consts::PI / 180.0;
    let t = match (mode, key) {
        (_, "generator" | "mode" | "generate" | "display" | "w_uv_orientation") => return None,
        ("Grid", "count") => ("grid_count", 1.0),
        ("Grid", "count_mode") => ("grid_mode", 1.0),
        ("Grid", "size") => ("grid_size", 1.0),
        ("Grid", "form") => ("grid_form", 1.0),
        ("Grid", "fill_percent") => ("fill", 0.01),
        ("Grid", "p") => ("clone_position", 1.0),
        ("Grid", "r_deg") => ("clone_rotation", deg),
        ("Grid", "s") => ("clone_scale", 1.0),
        ("Linear", "count") => ("linear_count", 1.0),
        ("Linear", "offset") => ("linear_offset", 1.0),
        ("Linear", "per_step_mode") => ("linear_mode", 1.0),
        ("Linear", "amount_percent") => ("amount", 0.01),
        ("Linear", "p") => ("transform_position", 1.0),
        ("Linear", "s_percent") => ("transform_scale", 0.01),
        ("Linear", "r_deg") => ("transform_rotation", deg),
        ("Linear", "step_mode") => ("step_mode", 1.0),
        ("Linear", "step_size_percent") => ("step_size", 0.01),
        ("Linear", "step_rotation_deg") => ("step_rotation", deg),
        ("Linear", "effects_p") => ("clone_position", 1.0),
        ("Linear", "effects_r_deg") => ("clone_rotation", deg),
        ("Linear", "s") => ("clone_scale", 1.0),
        (_, "color") => ("clone_color", 1.0),
        (_, "weight_percent") => ("clone_weight", 0.01),
        other => panic!("unmapped case key {other:?}"),
    };
    Some(t)
}

fn value(ty: &forge_core::NodeType, id: &str, raw: &toml::Value, scale: f64) -> Value {
    let desc = ty.find_param(id).unwrap();
    let v = match (&desc.kind, raw) {
        (ParamKind::Int { .. }, toml::Value::Integer(i)) => Value::Int(*i),
        (ParamKind::Float { .. }, v) => Value::Float(
            v.as_float()
                .unwrap_or_else(|| v.as_integer().unwrap() as f64)
                * scale,
        ),
        (ParamKind::Vec3 { .. }, v) => {
            let f = floats(v);
            Value::Vec3([f[0] * scale, f[1] * scale, f[2] * scale])
        }
        (ParamKind::Color, v) => {
            let f = floats(v);
            Value::Color([f[0], f[1], f[2]])
        }
        (ParamKind::Enum { options }, toml::Value::String(s)) => Value::Enum(
            options
                .iter()
                .find(|o| o.label.eq_ignore_ascii_case(s))
                .unwrap_or_else(|| panic!("{id}: no option {s:?}"))
                .id
                .clone(),
        ),
        other => panic!("{id}: unexpected {other:?}"),
    };
    desc.validate(v).expect("valid case value")
}

#[test]
fn matrix_cases_match_the_reference() {
    let reg = registry();
    let ty = reg.get("forge.matrix").unwrap().clone();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let ctx = EvalCtx::default();
    let mut matched = Vec::new();
    for case in discover_cases(&root).unwrap() {
        if case.feature != "matrix" {
            continue;
        }
        let m = case.manifest().unwrap();
        let mode = m.params["mode"].as_str().unwrap().to_string();
        let mut doc = Document::new(reg.clone());
        let id = doc
            .transact("case", |tx| {
                let id = tx.create("forge.matrix", None, None)?;
                tx.set_param(
                    id,
                    "mode",
                    value(&ty, "mode", &toml::Value::String(mode.clone()), 1.0),
                )?;
                for (k, raw) in &m.params {
                    if let Some((p, scale)) = target(&mode, k) {
                        tx.set_param(id, p, value(&ty, p, raw, scale))?;
                    }
                }
                Ok(id)
            })
            .unwrap();

        let data = matrix_data(doc.scene(), id, &ctx).unwrap().unwrap();
        assert!(data.is_consistent());
        let ours = Samples::Clones {
            tolerance: 1e-4,
            clones: (0..data.len())
                .map(|i| {
                    let m = data.matrices[i];
                    SampleClone {
                        pos: m.w_axis.truncate().to_array(),
                        x: m.x_axis.truncate().to_array(),
                        y: m.y_axis.truncate().to_array(),
                        z: m.z_axis.truncate().to_array(),
                        color: data.colors[i],
                        weight: data.weights[i],
                        visible: data.visible[i],
                    }
                })
                .collect(),
        };
        let (_, expected) = case.samples().unwrap().remove(0);
        let r = compare_samples(&expected, &ours);
        assert!(r.passed(), "{}: {:?}", case.name, r.failures);

        // The Matrix draws nothing; the case's mesh is the separate 20 cm cube.
        assert!(evaluate(doc.scene(), id, &ctx).unwrap().is_none());
        let cube = doc
            .transact("cube", |tx| {
                let c = tx.create("forge.cube", None, None)?;
                for a in ["size_x", "size_y", "size_z"] {
                    tx.set_param(c, a, Value::Float(20.0))?;
                }
                Ok(c)
            })
            .unwrap();
        let mesh = evaluate(doc.scene(), cube, &ctx).unwrap().unwrap();
        let ours = export_weld(&Mesh {
            positions: mesh.points.iter().map(|p| p.to_array()).collect(),
            faces: mesh.polys.clone(),
        });
        let want = export_weld(&case.expected_mesh().unwrap().unwrap());
        let g = compare_meshes(&want, &ours, &m.tolerance);
        assert!(g.passed(), "{}: {:?}", case.name, g.failures);
        matched.push(case.name.clone());
    }
    assert_eq!(matched.len(), 2, "{matched:?}");
}

#[test]
fn matrix_options_without_rules_are_unsupported() {
    let reg = registry();
    let mut doc = Document::new(reg);
    let id = doc
        .transact("m", |tx| {
            let id = tx.create("forge.matrix", None, None)?;
            tx.set_param(id, "generate", Value::Enum("thinking_particles".into()))?;
            Ok(id)
        })
        .unwrap();
    assert!(matches!(
        evaluate(doc.scene(), id, &EvalCtx::default()),
        Err(BuildError::Unsupported(_))
    ));
    // Object mode shares the cloner's (brief 066).
    doc.transact("o", |tx| {
        tx.set_param(id, "generate", Value::Enum("matrices_only".into()))
    })
    .unwrap();
    doc.transact("o", |tx| {
        tx.set_param(id, "mode", Value::Enum("object".into()))
    })
    .unwrap();
    assert!(matrix_data(doc.scene(), id, &EvalCtx::default()).is_err());
}
