//! Hooks the evaluation scheduler uses (docs/specs/CORE.md section 5b): parallel cloner
//! expansion equals the serial one bit for bit, sibling-deformer dependencies, time readers.

use std::sync::Arc;

use forge_clone_engine::{CloneToolset, effector_reads_time};
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{PARALLEL_POINTS, Primitives, eval_dependencies, evaluate, expand_clones};

fn doc() -> Document {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    reg.install(&forge_fields::FieldToolset).unwrap();
    reg.install(&forge_modifiers::Deformers).unwrap();
    Document::new(Arc::new(reg))
}

fn bits(m: &forge_geometry::PolyMesh) -> Vec<[u64; 3]> {
    m.points
        .iter()
        .map(|p| p.to_array().map(f64::to_bits))
        .collect()
}

#[test]
fn parallel_expansion_is_bit_identical_to_serial() {
    let mut d = doc();
    let c = d
        .transact("c", |tx| {
            let c = tx.create("forge.cloner", None, None)?;
            tx.set_param(c, "mode", Value::Enum("linear".into()))?;
            tx.set_param(c, "linear_count", Value::Int(700))?;
            tx.create("forge.sphere", Some(c), None)?;
            Ok(c)
        })
        .unwrap();
    let ctx = EvalCtx::default();
    let data = forge_generators::clone_data(d.scene(), c, &ctx)
        .unwrap()
        .unwrap();
    let proto = evaluate(d.scene(), d.scene().node(c).unwrap().children[0], &ctx)
        .unwrap()
        .unwrap();
    let serial = expand_clones(std::slice::from_ref(&proto), &data, false);
    let parallel = expand_clones(std::slice::from_ref(&proto), &data, true);
    assert_eq!(serial.polys, parallel.polys);
    assert_eq!(bits(&serial), bits(&parallel));
    assert!(serial.points.len() >= PARALLEL_POINTS);
    // and the public path (parallel by size) gives the same mesh
    let via_evaluate = evaluate(d.scene(), c, &ctx).unwrap().unwrap();
    assert_eq!(bits(&via_evaluate), bits(&serial));
}

#[test]
fn a_deformer_under_a_null_is_a_dependency_of_its_sibling_objects() {
    let mut d = doc();
    let (null, cube, bend, lone) = d
        .transact("s", |tx| {
            let null = tx.create("forge.null", None, None)?;
            let cube = tx.create("forge.cube", Some(null), None)?;
            let bend = tx.create("forge.bend", Some(null), None)?;
            let lone = tx.create("forge.cube", None, None)?;
            Ok((null, cube, bend, lone))
        })
        .unwrap();
    assert_eq!(eval_dependencies(d.scene(), cube), vec![bend]);
    assert!(eval_dependencies(d.scene(), lone).is_empty());
    assert!(eval_dependencies(d.scene(), null).is_empty());
}

#[test]
fn time_readers_are_noise_modes_with_animation() {
    let mut d = doc();
    let make = |d: &mut Document, ty: &str, mode_param: &str, mode: &str, speed: f64| -> NodeId {
        d.transact("n", |tx| {
            let n = tx.create(ty, None, None)?;
            tx.set_param(n, mode_param, Value::Enum(mode.into()))?;
            tx.set_param(n, "animation_speed", Value::Float(speed))?;
            Ok(n)
        })
        .unwrap()
    };
    let scene_of = |d: &Document| d.scene().clone();
    let f_noise = make(&mut d, "forge.field.random", "random_mode", "noise", 1.0);
    let f_still = make(&mut d, "forge.field.random", "random_mode", "noise", 0.0);
    let f_rand = make(&mut d, "forge.field.random", "random_mode", "random", 1.0);
    let e_noise = make(&mut d, "forge.effector.random", "random_mode", "noise", 1.0);
    let e_rand = make(
        &mut d,
        "forge.effector.random",
        "random_mode",
        "random",
        1.0,
    );
    let s = scene_of(&d);
    assert!(forge_fields::reads_time(&s, f_noise));
    assert!(!forge_fields::reads_time(&s, f_still));
    assert!(!forge_fields::reads_time(&s, f_rand));
    assert!(effector_reads_time(&s, e_noise));
    assert!(!effector_reads_time(&s, e_rand));
}
