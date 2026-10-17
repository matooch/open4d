//! docs/specs/SMOOTHING.md §2.

mod common;

use std::f64::consts::PI;

use forge_core::smoothing::{TAG, smoothing_angle};
use forge_core::{Category, ParamKind, Value};

#[test]
fn builtin_tag_matches_observed_defaults() {
    let r = common::registry();
    let t = r.get(TAG).expect("smoothing tag is built in");
    assert_eq!(t.category, Category::Tag);
    let p = |id: &str| t.find_param(id).unwrap();
    assert_eq!(p("angle_limit").default, Value::Bool(true));
    assert_eq!(p("edge_breaks").default, Value::Bool(true));
    let angle = p("angle");
    assert_eq!(angle.default, Value::Float(40f64.to_radians()));
    assert_eq!(angle.kind, ParamKind::Float { min: 0.0, max: PI });
    assert_eq!(angle.ui.unit, forge_core::Unit::Angle);
}

#[test]
fn first_smoothing_tag_decides() {
    let mut doc = common::doc();
    let (plain, tagged, off) = doc
        .transact("build", |tx| {
            let plain = tx.create("test.box", None, None)?;
            let tagged = tx.create("test.box", None, None)?;
            let t1 = tx.attach_tag(TAG, tagged, None)?;
            let t2 = tx.attach_tag(TAG, tagged, None)?;
            tx.set_param(t2, "angle", Value::Float(1.0))?; // second tag ignored
            let _ = t1;
            let off = tx.create("test.box", None, None)?;
            let t3 = tx.attach_tag(TAG, off, None)?;
            tx.set_param(t3, "angle_limit", Value::Bool(false))?;
            Ok((plain, tagged, off))
        })
        .unwrap();
    let s = doc.scene();
    assert_eq!(smoothing_angle(s, plain), None, "no tag: faceted");
    assert_eq!(smoothing_angle(s, tagged), Some(40f64.to_radians()));
    assert_eq!(
        smoothing_angle(s, off),
        Some(PI),
        "angle limit off: fully smooth"
    );
    assert_eq!(smoothing_angle(s, forge_core::NodeId(999)), None);
}
