//! RIG.md §12: tag priorities decide what each rig tag sees. An IK goal that a constraint
//! moves (the O36 "ik-then-constraint" idea; O36 recorded no such case, so these are unit
//! tests of our own rule).

use std::sync::Arc;

use forge_core::{Document, LinkRow, NodeId, Registry, Value};
use forge_rig::tags::{apply_rig_tags, document_tags, run_order};
use forge_rig::{CONSTRAINT_TAG, IK_TAG, JOINT, Rig};
use glam::DVec3;

fn registry() -> Arc<Registry> {
    let mut reg = Registry::with_builtins();
    reg.install(&Rig).unwrap();
    Arc::new(reg)
}

struct Setup {
    doc: Document,
    tip: NodeId,
    ik: NodeId,
    con: NodeId,
}

/// Two joints under J1 (bone 100 along Y), a goal Null at (0, 200, 0) that a PSR constraint
/// moves to the driver at (100, 100, 0), and an IK tag on J1 aiming at the goal.
fn setup() -> Setup {
    let mut doc = Document::new(registry());
    let (tip, ik, con) = doc
        .transact("s", |tx| {
            let j1 = tx.create(JOINT, None, None)?;
            tx.set_param(
                j1,
                "transform.rot",
                Value::Vec3([std::f64::consts::PI, 0.0, 0.0]),
            )?;
            let j2 = tx.create(JOINT, Some(j1), None)?;
            tx.set_param(j2, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]))?;
            let j3 = tx.create(JOINT, Some(j2), None)?;
            tx.set_param(j3, "transform.pos", Value::Vec3([0.0, 100.0, 0.0]))?;
            let goal = tx.create("forge.null", None, None)?;
            tx.set_param(goal, "transform.pos", Value::Vec3([0.0, 200.0, 0.0]))?;
            let driver = tx.create("forge.null", None, None)?;
            tx.set_param(driver, "transform.pos", Value::Vec3([100.0, 100.0, 0.0]))?;
            let ik = tx.attach_tag(IK_TAG, j1, None)?;
            tx.set_param(ik, "end", Value::Link(Some(j3)))?;
            tx.set_param(ik, "goal", Value::Link(Some(goal)))?;
            let con = tx.attach_tag(CONSTRAINT_TAG, goal, None)?;
            tx.set_param(
                con,
                "targets",
                Value::LinkRows(vec![LinkRow::new(driver, "normal")]),
            )?;
            Ok((j3, ik, con))
        })
        .unwrap();
    Setup { doc, tip, ik, con }
}

fn tip_pos(s: &Setup, scene: &forge_core::Scene) -> DVec3 {
    scene.world_matrix(s.tip).unwrap().w_axis.truncate()
}

#[test]
fn the_constraint_runs_before_the_ik_when_its_number_is_lower() {
    let mut s = setup();
    s.doc
        .transact("p", |tx| {
            tx.set_param(s.con, "priority", Value::Int(-1))?;
            tx.set_param(s.ik, "priority", Value::Int(1))
        })
        .unwrap();
    let out = apply_rig_tags(s.doc.scene().clone());
    assert!((tip_pos(&s, &out) - DVec3::new(100.0, 100.0, 0.0)).length() < 1e-9);
}

#[test]
fn the_ik_sees_the_stale_goal_when_it_runs_first() {
    let mut s = setup();
    s.doc
        .transact("p", |tx| {
            tx.set_param(s.ik, "priority", Value::Int(-1))?;
            tx.set_param(s.con, "priority", Value::Int(1))
        })
        .unwrap();
    let out = apply_rig_tags(s.doc.scene().clone());
    assert!((tip_pos(&s, &out) - DVec3::new(0.0, 200.0, 0.0)).length() < 1e-9);
}

#[test]
fn groups_come_before_numbers_and_ties_keep_document_order() {
    let mut s = setup();
    let ids = document_tags(s.doc.scene(), &[IK_TAG, CONSTRAINT_TAG]);
    assert_eq!(ids, vec![s.ik, s.con]);
    // equal priority: document order
    assert_eq!(run_order(s.doc.scene(), &ids), vec![s.ik, s.con]);
    // a later group beats a lower number
    s.doc
        .transact("g", |tx| {
            tx.set_param(s.ik, "priority_group", Value::Enum("dynamics".into()))?;
            tx.set_param(s.ik, "priority", Value::Int(-500))
        })
        .unwrap();
    assert_eq!(run_order(s.doc.scene(), &ids), vec![s.con, s.ik]);
}

#[test]
fn defaults_are_expression_zero_and_other_tags_sort_with_them() {
    let s = setup();
    assert_eq!(forge_rig::tags::priority_key(s.doc.scene(), s.ik), (2, 0));
    assert_eq!(forge_rig::tags::priority_key(s.doc.scene(), s.tip), (2, 0));
}
