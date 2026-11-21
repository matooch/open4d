//! RIG.md §12 through the scheduler's tag stage: tracks, then expression, IK and constraint
//! tags in priority order, before any generator.

use forge_app::default_registry;
use forge_app::schedule::{animate, animation_inputs};
use forge_core::{Document, Key, LinkRow, Value};
use std::sync::Arc;

#[test]
fn an_animated_driver_moves_a_constrained_goal_and_the_ik_chain() {
    let mut doc = Document::new(Arc::new(default_registry().unwrap()));
    let (tip, ik, con) = doc
        .transact("s", |tx| {
            let j1 = tx.create("forge.joint", None, None)?;
            tx.set_param(
                j1,
                "transform.rot",
                Value::Vec3([std::f64::consts::PI, 0.0, 0.0]),
            )?;
            let j2 = tx.create("forge.joint", Some(j1), None)?;
            tx.set_param(j2, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]))?;
            let j3 = tx.create("forge.joint", Some(j2), None)?;
            tx.set_param(j3, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]))?;
            let goal = tx.create("forge.null", None, None)?;
            let driver = tx.create("forge.null", None, None)?;
            let key = |t: f64, x: f64| Key {
                time: t,
                value: x,
                interp: forge_core::Interp::Linear,
            };
            tx.set_key(driver, "transform.pos.x", key(0.0, 0.0))?;
            tx.set_key(driver, "transform.pos.x", key(1.0, 100.0))?;
            tx.set_param(driver, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]))?;
            let ik = tx.attach_tag("forge.ik_tag", j1, None)?;
            tx.set_param(ik, "end", Value::Link(Some(j3)))?;
            tx.set_param(ik, "goal", Value::Link(Some(goal)))?;
            tx.set_param(ik, "priority", Value::Int(1))?;
            let con = tx.attach_tag("forge.constraint_tag", goal, None)?;
            tx.set_param(
                con,
                "targets",
                Value::LinkRows(vec![LinkRow::new(driver, "normal")]),
            )?;
            Ok((j3, ik, con))
        })
        .unwrap();
    let _ = (ik, con);
    let (tracks, tags, enabled) = animation_inputs(doc.scene());
    assert!(enabled && tags.len() == 2);
    let out = animate(doc.scene(), &tracks, &tags, enabled, 1.0);
    let p = out.scene.world_matrix(tip).unwrap().w_axis;
    assert!(
        (p.x - 100.0).abs() < 1e-6 && (p.y - 100.0).abs() < 1e-6,
        "{p:?}"
    );
    assert!(out.touched.iter().any(|(_, k)| k == "transform.rot"));
}
