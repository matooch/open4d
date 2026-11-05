//! The Delay effector through a scene (clone-toolset spec E23): the earlier frames are
//! replayed from the cloner and the effectors before the Delay.

use std::sync::Arc;

use forge_clone_engine::CloneToolset;
use forge_core::{Document, EvalCtx, NodeId, Registry, Value};
use forge_generators::{BuildError, Primitives, clone_data};

fn scene() -> (Document, NodeId, NodeId, NodeId) {
    let mut reg = Registry::with_builtins();
    reg.install(&Primitives).unwrap();
    reg.install(&CloneToolset).unwrap();
    let mut doc = Document::new(Arc::new(reg));
    let ids = doc
        .transact("scene", |tx| {
            let cloner = tx.create("forge.cloner", None, None)?;
            tx.set_param(cloner, "mode", Value::Enum("linear".into()))?;
            tx.set_param(cloner, "linear_count", Value::Int(1))?;
            tx.create("forge.cube", Some(cloner), None)?;
            // Moves 300 cm per second along y.
            let time = tx.create("forge.effector.time", None, None)?;
            tx.set_param(time, "position", Value::Bool(true))?;
            tx.set_param(time, "p", Value::Vec3([0.0, 300.0, 0.0]))?;
            tx.set_param(time, "rotation", Value::Bool(false))?;
            let delay = tx.create("forge.effector.delay", None, None)?;
            tx.set_param(cloner, "effectors", Value::LinkList(vec![time, delay]))?;
            Ok((cloner, time, delay))
        })
        .unwrap();
    (doc, ids.0, ids.1, ids.2)
}

fn y_at(doc: &Document, cloner: NodeId, frame: i64) -> f64 {
    clone_data(doc.scene(), cloner, &EvalCtx::at_frame(frame, 30.0))
        .unwrap()
        .unwrap()
        .matrices[0]
        .w_axis
        .y
}

#[test]
fn delay_lags_behind_a_moving_input_and_is_stable_per_frame() {
    let (doc, cloner, _, _) = scene();
    // The input is 10 cm per frame. Even mode at 50 %: y_f = y_(f-1) + (x_f - y_(f-1)) / 2.
    assert_eq!(y_at(&doc, cloner, 0), 0.0);
    let mut y = 0.0;
    for f in 1..=12 {
        y += (10.0 * f as f64 - y) * 0.5;
        assert!((y_at(&doc, cloner, f) - y).abs() < 1e-9, "frame {f}");
    }
    // Evaluating a frame twice, or after another frame, gives the same result.
    assert_eq!(y_at(&doc, cloner, 12), y_at(&doc, cloner, 12));
    // The delayed value stays behind the undelayed one.
    assert!(y_at(&doc, cloner, 12) < 120.0);
}

#[test]
fn delay_at_strength_zero_and_disabled_equal_the_input() {
    let (mut doc, cloner, _, delay) = scene();
    doc.transact("strength", |tx| {
        tx.set_param(delay, "strength", Value::Float(0.0))
    })
    .unwrap();
    assert!((y_at(&doc, cloner, 9) - 90.0).abs() < 1e-9);
    doc.transact("strength", |tx| {
        tx.set_param(delay, "strength", Value::Float(0.5))?;
        tx.set_param(delay, "enabled", Value::Bool(false))
    })
    .unwrap();
    assert!((y_at(&doc, cloner, 9) - 90.0).abs() < 1e-9);
}

#[test]
fn a_second_delay_is_unsupported() {
    let (mut doc, cloner, time, delay) = scene();
    let second = doc
        .transact("second", |tx| {
            let d = tx.create("forge.effector.delay", None, None)?;
            tx.set_param(cloner, "effectors", Value::LinkList(vec![time, delay, d]))?;
            Ok(d)
        })
        .unwrap();
    let _ = second;
    let err = clone_data(doc.scene(), cloner, &EvalCtx::at_frame(3, 30.0)).unwrap_err();
    assert!(matches!(err, BuildError::Unsupported(_)), "{err:?}");
}
