//! O22 remaining effectors (brief 071).

use forge_app::App;
use forge_clone_engine::CloneData;
use forge_core::{EvalCtx, NodeId, Value};
use forge_parity_harness::{Samples, compare_samples, samples::Clone as Sample};
use glam::DVec3;

fn observed(case: &str) -> (App, NodeId, Samples) {
    let folder = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/cloner")
        .join(case);
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(folder.join("case.toml")).unwrap()).unwrap();
    let p = &manifest["params"];
    let v = &p["effector"];
    let string = |key: &str| v[key].as_str().unwrap();
    let vec = |v: &toml::Value| {
        let v = v.as_array().unwrap();
        Value::Vec3(std::array::from_fn(|i| v[i].as_float().unwrap()))
    };
    let samples = forge_parity_harness::samples::parse_samples(
        &std::fs::read_to_string(folder.join("samples.json")).unwrap(),
    )
    .unwrap();
    let mut app = App::new_default();
    let cloner = app
        .doc
        .transact("O22 setup", |tx| {
            let cloner = tx.create("forge.cloner", None, None)?;
            tx.create("forge.cube", Some(cloner), None)?;
            let mode = p["mode"].as_str().unwrap();
            tx.set_param(cloner, "mode", Value::Enum(mode.into()))?;
            if mode == "grid" {
                tx.set_param(cloner, "grid_count", vec(&p["count"]))?;
                tx.set_param(cloner, "grid_size", vec(&p["size"]))?;
                tx.set_param(cloner, "grid_mode", Value::Enum("per_step".into()))?;
            } else {
                tx.set_param(
                    cloner,
                    "linear_count",
                    Value::Int(p["count"].as_integer().unwrap()),
                )?;
            }
            let kind = string("type");
            let e = tx.create(&format!("forge.effector.{kind}"), None, None)?;
            tx.set_param(cloner, "effectors", Value::LinkList(vec![e]))?;
            tx.set_param(
                e,
                "strength",
                Value::Float(v["strength_percent"].as_float().unwrap() / 100.0),
            )?;
            if let Some(p) = v.get("p") {
                tx.set_param(e, "position", Value::Bool(v["position"].as_bool().unwrap()))?;
                tx.set_param(e, "p", vec(p))?;
            }
            match kind {
                "shader" => {
                    let mat = tx.create_material("forge.material.standard", None)?;
                    let shader = tx.create_shader("forge.shader.noise", mat, None)?;
                    let p = &v["shader_object"];
                    for key in ["seed", "octaves"] {
                        let value = p[key]
                            .as_integer()
                            .or_else(|| p[key].as_float().map(|v| v as i64))
                            .unwrap();
                        tx.set_param(shader, key, Value::Int(value))?;
                    }
                    tx.set_param(e, "shader", Value::Link(Some(shader)))?;
                }
                "spline" => {
                    let path = tx.create("forge.helix_spline", None, None)?;
                    tx.set_param(e, "spline", Value::Link(Some(path)))?;
                    tx.set_param(e, "mode", Value::Enum(string("mode").to_lowercase()))?;
                }
                "push_apart" => {
                    tx.set_param(e, "radius", Value::Float(v["radius"].as_float().unwrap()))?;
                    tx.set_param(
                        e,
                        "iterations",
                        Value::Int(v["iterations"].as_integer().unwrap()),
                    )?;
                }
                "inheritance" => {
                    let src = tx.create("forge.cloner", None, None)?;
                    tx.create("forge.cube", Some(src), None)?;
                    tx.set_param(src, "mode", Value::Enum("radial".into()))?;
                    tx.set_param(e, "object", Value::Link(Some(src)))?;
                }
                "volume" => {
                    let src = tx.create("forge.cube", None, None)?;
                    let Value::Vec3(size) = vec(&v["volume_cube"]["size"]) else {
                        unreachable!()
                    };
                    for (axis, component) in ["x", "y", "z"].into_iter().zip(size) {
                        tx.set_param(src, &format!("size_{axis}"), Value::Float(component))?;
                    }
                    tx.set_param(e, "volume_object", Value::Link(Some(src)))?;
                }
                _ => unreachable!(),
            }
            Ok(cloner)
        })
        .unwrap();
    (app, cloner, samples)
}

fn data(app: &App, id: NodeId) -> CloneData {
    forge_generators::clone_data(app.doc.scene(), id, &EvalCtx::default())
        .unwrap()
        .unwrap()
}

#[test]
fn o22_remaining_effectors() {
    let mut mismatches = Vec::new();
    for name in [
        "shader-noise",
        "spline-helix",
        "spline-helix-segment",
        "push-apart",
        "push-apart-iterations",
        "inheritance",
        "volume",
    ] {
        let (app, id, expected) = observed(name);
        let d = data(&app, id);
        let actual = Samples::Clones {
            tolerance: expected.tolerance(),
            clones: d
                .matrices
                .iter()
                .enumerate()
                .map(|(i, m)| Sample {
                    pos: m.w_axis.truncate().to_array(),
                    x: m.x_axis.truncate().to_array(),
                    y: m.y_axis.truncate().to_array(),
                    z: m.z_axis.truncate().to_array(),
                    color: d.colors[i],
                    weight: d.weights[i],
                    visible: d.visible[i],
                })
                .collect(),
        };
        let r = compare_samples(&expected, &actual);
        if !r.passed() {
            mismatches.push(name);
        }
        let (
            Samples::Clones {
                clones: expected,
                tolerance,
            },
            Samples::Clones { clones: actual, .. },
        ) = (&expected, &actual)
        else {
            unreachable!()
        };
        assert_eq!(actual.len(), expected.len(), "{name}");
        assert!(actual.iter().all(|c| c.visible), "{name}");
        match name {
            "shader-noise" => {
                let mut shifts = Vec::new();
                for (i, a) in actual.iter().enumerate() {
                    let shift = (a.pos[1] - 50.0 * i as f64) / 100.0;
                    assert!((0.0..=1.0).contains(&shift));
                    assert_eq!((a.pos[0], a.pos[2]), (0.0, 0.0));
                    assert!((a.x[0] - (1.0 + 0.5 * shift)).abs() <= *tolerance);
                    shifts.push(shift);
                }
                assert!(shifts.windows(2).any(|w| (w[1] - w[0]).abs() > 1e-4));
                let observed_mean = expected
                    .iter()
                    .enumerate()
                    .map(|(i, a)| (a.pos[1] - 50.0 * i as f64) / 100.0)
                    .sum::<f64>()
                    / expected.len() as f64;
                let mean = shifts.iter().sum::<f64>() / shifts.len() as f64;
                assert!((mean - observed_mean).abs() < 0.2);
                assert_eq!(data(&app, id).matrices, d.matrices);
            }
            "spline-helix" | "spline-helix-segment" => {
                for (a, b) in actual.iter().zip(expected) {
                    assert!(
                        DVec3::from_array(a.pos).abs_diff_eq(DVec3::from_array(b.pos), *tolerance),
                        "{name}: position {:?} vs {:?}",
                        a.pos,
                        b.pos
                    );
                    let [x, y, z] = [a.x, a.y, a.z].map(DVec3::from_array);
                    assert!(
                        x.cross(y).abs_diff_eq(z, *tolerance),
                        "{name}: invalid frame"
                    );
                }
            }
            "push-apart" | "push-apart-iterations" => {
                let positions: Vec<_> = actual.iter().map(|a| DVec3::from_array(a.pos)).collect();
                assert!(positions.iter().copied().sum::<DVec3>().length() <= *tolerance);
                let nearest = positions
                    .iter()
                    .enumerate()
                    .flat_map(|(i, p)| positions[i + 1..].iter().map(move |q| p.distance(*q)))
                    .fold(f64::INFINITY, f64::min);
                assert!(nearest > 30.0, "{name}: nearest distance did not improve");
                for (a, b) in actual.iter().zip(expected) {
                    assert_eq!((a.x, a.y, a.z), (b.x, b.y, b.z));
                }
            }
            _ => {}
        }
    }
    assert_eq!(
        mismatches,
        [
            "shader-noise",
            "spline-helix",
            "spline-helix-segment",
            "push-apart",
            "push-apart-iterations"
        ]
    );
}

#[test]
fn group_applies_its_children_at_its_slot() {
    let mut app = App::new_default();
    let id = app
        .doc
        .transact("Grouped transforms", |tx| {
            let c = tx.create("forge.cloner", None, None)?;
            tx.set_param(c, "mode", Value::Enum("linear".into()))?;
            let group = tx.create("forge.effector.group", None, None)?;
            tx.create("forge.effector.plain", Some(group), None)?;
            let second = tx.create("forge.effector.plain", Some(group), None)?;
            tx.set_param(second, "p", Value::Vec3([12., 0., 0.]))?;
            tx.set_param(c, "effectors", Value::LinkList(vec![group]))?;
            Ok(c)
        })
        .unwrap();
    assert_eq!(
        data(&app, id).matrices[0].w_axis.truncate(),
        DVec3::new(12., 100., 0.)
    );
}
