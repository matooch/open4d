//! Connector and spring behaviour the golden cases cannot see (docs/specs/DYNAMICS.md §3).

mod common;

use common::*;
use forge_core::{NodeId, Value};
use forge_dynamics::joints::{JointKind, links_of_scene};
use glam::DVec3;

fn pair(kind: &str) -> (Built, NodeId, NodeId, NodeId) {
    let mut left = Obj::cube("left", [-26.0, 300.0, 0.0]);
    left.dynamic = "off";
    let right = Obj::cube("right", [26.0, 300.0, 0.0]);
    let mut b = build(&[Obj::plane(), left, right]);
    let (l, r) = (b.ids["left"], b.ids["right"]);
    let c = b
        .doc
        .transact("connector", |tx| {
            Ok(add_connector(tx, kind, Some(l), Some(r)))
        })
        .unwrap();
    (b, l, r, c)
}

#[test]
fn the_connector_default_is_a_hinge_and_links_are_read_in_document_order() {
    let t = forge_dynamics::connector_type();
    assert_eq!(
        t.find_param("type").unwrap().default,
        Value::Enum("hinge".into())
    );
    assert_eq!(
        t.find_param("ignore_collisions").unwrap().default,
        Value::Bool(true)
    );
    let (b, l, r, _) = pair("hinge");
    let links = links_of_scene(b.doc.scene());
    assert_eq!(links.joints.len(), 1);
    assert_eq!(links.joints[0].kind, JointKind::Hinge);
    assert_eq!(links.joints[0].axis, DVec3::Z);
    assert_eq!(
        links.joints[0].a.key,
        Some(forge_dynamics::BodyKey::Node(l))
    );
    assert_eq!(
        links.joints[0].b.key,
        Some(forge_dynamics::BodyKey::Node(r))
    );
    let s = forge_dynamics::spring_type();
    assert_eq!(
        s.find_param("rest_length").unwrap().default,
        Value::Float(100.0)
    );
    assert_eq!(
        s.find_param("stiffness").unwrap().default,
        Value::Float(1.0)
    );
    assert_eq!(s.find_param("damping").unwrap().default, Value::Float(0.2));
}

#[test]
fn a_connector_without_objects_is_skipped() {
    let mut b = build(&[Obj::plane()]);
    b.doc
        .transact("c", |tx| Ok(add_connector(tx, "hinge", None, None)))
        .unwrap();
    assert!(links_of_scene(b.doc.scene()).joints.is_empty());
}

#[test]
fn a_fixed_connector_holds_the_body_against_gravity() {
    let (b, _, r, _) = pair("fixed");
    let sim = simulate(&b, 60);
    let m = sim[&r][60];
    assert!(
        (m.w_axis.truncate() - DVec3::new(26.0, 300.0, 0.0)).length() < 0.5,
        "{:?}",
        m.w_axis
    );
    for a in hpb_deg(&m) {
        assert!(
            a.abs() < 0.5 || (a.abs() - 360.0).abs() < 0.5,
            "rotated {a}"
        );
    }
}

#[test]
fn a_ball_socket_lets_the_body_swing_out_of_the_plane() {
    // A push along z swings a ball-and-socket body out of the x-y plane; a hinge about z cannot.
    let run = |kind: &str| {
        let mut left = Obj::cube("left", [-26.0, 300.0, 0.0]);
        left.dynamic = "off";
        let mut right = Obj::cube("right", [26.0, 300.0, 0.0]);
        right.velocity = Some([0.0, 0.0, 100.0]);
        let mut b = build(&[Obj::plane(), left, right]);
        let (l, r) = (b.ids["left"], b.ids["right"]);
        b.doc
            .transact("c", |tx| Ok(add_connector(tx, kind, Some(l), Some(r))))
            .unwrap();
        simulate(&b, 20)[&r][20].w_axis.z
    };
    assert!(run("ball_socket").abs() > 20.0);
    assert!(run("hinge").abs() < 1.0);
}

#[test]
fn a_slider_moves_only_along_its_axis() {
    let mut left = Obj::cube("left", [-26.0, 300.0, 0.0]);
    left.dynamic = "off";
    let mut right = Obj::cube("right", [26.0, 300.0, 0.0]);
    right.velocity = Some([200.0, 50.0, 30.0]);
    let mut b = build(&[Obj::plane(), left, right]);
    let (l, r) = (b.ids["left"], b.ids["right"]);
    b.doc
        .transact("c", |tx| Ok(add_connector(tx, "slider", Some(l), Some(r))))
        .unwrap();
    let m = simulate(&b, 30)[&r][30];
    assert!(m.w_axis.x > 100.0, "slid {}", m.w_axis.x);
    assert!(
        (m.w_axis.y - 300.0).abs() < 1.0 && m.w_axis.z.abs() < 1.0,
        "{:?}",
        m.w_axis
    );
}

#[test]
fn a_hinge_limit_stops_the_swing() {
    let (mut b, _, r, c) = pair("hinge");
    set(&mut b.doc, c, "angular_limit", Value::Bool(true));
    set(&mut b.doc, c, "limit_from", Value::Float(0.0));
    set(&mut b.doc, c, "limit_to", Value::Float(0.5));
    let sim = simulate(&b, 90);
    // Bank grows from 0 as in the unlimited case, and stops at the limit (0.5 rad = 28.6 deg).
    let bank: Vec<f64> = sim[&r].iter().map(|m| hpb_deg(m)[2]).collect();
    let top = bank.iter().cloned().fold(f64::MIN, f64::max);
    assert!((27.0..=30.5).contains(&top), "{top}");
    assert!(bank.iter().all(|b| *b > -1.0 || *b > 359.0), "{bank:?}");
}

#[test]
fn a_connector_with_one_object_hangs_it_from_the_world() {
    let mut b = build(&[Obj::plane(), Obj::cube("only", [26.0, 300.0, 0.0])]);
    let o = b.ids["only"];
    b.doc
        .transact("c", |tx| Ok(add_connector(tx, "hinge", None, Some(o))))
        .unwrap();
    let m = simulate(&b, 30)[&o][30];
    let r = DVec3::new(m.w_axis.x, m.w_axis.y, 0.0).length();
    assert!((r - 301.0).abs() < 1.5, "{r}");
}

#[test]
fn spring_forces_are_a_pair_and_zero_stiffness_does_nothing() {
    // Two dynamic cubes joined by a spring conserve their centre of mass in a free pull.
    let mut a = Obj::cube("a", [-100.0, 150.0, 0.0]);
    a.dynamic = "on";
    let c = Obj::cube("b", [100.0, 150.0, 0.0]);
    let mut b = build(&[Obj::plane(), a, c]);
    let (ia, ic) = (b.ids["a"], b.ids["b"]);
    let s = b
        .doc
        .transact("spring", |tx| {
            let s = tx.create(forge_dynamics::SPRING, None, None)?;
            tx.set_param(s, "object_a", Value::Link(Some(ia)))?;
            tx.set_param(s, "object_b", Value::Link(Some(ic)))?;
            tx.set_param(s, "rest_length", Value::Float(100.0))?;
            Ok(s)
        })
        .unwrap();
    let sim = simulate(&b, 8);
    let (pa, pc) = (sim[&ia][8].w_axis, sim[&ic][8].w_axis);
    // Both fell by the same amount and approached each other symmetrically.
    assert!(((pa.y - pc.y).abs()) < 0.01, "{} {}", pa.y, pc.y);
    assert!((pa.x + pc.x).abs() < 0.01, "{} {}", pa.x, pc.x);
    assert!(
        pc.x - pa.x < 200.0 - 1.0,
        "pulled together: {}",
        pc.x - pa.x
    );
    // Stiffness 0: the cubes just fall.
    set(&mut b.doc, s, "stiffness", Value::Float(0.0));
    let sim = simulate(&b, 8);
    assert!((sim[&ic][8].w_axis.x - 100.0).abs() < 1e-6);
}
