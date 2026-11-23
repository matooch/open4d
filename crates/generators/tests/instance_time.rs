//! O50 default Object instance and Forge3D's authored Trail mode.

use std::{path::Path, sync::Arc};

use forge_core::{Document, EvalCtx, Key, KeyTangent, Registry, Track, Value};
use forge_generators::{Primitives, evaluate, instance_time, is_consumed};
use forge_parity_harness::{
    Mesh, Samples, compare_meshes, compare_samples, discover_cases, export_weld,
    samples::Clone as SampleClone,
};

fn doc() -> Document {
    let mut registry = Registry::with_builtins();
    registry.install(&Primitives).unwrap();
    Document::new(Arc::new(registry))
}

fn linear(keys: &[(f64, f64)]) -> Track {
    let mut track = Track::new(keys.iter().map(|&(t, v)| Key::new(t, v)).collect());
    for i in 0..keys.len() {
        track.set_tangent(i, KeyTangent::chord());
    }
    track
}

#[test]
fn o50_object_mode_matches_clone_and_exported_scene() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let cases = discover_cases(&root).unwrap();
    let selected: Vec<_> = cases
        .iter()
        .filter(|case| case.feature == "instance-time")
        .collect();
    assert_eq!(selected.len(), 1);
    let case = selected[0];
    assert_eq!(case.name, "defaults");
    let manifest = case.manifest().unwrap();
    let p = &manifest.params;
    assert_eq!(p["distribution"].as_str(), Some("Object"));
    let fps = p["document_fps"].as_integer().unwrap() as f64;
    let frame = p["export_frame"].as_integer().unwrap();
    let keys: Vec<_> = p["reference_key_x"]
        .as_array()
        .unwrap()
        .iter()
        .map(|key| {
            let k = key.as_array().unwrap();
            let x = k[1]
                .as_float()
                .or_else(|| k[1].as_integer().map(|x| x as f64))
                .unwrap();
            (k[0].as_integer().unwrap() as f64 / fps, x)
        })
        .collect();
    let mut doc = doc();
    let (node, cube) = doc
        .transact("O50 scene", |tx| {
            let cube = tx.create("forge.cube", None, None)?;
            tx.set_track(cube, "transform.pos.x", Some(linear(&keys)))?;
            let node = tx.create(instance_time::TYPE, None, None)?;
            tx.set_param(node, "reference", Value::Link(Some(cube)))?;
            Ok((node, cube))
        })
        .unwrap();
    let ctx = EvalCtx::at_frame(frame, fps);
    let scene = forge_anim::scene_at(doc.scene(), ctx.time);
    let clones = instance_time::clone_data(&scene, node, &ctx)
        .unwrap()
        .unwrap();
    let samples = Samples::Clones {
        tolerance: 1e-4,
        clones: clones
            .matrices
            .iter()
            .enumerate()
            .map(|(i, m)| SampleClone {
                pos: m.w_axis.truncate().to_array(),
                x: m.x_axis.truncate().to_array(),
                y: m.y_axis.truncate().to_array(),
                z: m.z_axis.truncate().to_array(),
                color: clones.colors[i],
                weight: clones.weights[i],
                visible: clones.visible[i],
            })
            .collect(),
    };
    let (_, expected) = case.samples().unwrap().remove(0);
    let result = compare_samples(&expected, &samples);
    assert!(result.passed(), "{:?}", result.failures);
    assert!(!is_consumed(&scene, cube));
    let mut mesh = Mesh::default();
    for id in [node, cube] {
        let local = evaluate(&scene, id, &ctx).unwrap().unwrap();
        let base = mesh.positions.len() as u32;
        let world = scene.world_matrix(id).unwrap();
        mesh.positions.extend(
            local
                .points
                .iter()
                .map(|p| world.transform_point3(*p).to_array()),
        );
        mesh.faces.extend(
            local
                .polys
                .iter()
                .map(|f| f.iter().map(|i| base + i).collect()),
        );
    }
    let want = export_weld(&case.expected_mesh().unwrap().unwrap());
    let result = compare_meshes(&want, &export_weld(&mesh), &manifest.tolerance);
    assert!(result.passed(), "{:?}", result.failures);
}

#[test]
fn trail_samples_ancestors_at_exact_frames_in_current_local_space() {
    let mut doc = doc();
    let (node, cube) = doc
        .transact("Trail scene", |tx| {
            let node = tx.create(instance_time::TYPE, None, None)?;
            tx.set_param(node, "mode", Value::Enum("trail".into()))?;
            tx.set_param(node, "start_frame", Value::Int(1))?;
            tx.set_param(node, "step", Value::Int(2))?;
            tx.set_param(node, "end_frame", Value::Int(5))?;
            tx.set_track(
                node,
                "transform.pos.x",
                Some(linear(&[(0.0, 0.0), (1.0, 30.0)])),
            )?;
            let cube = tx.create("forge.cube", Some(node), None)?;
            tx.set_track(
                cube,
                "transform.pos.x",
                Some(linear(&[(0.0, 0.0), (1.0, 300.0)])),
            )?;
            Ok((node, cube))
        })
        .unwrap();
    let ctx = EvalCtx::at_frame(4, 30.0);
    let scene = doc
        .scene()
        .with_params(forge_anim::overrides_exact(
            doc.scene(),
            &[node, cube],
            ctx.time,
        ))
        .0;
    let data = instance_time::clone_data(&scene, node, &ctx)
        .unwrap()
        .unwrap();
    let x: Vec<_> = data.matrices.iter().map(|m| m.w_axis.x).collect();
    let x_at = |id, frame| {
        let track = &scene.node(id).unwrap().tracks["transform.pos.x"];
        forge_anim::value_at_exact(track, &[], f64::from(frame) / ctx.fps)
    };
    for (got, frame) in x.iter().zip([1, 3]) {
        let expected = (x_at(cube, frame) + x_at(node, frame)) - x_at(node, 4);
        assert!((got - expected).abs() < 1e-6, "{x:?}");
    }
    assert_eq!(x.len(), 2);
    assert!(data.time.iter().all(|t| *t < 0.0));
    assert!(is_consumed(&scene, cube));
    let subframe = EvalCtx::at_time(2.75 / ctx.fps, ctx.fps);
    let data = instance_time::clone_data(&scene, node, &subframe)
        .unwrap()
        .unwrap();
    assert_eq!(data.len(), 1, "do not include future samples");
    assert_eq!(
        evaluate(&scene, node, &ctx).unwrap().unwrap().points.len(),
        16
    );
    assert!(
        evaluate(&scene, node, &EvalCtx::at_frame(0, 30.0))
            .unwrap()
            .is_none()
    );
}

#[test]
fn object_reference_cycle_does_not_produce_geometry() {
    let mut doc = doc();
    let node = doc
        .transact("Reference cycle", |tx| {
            let a = tx.create(instance_time::TYPE, None, None)?;
            let b = tx.create(instance_time::TYPE, None, None)?;
            tx.set_param(a, "reference", Value::Link(Some(b)))?;
            tx.set_param(b, "reference", Value::Link(Some(a)))?;
            Ok(a)
        })
        .unwrap();
    let ctx = EvalCtx::default();
    assert!(evaluate(doc.scene(), node, &ctx).unwrap().is_none());
    let data = instance_time::clone_data(doc.scene(), node, &ctx).unwrap();
    assert!(data.is_none());
}
